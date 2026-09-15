# Gypsy Solitaire Solvability

Compute how often two-deck Gypsy solitaire is winnable, and quantify how much
of that winnability depends on returning cards from the foundation to the
tableau ("worry-back").

Secondary goal: a public writeup and an interactive page where a visitor can
play a seeded deal and then find out whether it was ever winnable.

This document is a starting point, not a fixed plan. `DECISIONS.md` records what
the implementation actually settled on, what was rejected, and where reality has
contradicted what is written here. When the two disagree, `DECISIONS.md` is
newer and this file gets corrected.

## Why this is worth doing

The reference work on solitaire winnability is Solvitaire (Blake & Gent), which
covers 73 variants of 35 games and pins thoughtful Klondike at 81.945% ± 0.084%.
It is explicitly restricted to **single-deck** games. Gypsy is two-deck, and no
published winnability figure for it turned up in a search. That may just mean it
wasn't found — verify before claiming novelty — but a solid number with a
confidence interval is plausibly a new result rather than a reimplementation.

## Exact ruleset

Taken from the mobile implementation being played. Other Gypsy implementations
differ; **any published number is meaningless without this ruleset attached.**

- Two standard 52-card decks (104 cards).
- 8 tableau stacks, 8 foundation stacks, 1 stock.
- Initial deal: one row face down, two rows face up on top of it.
  (8 cards buried, 16 face up, 80 in stock.)
- Tableau builds **descending, alternating color**.
- **Any correct sequence may be moved as a unit** — alternating color, not
  same-suit-only. This is the permissive variant; classic Gypsy usually
  restricts group moves to same-suit runs. It raises winnability and widens
  the branching factor.
- Any card may be placed on an empty tableau stack.
- Foundations build ascending by suit; only an Ace starts an empty foundation.
- Stock: one press deals one card to **each** of the 8 tableau stacks.
  No redeal. No look-ahead.
- **Worry-back is legal**: cards may be moved from foundation back to tableau.

## Two questions

1. **Thoughtful winnability.** All card locations known from the start
   (including the 8 face-down cards and the full stock order). Cards still
   can't move until legally exposed. This is the standard measure in the
   literature and the upper bound on real play.
2. **The worry-back delta.** Solve every deal twice — worry-back disabled, then
   enabled — and report the difference in percentage points. This is the
   headline finding and it doubles as a staged build: the no-worry-back solver
   is much easier and ships first.

Possible phase two: real-play winnability with hidden information
(determinization / MCTS). Note that the stock deals 8 at a time, blind and
irreversible, with no waste-cycling to scout ahead — so the gap between
thoughtful and real-play winnability here should be driven almost entirely by
stock luck. Different character from Klondike.

## Architecture

**One implementation of the rules.** Rust core, compiled to WASM for the
browser. Do not write the rules a second time in JS for the front end — the two
will diverge silently and the published numbers will be wrong.

**Language: Rust for engine and solver, Python for analysis.** Decided, not
provisional. Rust because WASM is a first-class target (the batch runner and the
website run the same compiled core), because memory safety here protects the
*result* — a silent transposition-table bug produces wrong winnability numbers,
which is the worst failure mode this project has — and because a solver is the
easy case for Rust: flat arrays of copyable values, bit manipulation, one large
fixed-size table, almost no ownership complexity. `rayon` handles the parallel
batch. Analysis stays in Python because pandas and scikit-learn are better tools
for the statistical questions; the boundary is the results files on disk.

```
core/      rules, state, move generation, seeded deals  (Rust)
solver/    search, transposition table, dominances       (Rust)
cli/       batch runner, single-deal solve, replay dump
analysis/  notebooks, plots, stats                       (Python)
web/       WASM bindings + front end                     (last)
```

Seeded deals are load-bearing: every result must be reproducible from a deal
number, and every published claim should be replayable.

## Solver notes

Depth-first backtracking with a transposition table is the approach that works
for this class of problem. Specifics for this game:

- **Canonicalization, gated on the stock.** Two reductions this document
  originally claimed unconditionally are only sound once the stock is empty,
  because the stock deal sends card *i* to column *i*:

  | Reduction | Sound when |
  |---|---|
  | Remaining stock encoded as its **length** alone | always |
  | The two foundation slots of a suit sorted | always |
  | Tableau columns sorted | **stock empty only** |
  | Red-red and black-black suit swap | **stock empty only** |

  Column order is load-bearing while cards remain undealt: two positions
  identical up to a permutation of columns receive *different* cards on the
  next stock deal, so merging them in the table is wrong. The suit swap fails
  for the same reason — it does not apply to the 80 undealt cards, so the
  swapped position belongs to a different deal. Both hold again once the stock
  runs out, which is where most of the search sits, so both are worth having
  behind the gate. See `DECISIONS.md`, 2026-09-11.

  Duplicate cards *are* interchangeable everywhere, since the two decks are
  indistinguishable.
- **Zobrist hashing**, fixed-size table with replacement. Memory is the binding
  constraint, not CPU — and on the box that runs the batch it is tighter than
  this originally assumed: about 1 GiB per worker, not one large table with the
  host to itself. See *Where the batch runs* below.
- **Dominances** cut more than any micro-optimization. But see the warning
  below.
- **Node budget**, and report three outcomes: solvable, proven unsolvable,
  **unknown**. The honest third bucket is more credible than a forced binary,
  and how it shrinks as the budget rises is itself a result worth plotting.
- Embarrassingly parallel across deals. One process per deal, one core each,
  with bounded concurrency — four workers on fritter.lol, not eight.

### Worry-back breaks things — handle deliberately

1. It **invalidates the standard safe-autoplay dominance** ("this card can never
   be needed below, send it up"). That rule is what makes solitaire solvers
   tractable. It must be rederived for the worry-back case, and a conservative
   version prunes far less. Solvitaire's authors found the same incorrect
   worry-back dominance in both their own earlier solver and in an existing
   published Klondike solver. Two decks and eight foundations make it fiddlier.
   Prove the rule before trusting it.
2. It makes the state graph **cyclic** rather than a DAG. Handled by the
   transposition table, but branching factor goes up.

If full worry-back search proves too expensive, a bounded version (cap the
count, or only permit a worry-back that immediately unlocks a move) yields a
valid **lower bound**. Never report a bounded result as exact.

## Where the batch runs

Measured on fritter.lol, 2026-09-10, and these numbers are design inputs rather
than trivia:

- 4 physical cores / 8 threads, shared with a live production stack that has to
  stay up. Four workers is a ceiling to start from, revisited after measuring.
- 15 GiB RAM, but only ~4.9 GiB available and swap nearly exhausted. Size the
  transposition table from available memory and measure per-solve RSS before
  running multiple workers.
- Root filesystem 96% full, RAID0, no redundancy. Results written there are not
  archival.

Consequences for the CLI: bounded concurrency, per-deal records written
incrementally, resume after a kill without redoing completed deals, a
configurable output path, and a refusal to start or continue when free space
runs low. All of that is `gypsy batch` as of 2026-09-15. Storage and memory
provisioning is a prerequisite for a thousands-of-deals run, not something to
resolve mid-run.

**A worker costs more than its table, corrected 2026-09-15.** Peak resident
memory used to be about the table size only because the search was thrashing
rather than descending. With the probing table a 12M-node Klondike solve ran
413 MiB *over* its table, bounded by `--max-depth` at roughly four KiB a frame,
so around 390 MiB at the default. Four workers at 1 GiB of table each therefore
want about 5.5 GiB, not 4, against the ~4.9 GiB this box has available — so
four workers at that table size does not fit and never did. `gypsy batch`
refuses such a run up front rather than being killed hours in, which is what
happened here on 2026-09-15.

## Validation

Implement **Klondike** in the same engine and check it reproduces ~81.9%
thoughtful winnability. Cheap, and it proves the pipeline before any Gypsy
number is trusted. Do this before the first batch run, not after.

Also: hand-verify a handful of claimed solutions by replaying them through the
engine, and confirm every "unsolvable" verdict is exhaustive rather than
budget-exhausted.

## Build order

1. **Done.** Rust engine + CLI. Seeded deal, apply move, detect win, dump state.
2. **Done.** Debug visualizer. Not a game — a way to render a state and step
   through a move list. `gypsy replay --step` covers it; the solver feeds it a
   line via `--trace`. Watching the search make obviously stupid moves is how
   the bad dominance gets found.
3. **Done.** Baseline solver: DFS, transposition table, three-valued
   result, no dominances. Deliberately weak: it is the control that every later
   cut gets measured against, and it is expected to return mostly `unknown`.
4. **Done, and standing.** Klondike validation, before any dominance work
   rather than after. It is
   the only independent check on whether the search is correct, so a slow
   solver and a broken one stay distinguishable. Must reproduce ~81.9%.
   Implemented in `klondike/`, against the same `Game` trait the Gypsy side
   uses, so the search under test is the search that publishes. The variant is
   the one the published figure was measured on: 24-card stock drawn three at a
   time, redeals without limit, worry-back permitted.
5. **Fix the search shape**, before any dominance. Done 2026-09-13: the depth
   limit was the cause, not a parameter. The table indexed entries by the depth
   a search had in hand and answered a probe only for a visit with no more to
   spend, so positions were re-expanded 20 to 57 times. Depth has no place in
   the argument — expanding a position generates all its children, so skipping
   anything already expanded cannot hide a win — and the table is now a plain
   set of expanded positions.
6. **Make the table keep what it records.** Done 2026-09-15. The table was
   direct-mapped and replaced unconditionally, on the reasoning that a lost
   entry costs one re-expansion. Two keys sharing a slot evict each other
   indefinitely when both sit on a hot path: Gypsy deal 2 spent 91% of a 3M
   budget re-expanding, Klondike deal 3 wasted 32%. Probing eight slots
   forward fixed it. Worth about one fourfold budget step — Klondike's unknown
   bucket went 20 to 17, 17 to 14 and 13 to 11 at 3M, 12M and 48M — and it
   changed the memory profile, because the search now descends instead of
   thrashing. See `DECISIONS.md`.
7. **Dominances.** One per PR, each with a proof
   that it cannot discard a winning line. Candidates, cheapest and most clearly
   provable first:

   1. ~~*Interchangeable empty destinations.*~~ **Dead, 2026-09-14.** Both this
      and the next rest on the same relabelling argument, and it fails the
      same way: while the stock holds cards the columns are not
      interchangeable, because the stock deals card *i* to column *i*. Gated
      on an empty stock the argument is sound — and then the transposition
      key already folds column order, so the child has the parent's key and
      the table skips it anyway. Unsound before the gate, redundant after it.
      Measured: zero change in nodes expanded over 50 Klondike deals. See
      `DECISIONS.md`, 2026-09-14.
   2. ~~*Whole-column-onto-empty.*~~ **Dead, 2026-09-14.** Same argument, same
      two failures. Pinned by tests in `core/src/state.rs`.
   3. *Safe autoplay.* **Done for the no-worry-back game, 2026-09-15**, and
      it is the first cut that has paid: no verdict contradicted, three more
      deals resolved out of fifty, and 41% fewer nodes on the deals decided
      both with and without it. The rule needed correcting for two decks —
      all *four* opposite-colour foundation piles, not two.

      **Still open for the worry-back game, which is the headline figure.**
      The proof needs the opposite-colour cards to be on foundations and
      unable to leave, and worry-back is exactly the rule that lets them
      leave. The repair that suggests itself — play it up, worry it back if
      it is ever wanted — is circular under a transposition table and is
      written up in `DECISIONS.md` so nobody re-derives it. So the full game
      still has no dominance at all, and either one is found for it or the
      search needs something that is not a dominance.

   Every one of these is measured on the same Klondike deal set, and Klondike
   is a regression test with teeth: a dominance may change node counts and
   how many deals resolve, and must **not** change any verdict that was already
   decided, and must leave 81.945% inside the validation bracket. A dominance
   that flips a decided verdict is a wrong dominance, and that is exactly the
   failure this project is least able to detect any other way.

   **That harness has a hole, found 2026-09-15.** Klondike here is the
   published *worry-back* variant, so a dominance gated on worry-back being
   off — which safe autoplay is, and any correct version of the dead two would
   have been — never fires in it and is never checked by it. Safe autoplay was
   checked instead on Gypsy no-worry-back deals, where nine of fifty resolve
   against Klondike's thirty: far less signal, on the dominance this project
   is least able to detect an error in. **Giving `klondike/` a no-worry-back
   mode is the next thing worth doing**, and it needs the safe-autoplay rule
   implemented a second time for one deck and four foundations, with its own
   proof.
8. **Batch runner.** Done 2026-09-15: `gypsy batch`, generic over the `Game`
   trait, bounded concurrency, one record appended per deal as it finishes,
   `--resume` by seed, and refusals on memory and free space. The batch *runs*
   are still gated on the section below.
9. Batch runs. Worry-back enabled. Analysis + writeup.
10. WASM front end, last.

### Where work resumes

**Close the validation hole: give `klondike/` a no-worry-back mode and
implement safe autoplay for it.** Safe autoplay is the only dominance the
project has and the only one on the list that is not dead, and it is currently
checked on the weakest set available. Until Klondike can exercise it, the cut
carrying the most weight is the one with the least evidence behind it. The
Klondike rule is the single-deck one — two opposite-colour foundations, not
four — so it is a second implementation and needs its own proof.

After that, in rough order: a dominance that survives worry-back, which is the
only thing that moves the headline figure; then the 1,000-deal Klondike
validation the gate below is written against, which `gypsy batch` is now built
for.

### Do not run a Gypsy batch yet

The Klondike unknown bucket stands at **22% on 50 deals at a 48M budget**
(2026-09-15, probing table), and 34% at 3M. It was 40% at 3M on 2026-09-13.
While it is anywhere near this large the validation bracket spans tens of
points, which is consistent with a correct search and cannot distinguish one
from a search that misses wins systematically.

**Budget will not close it.** Swept at 3M, 12M and 48M, the bucket falls by a
factor of about 0.804 per fourfold step, and that slope did not change when
the table was fixed — the curve moved down, not round. From 11 unknown, the 5%
gate is about 6.8 further fourfold steps, roughly 10^4 times the budget. The
route has to be dominances or something that is not a dominance at all, not a
bigger number on `--budget`.

Two thresholds, and they are different:

- **Below about 5% unknown on a thousand Klondike deals**, validation is a real
  check rather than a formality, and Gypsy batch work can start.
- **Below about 1% unknown**, the +/-0.5% Gypsy figure this project exists to
  produce is reachable. Above that, the unknown bucket is wider than the
  confidence interval being claimed and the number cannot be published however
  many deals are run.

Solvitaire resolved essentially all 50,000 instances per game, so this is not
an unreasonable bar — it is the bar the published comparison was set at.

## Front end

Hosted at **solve.fritter.lol**, served statically from the existing fritter.lol
box alongside the other subdomains. No backend, no API, no server-side compute.

Not a live solver — hard deals need minutes and real memory, which won't run in
a browser tab. Instead precompute a few thousand seeded deals and ship the
results as static data.

- Show deal #N, its verdict, solution length; press play to watch the solution.
- Let the visitor play a deal themselves, then reveal whether it was ever
  winnable. Losing a game and learning it was unwinnable from the deal is the
  clearest possible demo of the finding.
- No backend, no compute budget, instant.

Watch the payload. A few thousand deals with full solution move lists gets large
fast — encode moves compactly (a byte or two each) and lazy-load per deal rather
than shipping one large bundle.

## Analysis beyond the headline number

- Wilson interval; how many deals are needed for ±0.5%.
- Predict winnability from the 16 opening face-up cards (logistic regression /
  gradient boosting, feature importances). Do visible aces matter? Suit spread?
- Solution length distribution; number of stock deals used by winning lines.
- How often worry-back is *necessary* versus merely available, among deals
  winnable only with it.
- Behavior of the unknown bucket as the node budget increases.

## Open questions

- What node budget makes the unknown bucket acceptably small?
- Is the correct worry-back dominance provable here, or only a conservative
  approximation?
- Confirm no published Gypsy winnability figure exists before claiming novelty.
- Does the mobile app's shuffle look uniform? (Probably unanswerable without
  extracting deals from it — leave it out unless there's a clean way.)
- Where does the full batch actually run? fritter.lol as it stands cannot host a
  multi-day memory-bound run without freeing storage and memory first. Provision
  the box, or find somewhere else for the big run and keep fritter.lol for
  development and the site.

## Scope discipline

The MVP is: engine → no-worry-back solver → 1,000 deals → writeup. Everything
after that is optional. The playable front end is the most fun part of this
project and therefore the most dangerous — it comes last, after there is a
number worth showing.
