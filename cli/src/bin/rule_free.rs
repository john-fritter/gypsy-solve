//! Solve a whole deal with every dominance removed.
//!
//! A refutation found by the solver trusts its pruning rules: a rule that
//! discards a winning line makes a winnable deal look lost. Re-solving the deal
//! in [`RulesOnly`] removes that dependency. Every rules-legal move is offered,
//! so an `unsolvable` here stands on the rules and the transposition key alone.
//!
//! It is expensive. Seed 188's full game took 1.08 billion nodes this way,
//! against 19.8 million with the dominances, so give it a large table and a
//! large budget. A full table does not make a verdict wrong: a displaced
//! position is expanded again, never skipped.
//!
//! One JSON record is printed, in the shape `gypsy batch` writes, with
//! `"dominances":"none"` added so it can't be mistaken for a solver run.
//!
//! Usage: `rule_free --seed 3966 [--arm full|no-worry-back] [--budget N] [--table-mib N]`

use std::process::ExitCode;

use gypsy_core::{MoveOptions, State};
use gypsy_solver::{solve, Config, Limit, RulesOnly, Table, Verdict};

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let mut seed: Option<u64> = None;
    let mut options = MoveOptions::ALL;
    let mut config = Config {
        node_budget: 10_000_000_000,
        table_entries: Table::entries_in(8192 << 20),
        ..Config::default()
    };
    while let Some(arg) = args.next() {
        let value = args.next().unwrap_or_default();
        let number = || {
            value
                .parse::<u64>()
                .map_err(|_| format!("{arg} wants a number"))
        };
        let parsed = match arg.as_str() {
            "--seed" => number().map(|n| seed = Some(n)),
            "--budget" => number().map(|n| config.node_budget = n),
            "--table-mib" => {
                number().map(|n| config.table_entries = Table::entries_in((n as usize) << 20))
            }
            "--arm" => match value.as_str() {
                "full" => Ok(options = MoveOptions::ALL),
                "no-worry-back" => Ok(options = MoveOptions::NO_WORRY_BACK),
                other => Err(format!("--arm wants full or no-worry-back, got {other:?}")),
            },
            other => Err(format!("unknown argument {other:?}")),
        };
        if let Err(message) = parsed {
            eprintln!("{message}");
            return ExitCode::FAILURE;
        }
    }
    let Some(seed) = seed else {
        eprintln!(
            "usage: rule_free --seed N [--arm full|no-worry-back] [--budget N] [--table-mib N]"
        );
        return ExitCode::FAILURE;
    };

    let report = match solve(&RulesOnly::new(options), &State::deal(seed), config) {
        Ok(report) => report,
        Err(bug) => panic!("seed {seed}: {bug}"),
    };
    let verdict = match report.verdict {
        Verdict::Solvable => "solvable",
        Verdict::Unsolvable => "unsolvable",
        Verdict::Unknown => "unknown",
    };
    let limit = match report.limit {
        None => "none",
        Some(Limit::Budget) => "budget",
        Some(Limit::Depth) => "depth",
    };
    let ruleset = if options.worry_back {
        "full"
    } else {
        "no-worry-back"
    };
    println!(
        r#"{{"game":"gypsy","seed":{seed},"ruleset":"{ruleset}","dominances":"none","verdict":"{verdict}","verdict_from":"search","limit":"{limit}","nodes":{},"line_length":{},"elapsed_ms":{},"node_budget":{},"max_depth":{},"table_capacity":{},"table_filled":{},"restarts_used":1}}"#,
        report.nodes,
        report.line.as_ref().map_or(0, Vec::len),
        report.elapsed.as_millis(),
        config.node_budget,
        config.max_depth,
        report.table_capacity,
        report.table_filled,
    );
    ExitCode::SUCCESS
}
