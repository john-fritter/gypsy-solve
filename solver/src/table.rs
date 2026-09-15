//! The transposition table: the set of positions already expanded.
//!
//! Nothing more than that is needed. Skipping any position that has already
//! been expanded is sound for the question being asked: expanding a position
//! generates all of its children, so along a shortest winning line every
//! position is either expanded or skipped-because-expanded, and the expansion
//! of the second-to-last produces the win. Depth never enters the argument.
//! See `search` for the full version.
//!
//! ## Why the table probes rather than replacing
//!
//! That argument has a premise: a position recorded as expanded stays
//! recorded. A fixed-size table can always lose an entry, and losing one is
//! sound — it costs a re-expansion, never a wrong verdict — so an earlier
//! version mapped each key to one slot and replaced whatever was there.
//!
//! The cost of that turned out not to be one re-expansion. Two keys sharing a
//! slot evict each other, and when both sit on a path the search walks often
//! they do it indefinitely: each eviction causes a re-expansion, which causes
//! the reverse eviction. Measured on 2026-09-15, Gypsy deal 2 stopped finding
//! new positions after about 266,000 of them and spent the remaining 91% of a
//! three-million-node budget re-expanding what it had already seen, and
//! Klondike deal 3 wasted 32% of the same budget the same way. Both came back
//! `unknown` for want of a budget they were mostly burning on repeats.
//!
//! Probing forward from the home slot fixes it by giving the second key a slot
//! of its own. The same two deals now re-expand 7 positions and 0 positions
//! respectively out of three million.
//!
//! Fixed size, so memory is decided up front rather than discovered during a
//! batch run.

/// A slot. An all-zero key means empty; a real key of zero would cost one
/// re-expansion every time it came up, at a probability of `2^-128`.
type Slot = u128;

/// How far forward `insert` and `contains` probe from the home slot.
///
/// Eight keeps the window inside one or two cache lines while making a
/// displacement need eight independent collisions, which at the load factors
/// this table runs at is rare enough to stop the thrashing described above.
const PROBES: usize = 8;

pub struct Table {
    slots: Vec<Slot>,
    mask: usize,
    filled: usize,
}

impl Table {
    /// Builds a table holding `entries` rounded up to a power of two, at least
    /// 1024. Each entry costs 16 bytes.
    pub fn with_entries(entries: usize) -> Table {
        let capacity = entries.max(1024).next_power_of_two();
        Table {
            slots: vec![0; capacity],
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

    /// True when this position has already been expanded.
    ///
    /// An empty slot ends the search: `insert` places a key at the first empty
    /// slot at or after its home, so an empty slot before the key means the
    /// key was never placed. Entries are never removed, only overwritten, so a
    /// chain once formed stays unbroken.
    pub fn contains(&self, key: u128) -> bool {
        if key == 0 {
            return false;
        }
        let mut index = (key as u64 as usize) & self.mask;
        for _ in 0..PROBES {
            match self.slots[index] {
                slot if slot == key => return true,
                0 => return false,
                _ => index = (index + 1) & self.mask,
            }
        }
        false
    }

    /// Records that this position has been expanded.
    pub fn insert(&mut self, key: u128) {
        if key == 0 {
            return;
        }
        let mut index = (key as u64 as usize) & self.mask;
        for _ in 0..PROBES {
            match self.slots[index] {
                slot if slot == key => return,
                0 => {
                    self.slots[index] = key;
                    self.filled += 1;
                    return;
                }
                _ => index = (index + 1) & self.mask,
            }
        }
        // Every slot in the window belongs to another key. Displace the last
        // one probed rather than `index`, which has run one past the window:
        // a key written outside the window is somewhere `contains` never
        // looks, so it would be re-expanded every time it came up.
        self.slots[index.wrapping_sub(1) & self.mask] = key;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unseen_key_is_absent() {
        let table = Table::with_entries(1024);
        assert!(!table.contains(12345));
    }

    #[test]
    fn an_inserted_key_is_found_again() {
        let mut table = Table::with_entries(1024);
        table.insert(99);
        assert!(table.contains(99));
        assert_eq!(table.filled(), 1);
    }

    #[test]
    fn a_full_key_mismatch_in_the_same_bucket_is_absent() {
        let table = Table::with_entries(1024);
        let capacity = table.capacity() as u128;
        // Nothing was inserted, so the home slot is empty and the chain ends.
        assert!(!table.contains(5 + capacity));
    }

    /// The point of probing: two keys sharing a home slot both keep their
    /// place, where the old direct-mapped table lost one of them.
    #[test]
    fn colliding_keys_both_stay() {
        let mut table = Table::with_entries(1024);
        let capacity = table.capacity() as u128;
        table.insert(7);
        table.insert(7 + capacity);
        assert!(table.contains(7), "the incumbent is not evicted");
        assert!(
            table.contains(7 + capacity),
            "the newcomer gets its own slot"
        );
        assert_eq!(table.filled(), 2);
    }

    #[test]
    fn reinserting_a_key_is_idempotent() {
        let mut table = Table::with_entries(1024);
        table.insert(42);
        table.insert(42);
        assert_eq!(table.filled(), 1);
    }

    /// A displaced key has to land where `contains` will look for it, or it is
    /// re-expanded every time it comes up.
    #[test]
    fn a_displaced_key_is_still_found() {
        let mut table = Table::with_entries(1024);
        let capacity = table.capacity() as u128;
        // Fill the whole probe window of home slot 3 with other keys.
        for step in 0..PROBES as u128 {
            table.insert(3 + step + capacity * (step + 1));
        }
        let crowded = 3 + capacity * 99;
        table.insert(crowded);
        assert!(table.contains(crowded), "displaced inside the probe window");
    }

    #[test]
    fn capacity_is_a_power_of_two_and_never_tiny() {
        assert_eq!(Table::with_entries(0).capacity(), 1024);
        assert_eq!(Table::with_entries(5000).capacity(), 8192);
    }
}
