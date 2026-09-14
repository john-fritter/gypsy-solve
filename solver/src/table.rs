//! The transposition table: the set of positions already expanded.
//!
//! Nothing more than that is needed, and the previous version's extra
//! machinery was actively harmful. It recorded how much depth a search had in
//! hand when it gave up, and answered a later probe only when that later visit
//! had no more depth to spend. Depth-first search reaches the same position
//! with a different amount of depth in hand almost every time, so most probes
//! missed and the position was searched again — 20 to 57 times over, measured.
//!
//! Skipping any position that has already been expanded is sound for the
//! question being asked. Expanding a position generates all of its children,
//! so along a shortest winning line every position is either expanded or
//! skipped-because-expanded, and the expansion of the second-to-last produces
//! the win. Depth never enters the argument. See `search` for the full
//! version.
//!
//! Fixed size and direct mapped, so memory is decided up front rather than
//! discovered during a batch run. Eviction costs a re-expansion and nothing
//! else.

/// A slot. An all-zero key means empty; a real key of zero would cost one
/// re-expansion every time it came up, at a probability of `2^-128`.
type Slot = u128;

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
    pub fn contains(&self, key: u128) -> bool {
        key != 0 && self.slots[(key as u64 as usize) & self.mask] == key
    }

    /// Records that this position has been expanded, evicting whatever shared
    /// its slot.
    pub fn insert(&mut self, key: u128) {
        let index = (key as u64 as usize) & self.mask;
        if self.slots[index] == 0 {
            self.filled += 1;
        }
        self.slots[index] = key;
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
        let mut table = Table::with_entries(1024);
        let capacity = table.capacity() as u128;
        table.insert(5);
        // Same bucket, different key: the stored key is compared in full.
        assert!(!table.contains(5 + capacity));
    }

    #[test]
    fn eviction_replaces_rather_than_corrupts() {
        let mut table = Table::with_entries(1024);
        let capacity = table.capacity() as u128;
        table.insert(7);
        table.insert(7 + capacity);
        assert!(table.contains(7 + capacity));
        assert!(!table.contains(7), "evicted, so it will be expanded again");
    }

    #[test]
    fn capacity_is_a_power_of_two_and_never_tiny() {
        assert_eq!(Table::with_entries(0).capacity(), 1024);
        assert_eq!(Table::with_entries(5000).capacity(), 8192);
    }
}
