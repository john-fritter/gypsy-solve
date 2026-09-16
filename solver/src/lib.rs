//! Search for solitaire deals, and the games it can search.
//!
//! The search is generic over [`Game`]. That is not decoration: the numbers
//! this project publishes are only worth anything if the search that produced
//! them is the same search that reproduces Klondike's known winnability, and
//! a second implementation validated separately would prove nothing about the
//! first.
//!
//! Cuts are conservative and each one is argued where it is made. Most are
//! transpositions — the same position, reached twice — or a limit, reported
//! honestly. The dominances are two, both in [`Gypsy`] and both proved on the
//! note on `Gypsy::legal_actions`: a forcing rule, which is safe autoplay in
//! the restricted arm and the two-deck safe-foundation rule in the full one,
//! and the split-run filter. They apply together at every position, and that
//! same note carries the argument that the pair is sound, not just each rule
//! alone. Klondike runs its own copies of both, which is how they are checked
//! against a game with a published answer.

mod game;
mod gypsy;
mod search;
mod table;

pub use game::Game;
pub use gypsy::Gypsy;
pub use search::{
    replay, solve, solve_restarting, Config, Limit, Report, UnverifiedSolution, Verdict,
};
pub use table::Table;

#[cfg(test)]
mod tests;
