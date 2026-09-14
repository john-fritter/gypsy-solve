//! Klondike solitaire, for validating the solver.
//!
//! This crate exists to answer one question: is the search correct, or only
//! slow? Klondike has a published thoughtful winnability — 81.945% ± 0.084%,
//! from Solvitaire (Blake & Gent) — so a search that reproduces it is working,
//! and one that does not is broken in a way no amount of measurement on Gypsy
//! would reveal. The same `gypsy_solver::solve` runs both games; validating a
//! separate implementation would prove nothing about the one that publishes.
//!
//! It is deliberately *not* part of `gypsy-core`. That crate is the single
//! implementation of the Gypsy rules and is what compiles to WASM for the
//! site, and a second game nobody plays there does not belong in it.

mod game;
mod rules;
mod zobrist;

pub use game::Klondike;
pub use rules::{
    Action, ParseActionError, Pile, Position, DEALT_TO_TABLEAU, DECK_SIZE, DRAW, FOUNDATIONS,
    PILES, STOCK_AT_DEAL,
};

#[cfg(test)]
mod tests;
