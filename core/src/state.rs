//! Game state, the seeded deal, move generation and move application.
//!
//! This module is the single source of truth for the rules. The exact ruleset
//! is the permissive variant documented in `DESIGN.md`: any alternating-colour
//! sequence moves as a unit, and worry-back is legal.

use std::fmt;

use crate::card::{Card, Suit, RANKS, SUITS};
use crate::moves::Move;
use crate::rng::SplitMix64;

/// Tableau columns.
pub const COLUMNS: usize = 8;
/// Foundation slots: two per suit, since the game uses two decks.
pub const FOUNDATIONS: usize = 8;
/// Cards in play: two standard decks.
pub const DECK_SIZE: usize = 104;
/// Cards buried face down by the initial deal, one per column.
pub const FACE_DOWN_AT_DEAL: usize = COLUMNS;
/// Cards left in the stock after the initial deal.
pub const STOCK_AT_DEAL: usize = DECK_SIZE - COLUMNS * 3;
/// Stock deals available, each turning up one card per column.
pub const STOCK_DEALS: usize = STOCK_AT_DEAL / COLUMNS;

/// The foundation slot indices belonging to a suit.
pub const fn foundation_slots(suit: Suit) -> [u8; 2] {
    let base = suit.index() * 2;
    [base, base + 1]
}

/// The suit built on a foundation slot.
pub const fn foundation_suit(slot: u8) -> Suit {
    Suit::from_index(slot / 2)
}

/// One tableau column, bottom card first.
///
/// The first `hidden` cards are face down. A face-down card at the top of a
/// column is turned up immediately, so `hidden` is always less than `len`
/// unless the column is empty.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Column {
    cards: Vec<Card>,
    hidden: usize,
}

impl Column {
    /// All cards, bottom first. The first [`Column::hidden`] are face down.
    pub fn cards(&self) -> &[Card] {
        &self.cards
    }

    /// How many cards at the bottom of the column are face down.
    pub fn hidden(&self) -> usize {
        self.hidden
    }

    /// The face-up cards, bottom first.
    pub fn face_up(&self) -> &[Card] {
        &self.cards[self.hidden..]
    }

    pub fn len(&self) -> usize {
        self.cards.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }

    /// The exposed card, if any. Always face up.
    pub fn top(&self) -> Option<Card> {
        self.cards.last().copied()
    }

    fn push(&mut self, card: Card) {
        self.cards.push(card);
    }

    /// Removes the top `count` cards and turns up an exposed face-down card.
    fn take(&mut self, count: usize) -> Vec<Card> {
        let taken = self.cards.split_off(self.cards.len() - count);
        self.turn_up_exposed();
        taken
    }

    fn turn_up_exposed(&mut self) {
        if self.hidden > 0 && self.hidden >= self.cards.len() {
            self.hidden = self.cards.len().saturating_sub(1);
        }
    }

    /// Length of the alternating-colour descending run at the top of the
    /// column, counting only face-up cards. Zero for an empty column.
    pub fn movable_run(&self) -> usize {
        let face_up = self.face_up();
        let mut run = 0;
        while run < face_up.len() {
            let index = face_up.len() - 1 - run;
            if run > 0 && !face_up[index + 1].stacks_on(face_up[index]) {
                break;
            }
            run += 1;
        }
        run
    }
}

/// A complete game position.
///
/// Foundation slot `i` builds suit `i / 2`; `foundations[i]` is the rank of its
/// top card, or 0 when empty. The two slots of a suit are interchangeable —
/// the solver will canonicalise them, but their indices stay fixed here so
/// that a recorded move list keeps its meaning.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct State {
    /// Tableau columns, left to right.
    pub columns: [Column; COLUMNS],
    /// Top rank of each foundation slot; 0 means empty.
    pub foundations: [u8; FOUNDATIONS],
    /// Undealt stock, next card first.
    pub stock: Vec<Card>,
}

/// Why a move could not be applied.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct IllegalMove {
    pub attempted: Move,
    pub reason: &'static str,
}

impl fmt::Display for IllegalMove {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "illegal move {}: {}", self.attempted, self.reason)
    }
}

impl std::error::Error for IllegalMove {}

/// Which moves [`State::legal_moves`] should generate.
///
/// This restricts the search, not the rules: [`State::apply`] always accepts
/// any move the game permits. Solving each deal with `worry_back` off and then
/// on is what produces the worry-back delta.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct MoveOptions {
    /// Generate foundation-to-tableau moves.
    pub worry_back: bool,
}

impl MoveOptions {
    /// The full ruleset.
    pub const ALL: MoveOptions = MoveOptions { worry_back: true };
    /// Worry-back suppressed: the easier game, and a lower bound on the full one.
    pub const NO_WORRY_BACK: MoveOptions = MoveOptions { worry_back: false };
}

impl Default for MoveOptions {
    fn default() -> MoveOptions {
        MoveOptions::ALL
    }
}

impl State {
    /// Builds the shuffled 104-card deck for a seed.
    ///
    /// The deck starts in suit-major, rank-ascending order with each card
    /// twice, and is shuffled with [`SplitMix64`]. Both are frozen: changing
    /// either changes what every seed means.
    pub fn deck(seed: u64) -> Vec<Card> {
        let mut deck = Vec::with_capacity(DECK_SIZE);
        for _ in 0..2 {
            for index in 0..RANKS * SUITS {
                deck.push(Card::from_index(index));
            }
        }
        SplitMix64::new(seed).shuffle(&mut deck);
        deck
    }

    /// Deals the opening position for a seed: one face-down row, then two
    /// face-up rows, then the rest to the stock.
    pub fn deal(seed: u64) -> State {
        let deck = State::deck(seed);
        let mut columns: [Column; COLUMNS] = Default::default();
        let mut dealt = deck.iter().copied();

        for row in 0..3 {
            for column in columns.iter_mut() {
                column.push(dealt.next().expect("deck covers the initial deal"));
                if row == 0 {
                    column.hidden = 1;
                }
            }
        }

        State {
            columns,
            foundations: [0; FOUNDATIONS],
            stock: dealt.collect(),
        }
    }

    /// Stock deals still available.
    pub fn deals_remaining(&self) -> usize {
        self.stock.len().div_ceil(COLUMNS)
    }

    /// Cards sitting on the foundations.
    pub fn foundation_count(&self) -> usize {
        self.foundations.iter().map(|&rank| rank as usize).sum()
    }

    /// True when every card has been played up.
    pub fn is_won(&self) -> bool {
        self.foundations.iter().all(|&rank| rank == RANKS)
    }

    /// The card on top of a foundation slot, if any.
    pub fn foundation_top(&self, slot: u8) -> Option<Card> {
        let rank = self.foundations[slot as usize];
        (rank > 0).then(|| Card::new(foundation_suit(slot), rank))
    }

    /// The foundation slot a card may be played to, if any.
    ///
    /// When both slots of the suit would accept the card they hold identical
    /// piles, so the lower index is returned and the duplicate move is never
    /// generated.
    pub fn foundation_target(&self, card: Card) -> Option<u8> {
        foundation_slots(card.suit())
            .into_iter()
            .find(|&slot| self.foundations[slot as usize] + 1 == card.rank())
    }

    /// True when `card` may be placed on column `to`.
    fn accepts(&self, to: usize, card: Card) -> bool {
        match self.columns[to].top() {
            None => true,
            Some(base) => card.stacks_on(base),
        }
    }

    /// Every legal move from this position, subject to `options`.
    ///
    /// Rules-legal only: no dominance or pruning is applied here. That belongs
    /// in the solver, where it has to be justified.
    pub fn legal_moves(&self, options: MoveOptions) -> Vec<Move> {
        let mut moves = Vec::new();

        for (from, column) in self.columns.iter().enumerate() {
            let Some(top) = column.top() else { continue };
            if let Some(foundation) = self.foundation_target(top) {
                moves.push(Move::ToFoundation {
                    from: from as u8,
                    foundation,
                });
            }

            let run = column.movable_run();
            for count in 1..=run {
                let card = column.cards()[column.len() - count];
                for to in 0..COLUMNS {
                    if to == from || !self.accepts(to, card) {
                        continue;
                    }
                    moves.push(Move::Tableau {
                        from: from as u8,
                        to: to as u8,
                        count: count as u8,
                    });
                }
            }
        }

        if options.worry_back {
            for slot in 0..FOUNDATIONS {
                // The two slots of a suit are interchangeable, so an identical
                // pair yields one move, not two.
                if slot % 2 == 1 && self.foundations[slot] == self.foundations[slot - 1] {
                    continue;
                }
                let Some(card) = self.foundation_top(slot as u8) else {
                    continue;
                };
                for to in 0..COLUMNS {
                    if self.accepts(to, card) {
                        moves.push(Move::WorryBack {
                            foundation: slot as u8,
                            to: to as u8,
                        });
                    }
                }
            }
        }

        if !self.stock.is_empty() {
            moves.push(Move::Stock);
        }

        moves
    }

    /// Checks a move against the rules without applying it.
    pub fn check(&self, mv: Move) -> Result<(), IllegalMove> {
        let deny = |reason| {
            Err(IllegalMove {
                attempted: mv,
                reason,
            })
        };

        match mv {
            Move::Stock => {
                if self.stock.is_empty() {
                    return deny("the stock is empty");
                }
            }
            Move::Tableau { from, to, count } => {
                if from as usize >= COLUMNS || to as usize >= COLUMNS {
                    return deny("column index out of range");
                }
                if from == to {
                    return deny("source and destination are the same column");
                }
                if count == 0 {
                    return deny("a move must carry at least one card");
                }
                let source = &self.columns[from as usize];
                if count as usize > source.movable_run() {
                    return deny("those cards are not a face-up alternating-colour run");
                }
                let card = source.cards()[source.len() - count as usize];
                if !self.accepts(to as usize, card) {
                    return deny("the destination does not accept that card");
                }
            }
            Move::ToFoundation { from, foundation } => {
                if from as usize >= COLUMNS {
                    return deny("column index out of range");
                }
                if foundation as usize >= FOUNDATIONS {
                    return deny("foundation index out of range");
                }
                let Some(card) = self.columns[from as usize].top() else {
                    return deny("the column is empty");
                };
                if card.suit() != foundation_suit(foundation) {
                    return deny("wrong suit for that foundation");
                }
                if self.foundations[foundation as usize] + 1 != card.rank() {
                    return deny("the foundation does not follow that card");
                }
            }
            Move::WorryBack { foundation, to } => {
                if foundation as usize >= FOUNDATIONS {
                    return deny("foundation index out of range");
                }
                if to as usize >= COLUMNS {
                    return deny("column index out of range");
                }
                let Some(card) = self.foundation_top(foundation) else {
                    return deny("the foundation is empty");
                };
                if !self.accepts(to as usize, card) {
                    return deny("the destination does not accept that card");
                }
            }
        }

        Ok(())
    }

    /// Applies a move, leaving the state untouched if it is illegal.
    pub fn apply(&mut self, mv: Move) -> Result<(), IllegalMove> {
        self.check(mv)?;

        match mv {
            Move::Stock => {
                let dealt = self.stock.len().min(COLUMNS);
                for (offset, card) in self.stock.drain(..dealt).enumerate() {
                    self.columns[offset].push(card);
                }
            }
            Move::Tableau { from, to, count } => {
                let cards = self.columns[from as usize].take(count as usize);
                self.columns[to as usize].cards.extend(cards);
            }
            Move::ToFoundation { from, foundation } => {
                let card = self.columns[from as usize].take(1)[0];
                self.foundations[foundation as usize] = card.rank();
            }
            Move::WorryBack { foundation, to } => {
                let card = self
                    .foundation_top(foundation)
                    .expect("checked above that the foundation is not empty");
                self.foundations[foundation as usize] -= 1;
                self.columns[to as usize].push(card);
            }
        }

        Ok(())
    }

    /// Every card currently in play, for invariant checks.
    pub fn all_cards(&self) -> Vec<Card> {
        let mut cards = self.stock.clone();
        for column in &self.columns {
            cards.extend_from_slice(column.cards());
        }
        for slot in 0..FOUNDATIONS {
            let suit = foundation_suit(slot as u8);
            for rank in 1..=self.foundations[slot] {
                cards.push(Card::new(suit, rank));
            }
        }
        cards
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every card of both decks, sorted, for multiset comparisons.
    fn full_deck_sorted() -> Vec<Card> {
        let mut deck = State::deck(0);
        deck.sort_unstable();
        deck
    }

    fn sorted(mut cards: Vec<Card>) -> Vec<Card> {
        cards.sort_unstable();
        cards
    }

    /// Builds a position directly, for tests that need a specific shape.
    /// Each column is given as (face-down count, cards bottom first).
    fn position(columns: [(usize, &[Card]); COLUMNS], foundations: [u8; FOUNDATIONS]) -> State {
        let columns = columns.map(|(hidden, cards)| Column {
            cards: cards.to_vec(),
            hidden,
        });
        State {
            columns,
            foundations,
            stock: Vec::new(),
        }
    }

    const fn card(suit: Suit, rank: u8) -> Card {
        Card::new(suit, rank)
    }

    /// `DESIGN.md` proposed dropping this move as "a relabelling of the
    /// position, and nothing more". The first half of that is true: moving a
    /// whole face-up column onto an empty one exchanges the two columns and
    /// does nothing else.
    #[test]
    fn a_whole_face_up_column_onto_an_empty_one_exchanges_the_two_columns() {
        let run = [card(Suit::Spades, 8), card(Suit::Hearts, 7)];
        let mut columns: [(usize, &[Card]); COLUMNS] = [(0, &[]); COLUMNS];
        columns[0] = (0, &run);
        let before = position(columns, [0; FOUNDATIONS]);

        let mut after = before.clone();
        after
            .apply(Move::Tableau {
                from: 0,
                to: 1,
                count: 2,
            })
            .expect("a face-up run may move onto an empty column");

        let mut exchanged = before.clone();
        exchanged.columns.swap(0, 1);
        assert_eq!(after, exchanged);
    }

    /// And the second half is false while the stock holds cards, which is what
    /// makes the unconditional dominance unsound. The stock deals card *i* to
    /// column *i*, so exchanging two columns changes which card lands on the
    /// run and which lands on the empty space. Here the move is the difference
    /// between building a run and burying it.
    #[test]
    fn exchanging_two_columns_changes_what_the_stock_deals_onto_them() {
        let alone = [card(Suit::Spades, 8)];
        let mut columns: [(usize, &[Card]); COLUMNS] = [(0, &[]); COLUMNS];
        columns[0] = (0, &alone);

        // Column 0 receives the king, column 1 the seven.
        let stock = vec![card(Suit::Spades, 13), card(Suit::Hearts, 7)];

        let mut stay = position(columns, [0; FOUNDATIONS]);
        stay.stock = stock.clone();
        let mut exchange = stay.clone();
        exchange
            .apply(Move::Tableau {
                from: 0,
                to: 1,
                count: 1,
            })
            .expect("a face-up run may move onto an empty column");

        stay.apply(Move::Stock).expect("the stock is not empty");
        exchange.apply(Move::Stock).expect("the stock is not empty");

        // Staying buries the eight under a king that does not stack on it.
        assert_eq!(stay.columns[0].movable_run(), 1);
        // Exchanging puts the seven on the eight and builds a run of two.
        assert_eq!(exchange.columns[1].movable_run(), 2);

        // So the two positions are not each other relabelled: no permutation
        // of the columns turns one into the other.
        let mut stay_shapes: Vec<Vec<Card>> = stay
            .columns
            .iter()
            .map(|column| column.cards().to_vec())
            .collect();
        let mut exchange_shapes: Vec<Vec<Card>> = exchange
            .columns
            .iter()
            .map(|column| column.cards().to_vec())
            .collect();
        stay_shapes.sort();
        exchange_shapes.sort();
        assert_ne!(stay_shapes, exchange_shapes);
    }

    #[test]
    fn deck_holds_two_of_every_card() {
        let deck = State::deck(99);
        assert_eq!(deck.len(), DECK_SIZE);
        let mut counts = [0u8; (RANKS * SUITS) as usize];
        for card in deck {
            counts[card.index() as usize] += 1;
        }
        assert!(counts.iter().all(|&count| count == 2));
    }

    #[test]
    fn deals_are_reproducible_and_seed_dependent() {
        assert_eq!(State::deal(2026), State::deal(2026));
        assert_ne!(State::deal(2026), State::deal(2027));
    }

    #[test]
    fn opening_position_matches_the_ruleset() {
        let state = State::deal(5);
        assert_eq!(state.stock.len(), STOCK_AT_DEAL);
        assert_eq!(state.deals_remaining(), STOCK_DEALS);
        assert_eq!(state.foundations, [0; FOUNDATIONS]);
        assert!(!state.is_won());

        let mut face_down = 0;
        for column in &state.columns {
            assert_eq!(column.len(), 3);
            assert_eq!(column.hidden(), 1);
            assert_eq!(column.face_up().len(), 2);
            face_down += column.hidden();
        }
        assert_eq!(face_down, FACE_DOWN_AT_DEAL);
        assert_eq!(sorted(state.all_cards()), full_deck_sorted());
    }

    #[test]
    fn deal_lays_rows_across_columns_not_down_them() {
        let deck = State::deck(17);
        let state = State::deal(17);
        for (index, column) in state.columns.iter().enumerate() {
            assert_eq!(
                column.cards(),
                [
                    deck[index],
                    deck[COLUMNS + index],
                    deck[2 * COLUMNS + index]
                ]
            );
        }
        assert_eq!(state.stock, deck[3 * COLUMNS..]);
    }

    #[test]
    fn stock_deals_one_card_to_every_column_and_runs_out() {
        let mut state = State::deal(3);
        for remaining in (0..STOCK_DEALS).rev() {
            let before: Vec<usize> = state.columns.iter().map(|c| c.len()).collect();
            state.apply(Move::Stock).expect("stock deal is legal");
            for (column, length) in state.columns.iter().zip(before) {
                assert_eq!(column.len(), length + 1);
            }
            assert_eq!(state.deals_remaining(), remaining);
        }
        assert!(state.stock.is_empty());
        assert_eq!(
            state.apply(Move::Stock).unwrap_err().reason,
            "the stock is empty"
        );
        assert_eq!(sorted(state.all_cards()), full_deck_sorted());
    }

    #[test]
    fn stock_deals_onto_empty_columns_too() {
        let mut state = State::deal(11);
        state.columns[0] = Column::default();
        let next = state.stock[0];
        state.apply(Move::Stock).expect("stock deal is legal");
        assert_eq!(state.columns[0].cards(), [next]);
        assert_eq!(state.columns[0].hidden(), 0);
    }

    #[test]
    fn a_run_moves_as_a_unit_regardless_of_suit() {
        // T0 holds a buried card under 9h-8s-7d; T1 offers a black ten.
        let state = position(
            [
                (
                    1,
                    &[
                        card(Suit::Clubs, 2),
                        card(Suit::Hearts, 9),
                        card(Suit::Spades, 8),
                        card(Suit::Diamonds, 7),
                    ],
                ),
                (0, &[card(Suit::Spades, 10)]),
                (0, &[]),
                (0, &[card(Suit::Hearts, 4)]),
                (0, &[card(Suit::Hearts, 5)]),
                (0, &[card(Suit::Clubs, 6)]),
                (0, &[card(Suit::Diamonds, 3)]),
                (0, &[card(Suit::Clubs, 9)]),
            ],
            [0; FOUNDATIONS],
        );

        assert_eq!(state.columns[0].movable_run(), 3);

        let mut moved = state.clone();
        moved
            .apply(Move::Tableau {
                from: 0,
                to: 1,
                count: 3,
            })
            .expect("a three-card alternating run fits on the ten");
        assert_eq!(moved.columns[0].len(), 1);
        assert_eq!(moved.columns[0].hidden(), 0, "the buried card turns up");
        assert_eq!(moved.columns[1].len(), 4);

        // The buried card is face down, so the run cannot extend past the nine.
        assert_eq!(
            state
                .check(Move::Tableau {
                    from: 0,
                    to: 1,
                    count: 4
                })
                .unwrap_err()
                .reason,
            "those cards are not a face-up alternating-colour run"
        );
    }

    #[test]
    fn any_card_may_start_an_empty_column() {
        let mut state = position(
            [
                (0, &[card(Suit::Hearts, 5)]),
                (0, &[]),
                (0, &[]),
                (0, &[]),
                (0, &[]),
                (0, &[]),
                (0, &[]),
                (0, &[]),
            ],
            [0; FOUNDATIONS],
        );
        state
            .apply(Move::Tableau {
                from: 0,
                to: 1,
                count: 1,
            })
            .expect("an empty column takes any card");
        assert!(state.columns[0].is_empty());
        assert_eq!(state.columns[1].top(), Some(card(Suit::Hearts, 5)));
    }

    #[test]
    fn foundations_build_up_by_suit_from_an_ace() {
        let mut state = position(
            [
                (0, &[card(Suit::Hearts, 1)]),
                (0, &[card(Suit::Hearts, 2)]),
                (0, &[card(Suit::Hearts, 4)]),
                (0, &[card(Suit::Spades, 2)]),
                (0, &[]),
                (0, &[]),
                (0, &[]),
                (0, &[]),
            ],
            [0; FOUNDATIONS],
        );

        // Only an ace starts a foundation, and only on its own suit's slots.
        assert!(state
            .check(Move::ToFoundation {
                from: 1,
                foundation: 2
            })
            .is_err());
        assert!(state
            .check(Move::ToFoundation {
                from: 0,
                foundation: 0
            })
            .is_err());

        let hearts = state
            .foundation_target(card(Suit::Hearts, 1))
            .expect("ace fits");
        assert_eq!(hearts, foundation_slots(Suit::Hearts)[0]);
        state
            .apply(Move::ToFoundation {
                from: 0,
                foundation: hearts,
            })
            .unwrap();
        assert_eq!(state.foundations[hearts as usize], 1);

        // Ranks must follow in order.
        assert!(state
            .check(Move::ToFoundation {
                from: 2,
                foundation: hearts
            })
            .is_err());
        state
            .apply(Move::ToFoundation {
                from: 1,
                foundation: hearts,
            })
            .unwrap();
        assert_eq!(state.foundations[hearts as usize], 2);
        assert_eq!(state.foundation_count(), 2);
    }

    #[test]
    fn a_second_ace_opens_the_suits_other_foundation() {
        let mut state = position(
            [
                (0, &[card(Suit::Spades, 1)]),
                (0, &[card(Suit::Spades, 1)]),
                (0, &[]),
                (0, &[]),
                (0, &[]),
                (0, &[]),
                (0, &[]),
                (0, &[]),
            ],
            [0; FOUNDATIONS],
        );
        let [first, second] = foundation_slots(Suit::Spades);
        state
            .apply(Move::ToFoundation {
                from: 0,
                foundation: first,
            })
            .unwrap();
        assert_eq!(state.foundation_target(card(Suit::Spades, 1)), Some(second));
        state
            .apply(Move::ToFoundation {
                from: 1,
                foundation: second,
            })
            .unwrap();
        assert_eq!(state.foundations[first as usize], 1);
        assert_eq!(state.foundations[second as usize], 1);
    }

    #[test]
    fn worry_back_returns_a_card_to_the_tableau() {
        let mut state = position(
            [
                (0, &[card(Suit::Spades, 6)]),
                (0, &[card(Suit::Spades, 2)]),
                (0, &[]),
                (0, &[]),
                (0, &[]),
                (0, &[]),
                (0, &[]),
                (0, &[]),
            ],
            [5, 0, 0, 0, 0, 0, 0, 0],
        );

        // The five of spades cannot go under a black six.
        assert!(state
            .check(Move::WorryBack {
                foundation: 0,
                to: 0
            })
            .is_err());
        state
            .apply(Move::WorryBack {
                foundation: 0,
                to: 2,
            })
            .expect("an empty column takes it");
        assert_eq!(state.foundations[0], 4);
        assert_eq!(state.columns[2].top(), Some(card(Suit::Spades, 5)));
    }

    #[test]
    fn worry_back_moves_appear_only_when_enabled() {
        let state = position(
            [
                (0, &[card(Suit::Hearts, 6)]),
                (0, &[]),
                (0, &[]),
                (0, &[]),
                (0, &[]),
                (0, &[]),
                (0, &[]),
                (0, &[]),
            ],
            [5, 0, 0, 0, 0, 0, 0, 0],
        );

        let with = state.legal_moves(MoveOptions::ALL);
        let without = state.legal_moves(MoveOptions::NO_WORRY_BACK);
        assert!(with.contains(&Move::WorryBack {
            foundation: 0,
            to: 1
        }));
        assert!(!without
            .iter()
            .any(|mv| matches!(mv, Move::WorryBack { .. })));

        // Disabling worry-back restricts generation, never the rules.
        assert!(state
            .check(Move::WorryBack {
                foundation: 0,
                to: 1
            })
            .is_ok());
    }

    #[test]
    fn interchangeable_foundations_generate_one_move_not_two() {
        let state = position(
            [
                (0, &[card(Suit::Spades, 9)]),
                (0, &[]),
                (0, &[]),
                (0, &[]),
                (0, &[]),
                (0, &[]),
                (0, &[]),
                (0, &[]),
            ],
            [5, 5, 0, 0, 0, 0, 0, 0],
        );
        let worry_backs: Vec<Move> = state
            .legal_moves(MoveOptions::ALL)
            .into_iter()
            .filter(|mv| matches!(mv, Move::WorryBack { .. }))
            .collect();
        // Seven empty columns take the five; the second spade foundation is
        // identical to the first, so it contributes nothing.
        assert!(worry_backs
            .iter()
            .all(|mv| matches!(mv, Move::WorryBack { foundation: 0, .. })));
        assert_eq!(worry_backs.len(), COLUMNS - 1);
    }

    #[test]
    fn generated_moves_are_legal_and_unique() {
        let mut state = State::deal(2024);
        let mut rng = SplitMix64::new(77);
        for _ in 0..400 {
            let moves = state.legal_moves(MoveOptions::ALL);
            if moves.is_empty() {
                break;
            }
            let mut seen = moves.clone();
            seen.sort_unstable();
            seen.dedup();
            assert_eq!(seen.len(), moves.len(), "duplicate move generated");
            for &mv in &moves {
                state.check(mv).expect("generated move should be legal");
            }
            let pick = rng.below(moves.len() as u64) as usize;
            state.apply(moves[pick]).expect("chosen move is legal");
        }
    }

    #[test]
    fn random_play_never_loses_or_invents_a_card() {
        let expected = full_deck_sorted();
        for seed in 0..40 {
            let mut state = State::deal(seed);
            let mut rng = SplitMix64::new(seed ^ 0xABCD);
            for _ in 0..300 {
                assert_eq!(sorted(state.all_cards()), expected, "seed {seed}");
                assert_eq!(
                    state.foundation_count()
                        + state.stock.len()
                        + state.columns.iter().map(|c| c.len()).sum::<usize>(),
                    DECK_SIZE
                );
                for column in &state.columns {
                    assert!(
                        column.hidden() < column.len() || column.is_empty(),
                        "a face-down card was left on top"
                    );
                }
                let moves = state.legal_moves(MoveOptions::ALL);
                if moves.is_empty() {
                    break;
                }
                let pick = rng.below(moves.len() as u64) as usize;
                state.apply(moves[pick]).expect("chosen move is legal");
            }
        }
    }

    #[test]
    fn illegal_moves_leave_the_position_untouched() {
        let state = State::deal(8);
        for mv in [
            Move::Tableau {
                from: 0,
                to: 0,
                count: 1,
            },
            Move::Tableau {
                from: 0,
                to: 9,
                count: 1,
            },
            Move::Tableau {
                from: 0,
                to: 1,
                count: 0,
            },
            Move::Tableau {
                from: 0,
                to: 1,
                count: 40,
            },
            Move::ToFoundation {
                from: 0,
                foundation: 8,
            },
            Move::WorryBack {
                foundation: 0,
                to: 1,
            },
        ] {
            let mut attempt = state.clone();
            assert!(attempt.apply(mv).is_err(), "{mv} should be illegal");
            assert_eq!(attempt, state, "{mv} changed the position");
        }
    }

    /// Pins one deal exactly. Seeds are published alongside results, so a
    /// change to the deck order or the shuffle silently invalidates every
    /// number already reported. If this test fails, that is what happened.
    #[test]
    fn seed_42_deals_the_same_cards_it_always_has() {
        let state = State::deal(42);
        let hidden: Vec<String> = state
            .columns
            .iter()
            .map(|c| c.cards()[0].to_string())
            .collect();
        let first_row: Vec<String> = state
            .columns
            .iter()
            .map(|c| c.cards()[1].to_string())
            .collect();
        let second_row: Vec<String> = state
            .columns
            .iter()
            .map(|c| c.cards()[2].to_string())
            .collect();

        assert_eq!(hidden, ["Qh", "6s", "9s", "5c", "Ah", "Th", "8d", "4c"]);
        assert_eq!(first_row, ["5s", "8c", "3c", "Js", "8s", "Qs", "Ac", "Qs"]);
        assert_eq!(second_row, ["5h", "7c", "Jh", "Kc", "9h", "3h", "Ts", "Ah"]);
        assert_eq!(
            state.stock[..8]
                .iter()
                .map(Card::to_string)
                .collect::<Vec<_>>(),
            ["9c", "5h", "4d", "4c", "Jd", "8h", "6d", "Jc"]
        );
    }

    #[test]
    fn a_full_set_of_foundations_is_a_win() {
        let mut state = State::deal(1);
        assert!(!state.is_won());
        state.foundations = [RANKS; FOUNDATIONS];
        assert!(state.is_won());
        assert_eq!(state.foundation_count(), DECK_SIZE);
    }
}
