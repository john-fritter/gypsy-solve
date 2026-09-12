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

---

## 2026-09-11 — Canonicalisation is gated on an empty stock

**Status:** firm

`DESIGN.md` claimed the transposition key could sort the tableau columns, and
could fold the two red suits together and the two blacks together for "a 4x
symmetry reduction for free". Both are wrong while the stock still holds cards,
and both are now gated on the stock being empty.

The stock deal sends card *i* to column *i*. Column identity therefore decides
who receives what next, and two positions that are identical as a multiset of
columns are not equivalent: deal seed 7, swap columns 0 and 1, apply `S` to
both, and the two positions differ up to column order. Merging them in the
table would discard a genuinely different future.

The suit swap fails by the same argument. Applying H↔D to a position does not
apply it to the 80 undealt cards, so the swapped position is a position from a
*different* deal. It is an isomorphism of the game, not of this deal.

Sound unconditionally, and kept:

- The remaining stock is encoded as its **length** alone. Nothing in the rules
  ever returns a card to the stock, so for a fixed seed the length determines
  the remaining sequence exactly. Eighty bytes of key become one. (The table is
  per-deal, which is what makes this valid.)
- The two foundation slots of a suit are interchangeable and get sorted in the
  key. They are rank counters; the stock does not distinguish them.

Sound only once the stock is empty, and applied behind that gate: sorting
columns, and the suit swap. That is not a small window — the search spends most
of its nodes after the last deal — but it needs the gate and it needs a test
that fails if the gate is removed.

**Why this is recorded at length:** it is the exact failure this project is
most exposed to. It does not crash and it does not look wrong. It merges two
positions with different futures, loses the branch holding the only solution,
and reports a winnable deal as unsolvable — and the published percentage is
quietly too low. Both throwaway prototypes written on 2026-09-11 used the
unsound sorted-column key before this was noticed.

---

## 2026-09-11 — Klondike validation comes before dominance work

**Status:** firm

Build order is: baseline solver, then Klondike validation, then dominances one
at a time. `DESIGN.md` previously implied dominance work could proceed in
parallel with validation.

Measurement forced the question. A naive DFS solved none of twelve deals at
500k nodes and none of three at 10M nodes. That leaves two indistinguishable
explanations — the search is wrong, or two-deck Gypsy is simply hard — and no
amount of dominance work separates them. Klondike does: the answer is known to
be ~81.9%, and a correct solver reaches it quickly. Tuning an unvalidated
search means a dominance bug and a search bug look the same, and the first
symptom of either is a wrong published number.

**Consequence:** the baseline solver keeps its search loop free of Gypsy
specifics, so extracting a game trait for Klondike is mechanical. The trait
itself is not written until Klondike needs it — one implementation is not
enough evidence to design an abstraction around.

**Rejected:** writing the solver against Klondike first, where the right answer
is known throughout. It is the most rigorous order and the slowest to a Gypsy
number, and the baseline is needed anyway as the control for dominance work.

---

## 2026-09-11 — The table separates refutation from abandonment

**Status:** firm

A transposition entry records either that a position was searched to
exhaustion and lost (**refuted**), or that the search gave up on it with a
recorded amount of depth in hand (**abandoned**). A probe reuses a refutation
at any depth; it reuses an abandonment only for a visit with no more depth to
spend than the visit that recorded it, and a caller that skips on one must
report `unknown`.

Collapsing the two is the bug that turns an exhausted budget into a confident
`unsolvable`. The first throwaway prototype written on 2026-09-11 had exactly
that shape — it marked a position seen the moment it was first reached and
never revisited it — so a position first met at depth 249 and cut there was
skipped forever when it was later reached at depth 10. Unsound, and slower.

**Why a refutation needs no depth attached:** a frame keeps its `complete`
flag only when every child was refuted and nothing below it was cut short, and
any depth cut underneath clears the flag all the way to the root. A subtree
still `complete` at the end therefore never wanted depth it did not have, so
its refutation holds however much depth a later visit brings.

Eviction from the fixed-size table costs re-searching and nothing else: a lost
entry is recomputed, never mis-answered.

---

## 2026-09-11 — Keys are 128 bits and compared in full

**Status:** firm

Table slots store the whole 128-bit key and compare all of it, so a bucket
clash is resolved rather than guessed at. A slot is 24 bytes.

**Rejected:** storing a 64-bit hash, which is the usual choice. At ten million
positions in a deal the birthday bound gives roughly a `3e-6` chance of a false
match per deal, and across a batch of thousands that is a near-certainty of
several. A false match merges two different futures, can drop the branch
holding the only win, and reports a winnable deal as `unsolvable` — with
nothing in the output to show it happened. Doubling the key makes that
`1e-25`. The cost is memory, which is measured and budgeted, rather than
correctness, which is not recoverable.

The key table is generated from a frozen seed. It does not decide which deals
exist, but it does decide which collisions are possible, and a published
verdict should be reproducible to that level.

---

## 2026-09-11 — Repetitions on the current path are cut conservatively

**Status:** provisional

Reaching a position already on the search stack cuts that branch — the earlier
visit has strictly more depth in hand and is enumerating the same moves — but
the frame is *also* marked incomplete, so no refutation is recorded through a
repetition.

The conservative half is deliberate. Whether the repeat is genuinely refuted
depends on how its ancestor resolves, which is not known while the ancestor is
still on the stack; recording a refutation that rests on an unresolved ancestor
would let a later search skip a live branch. This is the graph-history problem
and the safe direction is to give up the claim.

**Consequence:** `unsolvable` is much harder to prove than it looks, because
this game is full of short cycles — with worry-back, playing a card up and
worrying it straight back is one. Expect the `unknown` bucket to be dominated
by this rather than by the node budget.

**Provisional** because there is a known better answer: mark incomplete only up
to the ancestor that was repeated rather than all the way to the root. It is
fiddly and it is not worth writing before Klondike says the search is correct
at all. Revisit with a measured refutation rate, not with reasoning.

---

## 2026-09-11 — A claimed win is replayed before it is returned

**Status:** firm

`solve` replays its own move list from the opening position and checks it ends
won. If it does not, the call returns an error naming the line rather than a
verdict.

A solver that misses wins produces a number that is too low and an `unknown`
bucket that is too big — visible, and honest. A solver that invents wins
produces a number that is too high and looks exactly like success. The check
costs one replay per solved deal, against a search that took millions of nodes.

**Consequence:** the batch runner treats this error as a stop condition, not as
a deal-level result. It means the solver is wrong, not that the deal is
strange.

---

## 2026-09-11 — The baseline solver applies no dominances

**Status:** provisional

The first solver cuts only on transpositions and on its own limits. No safe
autoplay, no null-move filtering, no empty-column deduplication — even though
`legal_moves` offers moves that are visibly pointless, such as shifting a whole
face-up column onto an empty one. Move *ordering* is applied, since reordering
discards nothing.

This is the control. Every later cut has to show what it buys against these
numbers, and a cut that changes a verdict rather than a node count is a bug.

Measured on this machine, one core, `--max-depth 400`, 64 MiB table, 2026-09-11:

| Deal | Ruleset | Nodes | Time | Verdict |
|---|---|---|---|---|
| 0 | no worry-back | 1M | 5.3s | unknown (budget) |
| 1 | no worry-back | 1M | 7.2s | unknown (budget) |
| 2 | no worry-back | 1M | 4.0s | unknown (budget) |
| 3 | no worry-back | 1M | 2.4s | unknown (budget) |
| 0 | full | 1M | 5.8s | unknown (budget) |
| 1 | full | 1M | 7.7s | unknown (budget) |

Roughly 140-410k states per second. Slower than the throwaway prototype that
preceded it, which managed 250-330k with a 64-bit key, no cycle check and an
unsound table — the difference is what correctness costs here, and it is not
the thing to optimise before the search is known to be right.

**No real Gypsy deal has been solved yet.** That is expected at this budget and
it is exactly why Klondike validation comes next: it is the only thing that
separates "this search is too weak" from "this search is wrong".
