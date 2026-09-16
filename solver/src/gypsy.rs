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

/// True when this card can be forced to a foundation **with worry-back legal**,
/// for two decks.
///
/// Keller's rule, published for one deck and excluding duplicate cards in
/// terms, re-proved here with each threshold read as a minimum over that
/// suit's two slots and one condition added that duplicates force:
///
/// - every slot of the two opposite-colour suits at least *r-1*;
/// - both slots of the other same-colour suit at least *r-2*;
/// - **both slots of the card's own suit at least *r-1***.
///
/// The proof is on [`Gypsy::legal_actions`]. Aces need no special case: the
/// arithmetic is trivially true at rank 1.
fn safe_with_worry_back(state: &State, card: Card) -> bool {
    let rank = card.rank();
    Suit::ALL.iter().all(|&suit| {
        let reach = if suit.is_red() == card.is_red() && suit != card.suit() {
            2
        } else {
            1
        };
        foundation_slots(suit)
            .into_iter()
            .all(|slot| state.foundations[slot as usize] + reach >= rank)
    })
}

/// The first foundation play that is safe with worry-back legal, if there is
/// one. Gypsy has no waste, so every column top is a candidate.
fn forced_foundation_play(state: &State) -> Option<Move> {
    state.columns.iter().enumerate().find_map(|(from, column)| {
        let card = column.top()?;
        let foundation = state.foundation_target(card)?;
        safe_with_worry_back(state, card).then_some(Move::ToFoundation {
            from: from as u8,
            foundation,
        })
    })
}

/// True when this tableau move carries a strict suffix of a built run and the
/// card it would expose has nowhere to go.
///
/// This is Blake & Gent's Theorem 4 (JAIR 85, Appendix B.2) in the weaker form
/// Solvitaire implements: rather than requiring the exposed card to be built
/// *immediately*, require only that it could be. Splitting a run is worth doing
/// to free the card underneath; splitting it when that card is dead is a
/// shuffle.
///
/// Both gates are ours and neither is decoration. See `legal_actions`.
fn splits_a_run_for_nothing(state: &State, mv: Move) -> bool {
    let Move::Tableau { from, to, count } = mv else {
        return false;
    };
    // Gate 1. With cards still to deal, the reordering argument cannot pass a
    // stock move; see `legal_actions`.
    if !state.stock.is_empty() {
        return false;
    }
    // Gate 2. An empty column accepts every card, which is the one destination
    // the proof's substitution step cannot treat as a card.
    if state.columns[to as usize].is_empty() {
        return false;
    }

    let column = &state.columns[from as usize];
    let count = count as usize;
    // Moving the whole built run, or the whole column, is never restricted:
    // neither leaves a card of the run behind to be exposed.
    if count >= column.movable_run() || count >= column.len() {
        return false;
    }

    // A strict suffix of the run, so the card beneath it is face up and is the
    // run's next card.
    let exposed = column.cards()[column.len() - count - 1];
    state.foundation_target(exposed).is_none()
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

/// Which band of the move ordering this move falls in. Lower is tried first.
///
/// Only an ordering: every band is searched, so nothing here can discard a
/// win. It is separated out because a salted search shuffles *within* a band,
/// which keeps the heuristic and changes the descent.
fn ordering_class(position: &State, mv: Move) -> u8 {
    match mv {
        Move::ToFoundation { .. } => 0,
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
    }
}

/// Shuffles each band of an already-sorted move list.
///
/// Seeded from the position and the salt, so the order is a function of where
/// the search *is* rather than of how it got there: two routes to a position
/// generate the same children in the same order, and a run reproduces exactly
/// from its seed and salt.
fn shuffle_within_classes(position: &State, moves: &mut [Move], salt: u64, zobrist: &Zobrist) {
    let mut rng = SplitMix64::new((zobrist.key(position) as u64) ^ salt);
    let mut start = 0;
    while start < moves.len() {
        let class = ordering_class(position, moves[start]);
        let mut end = start + 1;
        while end < moves.len() && ordering_class(position, moves[end]) == class {
            end += 1;
        }
        rng.shuffle(&mut moves[start..end]);
        start = end;
    }
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
    ///
    /// # Safe foundation plays with worry-back legal, and the third condition
    ///
    /// With worry-back on, a card on top of a column is played and nothing
    /// else is considered when [`safe_with_worry_back`] holds. This is the
    /// rule the full game never had; the restricted game keeps its own, weaker
    /// condition above, which fires strictly more often.
    ///
    /// **What the conditions buy.** A slot showing *v* holds one copy of every
    /// rank up to *v*, so the three conditions together put on foundations:
    /// every opposite-colour card of rank *r-1* or less, every same-colour card
    /// of rank *r-2* or less, and both copies of every card of `X`'s own suit
    /// up to *r-1*. **The only cards of rank below *r* left anywhere are
    /// same-colour cards of rank exactly *r-1*, and those build on rank-*r*
    /// cards of the opposite colour — never on `X`.**
    ///
    /// **So nothing can be put on `X` except by worrying it back**, and
    /// whatever is worried back onto `X` can itself host nothing but further
    /// worried-back cards, by the same count one rank down. Any structure
    /// built on `X` is therefore foundation cards parked on each other: it
    /// hosts nothing, frees nothing, and every card in it is owed back to a
    /// foundation before the game is won. Delete it from a winning line along
    /// with the moves that return those cards — nothing else can depend on it,
    /// because the one thing a lowered foundation permits is playing a
    /// *duplicate* of the card just removed, and both copies of every card
    /// that could be are already up.
    ///
    /// With no structure on `X`, `X` is never covered, so its foundation play
    /// moves to the front of the line the way safe autoplay's does: drop `X`
    /// from any group that carries it — it is on top, so the group's bottom
    /// card and the destination test are unchanged — and delete its own play.
    ///
    /// **The third condition is what duplicate cards cost, and it is ours.**
    /// Playing `X` early raises its slot to *r*, and the only moves that need
    /// that slot at *r-1* are plays of a rank-*r* card of the same suit: `X`,
    /// and its duplicate. Without the condition the duplicate breaks the
    /// reordering — a line may play it to this slot first and `X` to the other
    /// slot later, and the rewritten line then has to hold the duplicate in the
    /// tableau until that second slot comes up, while the line it is copying
    /// builds on the card the duplicate was sitting on. Requiring both slots of
    /// the suit to be at *r-1* removes the case: either the second slot is past
    /// *r* and the duplicate is already up, or it shows exactly *r-1* and the
    /// duplicate goes up the moment the line played it. **An empty-stock gate
    /// does not fix this one**, which was checked before the condition was
    /// adopted. Blake & Gent exclude duplicate cards from their proof in terms,
    /// so this part is an extension of the paper rather than an application.
    ///
    /// **Why the stock needs no gate here.** A dealt card lands on a column
    /// whatever it is, but it can never *build* on `X` or on anything parked on
    /// it: those cards are all on foundations by the count above, so they are
    /// not in the stock either. Removing `X` leaves the cards above it sitting
    /// one deeper, which can only lengthen a run, never shorten one.
    ///
    /// # Splitting a built run for nothing, and why the two gates hold
    ///
    /// The second dominance, and the first one the *full* game has. A move
    /// carrying a strict suffix of a built run is not generated when the card
    /// it would expose has no foundation to go to — subject to the gates in
    /// [`splits_a_run_for_nothing`].
    ///
    /// **The published rule.** Blake & Gent prove (JAIR 85, Theorem 4,
    /// Appendix B.2) that for a game whose tableau builds down under an
    /// *indistinguishable* build policy, whose group moves follow the same
    /// policy as single cards, where a card leaves the tableau only for another
    /// tableau pile or a foundation, which is won by moving every card to a
    /// foundation, and which has no rule making a move's legality depend on its
    /// position in the sequence — any winnable instance stays winnable when an
    /// incomplete built pile may be moved only if the card above it is then
    /// built immediately. Unlike their safe-foundation theorem, this one is
    /// *deliberately* generalised past a single deck; their worked example is
    /// five identical decks.
    ///
    /// **Why it reaches Gypsy at all.** "Indistinguishable" asks that any two
    /// cards have identical or disjoint sets of places they can be built on,
    /// and that one policy governs both single cards and groups. Alternating
    /// colour gives the first: two red fives go on exactly the same black
    /// sixes, a red five and a black five share none. The second is the
    /// permissive variant this project is told not to "correct" — any
    /// alternating-colour sequence moves as a unit, exactly as a single card
    /// does. Standard Spider fails precisely here, its singles moving by any
    /// suit while its groups must share one, and the paper excludes it by name.
    ///
    /// # The proof, for Gypsy, with both gates in it
    ///
    /// Theorem 4 is a statement about a whole instance; this is a filter on
    /// move generation, applied at some positions and not others, under a
    /// transposition table. Rather than inherit a case analysis across that
    /// gap, the rule is proved here directly, in the shape of the safe
    /// autoplay proof above: take a winning line that makes the move, and
    /// build one that does not.
    ///
    /// **The twin, which is the whole engine of it.** Write the cut move as
    /// carrying a group `g` off column `i` onto the top card `d` of column
    /// `j`, and let `x` be the card left directly under `g`. Both `x` and `d`
    /// carry `g`'s bottom card, so both are one rank above it and of the
    /// opposite colour: **`x` and `d` have the same rank and the same
    /// colour**, and so accept exactly the same piles, building testing rank
    /// and colour and nothing else. They are usually different *suits*, so
    /// they are not interchangeable for a foundation play, and that one
    /// asymmetry is what the rest of the proof is about.
    ///
    /// **The copy.** Let `L` be a winning line from this position `Q` with as
    /// few cut moves in it as possible, and suppose it opens with this one,
    /// `m`. Delete `m` and follow the rest of `L`. Every position the copy
    /// reaches is the real one with the contents of two *slots* exchanged —
    /// a slot being a card with the pile built on it — starting with `g` on
    /// `x` in the copy and on `d` in the real line. Both slots accept both
    /// piles, by the twin property, so the exchange is a position either way.
    /// The copy mirrors each move by playing the same cards onto the same
    /// card, the two slots standing in for each other:
    ///
    /// - cards taken from inside a pile, or the slot card lifted with its
    ///   pile, or anything not in either pile: the same move, expose the same
    ///   card;
    /// - the whole of a pile lifted off its slot: the copy lifts *its* slot's
    ///   pile instead, onto the same destination, which is legal because both
    ///   piles fit both slots — and this again exposes the same slot card, the
    ///   exchange carrying over to the destination and the other slot;
    /// - a card played onto a bare slot: bare in one line means bare in the
    ///   other's twin, so the copy plays it onto the twin, which accepts it.
    ///
    /// **Every mirrored move therefore exposes the card its original exposed,
    /// from the same kind of destination — so it is cut exactly when its
    /// original was.** That is the step the whole termination argument rests
    /// on.
    ///
    /// **The deleted move comes back once, where it is owed.** What the copy
    /// cannot mirror is a foundation play of a slot card, which needs the
    /// suit. `L` plays `x` up only with `x` bare, which means the pile on `d`
    /// is empty, which means in the copy `d` is bare and `x` carries the other
    /// pile. So the copy plays `m` there — moves that pile onto `d` — and the
    /// two lines are in the same position from then on, move for move. **That
    /// deferred `m` is generated in the restricted game**: it lands on a card,
    /// and the card it exposes is `x`, which is played up on the very next
    /// move, so `x` has a foundation. The same repair covers a slot column
    /// emptying in one line and not the other, where the deferred move carries
    /// a whole column and is never cut for any reason.
    ///
    /// So the rewritten line wins, is no longer than `L`, and has one fewer
    /// cut move — `m` is gone and nothing else changed status. That
    /// contradicts the choice of `L` unless `L` had none. Hence a winnable
    /// position with an empty stock has a winning line the restricted
    /// generator offers, which is what the search needs.
    ///
    /// **Gate 2 is the sentence "both slots accept the same piles".** An empty
    /// column accepts *every* pile and no card does. Land `g` on one and the
    /// exposed `x` has no twin: the real line may drop a pile into that column
    /// that the copy, holding a card there, cannot legally match, and the
    /// mirror stops. So moves onto an empty column are not cut — which is also
    /// the move that makes working space, the last thing to take away from a
    /// game this cramped.
    ///
    /// **Gate 1 is that the stock deal is addressed by column.** It lands one
    /// card on *every* column, and the two exchanged piles are in different
    /// columns, so a deal adds a different card to each and the exchange is
    /// destroyed — the two lines stop being the same position with two slots
    /// swapped, and no later move can repair it. A dealt card also need not
    /// continue a run, so the pile the deferred move meant to carry can be
    /// buried outright. Gating on an empty stock removes the move entirely:
    /// **nothing ever returns a card to the stock**, so from an empty-stock
    /// position every continuation is stock-free, and the set of positions the
    /// rule fires at is closed under making a move. That closure is what lets
    /// a per-position filter stand in for a theorem about whole instances —
    /// the proof above never has to leave the restricted region, and positions
    /// with cards still to deal keep every move they had.
    ///
    /// **And the filter reads the position only.** Whether a move is cut is a
    /// function of the position — stock empty, destination non-empty, exposed
    /// card's foundation — and never of the path that reached it, so two
    /// routes to the same position generate the same children and the
    /// expanded-set induction (`DECISIONS.md`, 2026-09-13) is untouched. This
    /// is exactly what a *capped* worry-back arm could not say: worry-backs
    /// spent is path state, and a table that stores positions forgets it.
    ///
    /// **Why this one survives worry-back when safe autoplay does not.** It
    /// makes no claim about a card never being wanted again, so nothing in it
    /// has to hold in the future. A worry-back enters the proof only as a card
    /// placed on a column, which the copy mirrors like any other placement,
    /// and the deferred move's licence — that `x` has a foundation — is read
    /// at the position where that move is played, not promised in advance.
    /// That answers the second hypothesis left open on 2026-09-15: the
    /// argument nowhere needs foundations to be irremovable. Solvitaire ships
    /// the same rule for Klondike's published worry-back variant.
    ///
    fn legal_actions(&self, position: &State, salt: u64) -> Vec<Move> {
        // A forced play is forced under every ordering, so this comes before
        // any salt is applied.
        let forced = if self.options.worry_back {
            forced_foundation_play(position)
        } else {
            safe_autoplay(position)
        };
        if let Some(forced) = forced {
            return vec![forced];
        }

        let mut moves = position.legal_moves(self.options);
        moves.retain(|mv| !splits_a_run_for_nothing(position, *mv));
        moves.sort_by_key(|mv| ordering_class(position, *mv));
        if salt != 0 {
            shuffle_within_classes(position, &mut moves, salt, &self.zobrist);
        }
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
    use gypsy_core::state::{COLUMNS, FOUNDATIONS};
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
        let actions = Gypsy::new(MoveOptions::NO_WORRY_BACK).legal_actions(&state, 0);
        assert_eq!(actions.len(), 1, "everything else is dominated");
        assert!(matches!(actions[0], Move::ToFoundation { .. }));
    }

    /// The gate. With worry-back legal the four piles the rule checks can
    /// send their cards back down, so the rule proves nothing and the search
    /// keeps every move.
    #[test]
    fn safe_autoplay_is_suppressed_when_worry_back_is_legal() {
        let state = with_a_safe_autoplay(3);
        let actions = Gypsy::new(MoveOptions::ALL).legal_actions(&state, 0);
        assert!(
            actions.len() > 1,
            "the full game keeps its alternatives, got {actions:?}"
        );
    }

    /// A position whose column 0 top card is playable and safe with worry-back
    /// legal: opposite colours one rank behind, the same-colour twin two, and
    /// the card's own second slot level with the first.
    fn with_a_forced_foundation_play(seed: u64) -> State {
        let mut state = State::deal(seed);
        let top = state.columns[0].top().expect("a dealt column has cards");
        let mut foundations = [0u8; FOUNDATIONS];
        for suit in Suit::ALL {
            let reach = if suit.is_red() == top.is_red() && suit != top.suit() {
                2
            } else {
                1
            };
            for slot in foundation_slots(suit) {
                foundations[slot as usize] = top.rank() - reach;
            }
        }
        state.foundations = foundations;
        state
    }

    #[test]
    fn a_safe_card_is_forced_in_the_full_game_too() {
        let state = with_a_forced_foundation_play(3);
        let actions = Gypsy::new(MoveOptions::ALL).legal_actions(&state, 0);
        assert_eq!(actions.len(), 1, "everything else is dominated");
        assert!(matches!(actions[0], Move::ToFoundation { from: 0, .. }));
    }

    /// The condition duplicate cards force, and the one a ported single-deck
    /// rule has no reason to carry. With the card's own second slot a rank
    /// short, the duplicate cannot follow it up, and the reordering that
    /// justifies the rule breaks on exactly that.
    #[test]
    fn a_card_is_unsafe_until_its_own_second_slot_catches_up() {
        let mut state = with_a_forced_foundation_play(3);
        let top = state.columns[0].top().expect("a dealt column has cards");
        let slots = foundation_slots(top.suit());
        let second = slots[1] as usize;

        state.foundations[second] = top.rank() - 2;
        assert!(
            !safe_with_worry_back(&state, top),
            "the duplicate cannot follow this card up, so it is not forced"
        );

        state.foundations[second] = top.rank() - 1;
        assert!(safe_with_worry_back(&state, top));
    }

    /// And the same-colour twin, two ranks rather than one: it is the
    /// condition that stops a worried-back card finding something to host.
    #[test]
    fn a_card_is_unsafe_until_the_same_colour_twin_is_within_two() {
        let mut state = with_a_forced_foundation_play(3);
        let top = state.columns[0].top().expect("a dealt column has cards");
        let twin = Suit::ALL
            .into_iter()
            .find(|suit| suit.is_red() == top.is_red() && *suit != top.suit())
            .expect("every suit has a same-colour twin");

        state.foundations[foundation_slots(twin)[1] as usize] = top.rank() - 3;
        assert!(!safe_with_worry_back(&state, top));

        state.foundations[foundation_slots(twin)[1] as usize] = top.rank() - 2;
        assert!(safe_with_worry_back(&state, top));
    }

    /// Nothing builds on an ace, so it is forced whatever else is showing.
    #[test]
    fn an_ace_is_safe_with_worry_back_whatever_the_foundations_show() {
        let state = State::deal(3);
        for suit in Suit::ALL {
            assert!(safe_with_worry_back(&state, Card::new(suit, 1)));
        }
    }

    /// The worry-back rule is strictly the stronger of the two, which is why
    /// the restricted arm keeps its own and gains nothing from this one.
    #[test]
    fn the_worry_back_condition_implies_the_restricted_one() {
        let game = Gypsy::new(MoveOptions::ALL);
        let mut fired = 0;
        for seed in 0..4 {
            for state in descend(&game, seed, 800) {
                for column in &state.columns {
                    let Some(card) = column.top() else { continue };
                    if safe_with_worry_back(&state, card) {
                        fired += 1;
                        assert!(
                            never_wanted_in_the_tableau(&state, card),
                            "{card} is forced with worry-back on but not with it off"
                        );
                    }
                }
            }
        }
        assert!(fired > 0, "the test never met the case it is pinning");
    }

    /// Walks the search's first descent, collecting positions.
    fn descend(game: &Gypsy, seed: u64, steps: usize) -> Vec<State> {
        let mut state = State::deal(seed);
        let mut seen = std::collections::HashSet::new();
        let mut visited = Vec::new();
        seen.insert(game.key(&state));
        for _ in 0..steps {
            visited.push(state.clone());
            let moves = game.legal_actions(&state, 0);
            let Some(next) = moves.iter().find_map(|mv| {
                let mut child = state.clone();
                child.apply(*mv).ok()?;
                seen.insert(game.key(&child)).then_some(child)
            }) else {
                break;
            };
            state = next;
        }
        visited
    }

    /// Splitting a run to expose a card with nowhere to go is a shuffle, and
    /// once the stock is empty the search is not offered it.
    #[test]
    fn a_run_is_not_split_to_expose_a_dead_card() {
        let game = Gypsy::new(MoveOptions::ALL);
        let mut fired = 0;

        for seed in 0..6 {
            for state in descend(&game, seed, 4_000) {
                if !state.stock.is_empty() {
                    continue;
                }
                let offered = game.legal_actions(&state, 0);
                for mv in state.legal_moves(MoveOptions::ALL) {
                    if splits_a_run_for_nothing(&state, mv) {
                        fired += 1;
                        assert!(
                            !offered.contains(&mv),
                            "{mv} splits a run for nothing and was still offered"
                        );
                    }
                }
            }
        }

        assert!(fired > 0, "the test never met the case it is pinning");
    }

    /// Gate 1. While cards are still to be dealt the rule proves nothing,
    /// because a stock deal lands on every column and cannot be reordered
    /// past a tableau move. Those moves stay.
    #[test]
    fn a_run_may_be_split_for_nothing_while_the_stock_holds_cards() {
        let game = Gypsy::new(MoveOptions::ALL);
        let mut kept = 0;

        for seed in 0..6 {
            for state in descend(&game, seed, 1_500) {
                // A position with a forced foundation play offers that and
                // nothing else, which is a different rule's business.
                if state.stock.is_empty() || forced_foundation_play(&state).is_some() {
                    continue;
                }
                let offered = game.legal_actions(&state, 0);
                for mv in state.legal_moves(MoveOptions::ALL) {
                    let Move::Tableau { from, to, count } = mv else {
                        continue;
                    };
                    let column = &state.columns[from as usize];
                    let count = count as usize;
                    if count >= column.movable_run() || count >= column.len() {
                        continue;
                    }
                    if !state.columns[to as usize].is_empty()
                        && state
                            .foundation_target(column.cards()[column.len() - count - 1])
                            .is_none()
                    {
                        kept += 1;
                        assert!(offered.contains(&mv), "{mv} was cut behind the stock gate");
                    }
                }
            }
        }

        assert!(kept > 0, "the test never met the case it is pinning");
    }

    /// Gate 2. An empty column accepts every card, which is the destination
    /// the proof's substitution step cannot treat as a card, so those moves
    /// stay whatever they expose.
    #[test]
    fn a_move_onto_an_empty_column_is_never_cut() {
        let game = Gypsy::new(MoveOptions::ALL);

        for seed in 0..8 {
            for state in descend(&game, seed, 3_000) {
                for mv in state.legal_moves(MoveOptions::ALL) {
                    if let Move::Tableau { to, .. } = mv {
                        if state.columns[to as usize].is_empty() {
                            assert!(
                                !splits_a_run_for_nothing(&state, mv),
                                "{mv} lands on an empty column and must not be cut"
                            );
                        }
                    }
                }
            }
        }
    }

    /// Moving the whole run, or the whole column, exposes no card of the run
    /// and is never the rule's business.
    #[test]
    fn whole_run_and_whole_column_moves_are_never_cut() {
        let game = Gypsy::new(MoveOptions::ALL);

        for seed in 0..6 {
            for state in descend(&game, seed, 3_000) {
                for mv in state.legal_moves(MoveOptions::ALL) {
                    let Move::Tableau { from, count, .. } = mv else {
                        continue;
                    };
                    let column = &state.columns[from as usize];
                    if count as usize >= column.movable_run() || count as usize >= column.len() {
                        assert!(
                            !splits_a_run_for_nothing(&state, mv),
                            "{mv} carries the whole run or column and must not be cut"
                        );
                    }
                }
            }
        }
    }

    /// The step the split-run proof turns on. The card a cut move would
    /// expose and the card it would land on both carry the moved group's
    /// bottom card, so they are the same rank and the same colour and accept
    /// exactly the same piles. Everything else in that proof is bookkeeping
    /// around this one fact.
    #[test]
    fn a_cut_move_exposes_a_twin_of_the_card_it_lands_on() {
        let game = Gypsy::new(MoveOptions::ALL);
        let mut fired = 0;

        for seed in 0..6 {
            for state in descend(&game, seed, 4_000) {
                for mv in state.legal_moves(MoveOptions::ALL) {
                    if !splits_a_run_for_nothing(&state, mv) {
                        continue;
                    }
                    let Move::Tableau { from, to, count } = mv else {
                        unreachable!("only tableau moves are ever cut")
                    };
                    let column = &state.columns[from as usize];
                    let exposed = column.cards()[column.len() - count as usize - 1];
                    let landed_on = state.columns[to as usize]
                        .top()
                        .expect("gate 2 leaves only card destinations");
                    fired += 1;
                    assert_eq!(
                        (exposed.rank(), exposed.is_red()),
                        (landed_on.rank(), landed_on.is_red()),
                        "{mv} exposes {exposed} and lands on {landed_on}, which are not twins"
                    );
                }
            }
        }

        assert!(fired > 0, "the test never met the case it is pinning");
    }

    /// And twins really are interchangeable as bases: the same cards build on
    /// them. Cards that are not twins share no base at all, above the aces
    /// that nothing builds on.
    #[test]
    fn twins_accept_the_same_cards_and_non_twins_share_none() {
        let every: Vec<Card> = (0..CARDS as u8).map(Card::from_index).collect();

        for &base in &every {
            for &other in &every {
                let accepts_base: Vec<Card> = every
                    .iter()
                    .copied()
                    .filter(|card| card.stacks_on(base))
                    .collect();
                let accepts_other: Vec<Card> = every
                    .iter()
                    .copied()
                    .filter(|card| card.stacks_on(other))
                    .collect();

                if base.rank() == other.rank() && base.is_red() == other.is_red() {
                    assert_eq!(accepts_base, accepts_other, "{base} and {other} are twins");
                } else if base.rank() > 1 && other.rank() > 1 {
                    assert!(
                        accepts_base
                            .iter()
                            .all(|card| !accepts_other.contains(card)),
                        "{base} and {other} are not twins and must share no card"
                    );
                }
            }
        }
    }

    /// Gate 2, as a fact about the rules rather than about the code: an empty
    /// column accepts every card there is. That is why it cannot stand in for
    /// a twin — a line may drop a pile there that a copy holding a card in
    /// that column could not legally match.
    #[test]
    fn an_empty_column_accepts_every_run_on_offer() {
        let game = Gypsy::new(MoveOptions::ALL);
        let mut fired = 0;

        for seed in 0..8 {
            for state in descend(&game, seed, 3_000) {
                let Some(empty) = state.columns.iter().position(|column| column.is_empty()) else {
                    continue;
                };
                let offered = state.legal_moves(MoveOptions::ALL);
                for (from, column) in state.columns.iter().enumerate() {
                    if from == empty || column.is_empty() {
                        continue;
                    }
                    for count in 1..=column.movable_run() {
                        fired += 1;
                        assert!(
                            offered.contains(&Move::Tableau {
                                from: from as u8,
                                to: empty as u8,
                                count: count as u8,
                            }),
                            "an empty column must accept every run, and refused {count} \
                             cards from column {from}"
                        );
                    }
                }
            }
        }

        assert!(fired > 0, "the test never met the case it is pinning");
    }

    /// Gate 1 is sound only because the region it fires in is closed: nothing
    /// ever puts a card back in the stock, so every continuation of an
    /// empty-stock position is itself stock-free and the proof never has to
    /// leave the restricted game.
    #[test]
    fn nothing_ever_returns_a_card_to_the_stock() {
        let game = Gypsy::new(MoveOptions::ALL);
        let mut with_cards_left = 0;

        for seed in 0..4 {
            for state in descend(&game, seed, 1_500) {
                with_cards_left += usize::from(!state.stock.is_empty());
                for mv in state.legal_moves(MoveOptions::ALL) {
                    let mut child = state.clone();
                    child.apply(mv).expect("a generated move is legal");
                    assert!(
                        child.stock.len() <= state.stock.len(),
                        "{mv} put a card back in the stock"
                    );
                }
            }
        }

        assert!(
            with_cards_left > 0,
            "the test never met the case it is pinning"
        );
    }

    /// And the reason that gate is needed at all. The deal is addressed by
    /// column: it lands a *different* card on each one. The proof exchanges
    /// two piles that sit in different columns, so a deal appends different
    /// cards to them and the exchange is destroyed rather than carried.
    #[test]
    fn a_stock_deal_lands_a_different_card_on_each_column() {
        let mut differing = 0;

        for seed in 0..8 {
            let mut state = State::deal(seed);
            state.apply(Move::Stock).expect("stock deal is legal");
            let dealt: Vec<Card> = (0..COLUMNS)
                .map(|column| {
                    state.columns[column]
                        .top()
                        .expect("every column was dealt to")
                })
                .collect();
            differing += usize::from(dealt.iter().any(|card| *card != dealt[0]));
        }

        assert_eq!(
            differing, 8,
            "a deal that gave every column the same card would leave the exchange intact"
        );
    }

    /// A salt may reorder and nothing else. If it could add or drop an action
    /// it would be an unargued dominance, and one that fired on some restarts
    /// and not others — the worst shape of wrong there is here.
    #[test]
    fn a_salt_reorders_and_never_adds_or_drops_an_action() {
        let game = Gypsy::new(MoveOptions::ALL);
        let mut reordered = 0;

        for seed in 0..4 {
            for state in descend(&game, seed, 1_500) {
                let plain = game.legal_actions(&state, 0);
                for salt in [1u64, 0x9E37_79B9_7F4A_7C15, u64::MAX] {
                    let salted = game.legal_actions(&state, salt);
                    let mut left = plain.clone();
                    let mut right = salted.clone();
                    left.sort_by_key(|mv| format!("{mv}"));
                    right.sort_by_key(|mv| format!("{mv}"));
                    assert_eq!(left, right, "salt {salt} changed which moves exist");
                    reordered += usize::from(salted != plain);
                }
            }
        }

        assert!(reordered > 0, "no salt ever changed the order");
    }

    /// And the order is a function of the position, not of the path: the same
    /// position salted the same way twice gives the same order, so two routes
    /// to it generate the same children in the same sequence.
    #[test]
    fn a_salted_order_depends_only_on_the_position() {
        let game = Gypsy::new(MoveOptions::ALL);
        for state in descend(&game, 5, 400) {
            let once = game.legal_actions(&state, 12_345);
            let again = game.legal_actions(&state.clone(), 12_345);
            assert_eq!(once, again);
        }
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
