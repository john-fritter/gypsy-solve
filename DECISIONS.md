# Decision log

Append-only record of design decisions, why they were made, and what was
rejected. `DESIGN.md` is the starting point and the current plan; this file is
how the plan changed and what the implementation actually settled on. Entries
are never edited to look correct in hindsight — a decision that turns out wrong
gets its status changed and a new entry explaining the reversal.

**Status** is one of:

- **firm** — settled, needs a concrete reason to reopen.
- **provisional** — deliberately deferred or awaiting evidence; expected to change.
- **superseded** — replaced; the entry names its replacement.

Every PR that makes a non-obvious choice appends here. Keep entries short.

---

## 2026-09-09 — Two decks are indistinguishable

**Status:** firm

A `Card` is suit and rank only. There is no deck identifier, so the two copies
of 7♥ are the same value everywhere in the engine.

Nothing in the rules can tell them apart, and pretending otherwise would double
the state space for no gain — two positions differing only in which physical
7♥ sits where are the same position, and the solver's transposition table has
to treat them that way to be worth anything.

---

## 2026-09-09 — Foundations are eight fixed slots, never resorted

**Status:** firm

`foundations: [u8; 8]`, one number per pile holding the rank of its top card,
`0` for empty. Slots 0-1 are spades, 2-3 hearts, 4-5 clubs, 6-7 diamonds. Slot
indices never move.

**Rejected:** keeping each suit's pair sorted by rank. Attractive because the
two piles of a suit are interchangeable, but it makes slot numbers shift as a
side effect of unrelated moves, and a recorded move list like `T3>F2` would
stop meaning what it meant when it was written. Published claims have to be
replayable by a stranger, which a move list with drifting semantics is not.

**Consequence:** two identical piles of a suit would generate the same move
twice under different names. Deduplicated at generation instead —
`foundation_target` returns only the lowest matching slot, and worry-back
generation skips a suit's second slot when it equals the first. Canonicalisation
for hashing (sorting foundations, sorting columns, swapping the interchangeable
red and black suits) happens on a copy in the solver, which does not require
storing anything sorted.

---

## 2026-09-09 — Move generation is rules-legal only

**Status:** firm

`legal_moves` emits exactly what the rules permit. No dominance, no pruning,
not even the obviously-null move of shifting a whole column onto an empty one.

A wrong dominance rule does not crash and does not look wrong: it discards the
branch holding the only solution and reports a winnable deal as unsolvable, and
the published percentage is quietly incorrect. Worry-back specifically
invalidates safe autoplay, which is the rule solvers normally depend on.
Solvitaire's authors shipped that bug and found it in a published Klondike
solver too.

So every cut goes in the solver, individually, each with an argument for why it
cannot discard a winning line. Filtering "silly" moves in the rules layer would
entangle correctness of the rules with correctness of the search and leave
nowhere clean to test either.

---

## 2026-09-09 — Worry-back is a generation option, not a rules switch

**Status:** firm

`MoveOptions::NO_WORRY_BACK` restricts what `legal_moves` offers. `apply`
always executes any move the rules permit, with no flag.

This keeps the no-worry-back run a restricted search of the *same* game rather
than a second ruleset, which is what the one-implementation constraint
requires. It also means a solution found in either mode replays through an
identical code path, so verification never has to know which mode produced a
line. The no-worry-back figure is automatically a lower bound on the full-rules
figure.

---

## 2026-09-09 — The shuffle is a frozen in-repo SplitMix64

**Status:** firm

`core/src/rng.rs` contains SplitMix64 and a Fisher-Yates shuffle with rejection
sampling, about fifteen lines, tested against SplitMix64's published reference
output.

**Rejected:** the `rand` crate. Its default generator and shuffle internals
have changed across major versions. Publishing "deal 8,617,332 is unwinnable"
and later having a dependency update silently redefine which deal that is
breaks reproducibility with no error anywhere.

Rejection sampling rather than `% n` because modulo bias would systematically
tilt *which deals exist* — invisible in a spot check, contaminating in a
distribution.

`state::tests::seed_42_deals_the_same_cards_it_always_has` pins one deal's
entire opening. Any change to deck construction order, the generator, or the
shuffle fails it loudly instead of silently redefining every published seed.
Confirmed reproducing identically on fritter.lol, 2026-09-10.

---

## 2026-09-09 — Text move notation is the record format

**Status:** firm

`S` deals the stock, `T0>T3:2` moves cards between columns, `T4>F1` plays to a
foundation, `F1>T4` worries back. Parses and prints round-trip.

Human-readable because move lists are the artefact a stranger uses to verify a
claimed solution, and because reading a search trace by eye is the primary tool
for finding a bad dominance rule. A compact binary encoding will be needed for
the web payload later; that is a separate encoding of the same moves, not a
replacement for this one.

---

## 2026-09-09 — Columns stay `Vec<Card>` for now

**Status:** provisional

Tableau columns are heap-allocated `Vec`s and the solver will clone states
rather than make/unmake moves.

This is the wrong representation for a hot search loop — fixed-capacity inline
arrays of copyable values almost certainly win, and state is small enough for
that to be practical. It is left alone until there is a benchmark, because the
right time to change it is the solver PR where the cost is measurable. Revisit
with `criterion` numbers, not with reasoning.

---

## 2026-09-10 — Batch sizing is bound by what fritter.lol actually has

**Status:** firm

Reconnaissance on the box (Gizmo's report, kept at
`docs/reports/setup-report-20260910T000010Z.md`) changes the numbers the batch
runner has to be designed against:

- 4 physical cores / 8 threads, shared with a live production stack. Four
  workers is the starting ceiling, not eight.
- 15 GiB RAM total but **~4.9 GiB available**, and swap nearly full. The
  transposition table must be sized from available memory, not nominal, and per
  solve RSS gets measured before any multi-worker run.
- Root filesystem 96% used, on RAID0 with no redundancy. Batch output is not
  safe archival storage.

So: bounded concurrency, per-deal incremental writes, clean resume after a kill,
a configurable output path (default `/home/seeduser/gypsy-solve/results`), and a
hard stop before the filesystem fills. Storage and memory provisioning is a
prerequisite for a thousands-of-deals run, not a detail to sort out during one.

DESIGN.md said memory was the binding constraint. It is, and it is tighter than
assumed — the relevant figure is roughly 1 GiB per worker, not 15 GiB for one
big table.

---

## 2026-09-10 — Design decisions get recorded in the repo

**Status:** firm

This file exists because the four decisions above were explained in chat and
nowhere else, which means they would have been lost. Design rationale is part
of the deliverable: the writeup needs it, and a reviewer reading a solver
dominance rule in six months needs to know what was already considered and
rejected.

`DESIGN.md` is a starting point, not a fixed plan. When reality contradicts it,
the entry goes here and DESIGN.md gets corrected — it does not stay stale and it
does not get treated as settled.

Process note from the same day: the repo was initialised by pushing a feature
branch first, which made GitHub adopt that branch as the default and forced a
manual fix. On a fresh empty repo, push the default branch first.
