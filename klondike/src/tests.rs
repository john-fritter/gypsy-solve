//! Klondike through the solver: the key's symmetries, and end-to-end solves.

use gypsy_core::{Card, Suit};
use gypsy_solver::{solve, Config, Game, Verdict};

use crate::rules::{Action, Position, FOUNDATIONS, PILES};
use crate::Klondike;

const fn card(suit: Suit, rank: u8) -> Card {
    Card::new(suit, rank)
}

fn config(budget: u64, depth: u32) -> Config {
    Config {
        node_budget: budget,
        max_depth: depth,
        table_entries: 1 << 18,
    }
}

/// The contrast with Gypsy, and the reason both games are worth having. No
/// Klondike rule addresses a pile from outside — a draw goes to the waste —
/// so a permutation of the piles is only a relabelling.
#[test]
fn pile_order_never_matters_even_with_cards_left_to_draw() {
    let game = Klondike::new();
    let left = Position::deal(7);
    let mut right = left.clone();
    right.piles.swap(0, 6);

    assert!(!left.talon.is_empty(), "there are still cards to draw");
    assert_eq!(
        game.key(&left),
        game.key(&right),
        "nothing deals to a named pile, so pile order is a relabelling"
    );
}

#[test]
fn the_order_of_the_talon_matters() {
    let game = Klondike::new();
    let left = Position::deal(7);
    let mut right = left.clone();
    right.talon.swap(0, 1);
    assert_ne!(game.key(&left), game.key(&right));
}

#[test]
fn how_far_through_the_talon_matters() {
    let game = Klondike::new();
    let before = Position::deal(7);
    let mut after = before.clone();
    after.apply(Action::Draw).expect("drawing is legal");
    assert_ne!(game.key(&before), game.key(&after));
}

#[test]
fn the_same_position_always_hashes_the_same() {
    let game = Klondike::new();
    assert_eq!(game.key(&Position::deal(3)), game.key(&Position::deal(3)));
    assert_ne!(game.key(&Position::deal(3)), game.key(&Position::deal(4)));
}

#[test]
fn a_position_with_no_moves_is_proven_unsolvable() {
    // Four aces buried under nothing, no talon, and no legal move: the piles
    // hold single low cards of the same colour that cannot stack.
    let state = Position::from_parts(
        [
            (0, &[card(Suit::Spades, 5)]),
            (0, &[card(Suit::Clubs, 7)]),
            (0, &[card(Suit::Spades, 9)]),
            (0, &[card(Suit::Clubs, 11)]),
            (0, &[card(Suit::Spades, 3)]),
            (0, &[card(Suit::Clubs, 13)]),
            (0, &[card(Suit::Spades, 13)]),
        ],
        [0; FOUNDATIONS],
        &[],
        0,
    );
    assert!(state.legal_actions().is_empty(), "this position is stuck");

    let game = Klondike::new();
    let report = solve(&game, &state, config(10_000, 100)).expect("no solution to verify");
    assert_eq!(report.verdict, Verdict::Unsolvable);
    assert_eq!(report.limit, None);
}

#[test]
fn a_position_four_moves_from_home_is_solved_and_replays() {
    let mut piles: [(usize, &[Card]); PILES] = [(0, &[]); PILES];
    let kings = [
        [card(Suit::Spades, 13)],
        [card(Suit::Hearts, 13)],
        [card(Suit::Clubs, 13)],
        [card(Suit::Diamonds, 13)],
    ];
    for (slot, king) in kings.iter().enumerate() {
        piles[slot] = (0, king);
    }
    let state = Position::from_parts(piles, [12; FOUNDATIONS], &[], 0);

    let game = Klondike::new();
    let report = solve(&game, &state, config(10_000, 100)).expect("solution must verify");
    assert_eq!(report.verdict, Verdict::Solvable);
    let line = report.line.expect("a solvable deal carries its line");
    assert_eq!(line.len(), 4, "one king per foundation");

    let mut replayed = state.clone();
    for action in &line {
        replayed.apply(*action).expect("the reported line is legal");
    }
    assert!(replayed.is_won());
}

#[test]
fn a_spent_budget_is_unknown_and_never_unsolvable() {
    let game = Klondike::new();
    let report = solve(&game, &Position::deal(1), config(100, 300)).expect("verified");
    assert_eq!(report.verdict, Verdict::Unknown);
    assert!(report.limit.is_some());
}

/// The validation in miniature: at a real budget, real deals must produce
/// verified wins. A search that never wins here is broken, not slow.
#[test]
fn real_deals_are_solved_at_a_real_budget() {
    let game = Klondike::new();
    let solved = (0..6)
        .filter(|&seed| {
            let report = solve(&game, &Position::deal(seed), config(400_000, 400))
                .expect("any solution must verify");
            report.verdict == Verdict::Solvable
        })
        .count();
    assert!(
        solved > 0,
        "no Klondike deal solved in six tries: the search is wrong, not slow"
    );
}
