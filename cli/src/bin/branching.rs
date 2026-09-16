//! Throwaway: how much of Gypsy's branching factor is partial-group moves
//! whose exposed card cannot be played up.
//!
//! Positions are sampled along the search's own first descent — always the
//! first action in `legal_actions` order that reaches an unvisited position —
//! because that is where the search actually spends its budget.

use std::collections::HashSet;

use gypsy_core::state::COLUMNS;
use gypsy_core::{Move, MoveOptions, State};
use gypsy_solver::{Game, Gypsy};

#[derive(Default)]
struct Tally {
    positions: u64,
    moves: u64,
    partial: u64,
    partial_dead: u64,
    partial_dead_onto_card: u64,
    partial_dead_stock_empty: u64,
    partial_dead_both_gates: u64,
}

impl Tally {
    fn line(&self, name: &str) {
        let pct = |n: u64| 100.0 * n as f64 / self.moves as f64;
        println!(
            "{name:<22} positions {:>8}  moves {:>9}  \
             partial {:>6.2}%  removable {:>6.2}%  onto-card {:>6.2}%  \
             stock-empty {:>6.2}%  both-gates {:>6.2}%",
            self.positions,
            self.moves,
            pct(self.partial),
            pct(self.partial_dead),
            pct(self.partial_dead_onto_card),
            pct(self.partial_dead_stock_empty),
            pct(self.partial_dead_both_gates),
        );
    }
}

fn count(state: &State, moves: &[Move], tally: &mut Tally) {
    let stock_empty = state.stock.is_empty();
    tally.positions += 1;
    tally.moves += moves.len() as u64;

    for mv in moves {
        let Move::Tableau { from, to, count } = *mv else {
            continue;
        };
        let column = &state.columns[from as usize];
        let count = count as usize;
        // A partial move leaves cards of the built run behind. Moving the
        // whole run, or the whole pile, is never restricted by the rule.
        if count >= column.movable_run() || count >= column.len() {
            continue;
        }
        tally.partial += 1;

        let exposed = column.cards()[column.len() - count - 1];
        if state.foundation_target(exposed).is_some() {
            continue; // the rule would keep this move
        }
        tally.partial_dead += 1;

        let onto_card = !state.columns[to as usize].is_empty();
        if onto_card {
            tally.partial_dead_onto_card += 1;
        }
        if stock_empty {
            tally.partial_dead_stock_empty += 1;
        }
        if onto_card && stock_empty {
            tally.partial_dead_both_gates += 1;
        }
    }
}

fn walk(game: &Gypsy, seed: u64, steps: usize, tally: &mut Tally) {
    let mut state = State::deal(seed);
    let mut seen: HashSet<u128> = HashSet::new();
    seen.insert(game.key(&state));

    for _ in 0..steps {
        let moves = game.legal_actions(&state);
        if moves.is_empty() {
            break;
        }
        count(&state, &moves, tally);

        let mut advanced = false;
        for mv in &moves {
            let mut child = state.clone();
            if child.apply(*mv).is_err() {
                continue;
            }
            if seen.insert(game.key(&child)) {
                state = child;
                advanced = true;
                break;
            }
        }
        if !advanced {
            break;
        }
    }
}

fn main() {
    let seeds: Vec<u64> = (0..10).collect();
    let steps: usize = std::env::args()
        .nth(1)
        .and_then(|a| a.parse().ok())
        .unwrap_or(50_000);

    println!(
        "columns {COLUMNS}, {} seeds, {steps} steps each\n",
        seeds.len()
    );

    for (name, options) in [
        ("full (worry-back)", MoveOptions::ALL),
        ("no-worry-back", MoveOptions::NO_WORRY_BACK),
    ] {
        let game = Gypsy::new(options);
        let mut tally = Tally::default();
        for &seed in &seeds {
            walk(&game, seed, steps, &mut tally);
        }
        tally.line(name);
    }
    println!(
        "\npartial    = tableau moves carrying part of a built run\n\
         removable  = those whose exposed card cannot go to a foundation\n\
         onto-card  = removable, and the destination is not an empty column\n\
         stock-empty= removable, and the stock is already exhausted\n\
         both-gates = removable under both conservative gates at once"
    );
}
