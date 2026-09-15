//! Gypsy as the search sees it: the transposition key, the move ordering, and
//! the [`Game`] implementation that ties them to `gypsy_core`.
//!
//! # The transposition key
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
use gypsy_core::state::{foundation_slots, COLUMNS, DECK_SIZE, STOCK_AT_DEAL};
use gypsy_core::{Card, Move, MoveOptions, State, Suit};

use crate::game::Game;

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

/// Gypsy under a chosen ruleset.
///
/// `options` decides whether the search may worry back. It is a property of
/// the game being searched rather than a search limit, which is what makes the
/// no-worry-back figure a restricted search of the same game rather than a
/// second ruleset.
pub struct Gypsy {
    zobrist: Zobrist,
    options: MoveOptions,
}

impl Gypsy {
    pub fn new(options: MoveOptions) -> Gypsy {
        Gypsy {
            zobrist: Zobrist::new(),
            options,
        }
    }
}

/// True when no card can ever want to sit on this one again.
///
/// A tableau card of rank *r* and colour *C* is useful in the tableau for
/// exactly one thing: being a base for a card of rank *r-1* and the opposite
/// colour. With two decks the opposite colour is two suits and each suit has
/// two foundation piles, so all **four** of those piles must have passed
/// *r-1* before nothing can want this card. The single-deck rule checks two
/// piles and would be wrong here.
///
/// Aces and twos are always safe. Nothing stacks on an ace, so no ace ever
/// needs a base; and the only card that stacks on a two is an ace, which never
/// needs one either — an ace off the foundations implies a free slot of its
/// suit, since both slots of a suit can only be occupied by that suit's two
/// aces.
fn never_wanted_in_the_tableau(state: &State, card: Card) -> bool {
    if card.rank() <= 2 {
        return true;
    }
    let wanted = card.rank() - 1;
    Suit::ALL
        .iter()
        .filter(|suit| suit.is_red() != card.is_red())
        .flat_map(|&suit| foundation_slots(suit))
        .all(|slot| state.foundations[slot as usize] >= wanted)
}

/// The first safe foundation play in this position, if there is one.
fn safe_autoplay(state: &State) -> Option<Move> {
    state.columns.iter().enumerate().find_map(|(from, column)| {
        let card = column.top()?;
        let foundation = state.foundation_target(card)?;
        never_wanted_in_the_tableau(state, card).then_some(Move::ToFoundation {
            from: from as u8,
            foundation,
        })
    })
}

impl Game for Gypsy {
    type Position = State;
    type Action = Move;

    /// Rules-legal moves, reordered, with safe autoplay applied in the game
    /// where it is provable.
    ///
    /// # Safe autoplay, and why it is gated on worry-back
    ///
    /// When a card is safe by [`never_wanted_in_the_tableau`] the search may
    /// play it and consider nothing else at this position.
    ///
    /// **The argument, with worry-back off.** Let `L` be a winning line from
    /// this position and let `X` be the safe card, on top of its column.
    /// Winning puts every card on a foundation, so `L` plays `X` up at some
    /// point. Build `L'`: play `X` up first, then follow `L` with that play
    /// removed. Every move of `L'` is legal. No move of `L` can put a card on
    /// `X`, because the only cards that could are the four opposite-colour
    /// cards of rank one lower, all of which are already on foundations and —
    /// **this is the whole gate** — with worry-back off can never leave them.
    /// A move of `L` that carries a run including `X` carries `X` plus cards
    /// below it; drop `X` from that run and the move still works, because the
    /// destination only ever tests the run's bottom card, which is unchanged.
    /// So `L'` wins, and restricting this position to the single move `X` up
    /// cannot lose a win.
    ///
    /// **Why worry-back breaks it.** The gate is not caution, it is the load
    /// -bearing step. With worry-back legal, "on a foundation" stops meaning
    /// "out of the tableau for good": the four cards the condition checks can
    /// come back down, and one of them may then want `X` underneath it. The
    /// safety condition is a claim about the future and worry-back makes it
    /// false.
    ///
    /// **And the tempting repair does not work.** It looks like worry-back
    /// should make this *easier* — play `X` up, and if it is ever wanted,
    /// worry it straight back. That argument is circular under a
    /// transposition table. It justifies the restricted position `P'` by
    /// appealing to a path from `P'` back to `P`, but `P` has been expanded
    /// with only the forced move in it, so the table skips it and the search
    /// never reaches `P`'s alternatives from `P'` either. The win the argument
    /// promises is one the search can no longer find. This is the shape of
    /// error `CLAUDE.md` warns about and Solvitaire's authors hit twice.
    fn legal_actions(&self, position: &State) -> Vec<Move> {
        if !self.options.worry_back {
            if let Some(autoplay) = safe_autoplay(position) {
                return vec![autoplay];
            }
        }

        let mut moves = position.legal_moves(self.options);
        moves.sort_by_key(|mv| match *mv {
            Move::ToFoundation { .. } => 0u8,
            Move::Tableau { from, to, count } => {
                let source = &position.columns[from as usize];
                let takes_all = count as usize == source.len();
                let onto_empty = position.columns[to as usize].is_empty();
                if source.hidden() > 0 && count as usize == source.len() - source.hidden() {
                    1 // turns up a buried card
                } else if takes_all && !onto_empty {
                    2 // empties a column
                } else if takes_all && onto_empty {
                    6 // a relabelling of the position, and nothing more
                } else {
                    3
                }
            }
            Move::Stock => 4,
            Move::WorryBack { .. } => 5,
        });
        moves
    }

    fn apply(&self, position: &mut State, action: Move) -> Result<(), String> {
        position.apply(action).map_err(|error| error.to_string())
    }

    fn is_won(&self, position: &State) -> bool {
        position.is_won()
    }

    fn key(&self, position: &State) -> u128 {
        self.zobrist.key(position)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gypsy_core::state::FOUNDATIONS;
    use gypsy_core::Move;

    /// Nothing stacks on an ace, and only an ace stacks on a two.
    #[test]
    fn aces_and_twos_never_need_a_base() {
        let mut state = State::deal(3);
        state.foundations = [0; FOUNDATIONS];
        assert!(never_wanted_in_the_tableau(
            &state,
            Card::new(Suit::Hearts, 1)
        ));
        assert!(never_wanted_in_the_tableau(
            &state,
            Card::new(Suit::Hearts, 2)
        ));
        assert!(!never_wanted_in_the_tableau(
            &state,
            Card::new(Suit::Hearts, 3)
        ));
    }

    /// The two-deck correction, and the one a ported single-deck rule gets
    /// wrong. Hearts are slots 2 and 3, diamonds 6 and 7. Checking one pile
    /// per opposite suit — slots 2 and 6, both past the four — would call this
    /// safe while a second four of diamonds is still in play and may still
    /// want a black five under it.
    #[test]
    fn a_card_is_unsafe_until_all_four_opposite_piles_pass_it() {
        let mut state = State::deal(3);
        let black_five = Card::new(Suit::Spades, 5);

        state.foundations = [0, 0, 4, 4, 0, 0, 4, 3];
        assert!(
            !never_wanted_in_the_tableau(&state, black_five),
            "one red pile is still short, so a red four can still want this"
        );

        state.foundations[7] = 4;
        assert!(never_wanted_in_the_tableau(&state, black_five));
    }

    /// Builds a position whose column 0 top card is both playable and safe.
    fn with_a_safe_autoplay(seed: u64) -> State {
        let mut state = State::deal(seed);
        let top = state.columns[0].top().expect("a dealt column has cards");
        let mut foundations = [0u8; FOUNDATIONS];
        foundations[foundation_slots(top.suit())[0] as usize] = top.rank() - 1;
        for suit in Suit::ALL
            .iter()
            .filter(|suit| suit.is_red() != top.is_red())
        {
            for slot in foundation_slots(*suit) {
                foundations[slot as usize] = top.rank() - 1;
            }
        }
        state.foundations = foundations;
        state
    }

    #[test]
    fn a_safe_card_collapses_the_position_to_one_move() {
        let state = with_a_safe_autoplay(3);
        let actions = Gypsy::new(MoveOptions::NO_WORRY_BACK).legal_actions(&state);
        assert_eq!(actions.len(), 1, "everything else is dominated");
        assert!(matches!(actions[0], Move::ToFoundation { .. }));
    }

    /// The gate. With worry-back legal the four piles the rule checks can
    /// send their cards back down, so the rule proves nothing and the search
    /// keeps every move.
    #[test]
    fn safe_autoplay_is_suppressed_when_worry_back_is_legal() {
        let state = with_a_safe_autoplay(3);
        let actions = Gypsy::new(MoveOptions::ALL).legal_actions(&state);
        assert!(
            actions.len() > 1,
            "the full game keeps its alternatives, got {actions:?}"
        );
    }

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
