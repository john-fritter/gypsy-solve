//! Two-deck Gypsy solitaire: rules, state, move generation and seeded deals.
//!
//! This crate is the only implementation of the rules in the project. The
//! batch runner, the solver and the browser front end all compile against it,
//! so that a published winnability figure and the game a visitor plays cannot
//! drift apart.
//!
//! The ruleset is the permissive variant described in `DESIGN.md`: two decks,
//! eight columns, eight foundations, any alternating-colour sequence moves as
//! a unit, the stock deals one card to every column, and worry-back is legal.

pub mod card;
mod display;
pub mod moves;
pub mod rng;
pub mod state;

pub use card::{Card, Suit};
pub use moves::{parse_move_list, Move};
pub use state::{Column, IllegalMove, MoveOptions, State};
