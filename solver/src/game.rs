//! What the search needs from a game.
//!
//! The search knows nothing about cards. It asks for the legal actions in a
//! position, applies one, asks whether the position is won, and asks for a
//! transposition key. Everything that makes a game *that* game — what stacks
//! on what, what the stock does, which symmetries are safe to fold away —
//! lives behind this trait.
//!
//! The trait exists because of Klondike. A solver validated against a game
//! with a published answer is the only thing separating "this search is too
//! weak" from "this search is wrong", and validating a *different* search than
//! the one that produces the Gypsy numbers would prove nothing.

use std::fmt::{Debug, Display};

pub trait Game {
    /// A complete position. The search clones these.
    type Position: Clone;
    /// One move. Printed into solution lines, so it must be readable.
    type Action: Copy + Debug + Display;

    /// Every action the rules permit, in the order the search should try them.
    ///
    /// Ordering is the implementation's business and is free: reordering
    /// discards nothing. Leaving an action *out* is a dominance, and needs an
    /// argument for why it cannot discard a winning line.
    ///
    /// `salt` lets the search ask for a *different* order — it is how
    /// restarts are built. Zero is the game's own order, and every other value
    /// must permute the same set: a salt that changed which actions exist
    /// would be a dominance nobody argued for, and it would make a verdict
    /// depend on which restart happened to reach a position.
    fn legal_actions(&self, position: &Self::Position, salt: u64) -> Vec<Self::Action>;

    /// Applies an action. Errors describe why it was illegal; the search only
    /// ever passes actions that came from `legal_actions`, but verification
    /// replays lines through this and wants the failure.
    fn apply(&self, position: &mut Self::Position, action: Self::Action) -> Result<(), String>;

    fn is_won(&self, position: &Self::Position) -> bool;

    /// The transposition key.
    ///
    /// Two positions may share a key only when they have the same future. That
    /// is stricter than looking alike, and it is where games differ: Gypsy
    /// deals stock card *i* to column *i*, so its columns are not
    /// interchangeable until the stock runs out, while Klondike never
    /// addresses a pile from outside and so may sort its piles throughout.
    fn key(&self, position: &Self::Position) -> u128;
}
