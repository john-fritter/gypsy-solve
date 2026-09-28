//! Counts over a range of seeded deals, for testing that the shuffle is uniform.
//!
//! Every confidence interval this project quotes treats seeds `0..n` as an
//! i.i.d. sample of deals. The deals are SplitMix64 shuffles of sequential
//! seeds, so that is an assumption about the generator, and this is the data
//! that tests it. `analysis/shuffle_uniformity.py` does the statistics; this
//! only counts, so that the shuffle under test is `State::deck` itself rather
//! than a second implementation of it.
//!
//! Four tables, each with an exact combinatorial expectation under a uniform
//! shuffle:
//!
//! - **position**: how often each card lands at each of the 104 deck positions;
//! - **consecutive**: for seeds `s` and `s + 1`, how often card `a` in one deal
//!   sits at the same position as card `b` in the next. This is the test aimed
//!   at *sequential* seeds;
//! - **aces face up** and **aces face down** in the opening deal.
//!
//! `--control naive` deals with the textbook biased shuffle instead (swap each
//! position with any position, not any position at or below it). It must fail,
//! or the test has no teeth.
//!
//! Usage: `deal_stats --from 0 --to 1000000 [--control naive] > counts.json`

use std::process::ExitCode;

use gypsy_core::card::{Card, RANKS, SUITS};
use gypsy_core::rng::SplitMix64;
use gypsy_core::state::COLUMNS;
use gypsy_core::State;

const CARDS: usize = (RANKS * SUITS) as usize;
const DECK: usize = 2 * CARDS;
/// The deal takes the deck in order: one face-down row, then two face-up rows.
const FACE_DOWN: std::ops::Range<usize> = 0..COLUMNS;
const FACE_UP: std::ops::Range<usize> = COLUMNS..3 * COLUMNS;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let mut from: u64 = 0;
    let mut to: Option<u64> = None;
    let mut naive = false;
    while let Some(arg) = args.next() {
        let mut value = || args.next().unwrap_or_default();
        match arg.as_str() {
            "--from" => from = value().parse().expect("--from takes a seed"),
            "--to" => to = Some(value().parse().expect("--to takes a seed")),
            "--control" => match value().as_str() {
                "naive" => naive = true,
                other => {
                    eprintln!("unknown control {other:?}; the only one is `naive`");
                    return ExitCode::FAILURE;
                }
            },
            other => {
                eprintln!("unknown argument {other:?}");
                return ExitCode::FAILURE;
            }
        }
    }
    let Some(to) = to.filter(|&to| to > from + 1) else {
        eprintln!("usage: deal_stats --from A --to B [--control naive], with B > A + 1");
        return ExitCode::FAILURE;
    };

    let mut position = vec![[0u64; CARDS]; DECK];
    let mut consecutive = vec![[0u64; CARDS]; CARDS];
    let mut aces_up = [0u64; 9];
    let mut aces_down = [0u64; 9];
    let mut previous: Option<Vec<Card>> = None;

    for seed in from..to {
        let deck = if naive {
            naive_deck(seed)
        } else {
            real_deck(seed)
        };
        for (slot, card) in deck.iter().enumerate() {
            position[slot][card.index() as usize] += 1;
        }
        if let Some(previous) = &previous {
            for (a, b) in previous.iter().zip(&deck) {
                consecutive[a.index() as usize][b.index() as usize] += 1;
            }
        }
        let aces =
            |range: std::ops::Range<usize>| deck[range].iter().filter(|c| c.rank() == 1).count();
        aces_down[aces(FACE_DOWN)] += 1;
        aces_up[aces(FACE_UP)] += 1;
        previous = Some(deck);
    }

    let rows = |table: &[[u64; CARDS]]| {
        table
            .iter()
            .map(|row| format!("[{}]", join(row)))
            .collect::<Vec<_>>()
            .join(",")
    };
    println!(
        "{{\"from\":{from},\"to\":{to},\"control\":{},\"cards\":{CARDS},\"deck\":{DECK},\
         \"face_down\":{},\"face_up\":{},\
         \"position\":[{}],\"consecutive\":[{}],\"aces_face_down\":[{}],\"aces_face_up\":[{}]}}",
        if naive { "\"naive\"" } else { "null" },
        FACE_DOWN.len(),
        FACE_UP.len(),
        rows(&position),
        rows(&consecutive),
        join(&aces_down),
        join(&aces_up),
    );
    ExitCode::SUCCESS
}

/// The deck every result in this project was dealt from, checked against the
/// deal so that the face-up and face-down slices above mean what they say.
fn real_deck(seed: u64) -> Vec<Card> {
    let deck = State::deck(seed);
    let deal = State::deal(seed);
    for (index, column) in deal.columns.iter().enumerate() {
        assert_eq!(column.hidden(), 1);
        assert_eq!(column.cards()[0], deck[FACE_DOWN.start + index]);
        assert_eq!(column.cards()[1], deck[FACE_UP.start + index]);
        assert_eq!(column.cards()[2], deck[FACE_UP.start + COLUMNS + index]);
    }
    deck
}

/// The control: the same starting order and generator, with the biased swap.
/// This is a deliberately wrong shuffle, not a second copy of the real one.
fn naive_deck(seed: u64) -> Vec<Card> {
    let mut deck: Vec<Card> = (0..2)
        .flat_map(|_| 0..CARDS as u8)
        .map(Card::from_index)
        .collect();
    let mut rng = SplitMix64::new(seed);
    for i in 0..deck.len() {
        let j = rng.below(deck.len() as u64) as usize;
        deck.swap(i, j);
    }
    deck
}

fn join(values: &[u64]) -> String {
    values
        .iter()
        .map(u64::to_string)
        .collect::<Vec<_>>()
        .join(",")
}
