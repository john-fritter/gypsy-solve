//! A small, frozen PRNG so that a seed means the same deal forever.
//!
//! Deals have to be reproducible by a stranger years from now, so the shuffle
//! cannot depend on an external crate whose algorithm may change between
//! versions. This is SplitMix64, as published by Steele, Lea and Flood, with
//! rejection sampling for unbiased bounded draws.

/// SplitMix64 generator.
#[derive(Clone, Debug)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    pub const fn new(seed: u64) -> SplitMix64 {
        SplitMix64 { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform draw from `0..bound`, with no modulo bias. Panics on `bound == 0`.
    pub fn below(&mut self, bound: u64) -> u64 {
        assert!(bound > 0, "bound must be positive");
        // Values below the threshold would make `% bound` favour low results,
        // so draw again instead.
        let threshold = bound.wrapping_neg() % bound; // 2^64 mod bound
        loop {
            let value = self.next_u64();
            if value >= threshold {
                return value % bound;
            }
        }
    }

    /// In-place Fisher-Yates shuffle, walking from the top down.
    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = self.below(i as u64 + 1) as usize;
            items.swap(i, j);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_reference_splitmix64_output() {
        // Reference values for seed 0 from the original SplitMix64 publication.
        let mut rng = SplitMix64::new(0);
        assert_eq!(rng.next_u64(), 0xE220_A839_7B1D_CDAF);
        assert_eq!(rng.next_u64(), 0x6E78_9E6A_A1B9_65F4);
        assert_eq!(rng.next_u64(), 0x06C4_5D18_8009_454F);
    }

    #[test]
    fn shuffle_is_a_permutation_and_is_seed_stable() {
        let original: Vec<u32> = (0..64).collect();

        let mut a = original.clone();
        SplitMix64::new(7).shuffle(&mut a);
        let mut b = original.clone();
        SplitMix64::new(7).shuffle(&mut b);
        assert_eq!(a, b);

        let mut c = original.clone();
        SplitMix64::new(8).shuffle(&mut c);
        assert_ne!(a, c);

        let mut sorted = a.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, original);
    }

    #[test]
    fn below_stays_in_range_and_covers_it() {
        let mut rng = SplitMix64::new(12345);
        let mut seen = [false; 5];
        for _ in 0..1000 {
            let value = rng.below(5);
            assert!(value < 5);
            seen[value as usize] = true;
        }
        assert!(seen.iter().all(|&s| s));
    }

    #[test]
    fn below_one_is_always_zero() {
        let mut rng = SplitMix64::new(1);
        for _ in 0..10 {
            assert_eq!(rng.below(1), 0);
        }
    }
}
