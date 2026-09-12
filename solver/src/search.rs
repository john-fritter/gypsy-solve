//! Depth-first search with a transposition table.
//!
//! Three properties matter more than speed here, and each costs something:
//!
//! - **The verdict is three-valued.** Running out of budget or depth is
//!   `Unknown`, never `Unsolvable`. Only a subtree that was searched to
//!   exhaustion refutes anything.
//! - **The search is deterministic.** Every limit is a node count or a depth,
//!   never a clock, so the same seed and the same config give the same verdict
//!   on any machine. Elapsed time is reported and never consulted.
//! - **A claimed solution is replayed before it is believed.** A search bug
//!   that invents a win is worse than one that misses it, because it is the
//!   kind that gets published.
//!
//! The stack is explicit. A recursive version overflows: the state graph is
//! cyclic, so depth is bounded only by the depth limit, and a limit large
//! enough to hold a real solution is far past what the call stack takes.
//!
//! ## Why a refutation does not carry a depth
//!
//! A frame claims `complete` only when every child was refuted and nothing
//! below it was cut short. A depth cut anywhere underneath clears the flag and
//! the clearing propagates to the root. So a subtree that is still `complete`
//! at the end never reached for depth it did not have, and its refutation
//! holds however much depth a later visit brings. That is what lets the table
//! reuse a refutation at any depth while an abandonment has to carry one.
//!
//! ## Repetitions on the current path
//!
//! Returning to a position already on the stack cannot help: the earlier visit
//! has strictly more depth in hand and is enumerating the same moves. The
//! branch is therefore cut — but the frame is also marked incomplete, and that
//! is deliberately conservative. Whether the repeat is truly refuted depends
//! on how its ancestor resolves, which is not known yet, and recording a
//! refutation that turns out to rest on an unresolved ancestor would let a
//! later search skip a live branch. Giving up the claim costs `Unsolvable`
//! verdicts, which is the safe direction to be wrong in.

use std::collections::HashSet;
use std::time::{Duration, Instant};

use gypsy_core::{Move, MoveOptions, State};

use crate::table::{Probe, Table};
use crate::zobrist::Zobrist;

/// What the search concluded. `Unknown` is a real answer and is never folded
/// into `Unsolvable`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Verdict {
    /// A winning line was found and replayed successfully.
    Solvable,
    /// The whole reachable game was searched. There is no win.
    Unsolvable,
    /// The search stopped at a limit. Nothing is proven either way.
    Unknown,
}

/// Which limit stopped a search.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Limit {
    /// The node budget ran out.
    Budget,
    /// Some branch reached the depth limit, so the search was not exhaustive.
    Depth,
}

/// Search limits. All of them are deterministic by construction.
#[derive(Clone, Copy, Debug)]
pub struct Config {
    /// Which moves the search may use.
    pub options: MoveOptions,
    /// States expanded before giving up.
    pub node_budget: u64,
    /// Longest line the search may build.
    pub max_depth: u32,
    /// Transposition table size, rounded up to a power of two.
    pub table_entries: usize,
}

impl Config {
    /// A winning line plays 104 cards up and deals the stock ten times, so it
    /// cannot be shorter than this. The default depth limit leaves room for
    /// the tableau work on top.
    pub const MINIMUM_WIN_LENGTH: u32 = 114;
}

impl Default for Config {
    fn default() -> Config {
        Config {
            options: MoveOptions::ALL,
            node_budget: 10_000_000,
            max_depth: 600,
            table_entries: Table::entries_in(256 << 20),
        }
    }
}

/// The outcome of a search, including what it cost.
#[derive(Clone, Debug)]
pub struct Report {
    pub verdict: Verdict,
    /// The winning line, when there is one. Replayed before it is returned.
    pub line: Option<Vec<Move>>,
    /// States expanded.
    pub nodes: u64,
    /// Reported only. Never used as a cutoff.
    pub elapsed: Duration,
    pub table_capacity: usize,
    pub table_filled: usize,
    /// Why the search stopped, when it stopped early.
    pub limit: Option<Limit>,
}

/// A solver bug, not a game outcome: the search claimed a win whose move list
/// does not replay to a win.
#[derive(Clone, Debug)]
pub struct UnverifiedSolution {
    pub line: Vec<Move>,
    pub failure: String,
}

impl std::fmt::Display for UnverifiedSolution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "solver reported a win that does not replay ({}); line: {}",
            self.failure,
            self.line
                .iter()
                .map(|mv| mv.to_string())
                .collect::<Vec<_>>()
                .join(" ")
        )
    }
}

impl std::error::Error for UnverifiedSolution {}

/// Orders the legal moves. This only reorders — nothing is discarded, so it
/// cannot cost a solution. Dominances, which do discard, are a separate
/// question and are not applied here.
fn ordered(state: &State, options: MoveOptions) -> Vec<Move> {
    let mut moves = state.legal_moves(options);
    moves.sort_by_key(|mv| match *mv {
        Move::ToFoundation { .. } => 0u8,
        Move::Tableau { from, to, count } => {
            let source = &state.columns[from as usize];
            let takes_all = count as usize == source.len();
            let onto_empty = state.columns[to as usize].is_empty();
            if source.hidden() > 0 && count as usize == source.len() - source.hidden() {
                1 // turns up a buried card
            } else if takes_all && !onto_empty {
                2 // empties a column
            } else if takes_all && onto_empty {
                6 // a relabelling of the position, and nothing more
            } else {
                3
            }
        }
        Move::Stock => 4,
        Move::WorryBack { .. } => 5,
    });
    moves
}

/// One position on the search stack.
struct Frame {
    state: State,
    moves: Vec<Move>,
    next: usize,
    key: u128,
    /// Moves this frame may still add below it.
    depth: u32,
    /// True while every child so far has been refuted.
    complete: bool,
}

/// Searches a position.
///
/// Returns `Err` only when the search produced a line that does not replay to
/// a win, which is a bug in this crate rather than a property of the deal.
pub fn solve(start: &State, config: Config) -> Result<Report, UnverifiedSolution> {
    let began = Instant::now();
    let zobrist = Zobrist::new();
    let mut table = Table::with_entries(config.table_entries);

    let finish = |verdict, line, nodes, limit, table: &Table| Report {
        verdict,
        line,
        nodes,
        elapsed: began.elapsed(),
        table_capacity: table.capacity(),
        table_filled: table.filled(),
        limit,
    };

    if config.max_depth == 0 {
        return Ok(finish(
            Verdict::Unknown,
            None,
            0,
            Some(Limit::Depth),
            &table,
        ));
    }

    if start.is_won() {
        return Ok(finish(Verdict::Solvable, Some(Vec::new()), 0, None, &table));
    }

    let mut stack = vec![Frame {
        moves: ordered(start, config.options),
        state: start.clone(),
        next: 0,
        key: zobrist.key(start),
        depth: config.max_depth,
        complete: true,
    }];
    let mut path: HashSet<u128> = HashSet::from([zobrist.key(start)]);
    let mut line: Vec<Move> = Vec::new();
    let mut nodes: u64 = 1;
    let mut solution: Option<Vec<Move>> = None;
    let mut budget_spent = false;
    let mut depth_limited = false;
    let mut root_exhaustive = false;

    while let Some(top) = stack.len().checked_sub(1) {
        if stack[top].next == stack[top].moves.len() {
            let frame = stack.pop().expect("the stack is not empty here");
            if frame.complete {
                table.record_refuted(frame.key);
            } else {
                table.record_abandoned(frame.key, frame.depth);
            }
            path.remove(&frame.key);
            match stack.last_mut() {
                Some(parent) => {
                    parent.complete &= frame.complete;
                    line.pop();
                }
                // The root finished on its own terms rather than at a limit.
                None => root_exhaustive = frame.complete,
            }
            continue;
        }

        let mv = stack[top].moves[stack[top].next];
        stack[top].next += 1;

        let mut child = stack[top].state.clone();
        child
            .apply(mv)
            .expect("legal_moves only offers moves that apply");

        if child.is_won() {
            line.push(mv);
            solution = Some(line.clone());
            break;
        }

        // One move was spent reaching the child; what is left is what the
        // child has to work with.
        let depth = stack[top].depth - 1;
        if depth == 0 {
            stack[top].complete = false;
            depth_limited = true;
            continue;
        }

        if nodes >= config.node_budget {
            budget_spent = true;
            break;
        }

        let key = zobrist.key(&child);

        // A position already on the stack is being worked on above, with more
        // depth than this repeat would have. See the module note.
        if path.contains(&key) {
            stack[top].complete = false;
            continue;
        }

        match table.probe(key, depth) {
            // Proven lost wherever it is reached from: the parent can still
            // claim to have refuted this branch.
            Probe::Refuted => continue,
            // Looked at before, to at least this depth, without a result. Not
            // a refutation, so the parent loses its claim to exhaustiveness.
            Probe::Exhausted => {
                stack[top].complete = false;
                continue;
            }
            Probe::Unknown => {}
        }

        nodes += 1;
        line.push(mv);
        path.insert(key);
        stack.push(Frame {
            moves: ordered(&child, config.options),
            state: child,
            next: 0,
            key,
            depth,
            complete: true,
        });
    }

    if let Some(line) = solution {
        // A win is not believed until it replays from the opening position.
        if let Err(failure) = replay(start, &line) {
            return Err(UnverifiedSolution { line, failure });
        }
        return Ok(finish(Verdict::Solvable, Some(line), nodes, None, &table));
    }

    let (verdict, limit) = if budget_spent {
        (Verdict::Unknown, Some(Limit::Budget))
    } else if root_exhaustive {
        (Verdict::Unsolvable, None)
    } else {
        // Either the depth limit or a repetition stopped this being
        // exhaustive; both are reported as a depth limit.
        let _ = depth_limited;
        (Verdict::Unknown, Some(Limit::Depth))
    };

    Ok(finish(verdict, None, nodes, limit, &table))
}

/// Replays a line from the opening position and checks that it wins.
fn replay(start: &State, line: &[Move]) -> Result<(), String> {
    let mut state = start.clone();
    for (index, &mv) in line.iter().enumerate() {
        state
            .apply(mv)
            .map_err(|error| format!("move {} of {}: {error}", index + 1, line.len()))?;
    }
    if state.is_won() {
        Ok(())
    } else {
        Err(format!(
            "line ends with {} of 104 cards on the foundations",
            state.foundation_count()
        ))
    }
}
