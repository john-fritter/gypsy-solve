//! The composition argument, as a construction that can be run.
//!
//! The solver applies two dominances at once: a forcing rule (safe autoplay in
//! the restricted arm, the two-deck safe-foundation rule in the full arm) and
//! the split-run filter. Each is proved on its own. The argument that they are
//! sound *together* is in the doc comment on `Gypsy::legal_actions`, and it is
//! an induction on line length whose step is one of two rewrites:
//!
//! - **the forced-play deferral** — a winning line that does not open with the
//!   move the position forces is rewritten into one that does, no longer;
//! - **the cut-move deletion** — a winning line that opens with a move the
//!   split-run filter cuts is rewritten into one that does not, no longer.
//!
//! This binary runs that induction against a recorded winning line: walk the
//! line, and wherever the composed generator does not offer the next move,
//! apply whichever rewrite that position calls for and carry on. What comes
//! out is checked three ways — it replays from the deal to a win, every move
//! of it is offered by the composed generator at the position it is played
//! from, and it is no longer than the line that went in.
//!
//! Each rewrite is a simulation of the proof's own construction. Two lines are
//! stepped side by side, the recorded one and the copy being built, and after
//! every move the copy is checked against the invariant the proof claims for
//! it: for the deferral, that the copy is the recorded position with some
//! cards moved to foundations early; for the deletion, that it is the recorded
//! position with the piles on two slots exchanged. The copy's move is *chosen*
//! from a short list of candidates — the proof's cases — and only accepted if
//! the invariant survives it. That is what makes this a check of the argument
//! rather than a second argument: the invariant is stated once, and every step
//! has to satisfy it.
//!
//! Where the proof's own repair is needed the construction takes it: when the
//! copy can no longer mirror a move, but one exchanged pile is empty, the copy
//! plays the deleted move where it is owed and the two lines converge.

use std::collections::BTreeMap;
use std::fs;
use std::process::ExitCode;

use gypsy_core::card::Card;
use gypsy_core::state::{foundation_suit, COLUMNS, FOUNDATIONS};
use gypsy_core::{parse_move_list, Move, MoveOptions, State};
use gypsy_solver::{Game, Gypsy};

/// How many head-fixing rewrites one line is allowed before the run is called
/// stuck. The induction says each rewrite fixes one move and never lengthens
/// the line, so a line cannot need more rewrites than it has moves; the guard
/// is generous and exists only so a bug reports itself instead of spinning.
const REWRITE_GUARD: usize = 64;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let mut results: Option<String> = None;
    let mut moves_file: Option<String> = None;
    let mut seed: Option<u64> = None;
    let mut arms: Vec<MoveOptions> = Vec::new();
    let mut delay = false;
    let mut label = String::new();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--results" => results = args.next(),
            "--moves-file" => moves_file = args.next(),
            "--seed" => seed = args.next().and_then(|text| text.parse().ok()),
            "--delay-foundations" => delay = true,
            "--label" => label = args.next().unwrap_or_default(),
            "--arm" => match args.next().as_deref() {
                Some("full") => arms.push(MoveOptions::ALL),
                Some("no-worry-back") => arms.push(MoveOptions::NO_WORRY_BACK),
                Some("both") => arms.extend([MoveOptions::NO_WORRY_BACK, MoveOptions::ALL]),
                other => {
                    eprintln!("--arm wants full, no-worry-back or both, got {other:?}");
                    return ExitCode::FAILURE;
                }
            },
            other => {
                eprintln!("unknown argument {other:?}");
                eprintln!(
                    "usage: compose [--results PATH | --seed N --moves-file PATH] \
                     [--arm full|no-worry-back|both] [--delay-foundations] [--label TEXT]"
                );
                return ExitCode::FAILURE;
            }
        }
    }
    if arms.is_empty() {
        arms.push(MoveOptions::ALL);
    }

    let recorded = match (results, moves_file, seed) {
        (Some(path), None, None) => match recorded_lines(&path) {
            Ok(lines) => lines,
            Err(error) => {
                eprintln!("{path}: {error}");
                return ExitCode::FAILURE;
            }
        },
        (None, Some(path), Some(seed)) => match fs::read_to_string(&path) {
            Ok(text) => match parse_move_list(&text) {
                Ok(line) => vec![(seed, line)],
                Err(error) => {
                    eprintln!("{path}: {error}");
                    return ExitCode::FAILURE;
                }
            },
            Err(error) => {
                eprintln!("{path}: {error}");
                return ExitCode::FAILURE;
            }
        },
        _ => {
            eprintln!("give either --results PATH or --seed N --moves-file PATH");
            return ExitCode::FAILURE;
        }
    };

    if recorded.is_empty() {
        eprintln!("no recorded winning lines to rewrite");
        return ExitCode::FAILURE;
    }

    let recorded: Vec<(u64, Vec<Move>)> = if delay {
        recorded
            .into_iter()
            .map(|(seed, line)| {
                let delayed = delay_foundation_plays(seed, &line);
                (seed, delayed)
            })
            .collect()
    } else {
        recorded
    };

    let mut failures = 0;
    for options in arms {
        for (seed, line) in &recorded {
            let mut outcome = run(options, *seed, line);
            outcome.label = label.clone();
            outcome.delayed = delay;
            println!("{}", outcome.report());
            println!("{}", outcome.record());
            if !outcome.passed() {
                failures += 1;
            }
        }
    }

    if failures == 0 {
        ExitCode::SUCCESS
    } else {
        eprintln!("{failures} line(s) the construction could not close");
        ExitCode::FAILURE
    }
}

fn arm_name(options: MoveOptions) -> &'static str {
    if options.worry_back {
        "full"
    } else {
        "no-worry-back"
    }
}

/// What one line came to.
struct Outcome {
    arm: &'static str,
    seed: u64,
    recorded_length: usize,
    final_length: usize,
    deferrals: usize,
    deletions: usize,
    repairs: usize,
    skipped: usize,
    /// Slot relabellings, which are not rewrites: see `renamed_slot`.
    renames: usize,
    /// Where the line came from, and whether its foundation plays were pushed
    /// late before the construction ran.
    label: String,
    delayed: bool,
    /// Set when the construction could not close the line. Everything else is
    /// still reported, because where it stopped is the interesting part.
    failure: Option<String>,
}

impl Outcome {
    fn passed(&self) -> bool {
        self.failure.is_none()
    }

    fn report(&self) -> String {
        let head = format!(
            "seed {:<4} arm {:<13} recorded {:>6} moves  \
             deferrals {:>4}  deletions {:>4}  repairs {:>3}  relabels {:>4}  \
             dropped {:>5}  final {:>6}",
            self.seed,
            self.arm,
            self.recorded_length,
            self.deferrals,
            self.deletions,
            self.repairs,
            self.renames,
            self.skipped,
            self.final_length,
        );
        match &self.failure {
            None => format!("{head}  OK: wins, offered throughout, no longer"),
            Some(why) => format!("{head}  FAILED: {why}"),
        }
    }

    fn record(&self) -> String {
        let status = match &self.failure {
            None => "\"closed\":true".to_string(),
            Some(why) => format!("\"closed\":false,\"failure\":{}", quote(why)),
        };
        format!(
            "{{\"game\":\"gypsy\",\"seed\":{},\"arm\":\"{}\",\"source\":{},\
             \"foundations_delayed\":{},\"recorded_length\":{},\
             \"final_length\":{},\"deferrals\":{},\"deletions\":{},\"repairs\":{},\
             \"relabels\":{},\"moves_dropped\":{},{}}}",
            self.seed,
            self.arm,
            quote(&self.label),
            self.delayed,
            self.recorded_length,
            self.final_length,
            self.deferrals,
            self.deletions,
            self.repairs,
            self.renames,
            self.skipped,
            status,
        )
    }
}

/// Step-by-step output, for working out why a line did not close. Off unless
/// `COMPOSE_TRACE` is set, because a line runs to tens of thousands of steps.
fn trace(what: std::fmt::Arguments<'_>) {
    if std::env::var_os("COMPOSE_TRACE").is_some() {
        eprintln!("{what}");
    }
}

fn written(moves: &[Move]) -> String {
    if moves.is_empty() {
        return "-".to_string();
    }
    moves
        .iter()
        .map(|mv| mv.to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

fn quote(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// The induction, run: fix the head move, descend, repeat.
fn run(options: MoveOptions, seed: u64, recorded: &[Move]) -> Outcome {
    let game = Gypsy::new(options);
    let mut outcome = Outcome {
        arm: arm_name(options),
        seed,
        recorded_length: recorded.len(),
        final_length: recorded.len(),
        deferrals: 0,
        deletions: 0,
        repairs: 0,
        skipped: 0,
        renames: 0,
        label: String::new(),
        delayed: false,
        failure: None,
    };

    // The recorded line has to be a winning line of the game before any of
    // this means anything.
    if let Err(error) = replays_to_a_win(seed, recorded) {
        outcome.failure = Some(format!("the recorded line is not a win: {error}"));
        return outcome;
    }

    let mut line = recorded.to_vec();
    let mut state = State::deal(seed);
    let mut index = 0;
    let mut rewrites_here = 0;

    while index < line.len() {
        let offered = game.legal_actions(&state, 0);
        let head = line[index];
        if offered.contains(&head) {
            if let Err(error) = state.apply(head) {
                outcome.failure = Some(format!("move {index} ({head}) is illegal: {error}"));
                return outcome;
            }
            index += 1;
            rewrites_here = 0;
            continue;
        }

        // Not every move the rules allow is a move the generator emits, and
        // one case is not a dominance: the two slots of a suit that show the
        // same rank hold identical piles, so only the lower one is ever named
        // (`DECISIONS.md`, 2026-09-09). A line that names the other one is the
        // same line under a relabelling, and relabelling the whole rest of it
        // is sound exactly because the two piles are identical here.
        if let Some(renamed) = renamed_slot(&state, &offered, head) {
            rename_slot(&mut line[index..], renamed.0, renamed.1);
            outcome.renames += 1;
            continue;
        }

        rewrites_here += 1;
        if rewrites_here > REWRITE_GUARD {
            outcome.failure = Some(format!(
                "move {index} ({head}) still not offered after {REWRITE_GUARD} rewrites"
            ));
            return outcome;
        }

        let suffix = &line[index..];
        trace(format_args!(
            "rewrite at move {index}: head {head}, {} moves to go",
            suffix.len()
        ));
        let rewrite = match game.forced_action(&state) {
            // The position forces a move and the line makes a different one.
            Some(forced) => {
                outcome.deferrals += 1;
                defer_forced(&state, suffix, forced)
            }
            // Nothing is forced here, so the head is a move the split-run
            // filter cut.
            None => {
                outcome.deletions += 1;
                drop_cut(&state, suffix, options)
            }
        };

        let rewritten = match rewrite {
            Ok(rewritten) => rewritten,
            Err(why) => {
                outcome.failure = Some(format!("at move {index} ({head}): {why}"));
                return outcome;
            }
        };
        if rewritten.line.len() > suffix.len() {
            outcome.failure = Some(format!(
                "at move {index} ({head}): the rewrite lengthened the line, \
                 {} to {}",
                suffix.len(),
                rewritten.line.len()
            ));
            return outcome;
        }
        outcome.repairs += rewritten.repairs;
        outcome.skipped += suffix.len() - rewritten.line.len();
        line.truncate(index);
        line.extend(rewritten.line);
        outcome.final_length = line.len();
    }

    if !state.is_won() {
        outcome.failure = Some("the rewritten line runs out without winning".to_string());
        return outcome;
    }
    // Belt and braces: the line the loop consumed is replayed from the deal by
    // the engine, independently of everything above.
    if let Err(error) = replays_to_a_win(seed, &line) {
        outcome.failure = Some(format!("the rewritten line is not a win: {error}"));
        return outcome;
    }
    outcome.final_length = line.len();
    outcome
}

/// The pair of foundation slots to relabel, when the only thing wrong with a
/// move is which of a suit's two identical slots it names.
fn renamed_slot(state: &State, offered: &[Move], head: Move) -> Option<(u8, u8)> {
    let slot = match head {
        Move::ToFoundation { foundation, .. } | Move::WorryBack { foundation, .. } => foundation,
        _ => return None,
    };
    let twin = slot ^ 1;
    if state.foundations[slot as usize] != state.foundations[twin as usize] {
        return None;
    }
    let relabelled = match head {
        Move::ToFoundation { from, .. } => Move::ToFoundation {
            from,
            foundation: twin,
        },
        Move::WorryBack { to, .. } => Move::WorryBack {
            foundation: twin,
            to,
        },
        _ => return None,
    };
    offered.contains(&relabelled).then_some((slot, twin))
}

/// Swaps two foundation slot names throughout a line.
fn rename_slot(line: &mut [Move], one: u8, other: u8) {
    let swap = |slot: u8| {
        if slot == one {
            other
        } else if slot == other {
            one
        } else {
            slot
        }
    };
    for mv in line {
        *mv = match *mv {
            Move::ToFoundation { from, foundation } => Move::ToFoundation {
                from,
                foundation: swap(foundation),
            },
            Move::WorryBack { foundation, to } => Move::WorryBack {
                foundation: swap(foundation),
                to,
            },
            other => other,
        };
    }
}

/// One rewrite's output.
struct Rewritten {
    line: Vec<Move>,
    /// How often the copy had to play the deferred move to converge. The proof
    /// allows one per rewrite; more than that would mean the construction is
    /// not the construction the proof describes.
    repairs: usize,
}

/// Replays a line from the deal and reports whether it wins.
fn replays_to_a_win(seed: u64, line: &[Move]) -> Result<(), String> {
    let mut state = State::deal(seed);
    for (index, &mv) in line.iter().enumerate() {
        state
            .apply(mv)
            .map_err(|error| format!("move {index} ({mv}): {error}"))?;
    }
    if state.is_won() {
        Ok(())
    } else {
        Err(format!(
            "ends with {} of 104 cards up",
            state.foundation_count()
        ))
    }
}

// ---------------------------------------------------------------------------
// The forced-play deferral
// ---------------------------------------------------------------------------

/// Rewrites `line` into a winning line from `start` that opens with `forced`.
///
/// The copy plays the forced card up at move zero and then follows the
/// recorded line. The invariant it keeps is [`missing_shape`]: the copy is the
/// recorded position with some cards taken out of the tableau and put on
/// foundations early. The forced card is the first of them; a worry-back the
/// copy cannot mirror, because the card it wants is buried under a card the
/// copy played up, adds another.
fn defer_forced(start: &State, line: &[Move], forced: Move) -> Result<Rewritten, String> {
    let mut real = start.clone();
    let mut copy = start.clone();
    copy.apply(forced)
        .map_err(|error| format!("the forced move is illegal: {error}"))?;

    let mut out = vec![forced];
    for (step, &mv) in line.iter().enumerate() {
        let mut next_real = real.clone();
        next_real
            .apply(mv)
            .map_err(|error| format!("recorded move {step} ({mv}): {error}"))?;

        let mut taken = None;
        for candidate in deferral_candidates(&real, mv) {
            let mut trial = copy.clone();
            if candidate.iter().all(|&mv| trial.apply(mv).is_ok())
                && missing_shape(&next_real, &trial)
            {
                taken = Some((candidate, trial));
                break;
            }
        }
        let Some((candidate, next_copy)) = taken else {
            return Err(format!(
                "no mirror of recorded move {step} ({mv}) keeps the copy a \
                 deferral of the line{}",
                blocked_by(&real, mv)
            ));
        };
        out.extend(candidate);
        copy = next_copy;
        real = next_real;
    }

    if !copy.is_won() {
        return Err(format!(
            "the deferral ends with {} of 104 cards up",
            copy.foundation_count()
        ));
    }
    Ok(Rewritten {
        line: out,
        repairs: 0,
    })
}

/// The proof's cases for one move, as candidate mirrors, best first.
///
/// A foundation slot is never carried across: the copy's slots have drifted
/// from the recorded line's, so every slot index is re-offered and the
/// invariant picks the one that holds. That is the whole of "re-target the
/// slot names the two lines drift apart on".
fn deferral_candidates(real: &State, mv: Move) -> Vec<Vec<Move>> {
    match mv {
        Move::Stock => vec![vec![Move::Stock]],
        // The group may carry a card the copy has already played up — it is
        // the group's top card, since nothing can be built on it — in which
        // case the copy moves one card fewer, or nothing at all.
        Move::Tableau { from, to, count } => {
            let mut candidates = vec![vec![mv]];
            if count > 1 {
                candidates.push(vec![Move::Tableau {
                    from,
                    to,
                    count: count - 1,
                }]);
            } else {
                candidates.push(Vec::new());
            }
            candidates
        }
        // Either the copy plays the same card up — to whichever slot of the
        // suit is free in the copy — or this is the recorded line catching up
        // on a card the copy played up long ago, and the copy sits it out.
        Move::ToFoundation { from, foundation } => {
            let mut candidates = vec![vec![mv]];
            for slot in 0..FOUNDATIONS as u8 {
                if slot != foundation {
                    candidates.push(vec![Move::ToFoundation {
                        from,
                        foundation: slot,
                    }]);
                }
            }
            candidates.push(Vec::new());
            candidates
        }
        // A worry-back the copy cannot make is one whose card is buried under
        // a card the copy played up early. Dropping it is sound because that
        // card can host nothing — see `hosts_nothing`, which is checked.
        Move::WorryBack { foundation, to } => {
            let mut candidates = vec![vec![mv]];
            for slot in 0..FOUNDATIONS as u8 {
                if slot != foundation {
                    candidates.push(vec![Move::WorryBack {
                        foundation: slot,
                        to,
                    }]);
                }
            }
            if let Some(card) = real.foundation_top(foundation) {
                if hosts_nothing(real, card) {
                    candidates.push(Vec::new());
                }
            }
            candidates
        }
    }
}

/// True when no card outside the foundations can ever be built on this one.
///
/// The safe-foundation rule's conditions say this of the forced card, and the
/// same count one rank down says it of anything worried back onto it. The
/// construction does not take that on trust: a worry-back is only dropped when
/// this holds at the position that made it.
fn hosts_nothing(state: &State, card: Card) -> bool {
    let mut loose: Vec<Card> = state.stock.clone();
    for column in &state.columns {
        loose.extend_from_slice(column.cards());
    }
    !loose.iter().any(|&other| other.stacks_on(card))
}

/// Why a move could not be mirrored, when the reason is one the proof rules
/// out. Reported so a failure names the claim it falsifies.
fn blocked_by(real: &State, mv: Move) -> String {
    let landing = match mv {
        Move::Tableau { to, .. } | Move::WorryBack { to, .. } => Some(to),
        _ => None,
    };
    match landing.and_then(|to| real.columns[to as usize].top()) {
        Some(base) => format!(" (it builds on {base})"),
        None => String::new(),
    }
}

/// The deferral's invariant: `copy` is `real` with some cards lifted out of
/// the tableau and played to foundations instead.
///
/// Face-down counts are not compared. Taking a card out of a column can turn
/// one up early, which only ever gives the copy more moves than the recorded
/// line had, and the rewritten line is replayed by the engine from the deal
/// before it is believed.
fn missing_shape(real: &State, copy: &State) -> bool {
    if real.stock != copy.stock {
        return false;
    }
    let mut lifted: Vec<Card> = Vec::new();
    for column in 0..COLUMNS {
        let recorded = real.columns[column].cards();
        let mirrored = copy.columns[column].cards();
        let mut next = 0;
        for &card in recorded {
            if next < mirrored.len() && mirrored[next] == card {
                next += 1;
            } else {
                lifted.push(card);
            }
        }
        if next != mirrored.len() {
            return false;
        }
    }

    let mut wanted = foundation_cards(real);
    for card in lifted {
        *wanted.entry(card).or_insert(0) += 1;
    }
    wanted == foundation_cards(copy)
}

/// Every card sitting on a foundation, counted. A slot showing rank *v* holds
/// one copy of each rank up to *v* of its suit.
fn foundation_cards(state: &State) -> BTreeMap<Card, u8> {
    let mut cards = BTreeMap::new();
    for slot in 0..FOUNDATIONS {
        let suit = foundation_suit(slot as u8);
        for rank in 1..=state.foundations[slot] {
            *cards.entry(Card::new(suit, rank)).or_insert(0) += 1;
        }
    }
    cards
}

// ---------------------------------------------------------------------------
// The cut-move deletion
// ---------------------------------------------------------------------------

/// Where one of the two exchanged piles sits: the column, and how many cards
/// sit under the pile. The card at `base - 1` is the slot card; `base == 0` is
/// a pile standing in an empty column.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Slot {
    column: usize,
    base: usize,
}

/// How the copy stands against the recorded line.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Exchange {
    /// The two lines have converged; from here they are the same line.
    Same,
    /// The copy is the recorded position with the piles on these two slots
    /// exchanged.
    Swapped(Slot, Slot),
}

/// Rewrites `line`, which opens with a move the split-run filter cuts, into a
/// winning line from `start` that does not, and is no longer.
///
/// The copy skips the cut move and mirrors the rest, keeping
/// [`exchange_shape`]: the recorded position with the piles on two slots
/// exchanged. Where it can no longer mirror, it plays the skipped move where
/// the proof says it is owed — which is only possible with one pile empty, and
/// converges the two lines on the spot.
fn drop_cut(start: &State, line: &[Move], options: MoveOptions) -> Result<Rewritten, String> {
    let cut = line[0];
    let mut real = start.clone();
    real.apply(cut)
        .map_err(|error| format!("the cut move is illegal: {error}"))?;
    let mut copy = start.clone();

    let mut out: Vec<Move> = Vec::new();
    let mut repairs = 0;

    for (step, &mv) in line[1..].iter().enumerate() {
        let shape = exchange_shape(&real, &copy).ok_or_else(|| {
            format!("before recorded move {step} ({mv}) the copy is not an exchange of the line")
        })?;

        let mut next_real = real.clone();
        next_real
            .apply(mv)
            .map_err(|error| format!("recorded move {step} ({mv}): {error}"))?;

        // Convergence first: where the recorded line has caught up with a
        // move the copy already made, the copy sits the move out and the two
        // lines are the same from then on. Taking that as soon as it is there
        // is what makes the rewrite terminate — a line that shuffles a pile
        // back and forth can be mirrored forever otherwise, each rewrite
        // handing back the move it deleted.
        let mut taken = None;
        let candidates = deletion_candidates(&real, &copy, shape, mv, options);
        for converging in [true, false] {
            let mut best: Option<(Vec<Move>, State)> = None;
            for candidate in &candidates {
                // Playing the deleted move is the repair, and the repair is a
                // last resort: it is owed once, where the recorded line plays
                // a slot card up. Preferring it would just hand the move back
                // at the head and rewrite nothing.
                if converging && candidate.len() > 1 {
                    continue;
                }
                let mut trial = copy.clone();
                if !candidate.iter().all(|&mv| trial.apply(mv).is_ok()) {
                    continue;
                }
                let Some(shape) = exchange_shape(&next_real, &trial) else {
                    continue;
                };
                if converging && shape != Exchange::Same {
                    continue;
                }
                if best
                    .as_ref()
                    .is_none_or(|(taken, _)| candidate.len() < taken.len())
                {
                    best = Some((candidate.clone(), trial));
                }
            }
            if let Some(best) = best {
                taken = Some(best);
                break;
            }
        }
        let Some((candidate, next_copy)) = taken else {
            return Err(format!(
                "no mirror of recorded move {step} ({mv}) keeps the copy an \
                 exchange of the line (shape {shape:?})"
            ));
        };
        trace(format_args!(
            "  step {step:>5}: line {mv:<10} copy {:<22} {shape:?}",
            written(&candidate)
        ));
        if candidate.len() > 1 {
            repairs += 1;
            trace(format_args!(
                "  the copy plays the deleted move {cut} where it is owed, at step {step}"
            ));
        }
        out.extend(candidate);
        copy = next_copy;
        real = next_real;
    }

    if !copy.is_won() {
        return Err(format!(
            "the deletion ends with {} of 104 cards up",
            copy.foundation_count()
        ));
    }
    if repairs > 1 {
        return Err(format!(
            "the copy played the deleted move {repairs} times; the proof owes it once"
        ));
    }
    Ok(Rewritten { line: out, repairs })
}

/// The proof's cases for one move, as candidate mirrors, best first.
///
/// The recorded line and the copy hold the same cards in the same places bar
/// two piles, so a mirror is the same move with a source or a destination
/// redirected to the twin column, and — when the move lifts a slot card with
/// its pile — a different count, because the pile it carries is the other one.
fn deletion_candidates(
    real: &State,
    copy: &State,
    shape: Exchange,
    mv: Move,
    options: MoveOptions,
) -> Vec<Vec<Move>> {
    let Exchange::Swapped(first, second) = shape else {
        return vec![vec![mv]];
    };

    let twin = |column: u8| -> u8 {
        let column = column as usize;
        if column == first.column {
            second.column as u8
        } else if column == second.column {
            first.column as u8
        } else {
            column as u8
        }
    };

    let mut candidates: Vec<Vec<Move>> = Vec::new();
    match mv {
        // Gate 1 says this cannot happen: the rule only fires with the stock
        // empty, and nothing ever puts a card back. Offered anyway, so that a
        // deal would be reported as a broken mirror rather than assumed away.
        Move::Stock => candidates.push(vec![Move::Stock]),
        Move::Tableau { from, to, count } => {
            for source in [from, twin(from)] {
                for destination in [to, twin(to)] {
                    if source == destination {
                        continue;
                    }
                    for count in counts(real, copy, first, second, from, count) {
                        candidates.push(vec![Move::Tableau {
                            from: source,
                            to: destination,
                            count,
                        }]);
                    }
                }
            }
        }
        Move::ToFoundation { from, foundation } => {
            for source in [from, twin(from)] {
                candidates.push(vec![Move::ToFoundation {
                    from: source,
                    foundation,
                }]);
            }
        }
        Move::WorryBack { foundation, to } => {
            for destination in [to, twin(to)] {
                candidates.push(vec![Move::WorryBack {
                    foundation,
                    to: destination,
                }]);
            }
        }
    }

    // The repair. With one pile empty the copy can put the pile it is holding
    // where the recorded line has it, which is the deleted move played where
    // it is owed, and the two lines are the same from then on.
    for (holder, home) in [(first, second), (second, first)] {
        let carried = copy.columns[holder.column]
            .len()
            .saturating_sub(holder.base);
        if carried == 0 || copy.columns[home.column].len() != home.base {
            continue;
        }
        let repair = Move::Tableau {
            from: holder.column as u8,
            to: home.column as u8,
            count: carried as u8,
        };
        let mut with_repair = vec![repair];
        with_repair.push(mv);
        candidates.push(with_repair);
    }

    // Nothing at all: the recorded line has caught up with a move the copy
    // made when it skipped the cut.
    candidates.push(Vec::new());

    // Never offer the copy a move the rules do not allow this arm. Worry-back
    // is the only such move, and the restricted arm must not gain one.
    if !options.worry_back {
        candidates.retain(|candidate| {
            !candidate
                .iter()
                .any(|mv| matches!(mv, Move::WorryBack { .. }))
        });
    }
    candidates
}

/// Counts for a mirrored group move.
///
/// A group taken from inside a pile is the same cards in the copy, so the same
/// count. A group that reaches below the slot card carries the whole pile with
/// it, and the copy's pile is the other one, so its count differs by the two
/// piles' lengths.
fn counts(real: &State, copy: &State, first: Slot, second: Slot, from: u8, count: u8) -> Vec<u8> {
    let mut counts = vec![count];
    let from = from as usize;
    let here = if from == first.column {
        first
    } else if from == second.column {
        second
    } else {
        return counts;
    };

    // Does the group reach below the slot card? Then it takes the whole pile,
    // and the copy's move takes the whole of the pile it is holding instead.
    let start = real.columns[from].len().saturating_sub(count as usize);
    if start < here.base {
        let recorded = real.columns[here.column].len() - here.base;
        let mirrored = copy.columns[here.column].len().saturating_sub(here.base);
        let adjusted = (count as usize + mirrored).saturating_sub(recorded);
        if adjusted > 0 && adjusted <= u8::MAX as usize && adjusted != count as usize {
            counts.push(adjusted as u8);
        }
    }
    counts
}

/// The deletion's invariant: `copy` is `real` with the piles on two slots
/// exchanged, or the same position.
///
/// The decomposition is recovered rather than tracked. Two columns may differ,
/// and the exchanged parts fix each other's length, so there is one free
/// choice — how deep the slots sit — and the deepest pair that works is taken.
/// Face-down counts are not compared, for the reason [`missing_shape`] gives.
fn exchange_shape(real: &State, copy: &State) -> Option<Exchange> {
    if real.stock != copy.stock || real.foundations != copy.foundations {
        return None;
    }
    let differing: Vec<usize> = (0..COLUMNS)
        .filter(|&column| real.columns[column].cards() != copy.columns[column].cards())
        .collect();
    match differing.as_slice() {
        [] => Some(Exchange::Same),
        [one, other] => {
            let (one, other) = (*one, *other);
            let real_one = real.columns[one].cards();
            let real_other = real.columns[other].cards();
            let copy_one = copy.columns[one].cards();
            let copy_other = copy.columns[other].cards();

            // real_one[base_one..] == copy_other[base_other..] and the other
            // way round, so the two bases differ by a fixed amount.
            let drift = real_one.len() as isize - copy_other.len() as isize;
            let limit = common_prefix(real_one, copy_one);
            for base_one in (0..=limit).rev() {
                let base_other = base_one as isize - drift;
                if base_other < 0 {
                    continue;
                }
                let base_other = base_other as usize;
                if base_other > common_prefix(real_other, copy_other) {
                    continue;
                }
                if base_one > real_one.len()
                    || base_one > copy_one.len()
                    || base_other > real_other.len()
                    || base_other > copy_other.len()
                {
                    continue;
                }
                if real_one[base_one..] == copy_other[base_other..]
                    && real_other[base_other..] == copy_one[base_one..]
                {
                    return Some(Exchange::Swapped(
                        Slot {
                            column: one,
                            base: base_one,
                        },
                        Slot {
                            column: other,
                            base: base_other,
                        },
                    ));
                }
            }
            None
        }
        _ => None,
    }
}

fn common_prefix(left: &[Card], right: &[Card]) -> usize {
    left.iter().zip(right).take_while(|(a, b)| a == b).count()
}

// ---------------------------------------------------------------------------
// Making a line harder to rewrite
// ---------------------------------------------------------------------------

/// Pushes every foundation play as late in the line as it will commute.
///
/// A recorded line comes from a search that already forced its safe cards up,
/// so it violates the forcing rule barely at all and exercises the deferral
/// barely at all. Two moves that commute — applying them in either order
/// reaches the same position — can be swapped without touching the rest of the
/// line, so a foundation play can be walked to the right until it meets a move
/// it does not commute with. What comes out is the same win, still rules-legal,
/// playing its safe cards up as late as it can. That is the line the deferral
/// is for.
fn delay_foundation_plays(seed: u64, line: &[Move]) -> Vec<Move> {
    let mut line = line.to_vec();
    let mut state = State::deal(seed);
    let mut index = 0;
    while index + 1 < line.len() {
        let (here, next) = (line[index], line[index + 1]);
        if matches!(here, Move::ToFoundation { .. }) && commutes(&state, here, next) {
            line.swap(index, index + 1);
            if state.apply(next).is_err() {
                return line;
            }
        } else {
            if state.apply(here).is_err() {
                return line;
            }
        }
        index += 1;
    }
    line
}

/// True when two moves can be applied in either order, to the same effect.
fn commutes(state: &State, first: Move, second: Move) -> bool {
    let forwards = applied(state, &[first, second]);
    let backwards = applied(state, &[second, first]);
    match (forwards, backwards) {
        (Some(forwards), Some(backwards)) => forwards == backwards,
        _ => false,
    }
}

fn applied(state: &State, moves: &[Move]) -> Option<State> {
    let mut state = state.clone();
    for &mv in moves {
        state.apply(mv).ok()?;
    }
    Some(state)
}

// ---------------------------------------------------------------------------
// Recorded lines
// ---------------------------------------------------------------------------

/// Reads `(seed, line)` for every solved record in a results file that carries
/// its winning line.
fn recorded_lines(path: &str) -> Result<Vec<(u64, Vec<Move>)>, String> {
    let text = fs::read_to_string(path).map_err(|error| error.to_string())?;
    let mut lines = Vec::new();
    for record in text.lines().filter(|record| !record.trim().is_empty()) {
        let Some(line) = string_field(record, "line") else {
            continue;
        };
        let seed = number_field(record, "seed").ok_or("a record carries a line but no seed")?;
        lines.push((seed, parse_move_list(&line).map_err(|e| e.to_string())?));
    }
    Ok(lines)
}

fn string_field(record: &str, name: &str) -> Option<String> {
    let key = format!("\"{name}\":\"");
    let start = record.find(&key)? + key.len();
    let rest = &record[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn number_field(record: &str, name: &str) -> Option<u64> {
    let key = format!("\"{name}\":");
    let start = record.find(&key)? + key.len();
    let rest = &record[start..];
    let end = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    rest[..end].parse().ok()
}
