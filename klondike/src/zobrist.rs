//! The Klondike transposition key.
//!
//! The contrast with Gypsy is the point of having both. Gypsy's stock deals
//! card *i* to column *i*, so its columns carry identity until the stock is
//! spent and may only be sorted after that. Klondike never addresses a pile
//! from outside — a draw goes to the waste, never to a named pile — so a
//! permutation of the piles really is only a relabelling, and they are folded
//! in order-insensitively throughout.
//!
//! The talon is a different matter and gets encoded card by card in its
//! current order, together with how far through it the pass has got. Its
//! length says nothing: cards leave it from the middle when the waste is
//! played, redeals return to the start of what is left, and two positions with
//! the same number of cards left can have completely different ones ahead.

use gypsy_core::card::{RANKS, SUITS};
use gypsy_core::rng::SplitMix64;
use gypsy_core::Card;

use crate::rules::{Position, DECK_SIZE, STOCK_AT_DEAL};

const CARDS: usize = (RANKS * SUITS) as usize;
const FOUNDATION_RANKS: usize = RANKS as usize + 1;

/// Frozen, for the same reason the shuffle is: it decides which collisions are
/// possible, and a verdict should be reproducible down to that.
const KEY_SEED: u64 = 0x6B6C_6F6E_6469_6B65;

pub struct Zobrist {
    pile_card: Vec<u128>,
    talon_card: Vec<u128>,
    turned: Vec<u128>,
    foundation: Vec<u128>,
}

impl Zobrist {
    pub fn new() -> Zobrist {
        let mut rng = SplitMix64::new(KEY_SEED);
        let mut draw = |count: usize| -> Vec<u128> {
            (0..count)
                .map(|_| (u128::from(rng.next_u64()) << 64) | u128::from(rng.next_u64()))
                .collect()
        };
        Zobrist {
            pile_card: draw(DECK_SIZE * CARDS * 2),
            talon_card: draw(STOCK_AT_DEAL * CARDS),
            turned: draw(STOCK_AT_DEAL + 1),
            foundation: draw(SUITS as usize * FOUNDATION_RANKS),
        }
    }

    pub fn key(&self, position: &Position) -> u128 {
        let mut key = self.turned[position.turned];

        for (suit, &rank) in position.foundations.iter().enumerate() {
            key ^= self.foundation[suit * FOUNDATION_RANKS + rank as usize];
        }

        for (index, &card) in position.talon.iter().enumerate() {
            key ^= self.talon_card[index * CARDS + card.index() as usize];
        }

        // Summed, so that two identical piles reinforce rather than cancel.
        let mut piles = 0u128;
        for pile in &position.piles {
            let mut contribution = 0u128;
            for (depth, &card) in pile.cards().iter().enumerate() {
                contribution ^= self.card_key(depth, card, depth < pile.hidden());
            }
            piles = piles.wrapping_add(contribution);
        }

        key ^ piles
    }

    fn card_key(&self, depth: usize, card: Card, hidden: bool) -> u128 {
        self.pile_card[(depth * CARDS + card.index() as usize) * 2 + usize::from(hidden)]
    }
}

impl Default for Zobrist {
    fn default() -> Zobrist {
        Zobrist::new()
    }
}
