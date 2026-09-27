//! Position-level soundness audit for a forcing rule.
//!
//! A forcing rule collapses a position to one move. It is sound exactly when
//! every *winnable* position it fires in has a *winnable* forced child. This
//! binary checks that statement directly, one position at a time, on capped
//! deals (`--top-rank` below 13) small enough to search to exhaustion.
//!
//! **What makes it a test with teeth, and why the earlier checks were not:**
//!
//! - **Positions come from random walks**, not from the search. The solver's
//!   own first descent stays on winning ground, so positions sampled from it
//!   are nearly all winnable with winnable children, and a rule that loses a
//!   game in a tight spot is never put in one. Walks under the solver's move
//!   set wander into tight and lost positions. A first version sampled the
//!   search and found nothing wrong with a rule that is false.
//! - **Both searches use no dominance at all.** The position and its forced
//!   child are solved in the rules-legal game with every pruning rule
//!   removed, so a counterexample does not depend on any other rule being
//!   sound, and an audit of a shipped rule does not assume the rule under
//!   audit. Moves are ordered (foundation plays first, worry-backs last) and
//!   ordering discards nothing; without it a worry-back search with no
//!   forcing rule cannot decide even rank-4 positions on a budget.
//! - **A deal-level check only sees a rule that flips a whole deal.** A
//!   forcing rule that loses in a position the winning line never visits
//!   changes no verdict, and a deal set that proves nothing unsolvable cannot
//!   show one that does. This checks the rule wherever it fires.
//!
//! Outcomes per sampled position: the position is lost (no counterexample is
//! possible there), both are winnable (the rule survives), the child is lost
//! (**a counterexample**), or a search hit its budget (undecided). Undecided
//! children are where a counterexample could still hide, so the audit is
//! evidence, never a proof.
//!
//! Rules:
//!
//! - `shipped` — whatever `Gypsy::forced_action` forces: safe autoplay in the
//!   restricted arm, the two-deck safe-foundation rule in the full arm.
//! - `lagging-pile` — the rule rejected on 2026-09-26, kept as a control: a
//!   card of rank *r* is forced when each opposite-colour suit's *leading*
//!   pile has reached *r−1*, ignoring the second copies. It fires only where
//!   the shipped rule does not. It is false, so this must find
//!   counterexamples; if it does not, the audit has lost its teeth.
//!
//! A new forcing rule is added here as another `Rule` and audited before it
//! goes into `legal_actions`.
//!
//! The run is deterministic: walks are seeded from the deal number. It exits
//! non-zero when any counterexample is found.

use std::collections::HashSet;
use std::process::ExitCode;

use gypsy_core::rng::SplitMix64;
use gypsy_core::state::MIN_TOP_RANK;
use gypsy_core::{Card, Move, MoveOptions, State, Suit};
use gypsy_solver::{solve, Config, Game, Gypsy, Table, Verdict};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Rule {
    Shipped,
    LaggingPile,
}

/// The game with every dominance removed: all rules-legal moves, ordered
/// foundation plays first and worry-backs last. The ordering discards
/// nothing, so a verdict here trusts no pruning rule.
struct Raw<'a> {
    gypsy: &'a Gypsy,
    options: MoveOptions,
}

impl Game for Raw<'_> {
    type Position = State;
    type Action = Move;

    fn legal_actions(&self, position: &State, _salt: u64) -> Vec<Move> {
        let mut moves = position.legal_moves(self.options);
        moves.sort_by_key(|mv| match mv {
            Move::ToFoundation { .. } => 0,
            Move::Tableau { .. } => 1,
            Move::Stock => 2,
            Move::WorryBack { .. } => 3,
        });
        moves
    }

    fn apply(&self, position: &mut State, action: Move) -> Result<(), String> {
        self.gypsy.apply(position, action)
    }

    fn is_won(&self, position: &State) -> bool {
        self.gypsy.is_won(position)
    }

    /// The transposition key is not a dominance: two positions share it only
    /// when they have the same future. Using the solver's keeps the
    /// stock-gated canonicalisation it was proved with.
    fn key(&self, position: &State) -> u128 {
        self.gypsy.key(position)
    }
}

/// The higher of the two foundation piles of a suit.
fn leading_pile(state: &State, suit: Suit) -> u8 {
    let slot = suit.index() as usize * 2;
    state.foundations[slot].max(state.foundations[slot + 1])
}

/// The rejected lagging-pile test for one card.
fn lagging_pile_safe(state: &State, card: Card) -> bool {
    Suit::ALL
        .iter()
        .filter(|suit| suit.is_red() != card.is_red())
        .all(|&suit| leading_pile(state, suit) + 1 >= card.rank())
}

/// The move `rule` forces in this position, if any.
fn fires(rule: Rule, gypsy: &Gypsy, state: &State) -> Option<Move> {
    let shipped = gypsy.forced_action(state);
    match rule {
        Rule::Shipped => shipped,
        Rule::LaggingPile if shipped.is_some() => None,
        Rule::LaggingPile => state.columns.iter().enumerate().find_map(|(from, column)| {
            let card = column.top()?;
            let foundation = state.foundation_target(card)?;
            lagging_pile_safe(state, card).then_some(Move::ToFoundation {
                from: from as u8,
                foundation,
            })
        }),
    }
}

#[derive(Default)]
struct Tally {
    sampled: u64,
    parent_lost: u64,
    survived: u64,
    counterexamples: u64,
    undecided: u64,
}

struct Settings {
    rule: Rule,
    options: MoveOptions,
    top_rank: u8,
    first: u64,
    deals: u64,
    per_deal: usize,
    walks: usize,
    walk_length: usize,
    config: Config,
}

fn main() -> ExitCode {
    let settings = match parse(std::env::args().skip(1)) {
        Ok(settings) => settings,
        Err(message) => {
            eprintln!("{message}");
            eprintln!(
                "usage: forcing_audit [--rule shipped|lagging-pile] \
                 [--arm no-worry-back|full] [--top-rank 4..12] [--seed N] [--deals N] \
                 [--per-deal N] [--walks N] [--walk-length N] [--budget N] [--table-mib N]"
            );
            return ExitCode::FAILURE;
        }
    };

    let gypsy = Gypsy::new(settings.options);
    let raw = Raw {
        gypsy: &gypsy,
        options: settings.options,
    };
    let mut total = Tally::default();

    for seed in settings.first..settings.first + settings.deals {
        let start = State::deal_capped(seed, settings.top_rank);
        let mut deal = Tally::default();
        for (position, forced) in sample(&settings, &gypsy, &start, seed) {
            deal.sampled += 1;
            match solve(&raw, &position, settings.config).map(|r| r.verdict) {
                Ok(Verdict::Solvable) => {}
                Ok(Verdict::Unsolvable) => {
                    deal.parent_lost += 1;
                    continue;
                }
                Ok(Verdict::Unknown) => {
                    deal.undecided += 1;
                    continue;
                }
                Err(bug) => panic!("seed {seed}: {bug}"),
            }
            let mut child = position.clone();
            child.apply(forced).expect("a forced move is legal");
            match solve(&raw, &child, settings.config).map(|r| r.verdict) {
                Ok(Verdict::Solvable) => deal.survived += 1,
                Ok(Verdict::Unknown) => deal.undecided += 1,
                Ok(Verdict::Unsolvable) => {
                    deal.counterexamples += 1;
                    println!(
                        "COUNTEREXAMPLE seed {seed} forces {forced}: winnable before, lost after"
                    );
                    println!("{position}");
                }
                Err(bug) => panic!("seed {seed}: {bug}"),
            }
        }
        println!(
            "seed {seed}: sampled {} lost {} survived {} counterexamples {} undecided {}",
            deal.sampled, deal.parent_lost, deal.survived, deal.counterexamples, deal.undecided
        );
        total.sampled += deal.sampled;
        total.parent_lost += deal.parent_lost;
        total.survived += deal.survived;
        total.counterexamples += deal.counterexamples;
        total.undecided += deal.undecided;
    }

    println!(
        "total: deals {} sampled {} lost {} survived {} counterexamples {} undecided {}",
        settings.deals,
        total.sampled,
        total.parent_lost,
        total.survived,
        total.counterexamples,
        total.undecided
    );
    if total.counterexamples == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Distinct positions where the rule fires, gathered by random walks under
/// the solver's own move set, each paired with the move it forces.
fn sample(settings: &Settings, gypsy: &Gypsy, start: &State, seed: u64) -> Vec<(State, Move)> {
    // Salted so the walks are not correlated with the deal's own shuffle.
    let mut rng = SplitMix64::new(seed ^ 0x05A1_70FA_0D17);
    let mut seen = HashSet::new();
    let mut found = Vec::new();
    for _ in 0..settings.walks {
        let mut position = start.clone();
        for _ in 0..settings.walk_length {
            if let Some(forced) = fires(settings.rule, gypsy, &position) {
                if seen.insert(gypsy.key(&position)) {
                    found.push((position.clone(), forced));
                    if found.len() >= settings.per_deal {
                        return found;
                    }
                }
            }
            if gypsy.is_won(&position) {
                break;
            }
            let moves = gypsy.legal_actions(&position, 0);
            if moves.is_empty() {
                break;
            }
            let mv = moves[rng.below(moves.len() as u64) as usize];
            position.apply(mv).expect("generated moves are legal");
        }
    }
    found
}

fn parse(mut args: impl Iterator<Item = String>) -> Result<Settings, String> {
    let mut settings = Settings {
        rule: Rule::Shipped,
        options: MoveOptions::NO_WORRY_BACK,
        top_rank: 4,
        first: 0,
        deals: 100,
        per_deal: 20,
        walks: 200,
        walk_length: 400,
        config: Config {
            node_budget: 2_000_000,
            table_entries: Table::entries_in(64 << 20),
            ..Config::default()
        },
    };
    let number = |value: Option<String>, flag: &str| -> Result<u64, String> {
        value
            .and_then(|text| text.parse().ok())
            .ok_or_else(|| format!("{flag} wants a number"))
    };
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--rule" => {
                settings.rule = match args.next().as_deref() {
                    Some("shipped") => Rule::Shipped,
                    Some("lagging-pile") => Rule::LaggingPile,
                    other => {
                        return Err(format!(
                            "--rule wants shipped or lagging-pile, got {other:?}"
                        ))
                    }
                }
            }
            "--arm" => {
                settings.options = match args.next().as_deref() {
                    Some("no-worry-back") => MoveOptions::NO_WORRY_BACK,
                    Some("full") => MoveOptions::ALL,
                    other => {
                        return Err(format!("--arm wants no-worry-back or full, got {other:?}"))
                    }
                }
            }
            "--top-rank" => {
                let rank = number(args.next(), "--top-rank")?;
                // Thirteen ranks cannot be exhausted, so every check would
                // come back undecided.
                if !(u64::from(MIN_TOP_RANK)..=12).contains(&rank) {
                    return Err(format!("--top-rank wants {MIN_TOP_RANK} to 12, got {rank}"));
                }
                settings.top_rank = rank as u8;
            }
            "--seed" => settings.first = number(args.next(), "--seed")?,
            "--deals" => settings.deals = number(args.next(), "--deals")?,
            "--per-deal" => settings.per_deal = number(args.next(), "--per-deal")? as usize,
            "--walks" => settings.walks = number(args.next(), "--walks")? as usize,
            "--walk-length" => {
                settings.walk_length = number(args.next(), "--walk-length")? as usize
            }
            "--budget" => settings.config.node_budget = number(args.next(), "--budget")?,
            "--table-mib" => {
                let mib = number(args.next(), "--table-mib")? as usize;
                settings.config.table_entries = Table::entries_in(mib << 20);
            }
            other => return Err(format!("unknown argument {other:?}")),
        }
    }
    Ok(settings)
}
