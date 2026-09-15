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

use clap::{Args, ValueEnum};
use gypsy_core::{MoveOptions, State};
use gypsy_solver::{solve, Config, Game, Gypsy, Limit, Table, Verdict};
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
    #[arg(long)]
    no_worry_back: bool,
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

/// Seeds already recorded, so a resumed run does not redo them.
///
/// Scanned textually rather than parsed: a run killed mid-write can leave a
/// torn final line, and a torn line should cost its one deal rather than the
/// whole file.
fn recorded_seeds(path: &Path) -> io::Result<HashSet<u64>> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(HashSet::new()),
        Err(error) => return Err(error),
    };
    let mut seeds = HashSet::new();
    for line in BufReader::new(file).lines() {
        let line = line?;
        if !line.trim_end().ends_with('}') {
            continue;
        }
        if let Some(seed) = seed_of(&line) {
            seeds.insert(seed);
        }
    }
    Ok(seeds)
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

fn seed_of(line: &str) -> Option<u64> {
    let rest = line.split("\"seed\":").nth(1)?;
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
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
        recorded_seeds(&args.out)?
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
    };
    let sink = Mutex::new(BufWriter::new(
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&args.out)?,
    ));

    // Both games take the same restriction, and it means the same thing in
    // both: foundation-to-tableau moves are not generated. Only Klondike's
    // full arm validates against the published 81.945%, but the restricted
    // arm is the only deal set that exercises a worry-back-off dominance.
    let ruleset = if args.no_worry_back {
        "no-worry-back"
    } else {
        "full"
    };

    match args.game {
        BatchGame::Gypsy => drive(
            &Gypsy::new(if args.no_worry_back {
                MoveOptions::NO_WORRY_BACK
            } else {
                MoveOptions::ALL
            }),
            State::deal,
            "gypsy",
            ruleset,
            &todo,
            config,
            &args,
            &sink,
        ),
        BatchGame::Klondike => drive(
            &Klondike::new(if args.no_worry_back {
                klondike::MoveOptions::NO_WORRY_BACK
            } else {
                klondike::MoveOptions::ALL
            }),
            Position::deal,
            "klondike",
            ruleset,
            &todo,
            config,
            &args,
            &sink,
        ),
    }?;

    sink.lock().expect("no panic held the sink").flush()?;
    Ok(())
}

/// Solves every seed in `todo`, appending one record each as it lands.
///
/// Generic over the game for the same reason the solver is: the runner that
/// publishes the Gypsy numbers has to be the runner Klondike validated.
#[allow(clippy::too_many_arguments)]
fn drive<G, D>(
    game: &G,
    deal: D,
    game_name: &str,
    ruleset: &str,
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

            let report = match solve(game, &deal(seed), config) {
                Ok(report) => report,
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

            let line = match (&report.line, args.lines) {
                (Some(line), true) => format!(
                    ",\"line\":\"{}\"",
                    line.iter()
                        .map(|action| action.to_string())
                        .collect::<Vec<_>>()
                        .join(" ")
                ),
                _ => String::new(),
            };
            let record = format!(
                concat!(
                    r#"{{"game":"{}","seed":{},"ruleset":"{}","verdict":"{}","limit":"{}","#,
                    r#""nodes":{},"line_length":{},"elapsed_ms":{},"node_budget":{},"#,
                    r#""max_depth":{},"table_capacity":{},"table_filled":{}{}}}"#,
                    "\n"
                ),
                game_name,
                seed,
                ruleset,
                verdict_name(report.verdict),
                limit_name(report.limit),
                report.nodes,
                report.line.as_ref().map_or(0, |line| line.len()),
                report.elapsed.as_millis(),
                config.node_budget,
                config.max_depth,
                report.table_capacity,
                report.table_filled,
                line,
            );

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
            eprintln!(
                "  {done}/{} seed {seed} {} nodes {}",
                todo.len(),
                verdict_name(report.verdict),
                report.nodes
            );

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
        assert_eq!(recorded_seeds(&path).expect("scan"), HashSet::from([0]));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn a_whole_file_is_left_alone() {
        let path = scratch("whole");
        std::fs::write(&path, "{\"seed\":0}\n{\"seed\":1}\n").expect("write");
        assert_eq!(trim_partial_record(&path).expect("trim"), 0);
        assert_eq!(recorded_seeds(&path).expect("scan"), HashSet::from([0, 1]));
        std::fs::remove_file(&path).ok();
    }

    /// A worker costs its table and its stack, and the stack is not small.
    #[test]
    fn a_worker_is_charged_for_its_stack_as_well_as_its_table() {
        assert_eq!(worker_mib(1024, 100_000), 1024 + 391);
        assert!(worker_mib(256, 100_000) > 256);
    }
}
