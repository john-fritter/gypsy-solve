//! Solver for two-deck Gypsy solitaire.
//!
//! The rules live in `gypsy_core` and are not duplicated here: this crate only
//! decides which of the moves that crate offers are worth looking at, and in
//! what order. It applies no dominances — every cut it makes is either a
//! transposition (the same position, reached twice) or a limit it reports
//! honestly. That makes it slow and makes it the baseline that later cuts are
//! measured against.

mod search;
mod table;
mod zobrist;

pub use search::{solve, Config, Limit, Report, UnverifiedSolution, Verdict};
pub use table::Table;

#[cfg(test)]
mod tests;
