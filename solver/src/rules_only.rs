//! Gypsy with every dominance removed, for verification.
//!
//! A verdict this game returns trusts no pruning rule: it offers every
//! rules-legal move, and only orders them, foundation plays first and
//! worry-backs last. Ordering discards nothing. It is what a dominance is
//! checked against, never what a published number is searched with.

use gypsy_core::{Move, MoveOptions, State};

use crate::{Game, Gypsy};

pub struct RulesOnly {
    gypsy: Gypsy,
    options: MoveOptions,
}

impl RulesOnly {
    pub fn new(options: MoveOptions) -> RulesOnly {
        RulesOnly {
            gypsy: Gypsy::new(options),
            options,
        }
    }
}

impl Game for RulesOnly {
    type Position = State;
    type Action = Move;

    fn legal_actions(&self, position: &State, _salt: u64) -> Vec<Move> {
        let mut moves = position.legal_moves(self.options);
        moves.sort_by_key(|mv| match mv {
            Move::ToFoundation { .. } => 0,
            Move::Tableau { .. } => 1,
            Move::Stock => 2,
            Move::WorryBack { .. } => 3,
        });
        moves
    }

    fn apply(&self, position: &mut State, action: Move) -> Result<(), String> {
        self.gypsy.apply(position, action)
    }

    fn is_won(&self, position: &State) -> bool {
        self.gypsy.is_won(position)
    }

    /// The transposition key is not a dominance: two positions share it only
    /// when they have the same future. Using the solver's keeps the
    /// stock-gated canonicalisation it was proved with.
    fn key(&self, position: &State) -> u128 {
        self.gypsy.key(position)
    }
}
