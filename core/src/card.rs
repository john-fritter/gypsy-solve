//! Cards: rank, suit, colour, and a compact two-character text form.

use std::fmt;
use std::str::FromStr;

/// Number of ranks in a suit (A..K).
pub const RANKS: u8 = 13;
/// Number of suits.
pub const SUITS: u8 = 4;

/// Suits, ordered so that the discriminant's low bit is the colour:
/// even is black, odd is red.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[repr(u8)]
pub enum Suit {
    Spades = 0,
    Hearts = 1,
    Clubs = 2,
    Diamonds = 3,
}

impl Suit {
    /// All suits in index order.
    pub const ALL: [Suit; 4] = [Suit::Spades, Suit::Hearts, Suit::Clubs, Suit::Diamonds];

    /// Suit for an index in `0..4`. Panics outside that range.
    pub const fn from_index(index: u8) -> Suit {
        match index {
            0 => Suit::Spades,
            1 => Suit::Hearts,
            2 => Suit::Clubs,
            3 => Suit::Diamonds,
            _ => panic!("suit index out of range"),
        }
    }

    pub const fn index(self) -> u8 {
        self as u8
    }

    /// Hearts and diamonds are red; spades and clubs are black.
    pub const fn is_red(self) -> bool {
        (self as u8) & 1 == 1
    }

    pub const fn to_char(self) -> char {
        match self {
            Suit::Spades => 's',
            Suit::Hearts => 'h',
            Suit::Clubs => 'c',
            Suit::Diamonds => 'd',
        }
    }

    pub const fn from_char(c: char) -> Option<Suit> {
        match c {
            's' | 'S' => Some(Suit::Spades),
            'h' | 'H' => Some(Suit::Hearts),
            'c' | 'C' => Some(Suit::Clubs),
            'd' | 'D' => Some(Suit::Diamonds),
            _ => None,
        }
    }
}

impl fmt::Display for Suit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Suit::Spades => "s",
            Suit::Hearts => "h",
            Suit::Clubs => "c",
            Suit::Diamonds => "d",
        })
    }
}

const RANK_CHARS: [char; 13] = [
    'A', '2', '3', '4', '5', '6', '7', '8', '9', 'T', 'J', 'Q', 'K',
];

/// A playing card, stored as `suit * 13 + (rank - 1)`, so `0..52`.
///
/// The two decks are not distinguished: two copies of the same card are the
/// same value. Nothing in the rules can tell them apart, and the solver
/// depends on that being true.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Card(u8);

impl Card {
    /// Ranks run 1 (ace) to 13 (king). Panics outside that range.
    pub const fn new(suit: Suit, rank: u8) -> Card {
        assert!(rank >= 1 && rank <= RANKS, "rank out of range");
        Card((suit as u8) * RANKS + (rank - 1))
    }

    /// Card for an index in `0..52`. Panics outside that range.
    pub const fn from_index(index: u8) -> Card {
        assert!(index < RANKS * SUITS, "card index out of range");
        Card(index)
    }

    pub const fn index(self) -> u8 {
        self.0
    }

    pub const fn rank(self) -> u8 {
        self.0 % RANKS + 1
    }

    pub const fn suit(self) -> Suit {
        Suit::from_index(self.0 / RANKS)
    }

    pub const fn is_red(self) -> bool {
        self.suit().is_red()
    }

    /// True when `self` may be stacked directly on `base` in the tableau:
    /// one rank lower and the opposite colour.
    pub const fn stacks_on(self, base: Card) -> bool {
        base.rank() == self.rank() + 1 && base.is_red() != self.is_red()
    }
}

impl fmt::Display for Card {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}{}",
            RANK_CHARS[(self.rank() - 1) as usize],
            self.suit()
        )
    }
}

impl fmt::Debug for Card {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

/// Error returned when a card cannot be parsed from text.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ParseCardError;

impl fmt::Display for ParseCardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("expected a rank (A23456789TJQK) followed by a suit (shcd)")
    }
}

impl std::error::Error for ParseCardError {}

impl FromStr for Card {
    type Err = ParseCardError;

    /// Parses the `Display` form: rank character then suit character, e.g. `Ts`.
    fn from_str(s: &str) -> Result<Card, ParseCardError> {
        let mut chars = s.chars();
        let (Some(rank_char), Some(suit_char), None) = (chars.next(), chars.next(), chars.next())
        else {
            return Err(ParseCardError);
        };
        let rank = RANK_CHARS
            .iter()
            .position(|&c| c == rank_char.to_ascii_uppercase())
            .ok_or(ParseCardError)? as u8
            + 1;
        let suit = Suit::from_char(suit_char).ok_or(ParseCardError)?;
        Ok(Card::new(suit, rank))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_round_trips_through_suit_and_rank() {
        for index in 0..RANKS * SUITS {
            let card = Card::from_index(index);
            assert_eq!(card, Card::new(card.suit(), card.rank()));
            assert_eq!(card.index(), index);
        }
    }

    #[test]
    fn text_round_trips() {
        for index in 0..RANKS * SUITS {
            let card = Card::from_index(index);
            assert_eq!(card.to_string().parse::<Card>(), Ok(card));
        }
        assert_eq!("Ts".parse::<Card>(), Ok(Card::new(Suit::Spades, 10)));
        assert_eq!("Ad".parse::<Card>(), Ok(Card::new(Suit::Diamonds, 1)));
        assert!("".parse::<Card>().is_err());
        assert!("1s".parse::<Card>().is_err());
        assert!("Tx".parse::<Card>().is_err());
        assert!("Tss".parse::<Card>().is_err());
    }

    #[test]
    fn colours_alternate_by_suit_parity() {
        assert!(!Suit::Spades.is_red());
        assert!(Suit::Hearts.is_red());
        assert!(!Suit::Clubs.is_red());
        assert!(Suit::Diamonds.is_red());
    }

    #[test]
    fn stacking_needs_descending_rank_and_opposite_colour() {
        let red_nine = Card::new(Suit::Hearts, 9);
        let black_ten = Card::new(Suit::Spades, 10);
        let red_ten = Card::new(Suit::Diamonds, 10);
        assert!(red_nine.stacks_on(black_ten));
        assert!(!red_nine.stacks_on(red_ten));
        assert!(!black_ten.stacks_on(red_nine));
    }
}
