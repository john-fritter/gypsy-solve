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
//! branch is cut, and the frame records *why* it is incomplete, because the
//! two reasons are not worth the same.
//!
//! A repetition does not cost the root its proof. Take any winning line and
//! take the shortest one: it cannot visit a position twice, or the stretch
//! between the two visits could be deleted to give a shorter win. So the
//! shortest win is never the thing a repetition check cuts, and a search that
//! ran out of nothing but repetitions has still seen every win there is. If it
//! found none, there is none.
//!
//! That argument is about the root, and about repetitions against *this*
//! path. It does not survive the table. A frame cut only by repetition is
//! recorded as an abandonment rather than a refutation, and a later search
//! that skips on that entry treats it as a limit, not a repetition — because
//! the cut branches looped back to ancestors of the *old* path, which the new
//! one need not contain, so a win may have been missed down there. Inheriting
//! it as a limit costs the new search its proof and never gives it a wrong
//! one.
//!
//! So a proof of unsolvability rests only on repetitions the search saw
//! directly, against the path it was actually on.

use std::collections::HashSet;
use std::time::{Duration, Instant};

use crate::game::Game;
use crate::table::{Probe, Table};

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

/// Why a frame did not finish, worst case last. `None` is a frame that
/// searched everything below it.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Cut {
    /// Everything below was searched.
    None,
    /// Some branch looped back to a position already on the stack. Costs the
    /// table an entry, but not the root its proof.
    Repetition,
    /// Some branch ran out of depth or budget. This one does cost the proof.
    Limit,
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
///
/// Which moves exist is the game's business, not a search limit, so the
/// ruleset lives on the [`Game`] rather than here.
#[derive(Clone, Copy, Debug)]
pub struct Config {
    /// States expanded before giving up.
    pub node_budget: u64,
    /// Longest line the search may build.
    pub max_depth: u32,
    /// Transposition table size, rounded up to a power of two.
    pub table_entries: usize,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            node_budget: 10_000_000,
            max_depth: 600,
            table_entries: Table::entries_in(256 << 20),
        }
    }
}

/// The outcome of a search, including what it cost.
#[derive(Clone, Debug)]
pub struct Report<A> {
    pub verdict: Verdict,
    /// The winning line, when there is one. Replayed before it is returned.
    pub line: Option<Vec<A>>,
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
pub struct UnverifiedSolution<A> {
    pub line: Vec<A>,
    pub failure: String,
}

impl<A: std::fmt::Display> std::fmt::Display for UnverifiedSolution<A> {
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

impl<A: std::fmt::Debug + std::fmt::Display> std::error::Error for UnverifiedSolution<A> {}

/// One position on the search stack.
struct Frame<G: Game> {
    state: G::Position,
    moves: Vec<G::Action>,
    next: usize,
    key: u128,
    /// Moves this frame may still add below it.
    depth: u32,
    /// The worst reason any child had for not finishing.
    cut: Cut,
}

/// Searches a position.
///
/// Returns `Err` only when the search produced a line that does not replay to
/// a win, which is a bug in this crate rather than a property of the deal.
pub fn solve<G: Game>(
    game: &G,
    start: &G::Position,
    config: Config,
) -> Result<Report<G::Action>, UnverifiedSolution<G::Action>> {
    let began = Instant::now();
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

    if game.is_won(start) {
        return Ok(finish(Verdict::Solvable, Some(Vec::new()), 0, None, &table));
    }

    let mut stack: Vec<Frame<G>> = vec![Frame {
        moves: game.legal_actions(start),
        state: start.clone(),
        next: 0,
        key: game.key(start),
        depth: config.max_depth,
        cut: Cut::None,
    }];
    let mut path: HashSet<u128> = HashSet::from([game.key(start)]);
    let mut line: Vec<G::Action> = Vec::new();
    let mut nodes: u64 = 1;
    let mut solution: Option<Vec<G::Action>> = None;
    let mut budget_spent = false;
    let mut root_cut = Cut::Limit;

    while let Some(top) = stack.len().checked_sub(1) {
        if stack[top].next == stack[top].moves.len() {
            let frame = stack.pop().expect("the stack is not empty here");
            match frame.cut {
                Cut::None => table.record_refuted(frame.key),
                // An abandonment, never a refutation. It saves the re-search
                // without ever claiming the subtree held no win.
                // An abandonment, never a refutation, carrying why it stopped
                // so that a later search knows whether its proof survives.
                Cut::Repetition => table.record_abandoned(frame.key, frame.depth, true),
                Cut::Limit => table.record_abandoned(frame.key, frame.depth, false),
            }
            path.remove(&frame.key);
            match stack.last_mut() {
                Some(parent) => {
                    parent.cut = parent.cut.max(frame.cut);
                    line.pop();
                }
                None => root_cut = frame.cut,
            }
            continue;
        }

        let mv = stack[top].moves[stack[top].next];
        stack[top].next += 1;

        let mut child = stack[top].state.clone();
        game.apply(&mut child, mv)
            .expect("legal_actions only offers actions that apply");

        if game.is_won(&child) {
            line.push(mv);
            solution = Some(line.clone());
            break;
        }

        // One move was spent reaching the child; what is left is what the
        // child has to work with.
        let depth = stack[top].depth - 1;
        if depth == 0 {
            stack[top].cut = stack[top].cut.max(Cut::Limit);
            continue;
        }

        if nodes >= config.node_budget {
            budget_spent = true;
            break;
        }

        let key = game.key(&child);

        // A position already on the stack is being worked on above, with more
        // depth than this repeat would have. See the module note.
        if path.contains(&key) {
            stack[top].cut = stack[top].cut.max(Cut::Repetition);
            continue;
        }

        match table.probe(key, depth) {
            // Proven lost wherever it is reached from: the parent can still
            // claim to have refuted this branch.
            Probe::Refuted => continue,
            // Looked at before, to at least this depth, without a result. Not
            // a refutation, so the parent loses its claim to exhaustiveness.
            // Abandonments are only ever recorded by frames a limit stopped,
            // so inheriting one inherits a limit.
            // Skipping is safe either way; what carries over is whether the
            // earlier search kept its proof.
            Probe::Exhausted { repetition_only } => {
                stack[top].cut = stack[top].cut.max(if repetition_only {
                    Cut::Repetition
                } else {
                    Cut::Limit
                });
                continue;
            }
            Probe::Unknown => {}
        }

        nodes += 1;
        line.push(mv);
        path.insert(key);
        stack.push(Frame {
            moves: game.legal_actions(&child),
            state: child,
            next: 0,
            key,
            depth,
            cut: Cut::None,
        });
    }

    if let Some(line) = solution {
        // A win is not believed until it replays from the opening position.
        if let Err(failure) = replay(game, start, &line) {
            return Err(UnverifiedSolution { line, failure });
        }
        return Ok(finish(Verdict::Solvable, Some(line), nodes, None, &table));
    }

    let (verdict, limit) = if budget_spent {
        (Verdict::Unknown, Some(Limit::Budget))
    } else {
        match root_cut {
            // Nothing below the root went unsearched, or the only thing that
            // did was a loop back to a position already being searched.
            Cut::None | Cut::Repetition => (Verdict::Unsolvable, None),
            Cut::Limit => (Verdict::Unknown, Some(Limit::Depth)),
        }
    };

    Ok(finish(verdict, None, nodes, limit, &table))
}

/// Replays a line from the opening position and checks that it wins.
fn replay<G: Game>(game: &G, start: &G::Position, line: &[G::Action]) -> Result<(), String> {
    let mut state = start.clone();
    for (index, &action) in line.iter().enumerate() {
        game.apply(&mut state, action)
            .map_err(|error| format!("move {} of {}: {error}", index + 1, line.len()))?;
    }
    if game.is_won(&state) {
        Ok(())
    } else {
        Err("the line does not end in a win".to_string())
    }
}
