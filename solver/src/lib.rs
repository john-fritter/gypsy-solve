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
//! honestly. The one dominance is safe autoplay in [`Gypsy`], and it applies
//! only when worry-back is off, because that is the only case where it is
//! provable; see the note on `Gypsy::legal_actions`. Klondike has none, which
//! is why it cannot check that one.

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
