//! Tests for the search.
//!
//! Positions are built by putting chosen cards in the stock of an otherwise
//! empty position and dealing them, since `Column`'s internals are private to
//! `gypsy_core`. That deals one card to each of the eight columns, face up,
//! which is enough shape for the cases that matter here.

use gypsy_core::card::Suit;
use gypsy_core::card::RANKS;
use gypsy_core::state::FOUNDATIONS;
use gypsy_core::{Card, Move, MoveOptions, State};

use crate::{solve, solve_restarting, Config, Gypsy, Limit, Verdict};

const fn card(suit: Suit, rank: u8) -> Card {
    Card::new(suit, rank)
}

/// One card per column, nothing in the stock, foundations as given.
fn dealt_row(row: [Card; 8], foundations: [u8; FOUNDATIONS]) -> State {
    let mut state = State {
        columns: Default::default(),
        foundations: [0; FOUNDATIONS],
        stock: row.to_vec(),
        top_rank: RANKS,
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
        node_budget: budget,
        max_depth: depth,
        table_entries: 1 << 16,
        ordering_salt: 0,
    }
}

fn full_rules() -> Gypsy {
    Gypsy::new(MoveOptions::ALL)
}

#[test]
fn a_position_with_no_moves_is_proven_unsolvable() {
    let state = eight_kings([0; FOUNDATIONS]);
    assert!(state.legal_moves(MoveOptions::ALL).is_empty());

    let report = solve(&full_rules(), &state, config(1_000, 100)).expect("no solution to verify");
    assert_eq!(report.verdict, Verdict::Unsolvable);
    assert_eq!(report.limit, None);
    assert!(report.line.is_none());
}

#[test]
fn a_position_one_move_from_home_is_solved_and_the_line_replays() {
    // Every foundation on its queen, every king still out.
    let state = eight_kings([12; FOUNDATIONS]);
    let report = solve(&full_rules(), &state, config(10_000, 100)).expect("solution must verify");

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
        foundations: [RANKS; FOUNDATIONS],
        stock: Vec::new(),
        top_rank: RANKS,
    };
    let report = solve(&full_rules(), &state, config(10, 10)).expect("solution must verify");
    assert_eq!(report.verdict, Verdict::Solvable);
    assert_eq!(report.line, Some(Vec::new()));
}

#[test]
fn a_spent_budget_is_unknown_and_never_unsolvable() {
    let report =
        solve(&full_rules(), &State::deal(1), config(50, 400)).expect("no solution to verify");
    assert_eq!(report.verdict, Verdict::Unknown);
    assert_eq!(report.limit, Some(Limit::Budget));
    assert!(report.nodes <= 50 + 1);
}

#[test]
fn a_spent_depth_limit_is_unknown_and_never_unsolvable() {
    let report =
        solve(&full_rules(), &State::deal(1), config(100_000, 3)).expect("no solution to verify");
    assert_eq!(report.verdict, Verdict::Unknown);
    assert_eq!(report.limit, Some(Limit::Depth));
}

/// The whole point of the three-valued result: a real deal under a small
/// budget must not come back claiming a proof.
#[test]
fn real_deals_under_a_small_budget_never_claim_a_proof() {
    for seed in 0..8 {
        let report =
            solve(&full_rules(), &State::deal(seed), config(20_000, 400)).expect("verified");
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
    let first = solve(&full_rules(), &State::deal(5), config(30_000, 400)).expect("verified");
    let second = solve(&full_rules(), &State::deal(5), config(30_000, 400)).expect("verified");
    assert_eq!(first.verdict, second.verdict);
    assert_eq!(first.nodes, second.nodes);
    assert_eq!(first.line, second.line);
    assert_eq!(first.table_filled, second.table_filled);
}

#[test]
fn worry_back_can_only_widen_the_search() {
    let restricted = Gypsy::new(MoveOptions::NO_WORRY_BACK);
    let a = solve(&restricted, &State::deal(2), config(20_000, 400)).expect("verified");
    let b = solve(&full_rules(), &State::deal(2), config(20_000, 400)).expect("verified");
    // Nothing is asserted about which is faster, only that both stay honest.
    for report in [a, b] {
        assert!(report.verdict != Verdict::Solvable || report.line.is_some());
    }
}

/// A deep line must not blow the call stack, which a recursive search does.
#[test]
fn a_deep_search_does_not_overflow_the_stack() {
    let report = solve(&full_rules(), &State::deal(3), config(200_000, 5_000)).expect("verified");
    assert!(matches!(
        report.verdict,
        Verdict::Unknown | Verdict::Solvable | Verdict::Unsolvable
    ));
}

#[test]
fn a_zero_depth_limit_answers_unknown_rather_than_panicking() {
    let report = solve(&full_rules(), &State::deal(1), config(1_000, 0)).expect("verified");
    assert_eq!(report.verdict, Verdict::Unknown);
    assert_eq!(report.limit, Some(Limit::Depth));
}

/// A single restart is the search that existed before restarts did. Every
/// recorded result in `docs/results` was produced that way, so this is what
/// keeps them comparable.
#[test]
fn one_restart_is_the_plain_search() {
    let game = full_rules();
    let start = State::deal(21);
    let config = config(200_000, 100_000);

    let plain = solve(&game, &start, config).expect("a win replays");
    let restarting = solve_restarting(&game, &start, config, 1).expect("a win replays");

    assert_eq!(plain.verdict, restarting.verdict);
    assert_eq!(plain.nodes, restarting.nodes);
    assert_eq!(
        plain.line.map(|line| line.len()),
        restarting.line.map(|line| line.len())
    );
    assert_eq!(restarting.restarts_used, 1);
}

/// Restarts split the budget rather than multiplying it: eight of them cost
/// what one did, which is what makes the comparison fair.
#[test]
fn restarts_split_the_budget_they_are_given() {
    let game = full_rules();
    let start = State::deal(3);
    let budget = 40_000;
    let report = solve_restarting(&game, &start, config(budget, 100_000), 8).expect("no false win");

    assert_eq!(report.verdict, Verdict::Unknown);
    assert!(
        report.nodes <= budget,
        "restarts spent {} nodes of a {budget} budget",
        report.nodes
    );
    assert_eq!(report.restarts_used, 8);
}

/// The one thing a restart must never do. Exhausting a slice proves nothing,
/// so a deal that no slice can finish stays `unknown` however many slices it
/// is cut into.
#[test]
fn a_spent_slice_is_unknown_and_never_a_refutation() {
    let game = full_rules();
    let report =
        solve_restarting(&game, &State::deal(4), config(5_000, 100_000), 5).expect("no false win");
    assert_eq!(report.verdict, Verdict::Unknown);
    assert_eq!(report.limit, Some(Limit::Budget));
}

/// Why restarts are here at all, in one deal. Seed 7 spends a 5,000-node
/// budget on a single descent and decides nothing; the same budget cut five
/// ways wins on the third slice, in 2,901 nodes all told. The win is replayed
/// before it is returned, as every win is.
#[test]
fn restarts_find_a_win_one_descent_walks_past() {
    let game = full_rules();
    let start = State::deal(7);
    let budget = config(5_000, 100_000);

    let one = solve_restarting(&game, &start, budget, 1).expect("no false win");
    assert_eq!(one.verdict, Verdict::Unknown);

    let five = solve_restarting(&game, &start, budget, 5).expect("a win replays");
    assert_eq!(five.verdict, Verdict::Solvable);
    assert!(
        five.nodes < one.nodes,
        "the win cost {} nodes against a spent {}",
        five.nodes,
        one.nodes
    );
    assert!(five.restarts_used > 1, "the first ordering did not find it");
}

/// A refutation stands whatever ordering found it: the position with no moves
/// at all is refuted by the first slice, and the rest are not needed.
#[test]
fn a_refutation_from_the_first_slice_ends_the_run() {
    let game = full_rules();
    let start = eight_kings([0; FOUNDATIONS]);
    let report = solve_restarting(&game, &start, config(1_000, 100_000), 4).expect("no false win");
    assert_eq!(report.verdict, Verdict::Unsolvable);
    assert_eq!(report.restarts_used, 1);
}

/// The project's first refutation of a real Gypsy deal, and the only test here
/// that can catch a dominance which discards winning lines.
///
/// Every other Gypsy check is a win replayed from the deal, and replaying a win
/// cannot fail in that direction: a rule that throws away the *only* winning
/// line turns a solvable deal into `unsolvable`, and until there was a deal
/// proved unsolvable there was nothing to notice it on. Klondike supplies
/// refutations but is single-deck, so it never exercises duplicate cards,
/// group moves as a unit, or the deal-to-every-column stock — the three things
/// every two-deck proof in this crate had to extend past.
///
/// Rank cap 4, seed 1064, exhausted in both arms. If a future dominance makes
/// this deal `solvable`, either the rule or this verdict is wrong; if it makes
/// it `unknown`, the search stopped exhausting a game it used to finish.
#[test]
fn the_capped_deal_that_cannot_be_won_stays_unwon() {
    let state = State::deal_capped(1064, 4);
    for (name, rules) in [
        ("no-worry-back", Gypsy::new(MoveOptions::NO_WORRY_BACK)),
        ("full", full_rules()),
    ] {
        let report = solve(&rules, &state, config(20_000_000, 10_000))
            .expect("a refutation has no line to verify");
        assert_eq!(report.verdict, Verdict::Unsolvable, "{name} arm");
        assert_eq!(report.limit, None, "{name} arm stopped on a limit");
    }
}

/// A cap changes the deck and nothing else, so the search must still reach a
/// verdict rather than run off the end of a game it does not recognise.
#[test]
fn capped_deals_resolve_in_both_arms() {
    for seed in 0..25 {
        let state = State::deal_capped(seed, 4);
        for rules in [Gypsy::new(MoveOptions::NO_WORRY_BACK), full_rules()] {
            let report = solve(&rules, &state, config(5_000_000, 10_000)).expect("verified");
            assert_eq!(report.limit, None, "seed {seed} hit a limit");
            assert_ne!(report.verdict, Verdict::Unknown, "seed {seed}");
        }
    }
}


/// The counterexample that shows safe autoplay is unsound for two-deck Gypsy.
///
/// Seed 104720 at rank cap 4 has a 39-move win that uses **no worry-back at
/// all**, so the restricted arm must find it. With `never_wanted_in_the_tableau`
/// forcing, that arm instead exhausts the game in 27 nodes and reports
/// `unsolvable`: the rule discards every winning line. The same happens on
/// seeds 47318, 66930 and 179898, four false refutations in 200,000 cap-4
/// deals. With the rule removed all four come back `solvable`, and the other
/// 23 refutations in that set stand.
///
/// The divergence is at move 4, where nine moves are rules-legal and the rule
/// offers one: it forces a two up, on the argument that nothing ever needs a
/// two as a base. That argument is what licenses the rule at every rank cap,
/// the real game included, so a counterexample at cap 4 refutes it.
///
/// Fixed 2026-09-17 by dropping the rank-2 shortcut, so this now passes and is
/// a regression guard. See `DECISIONS.md`.
#[test]
fn safe_autoplay_must_not_refute_a_deal_that_has_a_worry_back_free_win() {
    let restricted = Gypsy::new(MoveOptions::NO_WORRY_BACK);
    for seed in [47318, 66930, 104720, 179898] {
        let state = State::deal_capped(seed, 4);
        let report = solve(&restricted, &state, config(50_000_000, 10_000)).expect("verified");
        assert_eq!(
            report.verdict,
            Verdict::Solvable,
            "seed {seed} has a win the restricted arm must find"
        );
    }
}



/// Seed 188: the only Gypsy deal at thirteen ranks known to be unwinnable, and
/// the only test here that can catch a dominance which discards winning lines
/// in the real game.
///
/// Every other Gypsy check replays a win, and replaying a win cannot fail in
/// that direction: a rule that throws away the *last* winning line turns a
/// solvable deal into `unsolvable`, which needs a deal with no winning line to
/// notice. The rank-4 set (`the_capped_deal_that_cannot_be_won_stays_unwon`)
/// supplies those at four ranks; this is the full game.
///
/// Proved on 2026-09-20 with each dominance removed in turn — 4,203,474 nodes
/// restricted with all of them, 226,504,270 with safe autoplay removed, and
/// 19,801,449 in the full arm, every one exhausted without touching a limit.
/// Worry-back does not save it either. See `DECISIONS.md`.
///
/// The restricted arm is the cheap one and is what runs here. If this ever
/// returns `solvable`, a dominance is discarding winning lines. If it returns
/// `unknown`, the search stopped exhausting a game it used to finish, which is
/// a budget question rather than a soundness one — raise it and re-check rather
/// than concluding anything.
#[test]
fn the_deal_that_cannot_be_won_stays_unwon() {
    let state = State::deal(188);
    // The shared `config` helper gives 65,536 entries, which a 4.2M-node
    // exhaustive search oversubscribes sixty-four fold: it would thrash and
    // never reach the end. An exhaustion test has to hold what it expands.
    let report = solve(
        &Gypsy::new(MoveOptions::NO_WORRY_BACK),
        &state,
        Config {
            node_budget: 12_000_000,
            max_depth: 100_000,
            table_entries: 1 << 23,
            ordering_salt: 0,
        },
    )
    .expect("a refutation has no line to verify");
    assert_eq!(report.verdict, Verdict::Unsolvable);
    assert_eq!(report.limit, None, "stopped on a limit rather than exhausting");
}
