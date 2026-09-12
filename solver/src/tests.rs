//! Tests for the search.
//!
//! Positions are built by putting chosen cards in the stock of an otherwise
//! empty position and dealing them, since `Column`'s internals are private to
//! `gypsy_core`. That deals one card to each of the eight columns, face up,
//! which is enough shape for the cases that matter here.

use gypsy_core::card::Suit;
use gypsy_core::state::FOUNDATIONS;
use gypsy_core::{Card, Move, MoveOptions, State};

use crate::{solve, Config, Limit, Verdict};

const fn card(suit: Suit, rank: u8) -> Card {
    Card::new(suit, rank)
}

/// One card per column, nothing in the stock, foundations as given.
fn dealt_row(row: [Card; 8], foundations: [u8; FOUNDATIONS]) -> State {
    let mut state = State {
        columns: Default::default(),
        foundations: [0; FOUNDATIONS],
        stock: row.to_vec(),
    };
    state.apply(Move::Stock).expect("stock deal is legal");
    state.foundations = foundations;
    state
}

/// Every king, one per column. Nothing stacks on a king and no column is
/// empty, so this position has no moves at all.
fn eight_kings(foundations: [u8; FOUNDATIONS]) -> State {
    dealt_row(
        [
            card(Suit::Spades, 13),
            card(Suit::Spades, 13),
            card(Suit::Hearts, 13),
            card(Suit::Hearts, 13),
            card(Suit::Clubs, 13),
            card(Suit::Clubs, 13),
            card(Suit::Diamonds, 13),
            card(Suit::Diamonds, 13),
        ],
        foundations,
    )
}

fn config(budget: u64, depth: u32) -> Config {
    Config {
        options: MoveOptions::ALL,
        node_budget: budget,
        max_depth: depth,
        table_entries: 1 << 16,
    }
}

#[test]
fn a_position_with_no_moves_is_proven_unsolvable() {
    let state = eight_kings([0; FOUNDATIONS]);
    assert!(state.legal_moves(MoveOptions::ALL).is_empty());

    let report = solve(&state, config(1_000, 100)).expect("no solution to verify");
    assert_eq!(report.verdict, Verdict::Unsolvable);
    assert_eq!(report.limit, None);
    assert!(report.line.is_none());
}

#[test]
fn a_position_one_move_from_home_is_solved_and_the_line_replays() {
    // Every foundation on its queen, every king still out.
    let state = eight_kings([12; FOUNDATIONS]);
    let report = solve(&state, config(10_000, 100)).expect("solution must verify");

    assert_eq!(report.verdict, Verdict::Solvable);
    let line = report.line.expect("a solvable deal carries its line");
    assert_eq!(line.len(), 8, "one king per foundation");

    let mut replayed = state.clone();
    for mv in &line {
        replayed.apply(*mv).expect("the reported line is legal");
    }
    assert!(replayed.is_won());
}

#[test]
fn an_already_won_position_needs_no_moves() {
    let state = State {
        columns: Default::default(),
        foundations: [13; FOUNDATIONS],
        stock: Vec::new(),
    };
    let report = solve(&state, config(10, 10)).expect("solution must verify");
    assert_eq!(report.verdict, Verdict::Solvable);
    assert_eq!(report.line, Some(Vec::new()));
}

#[test]
fn a_spent_budget_is_unknown_and_never_unsolvable() {
    let report = solve(&State::deal(1), config(50, 400)).expect("no solution to verify");
    assert_eq!(report.verdict, Verdict::Unknown);
    assert_eq!(report.limit, Some(Limit::Budget));
    assert!(report.nodes <= 50 + 1);
}

#[test]
fn a_spent_depth_limit_is_unknown_and_never_unsolvable() {
    let report = solve(&State::deal(1), config(100_000, 3)).expect("no solution to verify");
    assert_eq!(report.verdict, Verdict::Unknown);
    assert_eq!(report.limit, Some(Limit::Depth));
}

/// The whole point of the three-valued result: a real deal under a small
/// budget must not come back claiming a proof.
#[test]
fn real_deals_under_a_small_budget_never_claim_a_proof() {
    for seed in 0..8 {
        let report = solve(&State::deal(seed), config(20_000, 400)).expect("verified");
        assert_ne!(
            report.verdict,
            Verdict::Unsolvable,
            "seed {seed} cannot be refuted on 20k nodes"
        );
        if report.verdict == Verdict::Unknown {
            assert!(report.limit.is_some(), "unknown must say what stopped it");
        }
    }
}

#[test]
fn the_search_is_deterministic() {
    let first = solve(&State::deal(5), config(30_000, 400)).expect("verified");
    let second = solve(&State::deal(5), config(30_000, 400)).expect("verified");
    assert_eq!(first.verdict, second.verdict);
    assert_eq!(first.nodes, second.nodes);
    assert_eq!(first.line, second.line);
    assert_eq!(first.table_filled, second.table_filled);
}

#[test]
fn worry_back_can_only_widen_the_search() {
    let restricted = Config {
        options: MoveOptions::NO_WORRY_BACK,
        ..config(20_000, 400)
    };
    let full = config(20_000, 400);
    let a = solve(&State::deal(2), restricted).expect("verified");
    let b = solve(&State::deal(2), full).expect("verified");
    // Nothing is asserted about which is faster, only that both stay honest.
    for report in [a, b] {
        assert!(report.verdict != Verdict::Solvable || report.line.is_some());
    }
}

/// A deep line must not blow the call stack, which a recursive search does.
#[test]
fn a_deep_search_does_not_overflow_the_stack() {
    let report = solve(&State::deal(3), config(200_000, 5_000)).expect("verified");
    assert!(matches!(
        report.verdict,
        Verdict::Unknown | Verdict::Solvable | Verdict::Unsolvable
    ));
}

#[test]
fn a_zero_depth_limit_answers_unknown_rather_than_panicking() {
    let report = solve(&State::deal(1), config(1_000, 0)).expect("verified");
    assert_eq!(report.verdict, Verdict::Unknown);
    assert_eq!(report.limit, Some(Limit::Depth));
}
