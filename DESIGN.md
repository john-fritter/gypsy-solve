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
runs low. Storage and memory provisioning is a prerequisite for a
thousands-of-deals run, not something to resolve mid-run.

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
3. **Baseline solver.** No-worry-back DFS, transposition table, three-valued
   result, no dominances. Deliberately weak: it is the control that every later
   cut gets measured against, and it is expected to return mostly `unknown`.
4. **Klondike validation**, before any dominance work rather than after. It is
   the only independent check on whether the search is correct, so a slow
   solver and a broken one stay distinguishable. Must reproduce ~81.9%.
5. **Dominances**, one per PR, each with a proof that it cannot discard a
   winning line, each measured against step 3's baseline.
6. Batch runner, then batch runs. Worry-back enabled. Analysis + writeup.
7. WASM front end, last.

Measurements taken 2026-09-11 against a throwaway prototype, which is why
step 5 is on the critical path rather than filed as optimization: a naive DFS
with no dominances solved none of twelve deals at 500k nodes, and none of three
at 10M nodes (~40s and ~2 GiB of table each). Weighted-greedy rollouts reached
21-51 of 104 foundation cards and never won. Neither result says the deals are
unwinnable; they say a naive search is not close, and that nothing yet tells us
whether the search is even correct — hence step 4.

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
