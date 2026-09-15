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
//! ## Why previously-expanded positions can simply be skipped
//!
//! Expanding a position generates every child. So take a shortest winning line
//! `s0 → … → sk`. If `sj` has been expanded then `s(j+1)` was generated, and
//! the search either won on it, expanded it, or skipped it because it had
//! already been expanded. Either way `s(j+1)` gets expanded, so by induction
//! from the root every position on the line is expanded, and expanding
//! `s(k-1)` produces the win.
//!
//! Depth never appears in that argument, which is why the table does not
//! record any. An earlier version indexed entries by the depth the search had
//! in hand, and answered a probe only for a visit with no more depth to spend;
//! since depth-first search reaches a position with a different amount in hand
//! nearly every time, most probes missed and positions were re-expanded 20 to
//! 57 times over.
//!
//! A repetition on the current path needs no special handling either: an
//! ancestor is by definition already expanded, so the table skips it.
//!
//! The one thing that breaks the induction is a position that is *never*
//! expanded, which is what the node budget and the stack guard do. Either of
//! those, anywhere in the run, costs the search its proof — so unsolvability
//! is claimed only when neither was hit.

use std::time::{Duration, Instant};

use crate::game::Game;
use crate::table::Table;

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
    /// A line grew past the stack guard. Not a tuning knob — it is there so a
    /// pathological descent cannot exhaust memory, and hitting it is a sign
    /// something is wrong rather than a limit to raise.
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
    /// Longest line the search may build, as a memory guard rather than a
    /// search parameter. Positions are never re-expanded because of it.
    pub max_depth: u32,
    /// Transposition table size, rounded up to a power of two.
    pub table_entries: usize,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            node_budget: 10_000_000,
            max_depth: 100_000,
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

    if game.is_won(start) {
        return Ok(finish(Verdict::Solvable, Some(Vec::new()), 0, None, &table));
    }

    table.insert(game.key(start));
    let mut stack: Vec<Frame<G>> = vec![Frame {
        moves: game.legal_actions(start),
        state: start.clone(),
        next: 0,
    }];
    let mut line: Vec<G::Action> = Vec::new();
    let mut nodes: u64 = 1;
    let mut solution: Option<Vec<G::Action>> = None;
    let mut budget_spent = false;
    let mut depth_guarded = false;

    while let Some(top) = stack.len().checked_sub(1) {
        if stack[top].next == stack[top].moves.len() {
            stack.pop();
            if !stack.is_empty() {
                line.pop();
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

        // Already expanded, so its children have already been generated.
        // Skipping cannot hide a win; see the module note. An ancestor on the
        // current path is covered by this too, so loops need nothing special.
        let key = game.key(&child);
        if table.contains(key) {
            continue;
        }

        if nodes >= config.node_budget {
            budget_spent = true;
            break;
        }
        // A guard against a pathological descent eating memory, not a search
        // parameter: no position is ever re-expanded because of it.
        if stack.len() as u32 >= config.max_depth {
            depth_guarded = true;
            continue;
        }

        nodes += 1;
        line.push(mv);
        table.insert(key);
        stack.push(Frame {
            moves: game.legal_actions(&child),
            state: child,
            next: 0,
        });
    }

    if let Some(line) = solution {
        // A win is not believed until it replays from the opening position.
        if let Err(failure) = replay(game, start, &line) {
            return Err(UnverifiedSolution { line, failure });
        }
        return Ok(finish(Verdict::Solvable, Some(line), nodes, None, &table));
    }

    // Every reachable position was expanded unless something stopped one from
    // being, and only those two things can.
    let (verdict, limit) = if budget_spent {
        (Verdict::Unknown, Some(Limit::Budget))
    } else if depth_guarded {
        (Verdict::Unknown, Some(Limit::Depth))
    } else {
        (Verdict::Unsolvable, None)
    };

    Ok(finish(verdict, None, nodes, limit, &table))
}

/// Replays a line from the opening position and checks that it wins.
///
/// Public because a line is not always believed by the search that produced
/// it. A win found under a restricted ruleset is also a win under a
/// permissive one, and a caller carrying a line across that way owes the same
/// check as the search does: the point of replaying is that a claimed win is
/// never taken on trust, and a carried claim is still a claim.
pub fn replay<G: Game>(game: &G, start: &G::Position, line: &[G::Action]) -> Result<(), String> {
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
