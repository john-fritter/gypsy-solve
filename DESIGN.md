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
   time, redeals without limit, worry-back permitted. A restricted arm was
   added 2026-09-15 — same variant, worry-back suppressed — which validates
   against no published figure and exists to exercise dominances the full game
   cannot reach.
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

      **Validated properly, 2026-09-15.** Klondike now has a no-worry-back
      mode and the single-deck version of the rule, so the dominance is
      checked on a set with 9 to 10 proven-unsolvable deals and 35 of 50
      decided in both arms. No verdict contradicted; 22.5% fewer nodes; one
      deal newly proved. See the hole below, now closed.

      **Still open for the worry-back game.** The proof needs the
      opposite-colour cards to be on foundations and unable to leave, and
      worry-back is exactly the rule that lets them leave. The repair that
      suggests itself — play it up, worry it back if it is ever wanted — is
      circular under a transposition table and is written up in
      `DECISIONS.md` so nobody re-derives it. A worry-back-legal form does
      exist in the literature, at a stronger threshold, but is proved for a
      single deck only; see `docs/research/prior-art.md`.

   4. *Never split a built run to expose a dead card.* **Done 2026-09-16, and
      the first dominance the full game has ever had.** A move carrying a
      strict suffix of a built run is not offered when the card it would
      uncover has no foundation to go to. Blake & Gent's Theorem 4, which
      unlike their safe-foundation rule is proved for duplicate cards, and
      which reaches Gypsy because the permissive group-move variant gives the
      one policy for single cards and groups that the theorem requires —
      standard Spider fails exactly there. Two gates of our own for Gypsy, an
      exhausted stock and a non-empty destination; Klondike needs neither.

      Measured before it was built: it removes 42% of the full arm's
      generated moves. Klondike's unknown bucket fell 34% to 18% at 3M,
      better than the old search reached at 48M, with three new
      proven-unsolvable deals and no verdict contradicted. The Gypsy full arm
      went from 0 of 50 resolved to 29, and solved its first deal by itself.

   Every one of these is measured on the same Klondike deal set, and Klondike
   is a regression test with teeth: a dominance may change node counts and
   how many deals resolve, and must **not** change any verdict that was already
   decided, and must leave 81.945% inside the validation bracket. A dominance
   that flips a decided verdict is a wrong dominance, and that is exactly the
   failure this project is least able to detect any other way.

   **That harness had a hole, found and closed 2026-09-15.** Klondike here is
   the published *worry-back* variant, so a dominance gated on worry-back being
   off — which safe autoplay is, and any correct version of the dead two would
   have been — never fired in it and was never checked by it. It was checked
   instead on Gypsy no-worry-back deals, which resolve nine of fifty and prove
   **none** unsolvable — and a discarded winning line shows up precisely as a
   deal wrongly proved unsolvable, so that set could not have caught the error
   it was standing in for.

   `klondike/` now takes a `MoveOptions` like `gypsy_core` does, and the
   restricted arm carries the single-deck safe-autoplay rule: two
   opposite-colour foundations, not Gypsy's four, and the waste excluded as a
   source because playing a card off it re-aligns every later draw-three and
   the reordering proof does not survive that. That set resolves 35 of 50 in
   both arms and proves 9 to 10 unsolvable. Both rules and both proofs are in
   `DECISIONS.md`.
8. **Batch runner.** Done 2026-09-15: `gypsy batch`, generic over the `Game`
   trait, bounded concurrency, one record appended per deal as it finishes,
   `--resume` by seed, refusals on memory and free space, and `--both-arms` to
   solve each deal in both rulesets with each arm's proof carried to the other.
   The batch *runs* are still gated on the section below.
9. Batch runs. Worry-back enabled. Analysis + writeup.
10. WASM front end, last.

### Where work resumes

**Measured 2026-09-15: Klondike wins almost never use worry-back.** Of the 29
deals the 48M run proved solvable, 17 have winning lines containing no
worry-back at all and 28 are *proven* winnable with worry-back suppressed
entirely. One deal, seed 16, is unestablished, and its line uses a single
worry-back. Across fifty deals worry-back is not known to have changed one
verdict — while the published arm pays its branching factor at every node. See
`DECISIONS.md`.

**And then, same day, the Gypsy full arm was run for the first time** — 50
deals, 5M, the restricted arm's parameters exactly. It resolves **0 of 50**,
against the restricted arm's 12. Klondike's full arm at least resolves 33 of
50; Gypsy's resolves nothing. The headline figure is not reachable by direct
search on this shape of solver at any budget worth discussing.

Three things follow, in this order:

1. ~~**Carry verdicts between the two arms.**~~ **Done 2026-09-15**, as
   `gypsy batch --both-arms`. A restricted `solvable` is a full `solvable` and
   a full `unsolvable` is a restricted `unsolvable`; the carried line is
   replayed under the full game rather than assumed. Measured on the same 50
   deals at 5M: the full arm goes from **0 of 50 resolved to 12**, for 13.4%
   fewer nodes, and both arms reproduce the separate runs exactly. See
   `DECISIONS.md`.
2. ~~**Check Gypsy against the incomplete-pile theorem's hypotheses.**~~
   **Done 2026-09-16.** It qualifies, behind two gates of our own — an empty
   stock and a non-empty destination — and is implemented for both games. The
   Gypsy full arm goes from **0 of 50 resolved to 29** and solved its first
   deal by itself; Klondike's unknown bucket falls from 34% to 18% at 3M,
   better than the old search reached at 48M, with three new unsolvable
   proofs and no verdict contradicted. See `DECISIONS.md`. Next, only if
   still needed, the **capped arm**,
   `--worry-back-limit k` swept over *k*, as the fallback it was always meant
   to be: if the uncapped arm still resolves nothing, a lower
   bound is the only form the worry-back delta can take. One constraint
   settled in advance — worry-backs spent is path state and the table stores
   positions, so an exhausted capped search proves *no win within k*, never
   `Unsolvable`. `Unsolvable` must map to `Unknown` in that arm.
3. **Then re-measure.** Whether the delta is even visible between *k*=0 and
   small *k* decides whether the headline finding survives in any form.

**Tried and rejected, 2026-09-15: capping depth.** Gypsy winning lines run
1,301 to 99,982 moves against Klondike's 136 to 467, and seed 40's ends 18
frames short of the depth guard — so it looks as though the search finds wins
by plunging and a depth cap should find shorter ones sooner. It does not. Of
four deals with known wins, only one improved; two resolved at no cap tried,
each burning the whole budget. A cap makes the search drown in the breadth of a
game where any alternating-colour sequence moves as a unit; uncapped it drowns
in depth. See `DECISIONS.md`.

**A dominance that survives worry-back exists and is published — but not for two
decks.** Corrected twice on 2026-09-15: first against the reference solvers,
then against the paper itself. See `docs/research/prior-art.md` and the report
in `docs/reports/`.

Blake & Gent's safe-foundation rule does hold with worry-back legal, at a
stronger threshold than the no-worry-back one: opposite-colour foundations
within two ranks *and* the same-colour twin within three. A **worry-back ban**
comes with it as a corollary — never worry back a card that would immediately
be safely buildable again.

**Both are proved for a single deck only, and the paper says so outright:**
duplicate cards "lead to potential edge cases that we do not consider in this
proof". Solvitaire's two-deck guard is that boundary enforced. For Gypsy these
are unproven rather than unavailable, and adopting either means extending the
paper, with our own duplicate-card argument, exactly as our existing safe
autoplay was proved rather than inherited.

**The rule to take first is a different one: the incomplete-pile dominance**
(Appendix B.2), which *is* generalised past a single deck — an incomplete built
pile need only be moved when the card above it is built immediately to
foundation. Gypsy looks to qualify because of the permissive group-move rule:
its hypotheses want single-card and group moves to follow the same policy,
which is exactly what standard Spider fails and we satisfy. Two hypotheses are
unchecked — the deals-to-every-column stock, and whether the proof depends on
foundations being irremovable — and neither is to be assumed.

**Also open, and not a dominance:** in *Klondike's* full arm the wins are found
either almost instantly or at enormous cost — twelve of 29 under 500 nodes,
median 12,696, then a tail to 29.8M, while the unknowns — eleven then, six on
the current search — each burn the whole 48M.
That is the profile of a search committed to the wrong subtree near the root
rather than one facing a graph slightly too large, and it is consistent with
the slope surviving every improvement to the search. If it holds up, randomised restarts under a fixed total budget
are a lever that is not on that curve, and they cost nothing in rigour: a
restart phase can only turn `unknown` into `solvable`, never claim
`unsolvable`. Worth testing on the eleven unknown deals for the price of one
48M run.

After all that: the 1,000-deal Klondike validation the gate below is written
against, which `gypsy batch` is now built for.

**How a dominance gets checked**, settled 2026-09-15 and worth following
rather than re-deriving:

- Pick the arm the rule fires in. A worry-back dominance goes on the **full**
  Klondike arm, which also has the 81.945% bracket to stay inside. A rule
  gated on worry-back being off goes on `--no-worry-back`, which validates
  against no published figure and is only a before-and-after comparison.
- Build the comparison arm by copying the tree and deleting the rule from
  `legal_actions`. There is deliberately no runtime toggle: a dominance that
  can be switched off is one nobody has committed to.
- Run both arms over the same seeds and budget and compare per seed. The bar
  is: **no verdict contradicted and none regressed to `unknown`.** Node counts
  are the payoff, not the test.
- **Identical line lengths are a bar only for a rule that *forces* a move**,
  such as safe autoplay, where the move was already first in the ordering and
  the first descent barely moves. A rule that *removes* moves sends the search
  down a different path and will return different — in practice much shorter —
  winning lines. That is not a failure: every line is replayed from the deal
  before it is believed, and a shorter line is evidence the search stopped
  wandering. Amended 2026-09-16, when the split-run rule cut every Klondike
  line it touched, one from 1,351 moves to 231.
- The test that has teeth is the deals proved **unsolvable**, because that is
  the direction a discarded winning line fails in. A set that proves none —
  as the Gypsy no-worry-back set does — cannot catch the error at all, which
  is what went wrong the first time. `klondike --no-worry-back` proves 9 to 10
  of 50 at 3M; the full arm proves 9 of 50.
- Re-run the arm you did *not* change and diff it against the recorded run in
  `docs/results/`. Threading an option through `legal_actions` touches every
  caller, and "the other arm is untouched" is worth checking rather than
  assuming.
- Keep both result files in `docs/results/` as `<game>-<arm>-<budget>-<n>deals
  -{baseline,<rule>}.jsonl`, and append the decision entry with the table.

### Do not run a Gypsy batch yet

The Klondike unknown bucket stands at **18% on 50 deals at a 3M budget**, and
**12% at 48M** (2026-09-16, split-run dominance). The run of it: 40% at 3M on
2026-09-13, 34% once the table probed, 22% at 3M's *sixteenfold* budget of 48M —
and now 18% at 3M again, 12% at 48M. While it is anywhere near this large the
validation bracket spans tens of points, which is consistent with a correct
search and cannot distinguish one from a search that misses wins systematically.
It is 40.6 points wide at 3M and 34.6 at 48M, against 65.8 before.

**Budget will not close it; a dominance just did more than 16x the budget.**
Re-swept on the current search at 3M, 12M and 48M (2026-09-16): 18%, 16%, 12%
unknown, a factor of **0.817 per fourfold step** against the 0.804 measured
before either dominance existed. The slope has now survived two large
improvements unchanged — the curve moves down, not round. What the dominance
bought is height, and it is worth two orders of magnitude: the 5% gate is 400x
the 48M budget away, 1.9x10^10 nodes per deal, where the old curve put it at
4x10^4 times and 2x10^12.

**Still out of reach, and the binding constraint is now memory rather than
time.** 88 CPU-days would buy a thousand deals at that budget; a table sized to
it at this sweep's own protocol is 1.7 TB per worker. At any affordable table
the level stops being a measurement of budget and becomes one of eviction, so
the extrapolation fails its own conditions before the gate: nothing on this
curve is measurable much past 10^9 nodes per deal. The route is dominances, for
the third time and now for a third reason.

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
- ~~Confirm no published Gypsy winnability figure exists before claiming
  novelty.~~ **Answered 2026-09-15: none exists.** A clean negative across the
  paper, Solvitaire's presets and a bibliographic search. The one hit generates
  Gypsy rather than measuring it and is explicitly not peer reviewed. The close
  relatives — Irmgard, Blockade, Miss Milligan, Gargantua, Spider — are
  distinct games whose figures do not transfer. See `docs/reports/`.
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
