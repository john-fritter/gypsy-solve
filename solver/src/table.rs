//! The transposition table.
//!
//! The distinction this table exists to keep is between a position that was
//! **refuted** — its whole subtree was searched and held no win — and one that
//! was merely **abandoned** because the search ran out of depth. Collapsing
//! the two is the bug that turns a budget-exhausted search into a confident
//! "unsolvable", which is precisely the verdict this project is not allowed to
//! guess at. A refutation is reusable from anywhere; an abandonment is only
//! worth anything to a visit that has no more depth to spend than the one that
//! recorded it.
//!
//! The table is a fixed-size direct-mapped array so that memory is decided up
//! front rather than discovered during a batch run. Eviction costs work and
//! nothing else: a lost entry is re-searched, never mis-answered.

/// What a probe found.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Probe {
    /// Searched exhaustively before, and lost. Sound to reuse at any depth.
    Refuted,
    /// Searched before with at least this much depth and nothing was found.
    /// Not a refutation. `repetition_only` says whether that earlier search
    /// was stopped by nothing worse than a loop back onto its own path, which
    /// the caller needs in order to know whether its own proof survives.
    Exhausted { repetition_only: bool },
    /// Nothing useful known; search it.
    Unknown,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Empty,
    /// Gave up, with the reason recorded: a loop, or a real limit.
    Abandoned {
        repetition_only: bool,
    },
    Refuted,
}

#[derive(Clone, Copy)]
struct Slot {
    key: u128,
    /// Depth the recording search still had in hand. Meaningless when refuted.
    depth: u32,
    kind: Kind,
}

pub struct Table {
    slots: Vec<Slot>,
    mask: usize,
    filled: usize,
}

impl Table {
    /// Builds a table holding `entries` rounded up to a power of two, at least
    /// 1024. Each entry costs 24 bytes.
    pub fn with_entries(entries: usize) -> Table {
        let capacity = entries.max(1024).next_power_of_two();
        Table {
            slots: vec![
                Slot {
                    key: 0,
                    depth: 0,
                    kind: Kind::Empty,
                };
                capacity
            ],
            mask: capacity - 1,
            filled: 0,
        }
    }

    /// Entries a table of `bytes` bytes can hold.
    pub const fn entries_in(bytes: usize) -> usize {
        bytes / std::mem::size_of::<Slot>()
    }

    pub fn capacity(&self) -> usize {
        self.slots.len()
    }

    pub fn filled(&self) -> usize {
        self.filled
    }

    fn index(&self, key: u128) -> usize {
        (key as u64 as usize) & self.mask
    }

    /// What is known about `key` for a search with `depth` left to spend.
    pub fn probe(&self, key: u128, depth: u32) -> Probe {
        let slot = self.slots[self.index(key)];
        if slot.kind == Kind::Empty || slot.key != key {
            return Probe::Unknown;
        }
        match slot.kind {
            Kind::Refuted => Probe::Refuted,
            // A shallower previous visit says nothing about a deeper one.
            Kind::Abandoned { repetition_only } if slot.depth >= depth => {
                Probe::Exhausted { repetition_only }
            }
            _ => Probe::Unknown,
        }
    }

    /// Records that `key` was searched exhaustively and lost.
    pub fn record_refuted(&mut self, key: u128) {
        self.write(Slot {
            key,
            depth: 0,
            kind: Kind::Refuted,
        });
    }

    /// Records that `key` was searched with `depth` in hand and abandoned.
    /// `repetition_only` means nothing worse than a loop stopped it.
    pub fn record_abandoned(&mut self, key: u128, depth: u32, repetition_only: bool) {
        self.write(Slot {
            key,
            depth,
            kind: Kind::Abandoned { repetition_only },
        });
    }

    fn write(&mut self, slot: Slot) {
        let index = self.index(slot.key);
        let existing = self.slots[index];
        match existing.kind {
            Kind::Empty => self.filled += 1,
            // A proof is worth more than a partial result, and worth more than
            // a partial result for a different position.
            Kind::Refuted if slot.kind != Kind::Refuted && existing.key != slot.key => return,
            _ => {}
        }
        self.slots[index] = slot;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unseen_key_is_unknown() {
        let table = Table::with_entries(1024);
        assert_eq!(table.probe(12345, 10), Probe::Unknown);
    }

    #[test]
    fn a_refutation_is_reusable_at_any_depth() {
        let mut table = Table::with_entries(1024);
        table.record_refuted(99);
        assert_eq!(table.probe(99, 1), Probe::Refuted);
        assert_eq!(table.probe(99, 10_000), Probe::Refuted);
    }

    #[test]
    fn an_abandoned_entry_only_answers_shallower_visits() {
        let mut table = Table::with_entries(1024);
        table.record_abandoned(7, 50, false);
        let exhausted = Probe::Exhausted {
            repetition_only: false,
        };
        assert_eq!(table.probe(7, 50), exhausted);
        assert_eq!(table.probe(7, 20), exhausted);
        // More depth than last time means there is more to look at.
        assert_eq!(table.probe(7, 51), Probe::Unknown);
    }

    #[test]
    fn an_abandoned_entry_is_never_reported_as_a_refutation() {
        let mut table = Table::with_entries(1024);
        table.record_abandoned(3, 1000, false);
        assert_ne!(table.probe(3, 10), Probe::Refuted);
    }

    #[test]
    fn an_abandoned_entry_remembers_why_it_gave_up() {
        let mut table = Table::with_entries(1024);
        table.record_abandoned(11, 10, true);
        table.record_abandoned(12, 10, false);
        assert_eq!(
            table.probe(11, 5),
            Probe::Exhausted {
                repetition_only: true
            }
        );
        assert_eq!(
            table.probe(12, 5),
            Probe::Exhausted {
                repetition_only: false
            }
        );
    }

    #[test]
    fn a_full_key_mismatch_in_the_same_bucket_is_unknown() {
        let mut table = Table::with_entries(1024);
        let capacity = table.capacity() as u128;
        table.record_refuted(5);
        // Same bucket, different key: the stored key is compared in full.
        assert_eq!(table.probe(5 + capacity, 10), Probe::Unknown);
    }

    #[test]
    fn capacity_is_a_power_of_two_and_never_tiny() {
        assert_eq!(Table::with_entries(0).capacity(), 1024);
        assert_eq!(Table::with_entries(5000).capacity(), 8192);
    }
}
