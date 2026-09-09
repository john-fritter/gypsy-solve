# Gypsy Solitaire Solvability

Compute how often two-deck Gypsy solitaire is winnable, and quantify how much
of that winnability depends on returning cards from the foundation to the
tableau ("worry-back").

Secondary goal: a public writeup and an interactive page where a visitor can
play a seeded deal and then find out whether it was ever winnable.

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

- **Canonicalization.** The 8 tableau columns are unordered — hash on sorted
  columns. With alternating-color rules the two red suits are interchangeable
  and so are the two blacks, giving a 4x symmetry reduction for free. Two decks
  means duplicate cards are interchangeable too.
- **Zobrist hashing**, fixed-size table with replacement. Memory is the binding
  constraint, not CPU.
- **Dominances** cut more than any micro-optimization. But see the warning
  below.
- **Node budget**, and report three outcomes: solvable, proven unsolvable,
  **unknown**. The honest third bucket is more credible than a forced binary,
  and how it shrinks as the budget rises is itself a result worth plotting.
- Embarrassingly parallel across deals. One process per deal, one core each.

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

## Validation

Implement **Klondike** in the same engine and check it reproduces ~81.9%
thoughtful winnability. Cheap, and it proves the pipeline before any Gypsy
number is trusted. Do this before the first batch run, not after.

Also: hand-verify a handful of claimed solutions by replaying them through the
engine, and confirm every "unsolvable" verdict is exhaustive rather than
budget-exhausted.

## Build order

1. Rust engine + CLI. Seeded deal, apply move, detect win, dump state.
2. **Debug visualizer, early.** Not a game — a way to render a state and step
   through a move list. Terminal TUI or static HTML dump is fine. Watching the
   search make obviously stupid moves is how the bad dominance gets found.
3. No-worry-back solver. Klondike validation run.
4. Batch runs. Worry-back enabled. Analysis + writeup.
5. WASM front end, last.

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

## Scope discipline

The MVP is: engine → no-worry-back solver → 1,000 deals → writeup. Everything
after that is optional. The playable front end is the most fun part of this
project and therefore the most dangerous — it comes last, after there is a
number worth showing.
