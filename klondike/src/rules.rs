//! Klondike: position, deal, legal actions and application.
//!
//! The variant is the one Solvitaire's 81.945% ± 0.084% thoughtful figure was
//! measured on: a 24-card stock drawn three at a time, redeals without limit,
//! and cards may be worried back from the foundations to the tableau. Any
//! other variant is a different number and validates nothing.
//!
//! Two rules were not confirmed from the paper and follow the near-universal
//! convention: only a king may be placed on an empty pile, and any correctly
//! sequenced suffix of a pile's face-up cards may be moved, not just the whole
//! run. If the measured rate misses the target, these are the first suspects.

use std::fmt;
use std::str::FromStr;

use gypsy_core::card::{RANKS, SUITS};
use gypsy_core::rng::SplitMix64;
use gypsy_core::{Card, Suit};

/// Tableau piles.
pub const PILES: usize = 7;
/// One foundation per suit.
pub const FOUNDATIONS: usize = 4;
/// One standard deck.
pub const DECK_SIZE: usize = 52;
/// Cards dealt to the tableau: 1 + 2 + ... + 7.
pub const DEALT_TO_TABLEAU: usize = PILES * (PILES + 1) / 2;
/// Cards left in the stock after the deal.
pub const STOCK_AT_DEAL: usize = DECK_SIZE - DEALT_TO_TABLEAU;
/// Cards turned over per draw.
pub const DRAW: usize = 3;

/// One tableau pile, bottom card first. The first `hidden` are face down.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Pile {
    cards: Vec<Card>,
    hidden: usize,
}

impl Pile {
    pub fn cards(&self) -> &[Card] {
        &self.cards
    }

    pub fn hidden(&self) -> usize {
        self.hidden
    }

    pub fn face_up(&self) -> &[Card] {
        &self.cards[self.hidden..]
    }

    pub fn len(&self) -> usize {
        self.cards.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }

    pub fn top(&self) -> Option<Card> {
        self.cards.last().copied()
    }

    /// Length of the descending alternating-colour run at the top, counting
    /// only face-up cards.
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

    fn take(&mut self, count: usize) -> Vec<Card> {
        let taken = self.cards.split_off(self.cards.len() - count);
        if self.hidden > 0 && self.hidden >= self.cards.len() {
            self.hidden = self.cards.len().saturating_sub(1);
        }
        taken
    }
}

/// A complete Klondike position.
///
/// The stock and the waste are one list. Cards `[0, turned)` are the waste,
/// its top being `talon[turned - 1]`; the rest is the stock, next card first.
/// A redeal is `turned = 0`, which is exactly what flipping the waste back
/// does: the order is preserved.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Position {
    pub piles: [Pile; PILES],
    /// Top rank of each suit's foundation; 0 when empty.
    pub foundations: [u8; FOUNDATIONS],
    /// Stock and waste in one fixed order.
    pub talon: Vec<Card>,
    /// How many of the talon have been turned over this pass.
    pub turned: usize,
}

/// One move.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Action {
    /// Turn three from the stock, or recycle the waste when the stock is out.
    Draw,
    /// Play the top of the waste onto a tableau pile.
    WasteToPile { to: u8 },
    /// Play the top of the waste to its foundation.
    WasteToFoundation,
    /// Play the top of a pile to its foundation.
    PileToFoundation { from: u8 },
    /// Worry back: a foundation's top card onto a tableau pile.
    FoundationToPile { foundation: u8, to: u8 },
    /// Move `count` cards between piles.
    PileToPile { from: u8, to: u8, count: u8 },
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Action::Draw => f.write_str("D"),
            Action::WasteToPile { to } => write!(f, "W>T{to}"),
            Action::WasteToFoundation => f.write_str("W>F"),
            Action::PileToFoundation { from } => write!(f, "T{from}>F"),
            Action::FoundationToPile { foundation, to } => write!(f, "F{foundation}>T{to}"),
            Action::PileToPile { from, to, count } => write!(f, "T{from}>T{to}:{count}"),
        }
    }
}

/// Error returned when an action cannot be parsed.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ParseActionError(String);

impl fmt::Display for ParseActionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "cannot parse action {:?}", self.0)
    }
}

impl std::error::Error for ParseActionError {}

impl FromStr for Action {
    type Err = ParseActionError;

    fn from_str(s: &str) -> Result<Action, ParseActionError> {
        let text = s.trim();
        let fail = || ParseActionError(s.to_string());
        if text.eq_ignore_ascii_case("d") {
            return Ok(Action::Draw);
        }
        let (source, rest) = text.split_once('>').ok_or_else(fail)?;
        let (destination, count) = match rest.split_once(':') {
            Some((destination, count)) => {
                (destination, Some(count.parse::<u8>().map_err(|_| fail())?))
            }
            None => (rest, None),
        };
        let index = |part: &str| part[1..].parse::<u8>().map_err(|_| fail());

        match (source, destination) {
            ("W", "F") => Ok(Action::WasteToFoundation),
            ("W", d) if d.starts_with('T') => Ok(Action::WasteToPile { to: index(d)? }),
            (s, "F") if s.starts_with('T') => Ok(Action::PileToFoundation { from: index(s)? }),
            (s, d) if s.starts_with('F') && d.starts_with('T') => Ok(Action::FoundationToPile {
                foundation: index(s)?,
                to: index(d)?,
            }),
            (s, d) if s.starts_with('T') && d.starts_with('T') => Ok(Action::PileToPile {
                from: index(s)?,
                to: index(d)?,
                count: count.unwrap_or(1),
            }),
            _ => Err(fail()),
        }
    }
}

impl Position {
    /// The shuffled deck for a seed, using the same frozen generator as Gypsy.
    pub fn deck(seed: u64) -> Vec<Card> {
        let mut deck: Vec<Card> = (0..RANKS * SUITS).map(Card::from_index).collect();
        SplitMix64::new(seed).shuffle(&mut deck);
        deck
    }

    /// Deals a position: pile *i* gets *i + 1* cards with only the last face
    /// up, and the remaining 24 become the stock.
    ///
    /// Cards are taken pile by pile rather than row by row. The usual deal
    /// interleaves them, but both take a uniformly shuffled deck to a
    /// uniformly distributed position, and this one is easier to check.
    pub fn deal(seed: u64) -> Position {
        let deck = Position::deck(seed);
        let mut dealt = deck.iter().copied();
        let piles = std::array::from_fn(|index| Pile {
            cards: (0..=index)
                .map(|_| dealt.next().expect("deck covers the deal"))
                .collect(),
            hidden: index,
        });
        Position {
            piles,
            foundations: [0; FOUNDATIONS],
            talon: dealt.collect(),
            turned: 0,
        }
    }

    pub fn is_won(&self) -> bool {
        self.foundations.iter().all(|&rank| rank == RANKS)
    }

    /// The card on top of the waste, if any.
    pub fn waste_top(&self) -> Option<Card> {
        (self.turned > 0).then(|| self.talon[self.turned - 1])
    }

    /// The card on top of a suit's foundation, if any.
    pub fn foundation_top(&self, suit: u8) -> Option<Card> {
        let rank = self.foundations[suit as usize];
        (rank > 0).then(|| Card::new(Suit::from_index(suit), rank))
    }

    /// True when the foundations are ready to accept this card.
    fn foundation_accepts(&self, card: Card) -> bool {
        self.foundations[card.suit().index() as usize] + 1 == card.rank()
    }

    /// True when `card` may be placed on pile `to`. Only a king starts an
    /// empty pile.
    fn accepts(&self, to: usize, card: Card) -> bool {
        match self.piles[to].top() {
            None => card.rank() == RANKS,
            Some(base) => card.stacks_on(base),
        }
    }

    /// Every action the rules permit. No dominances, as in the Gypsy engine.
    pub fn legal_actions(&self) -> Vec<Action> {
        let mut actions = Vec::new();

        if let Some(card) = self.waste_top() {
            if self.foundation_accepts(card) {
                actions.push(Action::WasteToFoundation);
            }
            for to in 0..PILES {
                if self.accepts(to, card) {
                    actions.push(Action::WasteToPile { to: to as u8 });
                }
            }
        }

        for (from, pile) in self.piles.iter().enumerate() {
            if let Some(top) = pile.top() {
                if self.foundation_accepts(top) {
                    actions.push(Action::PileToFoundation { from: from as u8 });
                }
            }
            for count in 1..=pile.movable_run() {
                let card = pile.cards()[pile.len() - count];
                for to in 0..PILES {
                    if to != from && self.accepts(to, card) {
                        actions.push(Action::PileToPile {
                            from: from as u8,
                            to: to as u8,
                            count: count as u8,
                        });
                    }
                }
            }
        }

        for suit in 0..FOUNDATIONS {
            let Some(card) = self.foundation_top(suit as u8) else {
                continue;
            };
            for to in 0..PILES {
                if self.accepts(to, card) {
                    actions.push(Action::FoundationToPile {
                        foundation: suit as u8,
                        to: to as u8,
                    });
                }
            }
        }

        // Drawing deals three; when the stock is spent it recycles the waste,
        // which is legal without limit in this variant. Neither is available
        // once every card has left the talon.
        if !self.talon.is_empty() {
            actions.push(Action::Draw);
        }

        actions
    }

    /// Applies an action, leaving the position untouched if it is illegal.
    pub fn apply(&mut self, action: Action) -> Result<(), String> {
        let deny = |reason: &str| Err(format!("illegal action {action}: {reason}"));

        match action {
            Action::Draw => {
                if self.talon.is_empty() {
                    return deny("there is no stock and no waste");
                }
                self.turned = if self.turned >= self.talon.len() {
                    0 // recycle
                } else {
                    (self.turned + DRAW).min(self.talon.len())
                };
            }
            Action::WasteToFoundation => {
                let Some(card) = self.waste_top() else {
                    return deny("the waste is empty");
                };
                if !self.foundation_accepts(card) {
                    return deny("the foundation does not follow that card");
                }
                self.talon.remove(self.turned - 1);
                self.turned -= 1;
                self.foundations[card.suit().index() as usize] = card.rank();
            }
            Action::WasteToPile { to } => {
                if to as usize >= PILES {
                    return deny("pile index out of range");
                }
                let Some(card) = self.waste_top() else {
                    return deny("the waste is empty");
                };
                if !self.accepts(to as usize, card) {
                    return deny("the destination does not accept that card");
                }
                self.talon.remove(self.turned - 1);
                self.turned -= 1;
                self.piles[to as usize].cards.push(card);
            }
            Action::PileToFoundation { from } => {
                if from as usize >= PILES {
                    return deny("pile index out of range");
                }
                let Some(card) = self.piles[from as usize].top() else {
                    return deny("the pile is empty");
                };
                if !self.foundation_accepts(card) {
                    return deny("the foundation does not follow that card");
                }
                self.piles[from as usize].take(1);
                self.foundations[card.suit().index() as usize] = card.rank();
            }
            Action::FoundationToPile { foundation, to } => {
                if foundation as usize >= FOUNDATIONS || to as usize >= PILES {
                    return deny("index out of range");
                }
                let Some(card) = self.foundation_top(foundation) else {
                    return deny("the foundation is empty");
                };
                if !self.accepts(to as usize, card) {
                    return deny("the destination does not accept that card");
                }
                self.foundations[foundation as usize] -= 1;
                self.piles[to as usize].cards.push(card);
            }
            Action::PileToPile { from, to, count } => {
                if from as usize >= PILES || to as usize >= PILES {
                    return deny("pile index out of range");
                }
                if from == to {
                    return deny("source and destination are the same pile");
                }
                if count == 0 {
                    return deny("a move must carry at least one card");
                }
                let source = &self.piles[from as usize];
                if count as usize > source.movable_run() {
                    return deny("those cards are not a face-up alternating-colour run");
                }
                let card = source.cards()[source.len() - count as usize];
                if !self.accepts(to as usize, card) {
                    return deny("the destination does not accept that card");
                }
                let moved = self.piles[from as usize].take(count as usize);
                self.piles[to as usize].cards.extend(moved);
            }
        }

        Ok(())
    }

    /// Builds a position directly. Each pile is (face-down count, cards
    /// bottom first). Test-only: the fields are private so that the deal and
    /// `apply` stay the only ways to reach a position in real use.
    #[cfg(test)]
    pub(crate) fn from_parts(
        piles: [(usize, &[Card]); PILES],
        foundations: [u8; FOUNDATIONS],
        talon: &[Card],
        turned: usize,
    ) -> Position {
        Position {
            piles: piles.map(|(hidden, cards)| Pile {
                cards: cards.to_vec(),
                hidden,
            }),
            foundations,
            talon: talon.to_vec(),
            turned,
        }
    }

    /// Every card in play, for invariant checks.
    pub fn all_cards(&self) -> Vec<Card> {
        let mut cards = self.talon.clone();
        for pile in &self.piles {
            cards.extend_from_slice(pile.cards());
        }
        for suit in 0..FOUNDATIONS {
            for rank in 1..=self.foundations[suit] {
                cards.push(Card::new(Suit::from_index(suit as u8), rank));
            }
        }
        cards
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sorted(mut cards: Vec<Card>) -> Vec<Card> {
        cards.sort_unstable();
        cards
    }

    fn full_deck_sorted() -> Vec<Card> {
        sorted(Position::deck(0))
    }

    const fn card(suit: Suit, rank: u8) -> Card {
        Card::new(suit, rank)
    }

    use Position as P;
    fn position(
        piles: [(usize, &[Card]); PILES],
        foundations: [u8; FOUNDATIONS],
        talon: &[Card],
        turned: usize,
    ) -> Position {
        P::from_parts(piles, foundations, talon, turned)
    }

    #[test]
    fn the_deck_holds_one_of_every_card() {
        let deck = Position::deck(31);
        assert_eq!(deck.len(), DECK_SIZE);
        assert_eq!(sorted(deck), full_deck_sorted());
    }

    #[test]
    fn the_deal_matches_the_ruleset() {
        let state = Position::deal(5);
        for (index, pile) in state.piles.iter().enumerate() {
            assert_eq!(pile.len(), index + 1);
            assert_eq!(pile.hidden(), index);
            assert_eq!(pile.face_up().len(), 1);
        }
        assert_eq!(state.talon.len(), STOCK_AT_DEAL);
        assert_eq!(state.talon.len(), 24);
        assert_eq!(state.turned, 0);
        assert_eq!(state.foundations, [0; FOUNDATIONS]);
        assert!(!state.is_won());
        assert_eq!(sorted(state.all_cards()), full_deck_sorted());
    }

    #[test]
    fn deals_are_reproducible_and_seed_dependent() {
        assert_eq!(Position::deal(2026), Position::deal(2026));
        assert_ne!(Position::deal(2026), Position::deal(2027));
    }

    #[test]
    fn a_draw_turns_three() {
        let mut state = Position::deal(8);
        assert!(state.waste_top().is_none());
        state.apply(Action::Draw).expect("drawing is legal");
        assert_eq!(state.turned, 3);
        assert_eq!(state.waste_top(), Some(state.talon[2]));
    }

    /// The behaviour that makes draw-three more than a detail: playing a card
    /// off the waste shifts every later triple.
    #[test]
    fn playing_the_waste_reshapes_the_later_triples() {
        let cards: Vec<Card> = (0..6).map(Card::from_index).collect();
        let mut state = position(
            [(0, &[]); PILES],
            // Every foundation one below the card that will be played.
            [0; FOUNDATIONS],
            &cards,
            0,
        );
        state.foundations[cards[2].suit().index() as usize] = cards[2].rank() - 1;

        state.apply(Action::Draw).expect("drawing is legal");
        assert_eq!(state.waste_top(), Some(cards[2]));
        state
            .apply(Action::WasteToFoundation)
            .expect("the foundation follows that card");
        assert_eq!(state.turned, 2);
        assert_eq!(state.waste_top(), Some(cards[1]));

        // The next draw turns what were the fourth, fifth and sixth cards.
        state.apply(Action::Draw).expect("drawing is legal");
        assert_eq!(state.waste_top(), Some(cards[5]));
    }

    #[test]
    fn a_redeal_preserves_order_and_repeats_without_limit() {
        let mut state = Position::deal(12);
        let talon = state.talon.clone();
        for _ in 0..8 {
            state.apply(Action::Draw).expect("drawing is legal");
        }
        assert_eq!(state.turned, 24);

        state.apply(Action::Draw).expect("recycling is legal");
        assert_eq!(state.turned, 0, "the waste goes back under the stock");
        assert_eq!(state.talon, talon, "and keeps its order");

        // Round again, as many times as wanted.
        for _ in 0..9 {
            state.apply(Action::Draw).expect("recycling is legal");
        }
        assert_eq!(state.turned, 0);
        assert_eq!(state.talon, talon);
    }

    #[test]
    fn only_a_king_starts_an_empty_pile() {
        let king = card(Suit::Spades, 13);
        let queen = card(Suit::Hearts, 12);
        let mut state = position([(0, &[]); PILES], [0; FOUNDATIONS], &[queen, king], 2);

        assert_eq!(state.waste_top(), Some(king));
        assert!(state
            .legal_actions()
            .contains(&Action::WasteToPile { to: 0 }));
        state
            .apply(Action::WasteToPile { to: 0 })
            .expect("a king may start an empty pile");

        assert_eq!(state.waste_top(), Some(queen));
        assert!(
            !state
                .legal_actions()
                .contains(&Action::WasteToPile { to: 1 }),
            "a queen may not start an empty pile"
        );
        // But it may go on the king, which is one rank up and the other colour.
        assert!(state
            .legal_actions()
            .contains(&Action::WasteToPile { to: 0 }));
    }

    #[test]
    fn foundations_build_up_by_suit_from_an_ace() {
        let ace = card(Suit::Clubs, 1);
        let two = card(Suit::Clubs, 2);
        let mut state = position(
            [
                (0, &[two]),
                (0, &[ace]),
                (0, &[]),
                (0, &[]),
                (0, &[]),
                (0, &[]),
                (0, &[]),
            ],
            [0; FOUNDATIONS],
            &[],
            0,
        );

        // The two cannot go up before the ace.
        assert!(state.apply(Action::PileToFoundation { from: 0 }).is_err());
        state
            .apply(Action::PileToFoundation { from: 1 })
            .expect("an ace starts a foundation");
        state
            .apply(Action::PileToFoundation { from: 0 })
            .expect("the two follows the ace");
        assert_eq!(state.foundations[Suit::Clubs.index() as usize], 2);
    }

    #[test]
    fn a_card_can_be_worried_back_from_a_foundation() {
        let king = card(Suit::Spades, 13);
        let queen = card(Suit::Hearts, 12);
        let mut foundations = [0; FOUNDATIONS];
        foundations[Suit::Hearts.index() as usize] = 12;
        let mut state = position(
            [
                (0, &[king]),
                (0, &[]),
                (0, &[]),
                (0, &[]),
                (0, &[]),
                (0, &[]),
                (0, &[]),
            ],
            foundations,
            &[],
            0,
        );

        let worry = Action::FoundationToPile {
            foundation: Suit::Hearts.index(),
            to: 0,
        };
        assert!(state.legal_actions().contains(&worry));
        state.apply(worry).expect("worry-back is legal here");
        assert_eq!(state.piles[0].top(), Some(queen));
        assert_eq!(state.foundations[Suit::Hearts.index() as usize], 11);
    }

    #[test]
    fn a_suffix_of_a_run_moves_as_a_unit() {
        // Pile 0 holds a buried card under 9h-8s-7d; pile 1 offers a black ten.
        let run = [
            card(Suit::Clubs, 2),
            card(Suit::Hearts, 9),
            card(Suit::Spades, 8),
            card(Suit::Diamonds, 7),
        ];
        let state = position(
            [
                (1, &run),
                (0, &[card(Suit::Spades, 10)]),
                (0, &[]),
                (0, &[]),
                (0, &[]),
                (0, &[]),
                (0, &[]),
            ],
            [0; FOUNDATIONS],
            &[],
            0,
        );

        assert_eq!(state.piles[0].movable_run(), 3);
        let actions = state.legal_actions();
        assert!(actions.contains(&Action::PileToPile {
            from: 0,
            to: 1,
            count: 3
        }));
        // The eight is red-black-wrong for the ten, so only the whole run fits.
        assert!(!actions.contains(&Action::PileToPile {
            from: 0,
            to: 1,
            count: 2
        }));
    }

    #[test]
    fn emptying_a_pile_turns_up_the_card_beneath() {
        let mut state = position(
            [
                (1, &[card(Suit::Clubs, 4), card(Suit::Hearts, 12)]),
                (0, &[card(Suit::Spades, 13)]),
                (0, &[]),
                (0, &[]),
                (0, &[]),
                (0, &[]),
                (0, &[]),
            ],
            [0; FOUNDATIONS],
            &[],
            0,
        );
        state
            .apply(Action::PileToPile {
                from: 0,
                to: 1,
                count: 1,
            })
            .expect("a red queen goes on a black king");
        assert_eq!(state.piles[0].hidden(), 0);
        assert_eq!(state.piles[0].top(), Some(card(Suit::Clubs, 4)));
    }

    #[test]
    fn generated_actions_are_legal_and_unique() {
        let mut state = Position::deal(21);
        for _ in 0..40 {
            let actions = state.legal_actions();
            let mut seen = actions.clone();
            seen.sort_unstable();
            seen.dedup();
            assert_eq!(seen.len(), actions.len(), "duplicate action generated");
            for &action in &actions {
                let mut probe = state.clone();
                probe.apply(action).expect("generated actions are legal");
            }
            let Some(&first) = actions.first() else { break };
            state.apply(first).expect("generated actions are legal");
        }
    }

    #[test]
    fn random_play_never_loses_or_invents_a_card() {
        let mut rng = SplitMix64::new(0xA11CE);
        for seed in 0..20 {
            let mut state = Position::deal(seed);
            for _ in 0..300 {
                let actions = state.legal_actions();
                if actions.is_empty() {
                    break;
                }
                let pick = rng.below(actions.len() as u64) as usize;
                state
                    .apply(actions[pick])
                    .expect("generated action is legal");
                assert_eq!(sorted(state.all_cards()), full_deck_sorted());
                assert!(state.turned <= state.talon.len());
            }
        }
    }

    #[test]
    fn illegal_actions_leave_the_position_untouched() {
        let state = Position::deal(6);
        for action in [
            Action::WasteToFoundation,
            Action::WasteToPile { to: 0 },
            Action::PileToFoundation { from: 9 },
            Action::PileToPile {
                from: 0,
                to: 0,
                count: 1,
            },
            Action::PileToPile {
                from: 0,
                to: 1,
                count: 5,
            },
            Action::FoundationToPile {
                foundation: 0,
                to: 0,
            },
        ] {
            let mut probe = state.clone();
            assert!(probe.apply(action).is_err(), "{action} should be illegal");
            assert_eq!(probe, state, "{action} changed the position anyway");
        }
    }

    #[test]
    fn notation_round_trips() {
        for expected in [
            Action::Draw,
            Action::WasteToFoundation,
            Action::WasteToPile { to: 3 },
            Action::PileToFoundation { from: 6 },
            Action::FoundationToPile {
                foundation: 2,
                to: 4,
            },
            Action::PileToPile {
                from: 1,
                to: 5,
                count: 7,
            },
        ] {
            let text = expected.to_string();
            assert_eq!(text.parse::<Action>(), Ok(expected), "round trip of {text}");
        }
    }

    #[test]
    fn a_full_set_of_foundations_is_a_win() {
        let state = position([(0, &[]); PILES], [13; FOUNDATIONS], &[], 0);
        assert!(state.is_won());
    }
}
