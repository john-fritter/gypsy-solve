//! Command line front end for the Gypsy engine: deal a seed, list the legal
//! moves, replay a move list.

use std::fs;
use std::io::{self, Read, Write};
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};
use gypsy_core::{parse_move_list, Move, MoveOptions, State};

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
    }

    Ok(())
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
