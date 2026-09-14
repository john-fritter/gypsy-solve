//! Klondike as the search sees it.

use gypsy_solver::Game;

use crate::rules::{Action, Position};
use crate::zobrist::Zobrist;

/// Klondike under the Solvitaire variant: draw three, redeals without limit,
/// worry-back permitted.
pub struct Klondike {
    zobrist: Zobrist,
}

impl Klondike {
    pub fn new() -> Klondike {
        Klondike {
            zobrist: Zobrist::new(),
        }
    }
}

impl Default for Klondike {
    fn default() -> Klondike {
        Klondike::new()
    }
}

impl Game for Klondike {
    type Position = Position;
    type Action = Action;

    /// Rules-legal actions, reordered. Nothing is dropped — the same rule as
    /// the Gypsy side, and for the same reason: a wrong dominance here would
    /// corrupt the validation that is supposed to catch wrong dominances.
    fn legal_actions(&self, position: &Position) -> Vec<Action> {
        let mut actions = position.legal_actions();
        actions.sort_by_key(|action| match *action {
            Action::WasteToFoundation => 0u8,
            Action::PileToFoundation { .. } => 1,
            Action::PileToPile { from, to, count } => {
                let source = &position.piles[from as usize];
                if source.hidden() > 0 && count as usize == source.len() - source.hidden() {
                    2 // turns up a buried card
                } else if source.hidden() == 0 && count as usize == source.len() {
                    // Moving a whole face-up pile onto an empty one is a
                    // relabelling; a king already sitting alone is the case.
                    if position.piles[to as usize].is_empty() {
                        7
                    } else {
                        4
                    }
                } else {
                    4
                }
            }
            Action::WasteToPile { .. } => 3,
            Action::Draw => 5,
            Action::FoundationToPile { .. } => 6,
        });
        actions
    }

    fn apply(&self, position: &mut Position, action: Action) -> Result<(), String> {
        position.apply(action)
    }

    fn is_won(&self, position: &Position) -> bool {
        position.is_won()
    }

    fn key(&self, position: &Position) -> u128 {
        self.zobrist.key(position)
    }
}
