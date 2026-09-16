//! Klondike as the search sees it: move ordering, the one dominance, and the
//! [`Game`] implementation that ties them to the rules.

use gypsy_core::rng::SplitMix64;
use gypsy_core::{Card, Suit};
use gypsy_solver::Game;

use crate::rules::{Action, MoveOptions, Position};
use crate::zobrist::Zobrist;

/// Klondike under a chosen ruleset.
///
/// The variant is Solvitaire's — a 24-card stock drawn three at a time and
/// redeals without limit — and `options` decides whether the search may worry
/// back. Only [`MoveOptions::ALL`] validates against the published 81.945%;
/// the restricted arm exists because a dominance provable only with
/// worry-back off never fires in the full game, so the full game cannot check
/// it. See `DECISIONS.md`.
pub struct Klondike {
    zobrist: Zobrist,
    options: MoveOptions,
}

impl Klondike {
    pub fn new(options: MoveOptions) -> Klondike {
        Klondike {
            zobrist: Zobrist::new(),
            options,
        }
    }
}

impl Default for Klondike {
    fn default() -> Klondike {
        Klondike::new(MoveOptions::ALL)
    }
}

/// True when no card can ever want to sit on this one again.
///
/// A tableau card of rank *r* and colour *C* is useful in the tableau for
/// exactly one thing: being a base for a card of rank *r-1* and the opposite
/// colour. One deck, one foundation per suit, so that is **two** piles, and
/// both must have passed *r-1*. This is the familiar single-deck rule, and it
/// is deliberately a second implementation rather than a reuse of the Gypsy
/// one, which checks four piles because two decks give each suit two of them.
/// Neither is correct for the other game.
///
/// Aces and twos are always safe. Nothing stacks on an ace; and the only card
/// that stacks on a two is an ace, which never needs a base, because an ace
/// off the foundations means its suit's foundation is empty — one deck, one
/// ace per suit — and so will take it at any time.
fn never_wanted_in_the_tableau(position: &Position, card: Card) -> bool {
    if card.rank() <= 2 {
        return true;
    }
    let wanted = card.rank() - 1;
    Suit::ALL
        .iter()
        .filter(|suit| suit.is_red() != card.is_red())
        .all(|suit| position.foundations[suit.index() as usize] >= wanted)
}

/// The first safe foundation play from a *pile top*, if there is one.
///
/// The waste is excluded on purpose, and it is not caution — see the proof on
/// [`Klondike::legal_actions`].
fn safe_autoplay(position: &Position) -> Option<Action> {
    position.piles.iter().enumerate().find_map(|(from, pile)| {
        let card = pile.top()?;
        (position.foundation_accepts(card) && never_wanted_in_the_tableau(position, card))
            .then_some(Action::PileToFoundation { from: from as u8 })
    })
}

/// True when this move carries a strict suffix of a built run and the card it
/// would expose has nowhere to go.
///
/// Blake & Gent's Theorem 4 in the weaker form Solvitaire implements: require
/// only that the exposed card *could* be built, not that the next move builds
/// it. Splitting a run frees the card beneath; splitting it when that card is
/// dead is a shuffle. See `legal_actions` for why Klondike needs no gate.
fn splits_a_run_for_nothing(position: &Position, action: Action) -> bool {
    let Action::PileToPile { from, count, .. } = action else {
        return false;
    };
    let pile = &position.piles[from as usize];
    let count = count as usize;
    // Moving the whole run, or the whole pile, leaves no card of the run
    // behind to be exposed.
    if count >= pile.movable_run() || count >= pile.len() {
        return false;
    }
    let exposed = pile.cards()[pile.len() - count - 1];
    !position.foundation_accepts(exposed)
}

/// Which band of the move ordering this action falls in. Lower is tried first.
///
/// An ordering and nothing else — every band is searched. It is a function of
/// its own so that a salted search can shuffle within a band, keeping the
/// heuristic while changing the descent.
fn ordering_class(position: &Position, action: Action) -> u8 {
    match action {
        Action::WasteToFoundation => 0,
        Action::PileToFoundation { .. } => 1,
        Action::PileToPile { from, to, count } => {
            let source = &position.piles[from as usize];
            if source.hidden() > 0 && count as usize == source.len() - source.hidden() {
                2 // turns up a buried card
            } else if source.hidden() == 0 && count as usize == source.len() {
                // Moving a whole face-up pile onto an empty one is a
                // relabelling; a king already sitting alone is the case.
                if position.piles[to as usize].is_empty() {
                    7
                } else {
                    4
                }
            } else {
                4
            }
        }
        Action::WasteToPile { .. } => 3,
        Action::Draw => 5,
        Action::FoundationToPile { .. } => 6,
    }
}

/// Shuffles each band of an already-sorted action list, seeded from the
/// position and the salt so the order depends on where the search is rather
/// than on how it got there.
fn shuffle_within_classes(
    position: &Position,
    actions: &mut [Action],
    salt: u64,
    zobrist: &Zobrist,
) {
    let mut rng = SplitMix64::new((zobrist.key(position) as u64) ^ salt);
    let mut start = 0;
    while start < actions.len() {
        let class = ordering_class(position, actions[start]);
        let mut end = start + 1;
        while end < actions.len() && ordering_class(position, actions[end]) == class {
            end += 1;
        }
        rng.shuffle(&mut actions[start..end]);
        start = end;
    }
}

impl Game for Klondike {
    type Position = Position;
    type Action = Action;

    /// Rules-legal actions, reordered, with safe autoplay applied in the game
    /// where it is provable.
    ///
    /// # Safe autoplay
    ///
    /// When a pile's top card is safe by [`never_wanted_in_the_tableau`] the
    /// search plays it and considers nothing else at this position.
    ///
    /// **The argument, with worry-back off.** Let `L` win from this position
    /// and let `X` be the safe card, on top of pile `p`. Winning puts every
    /// card up, so `L` plays `X` up at some point. Build `L'`: play `X` up
    /// first, then follow `L` with that one play removed. Every move of `L'`
    /// is legal. No move of `L` puts a card on `X`, because the only cards
    /// that could are the two opposite-colour cards of rank *r-1*, both
    /// already on foundations and — this is the whole gate — unable to leave
    /// them with worry-back off. A move of `L` carrying a run that includes
    /// `X` carries `X` plus cards below it, because a run is a suffix and `X`
    /// is on top; drop `X` from it and the move still works, since the
    /// destination only tests the run's bottom card. A move carrying `X` alone
    /// is the removed play. In the window between, `p` is empty or shorter in
    /// `L'` than in `L`, and nothing `L` does needs it otherwise: placing onto
    /// `p` in that window would mean placing onto `X`, which is impossible.
    /// Turning up `p`'s next card earlier only adds options. So `L'` wins, and
    /// restricting this position to that one move cannot lose a win.
    ///
    /// **Why the waste is excluded.** The same reordering is unsound for a
    /// safe card on top of the waste, and the failure is specific to Klondike.
    /// Playing it up removes it from the talon, which shifts every card behind
    /// it down one index and moves `turned` back — so every later `Draw` turns
    /// a different group of three — the rules test
    /// `playing_the_waste_reshapes_the_later_triples` pins exactly that. `L'`
    /// is then playing a different sequence, and its `Draw`s no longer expose
    /// the cards its later moves need, so the step that carries the pile case,
    /// "the rest of `L` is still legal", simply fails. `WasteToFoundation`
    /// therefore stays an ordinary action among the alternatives. Gypsy has no
    /// waste and no such case.
    ///
    /// **Why worry-back breaks the rule itself.** The gate is the load-bearing
    /// step, not caution. With worry-back legal, "on a foundation" stops
    /// meaning "out of the tableau for good": the two cards the condition
    /// checks can come back down, and one of them may then want `X` under it.
    /// The tempting repair — play it up, worry it back if it is ever wanted —
    /// is circular under a transposition table, and is written up in
    /// `DECISIONS.md` so nobody re-derives it.
    ///
    /// # Splitting a built run for nothing
    ///
    /// Blake & Gent's Theorem 4 (JAIR 85, Appendix B.2), which for Klondike
    /// applies as published, with no gate. Every hypothesis holds here:
    ///
    /// - **Indistinguishable build policy.** Two cards of the same rank and
    ///   colour can be built on exactly the same cards; any other pair shares
    ///   none. Empty piles take kings only, so a king's build destinations are
    ///   empty and disjoint from every other card's — the case that forces
    ///   Gypsy to gate here does not arise.
    /// - **One policy for groups and single cards.** `legal_actions` generates
    ///   `PileToPile` for every prefix of the movable run under the same test.
    /// - **Cards leave the tableau only for another pile or a foundation.**
    /// - **Won by moving every card to a foundation.**
    /// - **No rule makes a move's legality depend on its position in the
    ///   sequence.** The stock deals to the *waste*, never to the piles, so a
    ///   tableau move and a `Draw` are genuinely unrelated and the proof may
    ///   swap them. Gypsy's stock deals to every column and cannot.
    ///
    /// Worry-back is no obstacle: the hypotheses restrict where a card may go
    /// *from* the tableau, not what may arrive in it, and Solvitaire ships this
    /// rule for Klondike with removable foundations — the published variant.
    ///
    /// This is why the rule is validated here rather than on Gypsy. It fires in
    /// the arm with the 81.945% bracket, on the only deal set this project has
    /// that proves deals unsolvable, which is the direction a wrong dominance
    /// fails in.
    fn legal_actions(&self, position: &Position, salt: u64) -> Vec<Action> {
        if !self.options.worry_back {
            if let Some(autoplay) = safe_autoplay(position) {
                return vec![autoplay];
            }
        }

        let mut actions = position.legal_actions(self.options);
        actions.retain(|action| !splits_a_run_for_nothing(position, *action));
        actions.sort_by_key(|action| ordering_class(position, *action));
        if salt != 0 {
            shuffle_within_classes(position, &mut actions, salt, &self.zobrist);
        }
        actions
    }

    fn apply(&self, position: &mut Position, action: Action) -> Result<(), String> {
        position.apply(action)
    }

    fn is_won(&self, position: &Position) -> bool {
        position.is_won()
    }

    fn key(&self, position: &Position) -> u128 {
        self.zobrist.key(position)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::{FOUNDATIONS, PILES};

    const fn card(suit: Suit, rank: u8) -> Card {
        Card::new(suit, rank)
    }

    /// Nothing stacks on an ace, and only an ace stacks on a two.
    #[test]
    fn aces_and_twos_never_need_a_base() {
        let position = Position::deal(3);
        assert!(never_wanted_in_the_tableau(
            &position,
            card(Suit::Hearts, 1)
        ));
        assert!(never_wanted_in_the_tableau(
            &position,
            card(Suit::Hearts, 2)
        ));
        assert!(!never_wanted_in_the_tableau(
            &position,
            card(Suit::Hearts, 3)
        ));
    }

    /// The single-deck rule checks both opposite-colour piles, and the Gypsy
    /// rule ported unchanged would check four and never fire here. Spades are
    /// slot 0, hearts 1, clubs 2, diamonds 3.
    #[test]
    fn a_card_is_unsafe_until_both_opposite_piles_pass_it() {
        let mut position = Position::deal(3);
        let black_five = card(Suit::Spades, 5);

        position.foundations = [0, 4, 0, 3];
        assert!(
            !never_wanted_in_the_tableau(&position, black_five),
            "diamonds are still short, so the four of diamonds can want this"
        );

        position.foundations[3] = 4;
        assert!(never_wanted_in_the_tableau(&position, black_five));
    }

    /// A position whose pile 0 top card is both playable and safe: a six with
    /// every opposite-colour five already up.
    fn with_a_safe_autoplay() -> Position {
        let six = [card(Suit::Spades, 6)];
        let king = [card(Suit::Spades, 13)];
        let mut piles: [(usize, &[Card]); PILES] = [(0, &[]); PILES];
        piles[0] = (0, &six);
        piles[1] = (0, &king);
        // Spades to the five, both red suits to the five, clubs untouched.
        Position::from_parts(piles, [5, 5, 0, 5], &[], 0)
    }

    #[test]
    fn a_safe_card_collapses_the_position_to_one_move() {
        let position = with_a_safe_autoplay();
        let actions = Klondike::new(MoveOptions::NO_WORRY_BACK).legal_actions(&position, 0);
        assert_eq!(actions, vec![Action::PileToFoundation { from: 0 }]);
    }

    /// The gate. With worry-back legal the piles the rule checks can send
    /// their cards back down, so it proves nothing and every move stays.
    #[test]
    fn safe_autoplay_is_suppressed_when_worry_back_is_legal() {
        let position = with_a_safe_autoplay();
        let actions = Klondike::new(MoveOptions::ALL).legal_actions(&position, 0);
        assert!(
            actions.len() > 1,
            "the full game keeps its alternatives, got {actions:?}"
        );
    }

    /// The Klondike-specific exclusion. A safe card on top of the waste is
    /// left as one option among many, because playing it up re-aligns every
    /// later draw and the reordering argument does not survive that.
    #[test]
    fn a_safe_card_on_the_waste_does_not_force_anything() {
        let king = [card(Suit::Spades, 13)];
        let mut piles: [(usize, &[Card]); PILES] = [(0, &[]); PILES];
        piles[0] = (0, &king);
        let waste = [card(Suit::Spades, 6), card(Suit::Hearts, 12)];
        let position = Position::from_parts(piles, [5, 5, 0, 5], &waste, 1);

        assert_eq!(position.waste_top(), Some(card(Suit::Spades, 6)));
        let actions = Klondike::new(MoveOptions::NO_WORRY_BACK).legal_actions(&position, 0);
        assert!(
            actions.contains(&Action::WasteToFoundation) && actions.len() > 1,
            "the safe waste card is an option, not a forced move, got {actions:?}"
        );
    }

    /// Every foundation index the rule reads is a suit index, which is what
    /// makes one array entry per suit the right shape. A guard against the
    /// Gypsy layout, two slots per suit, being carried over.
    #[test]
    fn there_is_one_foundation_per_suit() {
        assert_eq!(FOUNDATIONS, Suit::ALL.len());
    }
}
