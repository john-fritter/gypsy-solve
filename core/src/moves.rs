//! Moves and their text notation.

use std::fmt;
use std::str::FromStr;

/// A single move. Indices are column and foundation slot numbers; see
/// [`crate::state::State`] for their meaning.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Move {
    /// Deal one card from the stock onto each tableau column.
    Stock,
    /// Move `count` cards from the top of column `from` onto column `to`.
    Tableau { from: u8, to: u8, count: u8 },
    /// Move the top card of column `from` onto foundation slot `foundation`.
    ToFoundation { from: u8, foundation: u8 },
    /// Worry back: move the top card of `foundation` onto column `to`.
    WorryBack { foundation: u8, to: u8 },
}

impl fmt::Display for Move {
    /// Notation: `S` deals the stock, `T0>T3:2` moves two cards between
    /// columns, `T4>F1` plays to a foundation, `F1>T4` worries one back.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Move::Stock => f.write_str("S"),
            Move::Tableau { from, to, count } => write!(f, "T{from}>T{to}:{count}"),
            Move::ToFoundation { from, foundation } => write!(f, "T{from}>F{foundation}"),
            Move::WorryBack { foundation, to } => write!(f, "F{foundation}>T{to}"),
        }
    }
}

/// Error returned when a move cannot be parsed from text.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ParseMoveError(String);

impl fmt::Display for ParseMoveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "cannot parse move {:?}", self.0)
    }
}

impl std::error::Error for ParseMoveError {}

impl FromStr for Move {
    type Err = ParseMoveError;

    /// Parses the `Display` form. A tableau move with no `:count` suffix moves
    /// a single card.
    fn from_str(s: &str) -> Result<Move, ParseMoveError> {
        let text = s.trim();
        let fail = || ParseMoveError(s.to_string());

        if text.eq_ignore_ascii_case("s") {
            return Ok(Move::Stock);
        }

        let (source, rest) = text.split_once('>').ok_or_else(fail)?;
        let (destination, count) = match rest.split_once(':') {
            Some((destination, count)) => {
                (destination, Some(count.parse::<u8>().map_err(|_| fail())?))
            }
            None => (rest, None),
        };

        let index = |part: &str, tag: u8| -> Result<u8, ParseMoveError> {
            let bytes = part.as_bytes();
            if bytes.len() < 2 || !bytes[0].eq_ignore_ascii_case(&tag) {
                return Err(fail());
            }
            part[1..].parse::<u8>().map_err(|_| fail())
        };

        match (source.as_bytes().first(), destination.as_bytes().first()) {
            (Some(b'T' | b't'), Some(b'T' | b't')) => Ok(Move::Tableau {
                from: index(source, b'T')?,
                to: index(destination, b'T')?,
                count: count.unwrap_or(1),
            }),
            (Some(b'T' | b't'), Some(b'F' | b'f')) if count.is_none() => Ok(Move::ToFoundation {
                from: index(source, b'T')?,
                foundation: index(destination, b'F')?,
            }),
            (Some(b'F' | b'f'), Some(b'T' | b't')) if count.is_none() => Ok(Move::WorryBack {
                foundation: index(source, b'F')?,
                to: index(destination, b'T')?,
            }),
            _ => Err(fail()),
        }
    }
}

/// Parses a whitespace- or comma-separated move list. Anything from a `#` to
/// the end of a line is a comment.
pub fn parse_move_list(text: &str) -> Result<Vec<Move>, ParseMoveError> {
    text.lines()
        .map(|line| line.split('#').next().unwrap_or(""))
        .flat_map(|line| line.split([',', ' ', '\t']))
        .filter(|token| !token.is_empty())
        .map(Move::from_str)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notation_round_trips() {
        let moves = [
            Move::Stock,
            Move::Tableau {
                from: 0,
                to: 7,
                count: 1,
            },
            Move::Tableau {
                from: 3,
                to: 1,
                count: 13,
            },
            Move::ToFoundation {
                from: 2,
                foundation: 5,
            },
            Move::WorryBack {
                foundation: 7,
                to: 4,
            },
        ];
        for expected in moves {
            let text = expected.to_string();
            assert_eq!(text.parse::<Move>(), Ok(expected), "round trip of {text}");
        }
    }

    #[test]
    fn bare_tableau_move_means_one_card() {
        assert_eq!(
            "T0>T1".parse::<Move>(),
            Ok(Move::Tableau {
                from: 0,
                to: 1,
                count: 1
            })
        );
    }

    #[test]
    fn rejects_malformed_moves() {
        for text in [
            "", "X", "T0", "T0>", ">T1", "T0>F1:2", "F0>F1", "Ta>T1", "T0>T1:x",
        ] {
            assert!(text.parse::<Move>().is_err(), "{text:?} should not parse");
        }
    }

    #[test]
    fn move_lists_ignore_comments_and_separators() {
        let text = "S, T0>T1:2  # settle the run\n# nothing here\nT2>F0\n";
        assert_eq!(
            parse_move_list(text),
            Ok(vec![
                Move::Stock,
                Move::Tableau {
                    from: 0,
                    to: 1,
                    count: 2
                },
                Move::ToFoundation {
                    from: 2,
                    foundation: 0
                },
            ])
        );
    }
}
