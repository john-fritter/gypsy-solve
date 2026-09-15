//! Command line front end for the Gypsy engine: deal a seed, list the legal
//! moves, replay a move list.

use std::fs;
use std::io::{self, Read, Write};
use std::process::ExitCode;

mod batch;

use clap::{Args, Parser, Subcommand};
use gypsy_core::{parse_move_list, Move, MoveOptions, State};
use gypsy_solver::{solve, Config, Gypsy, Limit, Report, Verdict};
use klondike::{Klondike, Position};

#[derive(Parser)]
#[command(name = "gypsy", version, about = "Two-deck Gypsy solitaire engine")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Print the opening position for a seed.
    Deal(DealArgs),
    /// Print the legal moves in a position.
    Moves(MovesArgs),
    /// Apply a move list to a seeded deal.
    Replay(ReplayArgs),
    /// Search a seeded deal for a winning line.
    Solve(SolveArgs),
    /// Solve Klondike deals, to validate the solver against a known figure.
    Klondike(KlondikeArgs),
    /// Solve many deals in parallel, resumably, writing one record each.
    Batch(batch::BatchArgs),
}

#[derive(Args)]
struct DealArgs {
    /// Deal number. The same seed always gives the same deal.
    #[arg(long)]
    seed: u64,
}

#[derive(Args)]
struct MovesArgs {
    #[arg(long)]
    seed: u64,
    /// Moves to apply before listing, e.g. "S,T0>T1:2".
    #[arg(long, value_name = "LIST")]
    moves: Option<String>,
    /// Read the moves to apply from a file, or `-` for stdin.
    #[arg(long, value_name = "PATH", conflicts_with = "moves")]
    moves_file: Option<String>,
    /// Leave out foundation-to-tableau moves.
    #[arg(long)]
    no_worry_back: bool,
}

#[derive(Args)]
struct ReplayArgs {
    #[arg(long)]
    seed: u64,
    /// Moves to apply, e.g. "S,T0>T1:2".
    #[arg(long, value_name = "LIST")]
    moves: Option<String>,
    /// Read moves from a file, or `-` for stdin.
    #[arg(long, value_name = "PATH", conflicts_with = "moves")]
    moves_file: Option<String>,
    /// Print the position after every move, not just at the end.
    #[arg(long)]
    step: bool,
}

#[derive(Args)]
struct SolveArgs {
    #[arg(long)]
    seed: u64,
    /// States to expand before giving up. The limit is a node count, not a
    /// clock, so the verdict is the same on any machine.
    #[arg(long, default_value_t = 10_000_000)]
    budget: u64,
    /// Stack guard: longest line the search may build. Not a search
    /// parameter — no position is re-expanded because of it.
    #[arg(long, default_value_t = 100_000)]
    max_depth: u32,
    /// Transposition table size in MiB.
    #[arg(long, default_value_t = 256)]
    table_mib: usize,
    /// Solve the restricted game, with no foundation-to-tableau moves.
    #[arg(long)]
    no_worry_back: bool,
    /// Write the winning line here, for `gypsy replay --moves-file`.
    #[arg(long, value_name = "PATH")]
    trace: Option<String>,
    /// One JSON object instead of a human-readable report.
    #[arg(long)]
    json: bool,
}

#[derive(Args)]
struct KlondikeArgs {
    /// First deal number.
    #[arg(long, default_value_t = 0)]
    seed: u64,
    /// How many consecutive deals to solve.
    #[arg(long, default_value_t = 1)]
    deals: u64,
    #[arg(long, default_value_t = 10_000_000)]
    budget: u64,
    /// Stack guard, not a search parameter.
    #[arg(long, default_value_t = 100_000)]
    max_depth: u32,
    #[arg(long, default_value_t = 256)]
    table_mib: usize,
    /// Solve the restricted game, with no foundation-to-tableau moves.
    ///
    /// The published 81.945% is the worry-back figure, so this arm validates
    /// against nothing. It exists to exercise dominances that are only
    /// provable with worry-back off, which never fire in the full game.
    #[arg(long)]
    no_worry_back: bool,
    /// One JSON object per deal instead of a human-readable line.
    #[arg(long)]
    json: bool,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    let stdout = io::stdout();
    let mut out = stdout.lock();

    match cli.command {
        Command::Deal(args) => {
            writeln!(out, "seed {}", args.seed)?;
            write!(out, "{}", State::deal(args.seed))?;
        }
        Command::Moves(args) => {
            let moves = read_moves(args.moves.as_deref(), args.moves_file.as_deref())?;
            let mut state = State::deal(args.seed);
            apply_all(&mut state, &moves, false, &mut out)?;

            let options = if args.no_worry_back {
                MoveOptions::NO_WORRY_BACK
            } else {
                MoveOptions::ALL
            };
            let legal = state.legal_moves(options);
            writeln!(out, "{} legal move{}", legal.len(), plural(legal.len()))?;
            for mv in legal {
                writeln!(out, "  {mv}")?;
            }
        }
        Command::Replay(args) => {
            let moves = read_moves(args.moves.as_deref(), args.moves_file.as_deref())?;
            let mut state = State::deal(args.seed);
            writeln!(out, "seed {}", args.seed)?;
            if args.step {
                writeln!(out, "\n-- start")?;
                write!(out, "{state}")?;
            }
            apply_all(&mut state, &moves, args.step, &mut out)?;
            if !args.step {
                write!(out, "{state}")?;
            }
            writeln!(
                out,
                "\n{} move{} applied, {}",
                moves.len(),
                plural(moves.len()),
                if state.is_won() { "won" } else { "not won" }
            )?;
        }
        Command::Klondike(args) => {
            let config = Config {
                node_budget: args.budget,
                max_depth: args.max_depth,
                table_entries: gypsy_solver::Table::entries_in(args.table_mib << 20),
            };
            let game = Klondike::new(if args.no_worry_back {
                klondike::MoveOptions::NO_WORRY_BACK
            } else {
                klondike::MoveOptions::ALL
            });

            for seed in args.seed..args.seed + args.deals {
                let report = solve(&game, &Position::deal(seed), config)?;
                if args.json {
                    writeln!(
                        out,
                        concat!(
                            r#"{{"game":"klondike","seed":{},"ruleset":"{}","#,
                            r#""verdict":"{}","limit":"{}","#,
                            r#""nodes":{},"line_length":{},"elapsed_ms":{},"line":{}}}"#
                        ),
                        seed,
                        ruleset_name(args.no_worry_back),
                        verdict_name(report.verdict),
                        limit_name(report.limit),
                        report.nodes,
                        report.line.as_ref().map_or(0, |line| line.len()),
                        report.elapsed.as_millis(),
                        line_json(report.line.as_deref()),
                    )?;
                } else {
                    writeln!(
                        out,
                        "seed {seed:6}  {:13}  {:11}  nodes {:9}  {:6.2}s",
                        ruleset_name(args.no_worry_back),
                        verdict_name(report.verdict),
                        report.nodes,
                        report.elapsed.as_secs_f64()
                    )?;
                }
                // Long runs are piped into a collector, so do not make it wait.
                out.flush()?;
            }
        }
        Command::Batch(args) => batch::run(args)?,
        Command::Solve(args) => {
            let config = Config {
                node_budget: args.budget,
                max_depth: args.max_depth,
                table_entries: gypsy_solver::Table::entries_in(args.table_mib << 20),
            };
            let game = Gypsy::new(if args.no_worry_back {
                MoveOptions::NO_WORRY_BACK
            } else {
                MoveOptions::ALL
            });

            let state = State::deal(args.seed);
            let report = solve(&game, &state, config)?;

            if let (Some(path), Some(line)) = (&args.trace, &report.line) {
                let text: Vec<String> = line.iter().map(|mv| mv.to_string()).collect();
                fs::write(path, format!("{}\n", text.join("\n")))?;
            }

            if args.json {
                writeln!(out, "{}", json_report(args.seed, &args, &report))?;
            } else {
                write_report(&mut out, args.seed, &args, &report)?;
            }
        }
    }

    Ok(())
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

fn ruleset_name(no_worry_back: bool) -> &'static str {
    if no_worry_back {
        "no-worry-back"
    } else {
        "full"
    }
}

fn write_report(
    out: &mut impl Write,
    seed: u64,
    args: &SolveArgs,
    report: &Report<Move>,
) -> io::Result<()> {
    writeln!(
        out,
        "seed {seed}  ruleset {}",
        ruleset_name(args.no_worry_back)
    )?;
    writeln!(out, "verdict {}", verdict_name(report.verdict))?;
    if report.verdict == Verdict::Unknown {
        // The distinction the whole three-valued result exists to keep.
        writeln!(
            out,
            "  stopped at the {} limit; this is not a proof of anything",
            limit_name(report.limit)
        )?;
    }
    writeln!(
        out,
        "nodes {}  table {}/{}  {:.2}s",
        report.nodes,
        report.table_filled,
        report.table_capacity,
        report.elapsed.as_secs_f64()
    )?;
    if let Some(line) = &report.line {
        writeln!(out, "line {} moves", line.len())?;
        let text: Vec<String> = line.iter().map(|mv| mv.to_string()).collect();
        writeln!(out, "{}", text.join(" "))?;
    }
    Ok(())
}

/// The winning line as a JSON value: the move list as a string, or `null`.
///
/// A verdict without the line behind it cannot be audited, and counting what a
/// winning line actually uses is a measurement in its own right.
fn line_json<A: std::fmt::Display>(line: Option<&[A]>) -> String {
    match line {
        Some(line) => format!(
            "\"{}\"",
            line.iter()
                .map(|action| action.to_string())
                .collect::<Vec<_>>()
                .join(" ")
        ),
        None => "null".to_string(),
    }
}

/// A flat JSON object, one per deal, for the batch runner to collect.
fn json_report(seed: u64, args: &SolveArgs, report: &Report<Move>) -> String {
    let line = line_json(report.line.as_deref());
    format!(
        concat!(
            r#"{{"game":"gypsy","seed":{},"ruleset":"{}","verdict":"{}","limit":"{}","nodes":{},"#,
            r#""line_length":{},"elapsed_ms":{},"node_budget":{},"max_depth":{},"#,
            r#""table_capacity":{},"table_filled":{},"line":{}}}"#
        ),
        seed,
        ruleset_name(args.no_worry_back),
        verdict_name(report.verdict),
        limit_name(report.limit),
        report.nodes,
        report.line.as_ref().map_or(0, |line| line.len()),
        report.elapsed.as_millis(),
        args.budget,
        args.max_depth,
        report.table_capacity,
        report.table_filled,
        line,
    )
}

fn plural(count: usize) -> &'static str {
    if count == 1 {
        ""
    } else {
        "s"
    }
}

fn read_moves(
    inline: Option<&str>,
    path: Option<&str>,
) -> Result<Vec<Move>, Box<dyn std::error::Error>> {
    let text = match (inline, path) {
        (Some(text), _) => text.to_string(),
        (None, Some("-")) => {
            let mut buffer = String::new();
            io::stdin().read_to_string(&mut buffer)?;
            buffer
        }
        (None, Some(path)) => fs::read_to_string(path)?,
        (None, None) => String::new(),
    };
    Ok(parse_move_list(&text)?)
}

fn apply_all(
    state: &mut State,
    moves: &[Move],
    step: bool,
    out: &mut impl Write,
) -> Result<(), Box<dyn std::error::Error>> {
    for (index, &mv) in moves.iter().enumerate() {
        state
            .apply(mv)
            .map_err(|error| format!("move {} ({mv}): {error}", index + 1))?;
        if step {
            writeln!(out, "\n-- {} {mv}", index + 1)?;
            write!(out, "{state}")?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A missing line is JSON `null`, not an empty string: a record with no
    /// line is a deal that was never solved, and the two must not read alike
    /// to anything counting what winning lines contain.
    #[test]
    fn a_line_is_null_when_there_is_no_win() {
        assert_eq!(line_json::<Move>(None), "null");
    }

    #[test]
    fn a_line_is_the_move_list_as_one_string() {
        let line = [
            Move::Stock,
            Move::WorryBack {
                foundation: 7,
                to: 4,
            },
        ];
        assert_eq!(line_json(Some(&line[..])), r#""S F7>T4""#);
    }
}
