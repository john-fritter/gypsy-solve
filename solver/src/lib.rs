//! Search for solitaire deals, and the games it can search.
//!
//! The search is generic over [`Game`]. That is not decoration: the numbers
//! this project publishes are only worth anything if the search that produced
//! them is the same search that reproduces Klondike's known winnability, and
//! a second implementation validated separately would prove nothing about the
//! first.
//!
//! No dominances are applied to either game. Every cut is either a
//! transposition — the same position, reached twice — or a limit, reported
//! honestly. That makes it slow, and makes it the baseline that later cuts get
//! measured against.

mod game;
mod gypsy;
mod search;
mod table;

pub use game::Game;
pub use gypsy::Gypsy;
pub use search::{solve, Config, Limit, Report, UnverifiedSolution, Verdict};
pub use table::Table;

#[cfg(test)]
mod tests;
