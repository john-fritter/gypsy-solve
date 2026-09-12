//! The transposition key.
//!
//! Two positions get the same key only when they have the same future. That is
//! a narrower claim than "they look alike", and the difference is where this
//! game is unusual: the stock deal sends card *i* to column *i*, so column
//! order is part of the position for as long as cards remain undealt. See
//! `DECISIONS.md`, 2026-09-11.
//!
//! Sound unconditionally:
//!
//! - the remaining stock is folded in as its **length** alone, because nothing
//!   ever returns a card to the stock, so for one deal the length fixes the
//!   remaining sequence exactly;
//! - the two foundation slots of a suit are sorted, because they are rank
//!   counters and no rule distinguishes them.
//!
//! Sound only once the stock is empty, and gated on it here:
//!
//! - the eight columns are folded in order-insensitively.
//!
//! Keys are 128 bits and compared in full on lookup, so a false match needs a
//! genuine 128-bit collision rather than a bucket clash. At ten million states
//! in a deal that is around `1e-25`, which is the point: a false match merges
//! two different futures and can turn a winnable deal into a reported
//! `unsolvable`, and that error is invisible in the output.

use gypsy_core::card::{RANKS, SUITS};
use gypsy_core::rng::SplitMix64;
use gypsy_core::state::{COLUMNS, DECK_SIZE, STOCK_AT_DEAL};
use gypsy_core::{Card, State};

/// Deepest column the keys cover. A column cannot exceed the whole deck.
const MAX_COLUMN_DEPTH: usize = DECK_SIZE;

const CARDS: usize = (RANKS * SUITS) as usize;
/// Ranks a foundation slot can show: empty, then ace through king.
const FOUNDATION_RANKS: usize = RANKS as usize + 1;

/// The seed for the key table. Frozen, like the shuffle: it does not change
/// which deals exist, but it does decide which collisions are possible, and a
/// published verdict should be reproducible down to that.
const KEY_SEED: u64 = 0x6779_7073_795F_7A62;

/// Random keys for every component of a position.
pub struct Zobrist {
    /// Indexed by depth in the column, card, and whether it is face down.
    card_at: Vec<u128>,
    /// Mixed in per column while column order still matters.
    column_slot: [u128; COLUMNS],
    /// Indexed by suit, then by which of the sorted pair, then by rank.
    foundation: Vec<u128>,
    /// Indexed by the number of cards left in the stock.
    stock_len: Vec<u128>,
}

impl Zobrist {
    pub fn new() -> Zobrist {
        let mut rng = SplitMix64::new(KEY_SEED);
        let mut draw = |count: usize| -> Vec<u128> {
            (0..count)
                .map(|_| (u128::from(rng.next_u64()) << 64) | u128::from(rng.next_u64()))
                .collect()
        };

        let card_at = draw(MAX_COLUMN_DEPTH * CARDS * 2);
        let column_slot: [u128; COLUMNS] = draw(COLUMNS)
            .try_into()
            .expect("drew exactly one key per column");
        let foundation = draw(SUITS as usize * 2 * FOUNDATION_RANKS);
        let stock_len = draw(STOCK_AT_DEAL + 1);

        Zobrist {
            card_at,
            column_slot,
            foundation,
            stock_len,
        }
    }

    fn card_key(&self, depth: usize, card: Card, hidden: bool) -> u128 {
        self.card_at[(depth * CARDS + card.index() as usize) * 2 + usize::from(hidden)]
    }

    fn foundation_key(&self, suit: usize, which: usize, rank: u8) -> u128 {
        self.foundation[(suit * 2 + which) * FOUNDATION_RANKS + rank as usize]
    }

    /// The transposition key for a position.
    pub fn key(&self, state: &State) -> u128 {
        // Order stops mattering only when no card is waiting to be dealt onto
        // a particular column.
        let stock_empty = state.stock.is_empty();

        let mut key = self.stock_len[state.stock.len()];

        for suit in 0..SUITS as usize {
            let mut pair = [state.foundations[suit * 2], state.foundations[suit * 2 + 1]];
            pair.sort_unstable();
            key ^= self.foundation_key(suit, 0, pair[0]);
            key ^= self.foundation_key(suit, 1, pair[1]);
        }

        // Summed rather than XORed so that two identical columns reinforce
        // instead of cancelling to zero.
        let mut columns = 0u128;
        for (index, column) in state.columns.iter().enumerate() {
            let mut contribution = 0u128;
            for (depth, &card) in column.cards().iter().enumerate() {
                contribution ^= self.card_key(depth, card, depth < column.hidden());
            }
            if !stock_empty {
                contribution ^= self.column_slot[index];
            }
            columns = columns.wrapping_add(contribution);
        }

        key ^ columns
    }
}

impl Default for Zobrist {
    fn default() -> Zobrist {
        Zobrist::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gypsy_core::Move;

    #[test]
    fn the_same_position_always_hashes_the_same() {
        let zobrist = Zobrist::new();
        assert_eq!(zobrist.key(&State::deal(42)), zobrist.key(&State::deal(42)));
        assert_ne!(zobrist.key(&State::deal(42)), zobrist.key(&State::deal(43)));
    }

    /// The finding this whole module exists for. While cards are waiting to be
    /// dealt, swapping two columns changes who receives them, so the two
    /// positions must not share a key.
    #[test]
    fn column_order_matters_while_the_stock_holds_cards() {
        let zobrist = Zobrist::new();
        let left = State::deal(7);
        let mut right = left.clone();
        right.columns.swap(0, 1);

        assert!(!left.stock.is_empty());
        assert_ne!(
            zobrist.key(&left),
            zobrist.key(&right),
            "swapped columns receive different cards on the next stock deal"
        );

        // And the positions really do diverge, which is what makes that right.
        let mut dealt_left = left;
        let mut dealt_right = right;
        dealt_left.apply(Move::Stock).expect("stock deal is legal");
        dealt_right.apply(Move::Stock).expect("stock deal is legal");
        dealt_right.columns.swap(0, 1);
        assert_ne!(dealt_left.columns, dealt_right.columns);
    }

    #[test]
    fn column_order_stops_mattering_once_the_stock_is_empty() {
        let zobrist = Zobrist::new();
        let mut left = State::deal(7);
        while !left.stock.is_empty() {
            left.apply(Move::Stock).expect("stock deal is legal");
        }
        let mut right = left.clone();
        right.columns.swap(2, 5);

        assert_eq!(
            zobrist.key(&left),
            zobrist.key(&right),
            "with nothing left to deal, column order is only a relabelling"
        );
    }

    #[test]
    fn the_two_foundation_slots_of_a_suit_are_interchangeable() {
        let zobrist = Zobrist::new();
        let mut left = State::deal(1);
        left.foundations = [3, 5, 0, 0, 0, 0, 0, 0];
        let mut right = left.clone();
        right.foundations = [5, 3, 0, 0, 0, 0, 0, 0];
        assert_eq!(zobrist.key(&left), zobrist.key(&right));
    }

    #[test]
    fn different_suits_are_not_interchangeable() {
        let zobrist = Zobrist::new();
        let mut spades = State::deal(1);
        spades.foundations = [4, 0, 0, 0, 0, 0, 0, 0];
        let mut hearts = State::deal(1);
        hearts.foundations = [0, 0, 4, 0, 0, 0, 0, 0];
        assert_ne!(zobrist.key(&spades), zobrist.key(&hearts));
    }

    #[test]
    fn stock_depth_is_part_of_the_key() {
        let zobrist = Zobrist::new();
        let before = State::deal(9);
        let mut after = before.clone();
        after.apply(Move::Stock).expect("stock deal is legal");
        assert_ne!(zobrist.key(&before), zobrist.key(&after));
    }
}
