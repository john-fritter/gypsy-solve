//! The batch runner: many seeded deals, bounded concurrency, resumable.
//!
//! Batch solving is the compute-heavy core of this project — thousands of
//! deals, hours to days of wall clock, on a box that has to stay up for other
//! things. That shapes everything here:
//!
//! - **Records are written as each deal finishes**, not at the end, so a run
//!   that is killed keeps everything it had already proved.
//! - **`--resume` skips what is already recorded**, so the answer to a kill is
//!   to run the same command again.
//! - **The run refuses to start when it would not fit**, in memory or on disk,
//!   rather than being killed hours in. Both guards exist because both
//!   failures have already happened here.
//!
//! One JSON object per line, each carrying the configuration that produced it,
//! so a results file can be summarised without knowing how it was made. Rows
//! arrive in completion order rather than seed order; anything that cares
//! sorts.

use std::collections::HashSet;
use std::ffi::CString;
use std::fmt::Display;
use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::FileExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use clap::{Args, ValueEnum};
use gypsy_core::{MoveOptions, State};
use gypsy_solver::{
    replay, solve_restarting, Config, Game, Gypsy, Limit, Report, Table, UnverifiedSolution,
    Verdict,
};
use klondike::{Klondike, Position};

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
pub enum BatchGame {
    Gypsy,
    Klondike,
}

#[derive(Args)]
pub struct BatchArgs {
    /// Which game to solve. Klondike is the validation set, not a deliverable.
    #[arg(long, value_enum, default_value_t = BatchGame::Gypsy)]
    game: BatchGame,
    /// First deal number.
    #[arg(long, default_value_t = 0)]
    seed: u64,
    /// How many consecutive deals to solve.
    #[arg(long)]
    deals: u64,
    /// States to expand per deal before giving up.
    #[arg(long, default_value_t = 10_000_000)]
    budget: u64,
    /// Stack guard, and a memory knob: a worker's stack can reach roughly
    /// four KiB per unit of depth.
    #[arg(long, default_value_t = 100_000)]
    max_depth: u32,
    /// Transposition table size per worker, in MiB.
    #[arg(long, default_value_t = 256)]
    table_mib: usize,
    /// Deals solved at once. Each worker holds its own table and its own
    /// stack, so this multiplies memory.
    #[arg(long, default_value_t = 1)]
    workers: usize,
    /// Where per-deal records are appended, one JSON object per line.
    #[arg(long, value_name = "PATH")]
    out: PathBuf,
    /// Skip deals already recorded in the output file.
    #[arg(long)]
    resume: bool,
    /// Solve the restricted game, with no foundation-to-tableau moves.
    ///
    /// Applies to both games. For Klondike it does not validate against the
    /// published figure, which is the worry-back one; it is the deal set that
    /// exercises a dominance only provable with worry-back off.
    #[arg(long, conflicts_with = "both_arms")]
    no_worry_back: bool,
    /// Solve each deal in both rulesets, carrying each arm's proof to the
    /// other wherever that is sound. Two records per deal.
    ///
    /// This is the shape of the experiment `DESIGN.md` describes — every deal
    /// solved twice, worry-back off and then on — and solving them together is
    /// what makes the carry possible. It is strictly cheaper than two separate
    /// runs and never weaker: see `solve_both`.
    #[arg(long)]
    both_arms: bool,
    /// Searches per deal, splitting the budget between them and reordering
    /// the moves each time.
    ///
    /// The total work per deal is unchanged; what changes is that a deal gets
    /// several chances at a lucky descent instead of one. A restart can only
    /// turn `unknown` into a decision — see `gypsy_solver::solve_restarting`.
    #[arg(long, default_value_t = 1)]
    restarts: u32,
    /// Record the winning line in full, not just its length.
    #[arg(long)]
    lines: bool,
    /// Stop cleanly rather than fill the disk past this much free space.
    #[arg(long, default_value_t = 512)]
    min_free_mib: u64,
}

/// Memory one worker needs, in MiB: its table plus its search stack.
///
/// The stack term is not decoration. Before the table was made to probe
/// (2026-09-15) the search thrashed instead of descending and peak memory was
/// about the table size; now it descends, and a 12M-node Klondike solve ran
/// 413 MiB over its table. The term is bounded by `max_depth` because that is
/// what bounds the stack, at roughly four KiB a frame.
fn worker_mib(table_mib: usize, max_depth: u32) -> u64 {
    let table = (table_mib as u64).next_power_of_two().max(1);
    table + (max_depth as u64 * 4).div_ceil(1024)
}

/// What the kernel thinks can be handed out without swapping.
fn available_mib() -> Option<u64> {
    let text = std::fs::read_to_string("/proc/meminfo").ok()?;
    let line = text
        .lines()
        .find(|line| line.starts_with("MemAvailable:"))?;
    let kib: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
    Some(kib / 1024)
}

/// Free space on the filesystem holding `path`.
fn free_disk_mib(path: &Path) -> io::Result<u64> {
    let parent = path.parent().unwrap_or(Path::new("."));
    let dir = if parent.as_os_str().is_empty() {
        Path::new(".")
    } else {
        parent
    };
    let raw = CString::new(dir.as_os_str().as_bytes())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "path contains a NUL"))?;
    let mut stat: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statvfs(raw.as_ptr(), &mut stat) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(stat.f_bavail.saturating_mul(stat.f_frsize) >> 20)
}

/// Seeds already recorded for every ruleset in `wanted`, so a resumed run does
/// not redo them.
///
/// Scanned textually rather than parsed: a run killed mid-write can leave a
/// torn final line, and a torn line should cost its one deal rather than the
/// whole file.
///
/// `wanted` is what makes `--both-arms` safe to resume. That mode writes two
/// records per deal, and a kill between them leaves a seed with one. Counting
/// such a seed as done would drop an arm silently, so a seed is done only when
/// every ruleset asked for is present.
fn recorded_seeds(path: &Path, wanted: &[&str]) -> io::Result<HashSet<u64>> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(HashSet::new()),
        Err(error) => return Err(error),
    };
    let mut seen: std::collections::HashMap<u64, Vec<String>> = std::collections::HashMap::new();
    for line in BufReader::new(file).lines() {
        let line = line?;
        if !line.trim_end().ends_with('}') {
            continue;
        }
        if let Some(seed) = seed_of(&line) {
            let ruleset = field_of(&line, "ruleset").unwrap_or_else(|| "full".to_string());
            seen.entry(seed).or_default().push(ruleset);
        }
    }
    Ok(seen
        .into_iter()
        .filter(|(_, rulesets)| {
            wanted
                .iter()
                .all(|want| rulesets.iter().any(|have| have == want))
        })
        .map(|(seed, _)| seed)
        .collect())
}

/// Drops a trailing partial record, returning how many bytes went.
///
/// A run killed mid-write leaves a record with no newline after it. Appending
/// to that file would run the next record onto the end of the broken one,
/// destroying a *complete* result as well as the torn one — and since the
/// resume scan had already counted the good record as done, its deal would be
/// silently missing from the batch. Truncating back to the last whole line
/// costs the one deal that was genuinely interrupted.
fn trim_partial_record(path: &Path) -> io::Result<u64> {
    let file = match OpenOptions::new().read(true).write(true).open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(error),
    };
    let length = file.metadata()?.len();
    if length == 0 {
        return Ok(0);
    }

    const WINDOW: u64 = 64 * 1024;
    let mut end = length;
    let mut buffer = vec![0u8; WINDOW as usize];
    while end > 0 {
        let start = end.saturating_sub(WINDOW);
        let span = (end - start) as usize;
        file.read_exact_at(&mut buffer[..span], start)?;
        if let Some(offset) = buffer[..span].iter().rposition(|&byte| byte == b'\n') {
            let keep = start + offset as u64 + 1;
            if keep == length {
                return Ok(0);
            }
            file.set_len(keep)?;
            return Ok(length - keep);
        }
        end = start;
    }

    // Not one complete line in the file.
    file.set_len(0)?;
    Ok(length)
}

/// Drops a trailing deal that is recorded for only some of `wanted`.
///
/// Both arms of a deal go down in one write, so the file is a sequence of
/// whole per-deal blocks — except possibly the last, if a kill tore it. Once
/// `trim_partial_record` has removed the torn bytes, what can be left is a
/// *complete* record whose partner never made it. Counting that seed as done
/// would drop an arm silently; leaving it in place would double-count that arm
/// when the seed is solved again on resume. Either way the results file lies,
/// so the orphan goes.
///
/// Only the tail can be incomplete, which is why this truncates rather than
/// rewrites. A single-ruleset run can never have an orphan and this is a no-op
/// for it.
fn trim_incomplete_tail(path: &Path, wanted: &[&str]) -> io::Result<u64> {
    if wanted.len() < 2 {
        return Ok(0);
    }
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(error),
    };

    let mut offset = 0usize;
    let mut lines: Vec<(usize, &str)> = Vec::new();
    for line in text.split_inclusive('\n') {
        lines.push((offset, line));
        offset += line.len();
    }

    let Some(last_seed) = lines.last().and_then(|(_, line)| seed_of(line)) else {
        return Ok(0);
    };
    let first = lines
        .iter()
        .rposition(|(_, line)| seed_of(line) != Some(last_seed))
        .map_or(0, |at| at + 1);

    let present: Vec<String> = lines[first..]
        .iter()
        .filter_map(|(_, line)| field_of(line, "ruleset"))
        .collect();
    if wanted
        .iter()
        .all(|want| present.iter().any(|have| have == want))
    {
        return Ok(0);
    }

    let keep = lines[first].0 as u64;
    let length = text.len() as u64;
    OpenOptions::new().write(true).open(path)?.set_len(keep)?;
    Ok(length - keep)
}

fn seed_of(line: &str) -> Option<u64> {
    let rest = line.split("\"seed\":").nth(1)?;
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

/// A string field out of a record, read the same textual way as `seed_of`.
///
/// Records written before a field existed simply do not have it, and the
/// caller decides what that means rather than this failing.
fn field_of(line: &str, name: &str) -> Option<String> {
    let rest = line.split(&format!("\"{name}\":\"")).nth(1)?;
    Some(rest.chars().take_while(|&c| c != '"').collect())
}

fn verdict_name(verdict: Verdict) -> &'static str {
    match verdict {
        Verdict::Solvable => "solvable",
        Verdict::Unsolvable => "unsolvable",
        Verdict::Unknown => "unknown",
    }
}

fn limit_name(limit: Option<Limit>) -> &'static str {
    match limit {
        Some(Limit::Budget) => "budget",
        Some(Limit::Depth) => "depth",
        None => "none",
    }
}

/// One arm's answer for one deal, and where the answer came from.
struct Answer<A> {
    report: Report<A>,
    /// `"search"`, or the ruleset whose proof was carried into this one.
    from: &'static str,
}

impl<A> Answer<A> {
    fn searched(report: Report<A>) -> Answer<A> {
        Answer {
            report,
            from: "search",
        }
    }

    /// A verdict established by the other arm. It cost no nodes and used no
    /// table, and the record says so rather than borrowing the other arm's
    /// numbers.
    fn carried(verdict: Verdict, line: Option<Vec<A>>, from: &'static str) -> Answer<A> {
        Answer {
            report: Report {
                verdict,
                line,
                nodes: 0,
                elapsed: Duration::ZERO,
                table_capacity: 0,
                table_filled: 0,
                limit: None,
                restarts_used: 0,
            },
            from,
        }
    }
}

/// The rulesets a run solves: one arm, or both with proofs carried across.
#[derive(Clone, Copy)]
enum Arms<'a, G> {
    One {
        game: &'a G,
        /// Exactly one name, from `wanted_rulesets`, so that what a resumed
        /// run counts as done and what a run writes cannot drift apart.
        rulesets: &'static [&'static str],
    },
    Both {
        restricted: &'a G,
        full: &'a G,
    },
}

impl<G> Arms<'_, G> {
    /// The rulesets a record is written for, in the order they are written.
    fn rulesets(&self) -> &'static [&'static str] {
        match self {
            Arms::One { rulesets, .. } => rulesets,
            Arms::Both { .. } => BOTH_ARMS,
        }
    }
}

/// The two arms, in the order `solve_both` returns them.
const BOTH_ARMS: &[&str] = &["no-worry-back", "full"];

/// The rulesets this run will write, so a resumed run knows what "done" means.
fn wanted_rulesets(args: &BatchArgs) -> &'static [&'static str] {
    if args.both_arms {
        BOTH_ARMS
    } else if args.no_worry_back {
        &["no-worry-back"]
    } else {
        &["full"]
    }
}

/// Which arms this run solves.
///
/// Both games take the same restriction and it means the same thing in both:
/// foundation-to-tableau moves are not generated. Only Klondike's full arm
/// validates against the published 81.945%, but the restricted arm is the only
/// deal set that exercises a worry-back-off dominance.
fn arms_of<'a, G>(args: &BatchArgs, restricted: &'a G, full: &'a G) -> Arms<'a, G> {
    if args.both_arms {
        Arms::Both { restricted, full }
    } else if args.no_worry_back {
        Arms::One {
            game: restricted,
            rulesets: wanted_rulesets(args),
        }
    } else {
        Arms::One {
            game: full,
            rulesets: wanted_rulesets(args),
        }
    }
}

/// Both arms' answers for one deal, restricted first.
type BothAnswers<G> = (Answer<<G as Game>::Action>, Answer<<G as Game>::Action>);

/// Solves one deal in both arms, carrying each arm's proof to the other
/// wherever that is sound.
///
/// The restricted game generates a subset of the full game's moves and differs
/// in nothing else, so two implications hold:
///
/// - **A restricted win is a full win.** Every move of the line is legal in
///   the full game, so the line replays there move for move. It *is* replayed
///   rather than assumed — a carried claim is still a claim, and this project
///   does not take a claimed win on trust.
/// - **A full refutation is a restricted refutation.** An exhausted full
///   search visited every position the restricted game could have reached, so
///   if there is no win among them there is none among the subset either.
///
/// Neither carries the other way, and the asymmetry is the whole point. A full
/// win may have used worry-back, which the restricted game cannot do. A
/// restricted refutation says nothing about a game with strictly more moves in
/// it — that is exactly the worry-back delta this project exists to measure.
///
/// Of the two, only the first is expected to pay. Refuting the full game means
/// exhausting a strictly larger graph, so an arm that can do that at a given
/// budget can almost always refute the restricted game directly — the second
/// carry is kept because it is sound and free, not because it is expected to
/// fire often.
///
/// The restricted arm goes first because it is the cheaper one: it has safe
/// autoplay and a smaller branching factor, so the deals it wins cost the full
/// arm nothing at all. Measured on 50 Gypsy deals at 5M, that is the
/// difference between the full arm resolving 0 and resolving 12.
///
/// Only one search runs at a time, so a worker still costs one table.
fn solve_both<G: Game>(
    restricted: &G,
    full: &G,
    start: &G::Position,
    config: Config,
    restarts: u32,
) -> Result<BothAnswers<G>, UnverifiedSolution<G::Action>> {
    let restricted_report = solve_restarting(restricted, start, config, restarts)?;

    if restricted_report.verdict == Verdict::Solvable {
        let line = restricted_report
            .line
            .clone()
            .expect("a solvable report carries its line");
        if let Err(failure) = replay(full, start, &line) {
            return Err(UnverifiedSolution { line, failure });
        }
        return Ok((
            Answer::searched(restricted_report),
            Answer::carried(Verdict::Solvable, Some(line), "no-worry-back"),
        ));
    }

    let full_report = solve_restarting(full, start, config, restarts)?;

    // Only an `Unknown` is worth upgrading; a restricted arm that refuted the
    // deal itself already has the stronger result of its own.
    if full_report.verdict == Verdict::Unsolvable && restricted_report.verdict == Verdict::Unknown {
        return Ok((
            Answer::carried(Verdict::Unsolvable, None, "full"),
            Answer::searched(full_report),
        ));
    }

    Ok((
        Answer::searched(restricted_report),
        Answer::searched(full_report),
    ))
}

pub fn run(args: BatchArgs) -> Result<(), Box<dyn std::error::Error>> {
    if args.deals == 0 {
        return Err("--deals must be at least one".into());
    }
    if args.workers == 0 {
        return Err("--workers must be at least one".into());
    }

    // Refuse rather than be killed hours in. Three workers at four GiB of
    // table each is what the kernel took out on 2026-09-15.
    let needed = worker_mib(args.table_mib, args.max_depth) * args.workers as u64;
    if let Some(available) = available_mib() {
        if needed > available {
            return Err(format!(
                "{} workers need about {needed} MiB ({} MiB of table and {} MiB of stack each) \
                 and only {available} MiB is available; lower --workers or --table-mib",
                args.workers,
                (args.table_mib as u64).next_power_of_two(),
                worker_mib(args.table_mib, args.max_depth)
                    - (args.table_mib as u64).next_power_of_two(),
            )
            .into());
        }
    }

    let free = free_disk_mib(&args.out)?;
    if free < args.min_free_mib {
        return Err(format!(
            "{free} MiB free where results go, below the {} MiB floor; \
             free space or lower --min-free-mib",
            args.min_free_mib
        )
        .into());
    }

    let already = if args.resume {
        let dropped = trim_partial_record(&args.out)?;
        if dropped > 0 {
            eprintln!("batch: dropped {dropped} bytes of a record left torn by an earlier kill");
        }
        let orphaned = trim_incomplete_tail(&args.out, wanted_rulesets(&args))?;
        if orphaned > 0 {
            eprintln!("batch: dropped {orphaned} bytes of a deal recorded in only one arm");
        }
        recorded_seeds(&args.out, wanted_rulesets(&args))?
    } else {
        if args.out.exists() && args.out.metadata()?.len() > 0 {
            return Err(format!(
                "{} already holds results; pass --resume to add to it, or move it aside",
                args.out.display()
            )
            .into());
        }
        HashSet::new()
    };

    let todo: Vec<u64> = (args.seed..args.seed + args.deals)
        .filter(|seed| !already.contains(seed))
        .collect();

    eprintln!(
        "batch: {} deals, {} already recorded, {} to solve, {} workers, {needed} MiB, {free} MiB free",
        args.deals,
        already.len(),
        todo.len(),
        args.workers,
    );
    if todo.is_empty() {
        return Ok(());
    }

    let config = Config {
        node_budget: args.budget,
        max_depth: args.max_depth,
        table_entries: Table::entries_in(args.table_mib << 20),
        ordering_salt: 0,
    };
    let sink = Mutex::new(BufWriter::new(
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&args.out)?,
    ));

    match args.game {
        BatchGame::Gypsy => {
            let restricted = Gypsy::new(MoveOptions::NO_WORRY_BACK);
            let full = Gypsy::new(MoveOptions::ALL);
            drive(
                arms_of(&args, &restricted, &full),
                State::deal,
                "gypsy",
                &todo,
                config,
                &args,
                &sink,
            )
        }
        BatchGame::Klondike => {
            let restricted = Klondike::new(klondike::MoveOptions::NO_WORRY_BACK);
            let full = Klondike::new(klondike::MoveOptions::ALL);
            drive(
                arms_of(&args, &restricted, &full),
                Position::deal,
                "klondike",
                &todo,
                config,
                &args,
                &sink,
            )
        }
    }?;

    sink.lock().expect("no panic held the sink").flush()?;
    Ok(())
}

/// One record, carrying the configuration that produced it so a results file
/// can be summarised without knowing how it was made.
///
/// `verdict_from` is the field that keeps `--both-arms` honest: a carried
/// verdict cost no nodes and searched nothing, and a reader counting work done
/// or auditing a proof has to be able to tell which arm actually established
/// it.
fn record_of<A: Display>(
    game_name: &str,
    seed: u64,
    ruleset: &str,
    answer: &Answer<A>,
    config: Config,
    lines: bool,
) -> String {
    let report = &answer.report;
    let line = match (&report.line, lines) {
        (Some(line), true) => format!(
            ",\"line\":\"{}\"",
            line.iter()
                .map(|action| action.to_string())
                .collect::<Vec<_>>()
                .join(" ")
        ),
        _ => String::new(),
    };
    format!(
        concat!(
            r#"{{"game":"{}","seed":{},"ruleset":"{}","verdict":"{}","verdict_from":"{}","#,
            r#""limit":"{}","nodes":{},"line_length":{},"elapsed_ms":{},"node_budget":{},"#,
            r#""max_depth":{},"table_capacity":{},"table_filled":{},"restarts_used":{}{}}}"#,
            "\n"
        ),
        game_name,
        seed,
        ruleset,
        verdict_name(report.verdict),
        answer.from,
        limit_name(report.limit),
        report.nodes,
        report.line.as_ref().map_or(0, |line| line.len()),
        report.elapsed.as_millis(),
        config.node_budget,
        config.max_depth,
        report.table_capacity,
        report.table_filled,
        report.restarts_used,
        line,
    )
}

/// Solves every seed in `todo`, appending one record each as it lands.
///
/// Generic over the game for the same reason the solver is: the runner that
/// publishes the Gypsy numbers has to be the runner Klondike validated.
#[allow(clippy::too_many_arguments)]
fn drive<G, D>(
    arms: Arms<'_, G>,
    deal: D,
    game_name: &str,
    todo: &[u64],
    config: Config,
    args: &BatchArgs,
    sink: &Mutex<BufWriter<File>>,
) -> Result<(), Box<dyn std::error::Error>>
where
    G: Game + Sync,
    G::Action: Display,
    D: Fn(u64) -> G::Position + Sync,
{
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(args.workers)
        .build()?;
    let stop: AtomicBool = AtomicBool::new(false);
    let finished = AtomicU64::new(0);
    let failure: Mutex<Option<String>> = Mutex::new(None);

    pool.install(|| {
        use rayon::prelude::*;
        todo.par_iter().for_each(|&seed| {
            if stop.load(Ordering::Relaxed) {
                return;
            }

            let start = deal(seed);
            let answers = match arms {
                Arms::One { game, .. } => solve_restarting(game, &start, config, args.restarts)
                    .map(|report| vec![Answer::searched(report)]),
                Arms::Both { restricted, full } => {
                    solve_both(restricted, full, &start, config, args.restarts)
                        .map(|(restricted, full)| vec![restricted, full])
                }
            };
            let answers = match answers {
                Ok(answers) => answers,
                Err(unverified) => {
                    // A win that does not replay is a bug in the solver, and
                    // the one failure that must never be written to a results
                    // file and summarised as a number.
                    stop.store(true, Ordering::Relaxed);
                    *failure.lock().expect("no panic held the failure") =
                        Some(format!("deal {seed}: {unverified}"));
                    return;
                }
            };

            // Both arms of a deal go down in one write, so a resumed run never
            // sees half a deal. `recorded_seeds` checks for both anyway.
            let record: String = arms
                .rulesets()
                .iter()
                .zip(&answers)
                .map(|(ruleset, answer)| {
                    record_of(game_name, seed, ruleset, answer, config, args.lines)
                })
                .collect();

            {
                let mut sink = sink.lock().expect("no panic held the sink");
                // Flushed per deal: a killed run keeps what it proved.
                if let Err(error) = sink
                    .write_all(record.as_bytes())
                    .and_then(|()| sink.flush())
                {
                    stop.store(true, Ordering::Relaxed);
                    *failure.lock().expect("no panic held the failure") =
                        Some(format!("writing deal {seed}: {error}"));
                    return;
                }
            }

            let done = finished.fetch_add(1, Ordering::Relaxed) + 1;
            let told: Vec<String> = arms
                .rulesets()
                .iter()
                .zip(&answers)
                .map(|(ruleset, answer)| {
                    let verdict = verdict_name(answer.report.verdict);
                    match answer.from {
                        "search" => format!("{ruleset} {verdict} nodes {}", answer.report.nodes),
                        from => format!("{ruleset} {verdict} carried from {from}"),
                    }
                })
                .collect();
            eprintln!("  {done}/{} seed {seed} {}", todo.len(), told.join("; "));

            // Checked as we go, not only at the start: a long run can fill a
            // disk that was comfortable when it began.
            match free_disk_mib(&args.out) {
                Ok(free) if free < args.min_free_mib => {
                    stop.store(true, Ordering::Relaxed);
                    *failure.lock().expect("no panic held the failure") = Some(format!(
                        "stopped with {free} MiB free, below the {} MiB floor; \
                         completed deals are recorded, resume when there is room",
                        args.min_free_mib
                    ));
                }
                Ok(_) => {}
                Err(error) => {
                    stop.store(true, Ordering::Relaxed);
                    *failure.lock().expect("no panic held the failure") =
                        Some(format!("checking free space: {error}"));
                }
            }
        });
    });

    match failure.into_inner().expect("no panic held the failure") {
        Some(reason) => Err(reason.into()),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("gypsy-batch-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_file(&path);
        path
    }

    #[test]
    fn a_seed_is_read_out_of_a_record() {
        assert_eq!(
            seed_of(r#"{"game":"gypsy","seed":417,"verdict":"unknown"}"#),
            Some(417)
        );
        assert_eq!(seed_of(r#"{"game":"gypsy","verdict":"unknown"}"#), None);
    }

    #[test]
    fn a_torn_record_is_dropped_so_the_next_append_is_clean() {
        let path = scratch("torn");
        let mut file = File::create(&path).expect("create");
        writeln!(file, r#"{{"seed":0,"verdict":"solvable"}}"#).expect("write");
        write!(file, r#"{{"seed":1,"verd"#).expect("write");
        drop(file);

        assert!(trim_partial_record(&path).expect("trim") > 0);
        let text = std::fs::read_to_string(&path).expect("read");
        assert!(text.ends_with('\n'), "a whole record and nothing after it");
        assert_eq!(text.lines().count(), 1);
        assert_eq!(
            recorded_seeds(&path, &["full"]).expect("scan"),
            HashSet::from([0])
        );
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn a_whole_file_is_left_alone() {
        let path = scratch("whole");
        std::fs::write(&path, "{\"seed\":0}\n{\"seed\":1}\n").expect("write");
        assert_eq!(trim_partial_record(&path).expect("trim"), 0);
        assert_eq!(
            recorded_seeds(&path, &["full"]).expect("scan"),
            HashSet::from([0, 1])
        );
        std::fs::remove_file(&path).ok();
    }

    /// `--both-arms` writes two records per deal. A kill between them must not
    /// let a resumed run count the deal as done and drop an arm silently.
    #[test]
    fn a_deal_recorded_in_only_one_arm_is_not_done() {
        let path = scratch("one-arm");
        std::fs::write(
            &path,
            concat!(
                r#"{"seed":0,"ruleset":"no-worry-back"}"#,
                "\n",
                r#"{"seed":1,"ruleset":"no-worry-back"}"#,
                "\n",
                r#"{"seed":1,"ruleset":"full"}"#,
                "\n",
            ),
        )
        .expect("write");
        assert_eq!(
            recorded_seeds(&path, BOTH_ARMS).expect("scan"),
            HashSet::from([1]),
            "seed 0 has only its restricted arm, so it is not done"
        );
        std::fs::remove_file(&path).ok();
    }

    /// The orphan case: a kill between the two arms of one deal leaves a
    /// complete record with no partner. Keeping it would double-count that arm
    /// once the deal is solved again.
    #[test]
    fn a_deal_left_in_one_arm_is_dropped_before_resuming() {
        let path = scratch("orphan");
        let whole = concat!(
            r#"{"seed":7,"ruleset":"no-worry-back"}"#,
            "\n",
            r#"{"seed":7,"ruleset":"full"}"#,
            "\n",
        );
        std::fs::write(
            &path,
            format!("{whole}{}", "{\"seed\":8,\"ruleset\":\"no-worry-back\"}\n"),
        )
        .expect("write");

        assert!(trim_incomplete_tail(&path, BOTH_ARMS).expect("trim") > 0);
        assert_eq!(std::fs::read_to_string(&path).expect("read"), whole);
        assert_eq!(
            recorded_seeds(&path, BOTH_ARMS).expect("scan"),
            HashSet::from([7])
        );
        std::fs::remove_file(&path).ok();
    }

    /// A complete file is left alone, and a single-arm run has no orphans to
    /// find in the first place.
    #[test]
    fn a_complete_tail_survives_the_orphan_check() {
        let path = scratch("complete");
        let whole = concat!(
            r#"{"seed":7,"ruleset":"no-worry-back"}"#,
            "\n",
            r#"{"seed":7,"ruleset":"full"}"#,
            "\n",
        );
        std::fs::write(&path, whole).expect("write");
        assert_eq!(trim_incomplete_tail(&path, BOTH_ARMS).expect("trim"), 0);
        assert_eq!(trim_incomplete_tail(&path, &["full"]).expect("trim"), 0);
        assert_eq!(std::fs::read_to_string(&path).expect("read"), whole);
        std::fs::remove_file(&path).ok();
    }

    fn cheap() -> Config {
        Config {
            node_budget: 200_000,
            max_depth: 100_000,
            table_entries: Table::entries_in(16 << 20),
            ordering_salt: 0,
        }
    }

    /// The carry that pays. A deal the restricted arm wins is a full-arm win
    /// for no nodes at all, and the line comes across with it. Seed 21 is the
    /// cheapest such Gypsy deal on record, at 2,167 nodes.
    #[test]
    fn a_restricted_win_is_carried_into_the_full_arm() {
        let restricted = Gypsy::new(MoveOptions::NO_WORRY_BACK);
        let full = Gypsy::new(MoveOptions::ALL);
        let (left, right) = solve_both(&restricted, &full, &State::deal(21), cheap(), 1)
            .expect("the carried line replays in the full game");

        assert_eq!(left.report.verdict, Verdict::Solvable);
        assert_eq!(left.from, "search");
        assert_eq!(right.report.verdict, Verdict::Solvable);
        assert_eq!(right.from, "no-worry-back");
        assert_eq!(right.report.nodes, 0, "the full arm searched nothing");
        assert_eq!(
            right.report.line.as_ref().map(Vec::len),
            left.report.line.as_ref().map(Vec::len),
            "the same line, and it is the one that was replayed"
        );
    }

    /// A carried verdict must be legible as carried: it cost no nodes, and a
    /// reader auditing which arm proved what cannot be left to guess.
    #[test]
    fn a_carried_verdict_says_so_in_its_record() {
        let answer: Answer<gypsy_core::Move> =
            Answer::carried(Verdict::Solvable, None, "no-worry-back");
        let text = record_of("gypsy", 21, "full", &answer, cheap(), false);
        assert!(text.contains(r#""ruleset":"full""#), "{text}");
        assert!(text.contains(r#""verdict_from":"no-worry-back""#), "{text}");
        assert!(text.contains(r#""nodes":0"#), "{text}");
    }

    /// A searched verdict says that too, so the field is never ambiguous.
    #[test]
    fn a_searched_verdict_is_labelled_search() {
        let restricted = Gypsy::new(MoveOptions::NO_WORRY_BACK);
        let report =
            solve_restarting(&restricted, &State::deal(21), cheap(), 1).expect("seed 21 solves");
        let text = record_of(
            "gypsy",
            21,
            "no-worry-back",
            &Answer::searched(report),
            cheap(),
            false,
        );
        assert!(text.contains(r#""verdict_from":"search""#), "{text}");
    }

    /// A worker costs its table and its stack, and the stack is not small.
    #[test]
    fn a_worker_is_charged_for_its_stack_as_well_as_its_table() {
        assert_eq!(worker_mib(1024, 100_000), 1024 + 391);
        assert!(worker_mib(256, 100_000) > 256);
    }
}
