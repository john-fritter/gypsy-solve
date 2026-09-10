//! Text rendering of a position, for the CLI and for reading solver traces.

use std::fmt;

use crate::state::{foundation_suit, State, COLUMNS, DECK_SIZE, FOUNDATIONS};

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "stock {} ({} deal{} left)   foundations {}/{}",
            self.stock.len(),
            self.deals_remaining(),
            if self.deals_remaining() == 1 { "" } else { "s" },
            self.foundation_count(),
            DECK_SIZE
        )?;

        for slot in 0..FOUNDATIONS {
            let top = match self.foundation_top(slot as u8) {
                Some(card) => card.to_string(),
                None => format!("-{}", foundation_suit(slot as u8)),
            };
            write!(f, "{}F{slot}:{top}", if slot == 0 { "" } else { "  " })?;
        }
        writeln!(f)?;
        writeln!(f)?;

        for column in 0..COLUMNS {
            write!(f, " {:>3}", format!("T{column}"))?;
        }
        writeln!(f)?;

        let depth = self.columns.iter().map(|c| c.len()).max().unwrap_or(0);
        for row in 0..depth {
            for column in &self.columns {
                let cell = match column.cards().get(row) {
                    None => String::new(),
                    Some(_) if row < column.hidden() => "??".to_string(),
                    Some(card) => card.to_string(),
                };
                write!(f, " {cell:>3}")?;
            }
            writeln!(f)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::COLUMNS;

    #[test]
    fn opening_position_renders_one_hidden_row_and_two_face_up() {
        let state = State::deal(1);
        let text = state.to_string();
        let lines: Vec<&str> = text.lines().collect();

        assert!(lines[0].starts_with("stock 80 (10 deals left)"));
        assert_eq!(lines[3].split_whitespace().count(), COLUMNS);
        assert_eq!(
            lines[4].split_whitespace().collect::<Vec<_>>(),
            vec!["??"; COLUMNS]
        );
        assert_eq!(lines[5].split_whitespace().count(), COLUMNS);
        assert_eq!(lines[6].split_whitespace().count(), COLUMNS);
        assert_eq!(lines.len(), 7);
    }

    #[test]
    fn empty_foundations_show_their_suit() {
        let text = State::deal(1).to_string();
        assert!(text.contains("F0:-s"), "{text}");
        assert!(text.contains("F7:-d"), "{text}");
    }
}
