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

**Status:** superseded by *The table is a set of expanded positions, with no
depth* (2026-09-13)

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

**Status:** superseded by *Repetitions cost the table an entry, not the root its
proof* (2026-09-12)

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

---

## 2026-09-11 — Measured: the depth limit makes the table re-expand states

**Status:** firm (a measurement, not a choice)

The baseline solver re-expands the same positions roughly twenty times each.
Seed 2, no worry-back, 1M nodes, 64 MiB table (2,097,152 slots):

| Depth limit | States expanded | Distinct states recorded | Re-expansion |
|---|---|---|---|
| 120 | 1,000,000 | 45,401 | 22x |
| 150 | 1,000,000 | 32,302 | 31x |
| 200 | 1,000,000 | 22,973 | 44x |
| 300 | 1,000,000 | 18,177 | 55x |
| 400 | 1,000,000 | 17,424 | 57x |

And at 100M nodes on the same deal, depth 400, a 1 GiB table: 631,875 distinct
states. Half a billion states expanded per million recorded.

The table is not the constraint — 45k entries in 2M slots means eviction is not
happening. The cause is the interaction between the depth limit and the
abandonment rule: an entry recorded at depth *d* only answers a visit with no
more depth than *d*, and a depth-first search arrives at the same position with
a different amount of depth in hand almost every time, so most probes miss and
the position is searched again from scratch. A wider depth limit makes it
worse, because it spreads arrivals over more distinct depths.

This is the gap between the baseline and a usable solver, and it is larger than
anything a dominance will recover. Twenty to fifty times the work is being
spent re-deriving results already computed.

**Not fixed here, and deliberately.** The candidate answers — iterative
deepening so that each pass has one depth to record against, a progress measure
that makes the graph acyclic enough to drop the depth limit, or the
repeated-ancestor fix noted above — change the shape of the search, and
choosing between them on reasoning rather than evidence is how the wrong one
gets built. Klondike validation comes first: it says whether the search is
correct, and it supplies deals with known answers to measure a replacement
against. A search this inefficient will still reproduce ~81.9% if it is right,
only slowly.

**Consequence for the ordering already decided:** unchanged, and reinforced.
Dominance work on top of a search doing fifty times redundant work would be
measuring the wrong thing.

---

## 2026-09-12 — Klondike is a separate crate behind a `Game` trait

**Status:** firm

The solver is generic over a `Game` trait — legal actions, apply, is-won, and a
transposition key — and Klondike lives in its own `klondike/` crate
implementing it. `gypsy-core` is untouched.

The trait is not there for elegance. Klondike's job is to say whether the
search is correct, and a validation that exercised a *different* search would
say nothing about the one that publishes the Gypsy numbers. Generic means the
thing under test is literally the thing that ships.

**Not in `gypsy-core`,** because that crate is the single implementation of the
Gypsy rules and is what compiles to WASM for the site. A game nobody plays
there does not belong in that payload.

**Rejected:** waiting for a second implementation before extracting the trait,
which is the usual discipline and was the plan recorded on 2026-09-11. Klondike
*is* the second implementation, and writing it against a concrete Gypsy search
would have meant writing the abstraction anyway, one commit later and with a
throwaway in between.

The key is where the games genuinely differ, and the contrast is worth stating:
Gypsy may not sort its columns until the stock is spent, because the stock
deals card *i* to column *i*; Klondike sorts its piles throughout, because a
draw goes to the waste and no rule ever names a pile from outside. Klondike's
talon is encoded card by card in its current order plus how far through the
pass has got — its *length* says nothing, since cards leave from the middle and
redeals return to the start of what is left.

---

## 2026-09-12 — Repetitions cost the table an entry, not the root its proof

**Status:** superseded by *The table is a set of expanded positions, with no
depth* (2026-09-13)

A frame now records *why* it did not finish: nothing, a repetition, or a limit.
A root that ran out of nothing worse than repetitions returns `unsolvable`,
where before it returned `unknown`.

**The argument.** Take any winning line from the root and take a shortest one.
It cannot visit a position twice — the stretch between two visits could be
deleted for a shorter win. So a shortest win is never what a repetition check
cuts, and a search stopped by nothing but repetitions has still seen every win
there is. Finding none is a proof there is none.

**It survives the transposition table**, which is the part that looked wrong
at first. A table entry was recorded against a *different* path, and it cut
branches looping back to ancestors the current path need not contain, so it
seems it could hide a win. It cannot. Take the shortest win `s0 → … → sk`. If
`sj` was ever expanded then `s(j+1)` was generated, and the search either won
on it, expanded it, skipped it as on-path, or skipped it on a table entry —
and *both* kinds of skip mean it had already been expanded. By induction from
the root every `sj` is expanded, and expanding `s(k-1)` produces the win. The
only step that breaks the chain is a limit cut, where a position is never
expanded at all.

So an abandonment records why it gave up, and a search that skips on one
inherits that reason. A proof survives repetitions from either source and never
survives a limit.

The first attempt left repetition-cut frames out of the table instead, which is
also sound but cost the table too much: on the same 25 Klondike deals it took
solved deals from 9 down to 6, because the re-searching ate the budget. That is
what sent me back to check whether the table-inherited case was really unsound.

None of this licences recording a *refutation* through a repetition, whose
truth can still depend on an ancestor sitting unresolved on the stack — the
graph-history problem the superseded entry was right to be careful about. Those
stay abandonments.

**Why it was worth revisiting now.** The superseded rule made `unsolvable`
nearly unreachable, and Klondike's known answer is roughly 18% unsolvable — so
validation could not have used half of its own signal. Klondike deal 2 was
exhausting its entire search tree in 16,699 nodes, unchanged whether the depth
limit was 400 or 10,000, and still coming back `unknown`. It now comes back
`unsolvable` in the same 16,699 nodes.

---

## 2026-09-12 — The Klondike variant is the one the published figure measures

**Status:** firm

24-card stock drawn three at a time, redeals without limit, worry-back from the
foundations permitted. This is the variant behind Solvitaire's 81.945% ±
0.084% thoughtful figure (Blake & Gent). Draw-one is roughly nine points higher
and would validate nothing.

Worry-back being part of it is a bonus rather than a nuisance: it is the risky
path on the Gypsy side too, so validation exercises it rather than stepping
around it.

**Two rules were not confirmed from the paper** and follow the near-universal
convention: only a king may be placed on an empty pile, and any correctly
sequenced suffix of a pile's face-up cards may be moved rather than only the
whole run. If the measured rate misses the target these are the first suspects,
and each is a one-line change to test.

The deal takes cards pile by pile rather than interleaving rows as a physical
deal does. Both map a uniformly shuffled deck to a uniformly distributed
position, and this one is easier to check.

---

## 2026-09-12 — Klondike validation: consistent, and too weak to be worth much

**Status:** firm (a measurement)

200 Klondike deals, 3M node budget, depth 400, one core. Raw results kept at
`docs/results/klondike-3M-200deals.jsonl`; summarise with
`analysis/klondike_validation.py`.

| | Count | Share |
|---|---|---|
| solvable (each replayed to a win) | 76 | 38.0% |
| unsolvable (each an exhaustive refutation) | 24 | 12.0% |
| unknown | 100 | 50.0% |

Every unknown was stopped by the node budget. Not one hit the depth limit, so
depth 400 is not binding and the earlier worry about winning lines being longer
than the limit was unfounded for Klondike.

**What this establishes.** True winnability is at least the solvable rate and
at most one minus the proven-unsolvable rate, which after widening for sampling
error is **31.6% to 91.8%**. The published 81.945% sits inside. So does 50%,
and so does 85%. A 60-point bracket is consistent with correctness and is not
evidence of it.

**The one part that does discriminate.** A proven-unsolvable verdict is
exhaustive, so that rate can only ever be a lower bound on the true 18.06%. It
came in at 12.0%, Wilson 95% [8.2%, 17.2%] — under the ceiling, with the top of
the interval just below it. A search that refutes deals it should not would
push through 18.06%, and this does not. It is a one-sided check and it passes.

**What is not established.** Nothing rules out a solver that misses wins for a
systematic reason rather than a budget reason. Half the sample is unknown, and
until that shrinks the two look identical from here.

**The blocker is the one already measured.** Every unknown is budget-capped,
and the re-expansion cost recorded on 2026-09-11 says most of that budget goes
on positions the search has already seen 20 to 57 times. That is the thing to
fix, and Klondike now gives it a scoreboard: the bracket narrows as the search
improves, and the published figure staying inside it is the regression test.

---

## 2026-09-12 — Measured: the search does not converge with budget

**Status:** superseded by *Re-measured: the search does converge with budget,
far too slowly* (2026-09-14). The measurement was taken on the depth-indexed
table and does not describe the search that replaced it.

The same 50 Klondike deals, at 3M and at 12M nodes. Raw results in
`docs/results/`.

| Budget | Solvable | Unsolvable | Unknown |
|---|---|---|---|
| 3M | 16 | 7 | 27 |
| 12M | 17 | 8 | 25 |

Four times the budget resolved **two more deals**, 7% of the unknown bucket, at
a cost of 1,483 seconds for the 50. No verdict regressed, which is the
consistency check worth having: nothing that was decided at 3M became unknown
at 12M, and nothing flipped between solvable and unsolvable.

Extrapolate that and clearing the remaining 25 needs a budget nobody is going
to spend. The unknown bucket is not shrinking with compute in any way that
matters, which is a different claim from "the search is slow" and a much worse
one. It means the honest ±0.5% Gypsy figure this project is for cannot be
reached by turning the budget up, on hardware we have or on any hardware.

**This reorders the build.** DESIGN.md had dominances next. They are not next.
A dominance removes part of the tree; the measured problem is that whatever
tree remains gets searched twenty to fifty times over, so the multiplier
applies to the smaller tree just the same. The re-expansion cost recorded on
2026-09-11 is now the critical path, and the candidates named there —
iterative deepening so each pass records against a single depth, a progress
measure that retires the depth limit, or something else — get chosen on
Klondike, where the bracket narrowing is the scoreboard and 81.945% staying
inside it is the regression test.

**What is not in doubt.** Nothing here suggests the search is wrong. The
verdicts are monotone in budget, the proven-unsolvable rate stays under its
ceiling, and every reported win replays. It is the right search, built badly.

---

## 2026-09-13 — The table is a set of expanded positions, with no depth

**Status:** firm. Supersedes *The table separates refutation from abandonment*
(2026-09-11) and *Repetitions cost the table an entry, not the root its proof*
(2026-09-12).

A transposition entry is now a key and nothing else. A position already
expanded is skipped. Positions are never re-expanded, loops need no special
handling, and the refutation/abandonment distinction is gone along with the
depth that made it necessary.

**The argument.** Expanding a position generates every child. Take a shortest
winning line `s0 → … → sk`. If `sj` has been expanded then `s(j+1)` was
generated, and the search either won on it, expanded it, or skipped it as
already expanded — so `s(j+1)` gets expanded either way. By induction every
position on the line is expanded, and expanding `s(k-1)` produces the win.
Depth never enters it. Nor do repetitions: an ancestor on the current path is
already expanded, so the table skips it with no separate check.

What breaks the induction is a position that is *never* expanded, which only
the node budget and the stack guard can cause. So `unsolvable` is claimed
exactly when neither was hit, and the whole three-way verdict falls out of two
booleans instead of the propagating `Cut` lattice it replaced.

**Why the superseded designs existed.** Both were sound. Both were built around
a depth limit that should not have been there. Indexing entries by remaining
depth meant a probe answered only a visit with no more depth to spend, and
depth-first search arrives with a different amount in hand nearly every time —
hence the 20 to 57 times re-expansion measured on 2026-09-11, and the failure
to converge with budget measured on 2026-09-12.

Getting here went through those designs rather than around them. The repetition
work is what first produced `unsolvable` verdicts, which is what made Klondike
validation discriminate at all, and the induction that justified inheriting a
repetition through the table is the same induction that turns out to justify
dropping depth entirely. The intermediate steps were how the argument was
found.

**Measured**, same 50 Klondike deals, 3M budget, one core:

| Table | Solvable | Unsolvable | Unknown | Resolved | Wall |
|---|---|---|---|---|---|
| depth-indexed | 16 | 7 | 27 | 46% | 373s |
| expanded-set | 21 | 9 | 20 | 60% | 330s |

Seven deals moved, every one of them from `unknown` to decided, and no verdict
contradicted the old search — nothing previously decided changed, which is the
check that matters. It also ran slightly faster while resolving more, so the
extra coverage is not being bought with time.

Raw results in `docs/results/klondike-3M-50deals-expanded-set.jsonl` against
the first 50 rows of `klondike-3M-200deals.jsonl`.

**Keep the gain in proportion.** Eighteen points, not the twenty- to fiftyfold
the re-expansion figure might suggest. Killing re-expansion means a budget unit
now buys a position the search has not seen, rather than one it has seen fifty
times, so coverage rose by about that factor — but Klondike's hard deals need
far more than three million distinct positions, and most of the new coverage
lands short of them.

**`max_depth` survives only as a stack guard.** It bounds memory against a
pathological descent; nothing is re-expanded because of it, and hitting it
means something is wrong rather than that it needs raising. Default 100,000.

**Slots are 16 bytes**, down from 24, since only the key is stored. An all-zero
key reads as an empty slot, which would cost one re-expansion per encounter at
a probability of `2^-128`.

**This does not close the gap.** Klondike still leaves a large unknown bucket,
and Solvitaire resolves essentially all 50,000 of its instances. The remaining
distance is dominances, which are now worth having: a cut that removes part of
the tree was nearly worthless while whatever remained was searched fifty times
over.

---

## 2026-09-14 — No Gypsy batch until Klondike's unknown bucket shrinks

**Status:** firm

Gypsy batch runs wait on Klondike validation getting below roughly **5%
unknown** on a thousand deals. Publishing a Gypsy figure waits on roughly **1%**.

At 40% unknown, where the solver stood on 2026-09-13, the validation bracket
spans about thirty points. The published 81.945% sits inside it and so does
almost everything else, so the check passes without discriminating: a search
that misses wins systematically and one that is merely slow produce the same
result. Running thousands of Gypsy deals against a solver in that state buys a
number nobody should believe, after days of compute on a box that has to stay
up for other things.

The second threshold is arithmetic rather than judgement. The unknown bucket is
a hard floor on the width of any interval that can honestly be quoted, because
every unknown deal could go either way. A ±0.5% claim with a 5% unknown bucket
is not a tighter measurement, it is a false one. `CLAUDE.md` already forbids
reporting a bounded search as exact; this is the same rule with a number
attached.

**Rejected:** running the batch anyway and reporting a lower bound. That is
legitimate and it is what a capped worry-back search will have to do, but a
lower bound of "at least 40% of Gypsy deals are winnable" is not a result worth
days of compute, and it would make the project look finished when it is not.

**Not a reason to delay:** the two cheap dominances. They are provable in a
paragraph each, and Klondike is what checks them.

---

## 2026-09-14 — Re-measured: the search does converge with budget, far too slowly

**Status:** firm (a measurement). Supersedes *Measured: the search does not
converge with budget* (2026-09-12).

That entry was taken on the depth-indexed table, which was replaced on
2026-09-13. It reordered the build away from dominances, and it was describing
a search that no longer exists. Re-swept on the expanded-set table: the same 50
Klondike deals at 3M, 12M and 48M nodes, with table entries held at about four
times the node budget at every level so that what varies between levels is the
budget and not table pressure. Raw results in `docs/results/`, files
`klondike-sweep-{3M,12M,48M}-50deals.jsonl`.

| Budget | Table | Solvable | Unsolvable | Unknown | Bracket |
|---|---|---|---|---|---|
| 3M | 256 MiB | 21 | 9 | 20 (40%) | 60.9 pts |
| 12M | 1 GiB | 23 | 10 | 17 (34%) | 55.8 pts |
| 48M | 4 GiB | 27 | 10 | 13 (26%) | 48.4 pts |

**The old conclusion was too strong.** The unknown bucket does shrink with
budget, by a factor of about 0.81 per fourfold step — 0.850 then 0.765, so if
anything the return is improving rather than decaying. The 2026-09-12 figure of
two deals per fourfold step is now three, then four. Killing re-expansion did
not just buy a one-off 18 points of coverage; it restored a real, if slow,
exchange rate between compute and resolved deals.

**The old conclusion's consequence survives anyway.** Extrapolating the 0.81
ratio, reaching the 5% unknown gate needs about 7.7 further fourfold steps —
roughly 4x10^4 times the budget, around 2x10^12 nodes per deal, some 60 days
per deal at the 262k nodes/second measured here. The 1% publishing gate needs
about 10^9 times the budget. Both are out of reach by many orders of magnitude,
on this box or any other. So dominances remain the critical path, and the build
order set on 2026-09-12 stands — but it stands for a corrected reason, and
"the search does not converge" should not be repeated. It converges; the rate
is simply nowhere near enough.

**No verdict regressed.** Across all three levels and both directions, nothing
decided at a lower budget changed at a higher one, and nothing flipped between
solvable and unsolvable. Every unknown at every level was stopped by the node
budget; the stack guard was never reached. The seven deals that resolved were
six solvable and one unsolvable.

**Determinism confirmed across machines.** The 3M level reproduces
`klondike-3M-50deals-expanded-set.jsonl` exactly — all 50 seeds agree on
verdict *and* on node count, on different hardware from the original run. The
README's claim that a verdict is reproducible on any machine is now tested
rather than asserted.

**A warning about the one-sided check.** The 2026-09-12 entry leaned on the
proven-unsolvable rate staying under Klondike's true 18.06%, since an
exhaustive refutation cannot be inflated by a weak search. It was 12.0% then.
It is now 20.0%, above the ceiling. This is not evidence of over-refutation —
at n=50, 10/50 has a Wilson 95% interval of [11.2%, 33.0%] and the ceiling sits
comfortably inside — but the margin the check used to have is gone, and at
n=50 it can no longer discriminate at all. It only becomes a real test at
n=1000, where a sustained 20% would put the ceiling at the very edge of the
interval. Another reason the validation set has to grow, and growing it needs
the parallel runner.

**Rejected:** raising the budget as the route to the gate, which is what this
sweep was run to test. Also rejected: reading the improved exchange rate as a
reason to defer dominances again. Four orders of magnitude is not a tuning
problem.

---

## 2026-09-14 — The two cheap dominances are dead: unsound gated wrong, redundant gated right

**Status:** firm

Neither *interchangeable empty destinations* nor *whole-column-onto-empty*
goes in. Both were listed in `DESIGN.md` as provable in a paragraph and
unconditional. Both are wrong as stated, and worthless once corrected.

**The argument they rest on.** Each says a move only relabels the position:
with two empty columns it does not matter which one a card goes to, and moving
a whole face-up column onto an empty column just exchanges two columns. The
first half is true. `core/src/state.rs` now pins it — the move's result is
exactly the position before it with the two columns swapped.

**Why unconditional is unsound.** The stock deals card *i* to column *i*,
empty columns included. So while cards remain undealt the columns are not
interchangeable: exchanging two of them changes which card lands on the run
and which lands on the empty space. The second new test is the concrete case —
an eight alone in column 0 with the stock about to deal it a king, versus the
same eight moved to column 1 where the stock deals it a seven. One buries the
eight, the other builds a run of two, and no permutation of the columns turns
either position into the other. This is the same gate recorded on 2026-09-11
for canonicalisation, and it applies here for the same reason. A move that
reaches a genuinely different position cannot be dropped without an argument,
and the offered argument is the one that just failed.

**Why gated is redundant.** Gate it on an empty stock and the argument holds —
nothing addresses a column by index again, so the exchange really is a
relabelling. But `Zobrist::key` folds the columns order-insensitively under
exactly that condition, so the child's key *is* the parent's key, and the
parent is in the table because the search is standing on it. The table already
skips the child. The dominance removes nothing the search was going to do.

**Measured**, both cuts implemented and then reverted. 50 Klondike deals, 3M
budget: **node counts identical on every deal**, to the node, and no verdict
changed. That is the redundancy, observed rather than argued. Throughput,
measured single-threaded and interleaved to keep worker contention out of it:
Klondike about 4% faster (1.5% to 6% across deals), Gypsy about 1% slower
(0.4% to 2%). Klondike gains more because there the cut needs no gate —
nothing ever addresses a pile from outside, so its key sorts piles throughout.

A first pass measured 11% on Klondike and 5.8% on Gypsy. Both were four
workers on four cores and both were mostly contention. The interleaved
single-threaded numbers are the ones above.

**Rejected:** shipping the cut for the Klondike gain. It is a throughput
tweak, not a dominance, it is a small loss on the game that actually gets
published, and `CLAUDE.md` puts correctness ahead of speed until validation
passes. Carrying a rule that must be re-proved whenever the key changes, in
exchange for nothing measurable, is the complexity that file says to resist.

**What this costs the plan.** `DESIGN.md`'s dominance list had three entries
and now has one: safe autoplay, the dangerous one. The two that were meant to
be easy wins and to warm up the regression harness were neither. So the route
from a 26% unknown bucket to the 5% gate now runs entirely through safe
autoplay, or through something that is not a dominance at all. That is worth
knowing before the harness work rather than after.

**What it was worth anyway.** The proof obligation in `CLAUDE.md` did exactly
what it is there for: two inherited-looking rules, both stated confidently in
the design document, both wrong. Had either gone in ungated it would have
pruned winning lines, and the only symptom would have been a Gypsy winnability
figure that came out slightly too low — with no test failing and nothing to
notice.

---

## 2026-09-15 — The table probes; unconditional replacement was thrashing

**Status:** firm. Amends *The table is a set of expanded positions, with no
depth* (2026-09-13), which stands except for one sentence.

That entry said eviction "costs a re-expansion and nothing else". It does not.
Two keys that share a slot evict each other, and when both sit on a path the
search walks often they do it indefinitely: every eviction causes a
re-expansion, and every re-expansion causes the reverse eviction. The table is
now open-addressed, probing eight slots forward from the home slot, so the
second key gets a place of its own.

**How it was found.** Not by looking for it. A Gypsy run reported
`table_filled` of 265,363 after expanding two million positions, against
1.9 million for the deal beside it. Expansion only happens on a table miss, so
the gap could not be explained by anything benign.

**Measured before the fix**, 3M node budget, instrumented:

| Deal | distinct positions found | re-expansions |
|---|---|---|
| Gypsy 2 | 266,385, then flat | 2,733,615 — 91% of the budget |
| Klondike 3 | 2,046,484 | 953,516 — 32% |
| Klondike 1 | 2,966,919 | 33,081 — 1% |

Gypsy deal 2 is the shape of the bug: it stopped finding new positions after
about 266,000 of them and spent the rest of its budget walking the same region,
then reported `unknown` for want of a budget it was mostly wasting. Whether a
deal suffered was luck — which keys collided, and whether they sat anywhere hot.

The hash was never at fault. Those 266,385 keys collide 1,022 times in
33.5 million slots, against a birthday expectation of about 1,057. It was the
replacement policy alone.

**After the fix** the same two deals re-expand 0 and 7 positions out of three
million.

**The scoreboard**, 50 Klondike deals, table entries at ~4x the budget as
before. Raw results in `docs/results/klondike-probe-*.jsonl`.

| Budget | Direct-mapped | Probing |
|---|---|---|
| 3M | 21 / 9 / 20 unknown (40%) | 24 / 9 / **17** (34%) |
| 12M | 23 / 10 / 17 (34%) | 26 / 10 / **14** (28%) |
| 48M | 27 / 10 / 13 (26%) | 29 / 10 / **11** (22%) |

No verdict was contradicted at any level and none regressed to `unknown`.
Every deal that moved moved from `unknown` to decided, and all seven that
moved were wins — consistent with a search that was missing wins because it
never got deep enough, which is what the thrashing did.

**Keep it in proportion.** Three deals at every level, worth about one
fourfold budget step, and the slope is unchanged: the unknown bucket still
falls by 0.804 per 4x step against 0.806 before. The curve moved down, not
round. Reaching the 5% gate from 11 unknown still needs about 6.8 further
fourfold steps, roughly 10^4 times the budget. This was a real bug and fixing
it was necessary; it is not a route to the gate.

**A subtlety worth the line it cost.** When the whole probe window belongs to
other keys the new key must be displaced *inside* the window. The first
version wrote it one slot past, where `contains` never looks, so every
displaced key was re-expanded on every visit — which made Klondike deal 1
thirty times worse than the bug being fixed. `a_displaced_key_is_still_found`
pins it.

**Consequence for the batch runner, and it contradicts `DESIGN.md`.** Peak
resident memory is no longer approximately the table size. The search now
descends instead of thrashing, so far more frames stay live:

| Run | Table | Peak RSS | Over table |
|---|---|---|---|
| Klondike 3, 3M | 256 MiB | 362 MiB | 107 MiB |
| Klondike 3, 12M | 1024 MiB | 1437 MiB | 413 MiB |

The excess is bounded by `max_depth` — 100,000 frames at roughly 4 KiB each,
so it saturates near 400 MiB — but it is not negligible, and it killed the
first 48M run outright: three workers at 4 GiB of table each went over the box
and the kernel took them. `DESIGN.md` sizes fritter.lol at four workers with
about 4.9 GiB available, which now means four times a table *plus* up to four
times a stack. Workers must be sized as `table + stack allowance`, and
`max_depth` is a memory knob as well as the stack guard the 2026-09-13 entry
called it.

**Rejected:** a larger table. It does not touch the mechanism — the colliding
pairs are as likely at any size, and Gypsy deal 2 was thrashing in a table 1%
full. Also rejected: N-way set association, which would work, for being more
machinery than probing needs at these load factors.

---

## 2026-09-15 — The batch runner is resumable by seed, and refuses runs that will not fit

**Status:** firm

`gypsy batch` solves many deals with bounded concurrency and appends one JSON
object per deal as that deal finishes. It is generic over the `Game` trait for
the same reason the search is: the runner that publishes the Gypsy numbers has
to be the runner Klondike validated, so `--game klondike` is the same code
path and a 1,000-deal validation set is the same command.

**Records are written and flushed per deal**, not buffered to the end. A run
killed at hour six keeps everything it proved in the first six.

**Resume is by seed, read back out of the output file.** No separate state
file and no checkpoint format: the results *are* the checkpoint, so they cannot
disagree with one. `--resume` skips every seed already recorded; without it,
starting onto a non-empty file is refused rather than appended to or clobbered.

**A torn final record is truncated on resume**, and this is the one piece of
real machinery here. A kill mid-write leaves a record with no newline after
it. Appending to that file runs the next record onto the end of the broken one,
which destroys a *complete* result as well as the torn one — and because the
resume scan counts the good record as done, its deal then goes silently missing
from the batch. That is precisely the failure a resumable runner exists to
prevent, and it was in the first version until a test with a deliberately torn
file caught it. Truncating back to the last whole line costs the one deal that
was genuinely interrupted.

**A worker is charged for its stack as well as its table.** The run refuses to
start when workers times (table + stack) exceeds `MemAvailable`. Before the
probing table landed earlier today the stack term would have been noise; now
the search descends instead of thrashing and a 12M-node Klondike solve ran
413 MiB over its table. The estimate is `max_depth` frames at about four KiB,
so roughly 390 MiB at the default depth. `DESIGN.md`'s "four workers on
fritter.lol" needs re-reading with that in mind: four workers at 1 GiB of table
each want about 5.5 GiB, and the box had about 4.9 GiB available.

**Free space is checked as the run goes**, not only at the start, and dropping
below `--min-free-mib` stops the run cleanly with everything so far recorded
and resumable. A long run can fill a disk that was comfortable when it began,
and `DESIGN.md` records that root was already 96% full.

**A win that does not replay stops the whole run.** The solver returns that as
an error rather than a verdict, and it is the one failure that must never reach
a results file and be summarised into a number.

**Rows land in completion order, not seed order.** Ordering them would mean
holding results back, which is the opposite of the point. Anything that cares
sorts, and the analysis script already does.

**Rejected:** a checkpoint or manifest file alongside the results, which is a
second source of truth that can disagree with the first. Also rejected:
`--force` escapes on the two guards. Both guards fire on conditions that have
already killed a run here, and both name the knob to turn in the refusal.

---

## 2026-09-15 — Safe autoplay, proved and applied to the no-worry-back game only

**Status:** firm

When a card can never be wanted in the tableau again the search plays it and
considers nothing else at that position. This is the dominance `CLAUDE.md`
singles out as the dangerous one, and it is the first cut in this project that
has actually paid.

**The rule, corrected for two decks.** A tableau card of rank *r* and colour
*C* is useful in the tableau for exactly one thing: being a base for a card of
rank *r-1* and the opposite colour. The opposite colour is two suits, and with
two decks each suit has two foundation piles, so **all four** must have passed
*r-1*. The familiar single-deck rule checks two piles; ported unchanged it
would call a black five safe while a second red four was still in play.
`a_card_is_unsafe_until_all_four_opposite_piles_pass_it` pins that exact case.
Aces and twos are always safe: nothing stacks on an ace, and the only card that
stacks on a two is an ace, which never needs a base either — an ace off the
foundations implies a free slot of its suit, since only that suit's two aces
can occupy its two slots.

**The proof.** Let `L` win from this position and let `X` be the safe card, on
top of its column. Winning puts every card up, so `L` plays `X` up at some
point. Play `X` up first and follow `L` with that play removed. No move of `L`
can put a card on `X`: the only candidates are the four opposite-colour cards
of rank *r-1*, all on foundations, and with worry-back off they never come
back. A move of `L` carrying a run that includes `X` carries `X` plus cards
*below* it, so dropping `X` leaves the run's bottom card unchanged and the
destination still accepts it; a move carrying `X` alone simply disappears.
Exposing the card under `X` earlier only adds options. So the reordered line
wins, and restricting the position to that one move cannot lose a win.

**The gate is the proof, not caution.** With worry-back legal the four cards
the condition checks can come back down and want `X` underneath them. The
condition is a claim about the future, and worry-back makes it false.

**The repair that does not work, recorded so it is not re-derived.** Worry-back
looks like it should make this *easier*: play the card up, and worry it back if
it is ever wanted. That argument is circular under a transposition table. It
justifies the restricted position `P'` by appealing to a path from `P'` back to
`P` — but `P` has been expanded with only the forced move in it, so the table
skips it and the search never reaches `P`'s alternatives from `P'` either. The
win the argument promises is one the search can no longer find. This is the
`CLAUDE.md` warning wearing the costume of a fix, and it is presumably close to
what Solvitaire's authors hit twice.

**Measured**, 50 Gypsy deals, worry-back off, 5M budget, table 512 MiB. Raw
results in `docs/results/gypsy-nwb-5M-50deals-{baseline,autoplay}.jsonl`.

| | Solvable | Unknown |
|---|---|---|
| without | 9 | 41 |
| with | **12** | 38 |

No verdict was contradicted and none regressed. On the nine deals decided in
both arms, nodes fell **41.5%** in total — deal 30 by 65.6%, deal 21 by 54.5%,
deal 9 by 35.7%, and two deals not at all.

**Why two deals did not move, and it is not a fault.** `ToFoundation` was
already first in the move ordering, so on the first descent the search was
playing these cards anyway; forcing them changes nothing until something
backtracks. The rule fires on about 3% of expansions — 9,741 of 300,000 on
deal 0 — and the whole gain is in the alternatives it stops the search
revisiting. A deal solved with little backtracking sees no change at all.

**What it does not do.** Nothing for the full game. The headline figure this
project exists to produce is the worry-back one, and it still has no dominance.
It helps the no-worry-back figure, which `DESIGN.md` has shipping first, and
which is half of the worry-back delta.

**The validation is weaker than `DESIGN.md` asks for, and that is worth
stating.** Dominances are supposed to be checked on the Klondike deal set, but
Klondike here is the published worry-back variant, so this dominance never
fires in it. The check above is before-and-after agreement on Gypsy deals,
where only nine of fifty resolve — far less signal than Klondike's sixty
percent. Giving Klondike a no-worry-back mode would restore the teeth and is
the obvious follow-up; it is not folded in here because it needs the rule
implemented a second time, for one deck and four foundations, and that is its
own PR with its own proof.

## 2026-09-15 — Klondike gets a no-worry-back mode, and its own safe autoplay

**Status:** firm

Klondike here is the published worry-back variant, so a dominance gated on
worry-back being *off* never fires in it and the validation set could not check
it. Safe autoplay is that shape of dominance, and it is the only live cut this
project has. `klondike/` now takes a `MoveOptions` the way `gypsy_core` does,
and the restricted arm has the single-deck safe autoplay implemented against
it.

**Two implementations, on purpose.** The Gypsy rule checks four foundation
piles because two decks give each suit two of them; the Klondike rule checks
two. Neither is correct for the other game, and a shared one would be a
parameterised rule whose proof is two proofs. `MoveOptions` is likewise
mirrored rather than shared: `gypsy_core::MoveOptions` belongs to the one
implementation of the *Gypsy* rules, and the two games agreeing on a boolean
today is not a reason to couple their rulesets.

**The rule.** A tableau card of rank *r* and colour *C* is useful in the
tableau only as a base for a card of rank *r-1* and the opposite colour — two
suits, one pile each, and both must have passed *r-1*. Aces and twos are
always safe: nothing stacks on an ace, and the only card that stacks on a two
is an ace, which never needs a base, because an ace off the foundations means
its suit's foundation is empty and will take it at any time. That last step is
simpler than the Gypsy one, where two slots per suit have to be argued about.

**The proof** is the Gypsy reordering argument transplanted: given a winning
line `L` and the safe card `X` on top of pile `p`, play `X` up first and follow
`L` with that play removed. Nothing in `L` can land on `X`, because the only
cards that could are the two opposite-colour *r-1* cards, both on foundations
and, with worry-back off, unable to leave. A run carrying `X` carries `X` plus
cards below it — a run is a suffix — so dropping `X` leaves the bottom card and
the destination test unchanged. Turning up `p`'s next card earlier only adds
options.

**The waste is excluded, and that is the Klondike-specific finding.** The same
reordering is unsound for a safe card on top of the waste. Playing it up
removes it from the talon, which shifts every card behind it down one index and
moves `turned` back, so every later `Draw` turns a different group of three.
The reordered line is then playing a different sequence and its draws no longer
expose the cards its later moves need — the step that carries the pile case,
"the rest of `L` is still legal", fails outright. The behaviour was already
pinned by `playing_the_waste_reshapes_the_later_triples`, written for the deal
rules and now load-bearing for a proof. `WasteToFoundation` stays an ordinary
action among the alternatives. Gypsy has no waste and no such case, so nothing
in the Gypsy rule needed revisiting.

**Measured**, 50 Klondike deals, worry-back off, 3M budget, table 256 MiB. Raw
results in `docs/results/klondike-nwb-3M-50deals-{baseline,autoplay}.jsonl`.

| | Solvable | Unsolvable | Unknown |
|---|---|---|---|
| without | 26 | 9 | 15 |
| with | 26 | **10** | 14 |

No verdict contradicted, none regressed to `unknown`, and every one of the 26
solvable deals came back with an identical line length. Seed 13 went from
`unknown` to a proof of unsolvable. On the 35 deals decided in both arms, nodes
fell **22.5%** — seed 17 by 82.4%, seed 38 by 44.0%, and 11 of the 35 not at
all, which is the same pattern as Gypsy: foundation plays are already first in
the ordering, so forcing them changes nothing until something backtracks.

**This is the check the Gypsy set could not give.** The failure mode that
matters is a dominance discarding a winning line and turning a solvable deal
into a reported `unsolvable`. Detecting it needs deals the search proves
unsolvable. The Gypsy no-worry-back set has **zero** — 0 of 50 in both arms,
41 and 38 unknown — so it could not have caught that error at all; all it
established was that the 9 wins it found were still found. The Klondike
restricted set has 9 and 10, alongside 35 of 50 decided in both arms against
Gypsy's 9. That is the teeth `DESIGN.md` asked for.

**The published arm is untouched, and checked rather than assumed.** Threading
`MoveOptions` through `Position::legal_actions` changed the signature every
caller uses, so the full variant was re-run at 3M over the same 50 deals and
compared with `docs/results/klondike-probe-3M-50deals.jsonl`: identical
verdicts, node counts and line lengths on all 50. The 81.945% bracket is
unmoved because nothing about that arm moved.

**The restricted arm validates against no published figure**, and is not
offered as one. Solvitaire's number is the worry-back variant. What the
restricted arm is for is exercising a rule the full game cannot reach. Worth
noting in passing that it resolves *more* at the same budget — 36 of 50 against
the full arm's 33 — despite having strictly fewer moves, because the smaller
branching factor finds the wins that are there. Both remain lower bounds.

**The analysis script refuses the comparison rather than making it.** Every
Klondike record now carries a `ruleset` field, and
`analysis/klondike_validation.py` prints the restricted arm's bracket but
withholds the 81.945% verdict, because that figure is the worry-back variant
and a restricted run is a different game. Files without the field predate it
and are the full variant. The alternative — letting it compare anyway — would
manufacture an INCONSISTENT result and, worse, could manufacture a CONSISTENT
one while the brackets stay this wide.

**Baseline measurement method.** The comparison arm was a copy of this tree
with the autoplay branch in `Klondike::legal_actions` deleted, built and run
separately; no toggle for it exists in the shipped binary, and none should,
because a dominance that can be switched off at runtime is a dominance nobody
has committed to.

**What this does not do.** Nothing for the full game. The headline figure is
the worry-back one and it still has no dominance at all. This closes the
validation hole; it does not move the number.

## 2026-09-15 — Measured: Klondike wins almost never need worry-back

**Status:** firm as a measurement of Klondike. What it points at for the search
is provisional until the same measurement exists for Gypsy.

`DESIGN.md` sanctions capping the number of worry-backs as a valid lower bound
if the full search proves too expensive. Before building a cap, the cheap
question is whether one would bind at all: how much worry-back do the wins we
already have actually contain?

**Method.** `gypsy klondike --json` now emits the winning line, the way
`gypsy solve --json` always has. The 29 deals that `klondike-probe-48M-50deals`
proved solvable were re-solved at the same budget and their lines counted.
Both games print a worry-back as `F<slot>>T<pile>`, so one counter serves both.
Raw lines in `docs/results/klondike-full-48M-29wins-lines.jsonl`.

27 of the 29 reproduced the recorded run exactly, node for node. Seeds 3 and 16
were killed by the memory cgroup at the 3 GiB table the original run used and
were re-run at 1 GiB; they expanded 705 and 6 more nodes — table displacement,
exactly what `table.rs` predicts — and returned identical line lengths.

**How much worry-back a winning line contains:**

| Worry-backs | Lines | Cumulative |
|---|---|---|
| 0 | 17 | 58.6% |
| 1 | 7 | 82.8% |
| 2 | 1 | 86.2% |
| 3 | 1 | 89.7% |
| 4 | 2 | 96.6% |
| 42 | 1 | 100% |

**This is a measurement of the search, not of the game**, and seed 38 is the
proof of that. Its line uses 42 worry-backs across 1,351 moves — and the
restricted arm solves the same deal outright. All 42 were the search wandering.
Move ordering puts worry-back second to last, so these counts are biased low;
the bias runs the right way for a cap, because a line using *k* worry-backs
witnesses that a win exists within *k*. It says nothing about what was needed.

**The stronger result: 28 of the 29 wins are winnable with worry-back off, and
that is proven rather than inferred.** Two independent witnesses, either of
which settles a deal:

- the restricted arm returns `solvable` — 26 deals;
- the full-arm line contains no worry-back at all — which adds seeds 3 and 47.

The second witness is worth having because the restricted arm ran at 3M and the
full arm at 48M, so a restricted `unknown` is often just a deal that could not
be afforded. Seeds 3, 16 and 47 are precisely the three most expensive wins in
the set — 22.3M, 13.4M and 29.8M nodes — and two of them have zero-worry-back
lines. It is sound because `NO_WORRY_BACK` suppresses the foundation-to-pile
actions and nothing else, pinned by
`no_worry_back_suppresses_only_the_foundation_to_pile_actions`, so a line using
none is already a line of the restricted game.

That leaves **seed 16 alone** unestablished, and its line uses exactly one
worry-back. Across fifty Klondike deals, worry-back is not known to have changed
a single verdict.

**What this does not say.** Klondike is not Gypsy. Gypsy has eight foundations
to Klondike's four, eight columns, no waste, a permissive any-alternating-colour
group move, and a stock that deals onto the columns. The Gypsy worry-back delta
is the headline figure this project exists to produce and **nothing here
measures it.** Gypsy's delta may be large while Klondike's is near zero.

**What it does say** is about the search, and it holds regardless: the published
arm pays worry-back's branching factor at every single node, for something that
across fifty deals changed at most one verdict. That is the wrong price, and it
is the first concrete reason to think the full arm's 22% unknown bucket is
partly self-inflicted rather than intrinsic.

**Consequence: a capped arm is worth building, and one subtlety has to be
settled first.** Worry-backs spent is a property of the *path*, not of the
position. Under a table that stores positions, a deal reached with the cap
exhausted is indistinguishable from the same position reached with the cap
untouched, and whichever arrives first suppresses the other. So a capped search
is a lower bound twice over, and **exhausting it proves only "no win within
*k*", never `Unsolvable`.** The verdict mapping must be `Solvable` → `Solvable`,
`Unsolvable` → `Unknown`, `Unknown` → `Unknown`. This is the same shape of error
as the safe-autoplay repair recorded above — a claim about the path being
checked against a table that has forgotten the path — and it is written down
here so nobody re-derives it as a bug.

**Considered, not taken:** putting worry-backs-spent into the transposition key.
That makes the cap sound and the `Unsolvable` verdict available again, but it
multiplies the state space by *k+1* and gives up exactly the merging the table
exists for. Not measured, so not rejected on evidence — recorded as the
alternative and the reason it was passed over.

**Candidate for later, not a decision:** once a line has spent its cap, the rest
of the game is exactly the restricted game, so safe autoplay is sound again from
that point. Attractive, and subject to the same path-versus-position problem, so
it needs its own proof rather than an appeal to this entry.

## 2026-09-15 — The Gypsy full arm resolves nothing, and a depth cap does not rescue it

**Status:** firm as a measurement.

Every Gypsy result in this repo was the restricted arm. The headline figure is
the worry-back one, so the full arm was run — 50 deals, 5M budget, 512 MiB
table, `max_depth` 100,000, which are the restricted arm's parameters exactly,
so the two are directly comparable. Raw results in
`docs/results/gypsy-full-5M-50deals.jsonl`.

| Gypsy, 50 deals, 5M | Solvable | Unsolvable | Unknown |
|---|---|---|---|
| worry-back off, with safe autoplay | 12 | 0 | 38 |
| worry-back on | **0** | 0 | **50** |

All 50 stopped on the budget; none hit the depth guard. Neither arm is
thrashing — both fill about 5M distinct positions out of 5M expansions, so this
is the size of the reachable graph, not a repeat of the table bug.

**The headline figure is not reachable by direct search.** The worry-back delta
needs both arms, and the full arm resolves nothing whatsoever. Klondike's full
arm at least resolves 33 of 50; Gypsy's resolves zero. Whatever else follows,
no amount of budget on this shape of search produces the number this project
exists to produce.

**Free, and now the most valuable cheap change available.** The restricted
game's moves are a subset of the full game's, so a restricted `solvable` is a
full `solvable` — the same line replays move for move. Carrying the restricted
arm's verdicts across takes the full arm from 0 of 50 to **12 of 50** at zero
compute. On Klondike the same trick was worth 2 deals of 50 and was ranked low;
on Gypsy it is worth every verdict the full arm has. The converse holds too: a
full-arm `unsolvable` is a restricted `unsolvable`. Every deal is solved twice
by design, so neither arm should be re-deriving what the other proved.

**Gypsy winning lines are enormous, and nobody had looked.** The recorded
restricted-arm wins run **1,301 to 99,982 moves**, against Klondike's 136 to
467. Seed 40's line ends 18 frames short of the 100,000 depth guard. A 104-card
game needs about 104 foundation plays; the rest is shuffling. `DESIGN.md` calls
that guard "not a tuning knob — hitting it is a sign something is wrong rather
than a limit to raise", and it is very nearly binding. The 4 KiB per frame it
permits is exactly the memory cost recorded on 2026-09-15.

**The obvious inference from that is wrong, and it was tested rather than
assumed.** If the search finds wins by plunging, capping depth should find
shorter wins sooner. Measured on four deals whose uncapped wins are known,
restricted arm, 5M budget. Raw results in
`docs/results/gypsy-nwb-5M-depth-sweep.jsonl`.

| Seed | uncapped | cap 200 | cap 400 | cap 800 | cap 1600 |
|---|---|---|---|---|---|
| 21 | 1,301 | — | — | — | 1,301 |
| 9 | 1,520 | — | — | **799** | 1,520 |
| 10 | 8,480 | — | — | — | — |
| 16 | 7,953 | — | — | — | — |

A dash is `unknown` after burning the whole 5M. Only seed 9 improved. Seed 21
needs a cap of 1,600 to find the win it finds uncapped in 2,167 nodes, and
seeds 10 and 16 resolve at no cap tried.

**Why it fails, and it is worth stating because it characterises the game.**
A cap makes depth-first search backtrack constantly, and it then drowns in the
*breadth* of a game where any alternating-colour sequence moves as a unit and
almost every position offers dozens of shuffles. Uncapped, it drowns in depth
instead. There is no middle setting, and a capped search that fails costs the
entire budget rather than failing cheaply. **Rejected as a lever**, on this
evidence; one deal improving is not enough to carry it.

**What this does not settle.** Shorter wins may well exist for seeds 10 and 16 —
the test shows only that a depth-capped search does not find them within 5M
nodes, which is a statement about the search. Line length remains a measurement
of how the solver wanders, not of the game, exactly as the worry-back counts
were.

## 2026-09-15 — Both arms in one run, with each arm's proof carried to the other

**Status:** firm.

`DESIGN.md` has always described the experiment as every deal solved twice,
worry-back off and then on. That meant two independent batch runs, each
re-deriving what the other had already proved. `gypsy batch --both-arms` solves
both rulesets per deal and writes a record for each.

**The two implications, and they are not symmetric.** The restricted game
generates a subset of the full game's moves and differs in nothing else, so:

- **a restricted win is a full win** — every move of the line is legal in the
  full game, so it replays there move for move;
- **a full refutation is a restricted refutation** — an exhausted full search
  visited every position the subset could have reached.

Neither carries back. A full win may have used worry-back, which the restricted
game cannot do; a restricted refutation says nothing about a game with strictly
more moves in it. That second gap *is* the worry-back delta, which is why it
cannot be short-circuited.

**The carried line is replayed, not assumed.** A claimed win has been replayed
before it was believed since 2026-09-11, and a carried claim is still a claim.
A line that failed to replay under the full game would stop the run the same
way a bad search result does.

**Only the first carry is expected to pay.** Refuting the full game means
exhausting a strictly larger graph, so an arm that can do that at a given
budget can almost always refute the restricted game directly. Klondike seed 2
refutes in both arms independently, at 909 and 1,231 nodes. The second carry is
kept because it is sound and free, not because it is expected to fire.

**Measured**, 50 Gypsy deals, 5M budget, 512 MiB table — the parameters of the
two runs it replaces, so the comparison is exact. Raw results in
`docs/results/gypsy-both-5M-50deals.jsonl`.

| Gypsy, 5M | Solvable | Unknown | of which carried |
|---|---|---|---|
| no-worry-back | 12 | 38 | 0 |
| full | **12** | 38 | **12** |

The full arm resolved **0 of 50 on its own and 12 of 50 with the carry**. The
restricted arm reproduced `gypsy-nwb-5M-50deals-autoplay` exactly — verdict,
nodes and line length on all 50 — and every searched full-arm record reproduced
`gypsy-full-5M-50deals` exactly. The carried seeds are exactly the restricted
arm's twelve wins.

Nodes fell from 449,029,778 to 389,029,778, **13.4% less** for twelve more
verdicts. The saving is precisely the 5M the full arm was spending on each of
those twelve deals and now spends on none; how large it is in general depends
on what the full arm would otherwise have spent, and here that was the whole
budget every time.

**`verdict_from` is new on every record**, naming the arm that established the
verdict — `"search"`, or the ruleset it came from. A carried verdict spent no
nodes and filled no table, and a reader counting work or auditing which arm
proved what should not have to infer it from a zero. Single-arm runs write
`"search"` throughout, so the schema does not fork.

**Resume needed two fixes, and both were silent-corruption risks** that only
exist once a deal writes two records. Both arms go down in one write, but a
kill can still land between them:

- `recorded_seeds` now counts a seed as done only when every ruleset the run
  writes is present. Without that, a resumed run skips a deal that has one arm
  and the results file is quietly missing it.
- A *complete but orphaned* record left at the tail is dropped before resuming.
  Leaving it would double-count that arm once the deal is solved again — which
  is worse than the first failure, because it inflates a count rather than
  shrinking one.

Both are pinned by tests. Only the tail can be incomplete, which is why the
second truncates rather than rewriting the file.

**The validation script summarises the arms separately and never pools them.**
It used to refuse a mixed file, which was right when nothing produced one.
Pooling two different games would manufacture a figure belonging to neither,
and the danger is a manufactured *pass*.

**A hole found while testing that, and it predates this change.**
`analysis/klondike_validation.py` never checked which game it was reading, so
handed a Gypsy file it printed `CONSISTENT: the published figure is inside the
bracket` against Klondike's 81.945%. It now refuses any run that does not say
`"game":"klondike"`, including one that says nothing — an unlabelled file is
exactly the case that cannot be checked. `gypsy solve --json` now names its
game too, as the Klondike and batch writers already did. The two Gypsy files in
`docs/results` written before this are unlabelled and are correctly refused.

**Rejected: a `--carry-from FILE` flag** reading a previous arm's results. It
needs seed matching, budget and ruleset validation, and a policy for missing
seeds — more plumbing and more ways to be wrong — and it still forces two
sequential runs to get what one run now does. Solving the pair together is what
makes the carry free.

## 2026-09-15 — The worry-back dominance exists and is published; we were re-deriving it

**Status:** firm as a finding. The ports it calls for are not yet made.

After the Gypsy full arm resolved 0 of 50, the plan asked for more dominances
and had no candidate to offer. Before deriving one, the reference solvers were
read. Full survey in `docs/research/prior-art.md`; this entry records what it
changes.

**`DESIGN.md` was wrong, and is corrected.** It said the worry-back game "still
has no dominance at all" and that any next rule "has to be a different shape of
argument". Two rules for exactly this case are published with proofs and ship in
both [Solvitaire](https://github.com/thecharlieblake/Solvitaire) and
[lonelybot](https://github.com/vuonghy2442/lonelybot):

- **Keller's rule** — with worry-back legal, a playable card of rank *r* and
  colour *c* is forcible when `f(both opposite-colour suits) >= r - 1` **and**
  `f(the other suit of colour c) >= r - 2`. Blake & Gent, Appendix B.1.
- **The worry-back ban** — never worry a card back while it is
  safe-automovable. A corollary of their Theorem 1, so it needs no separate
  compatibility proof with the rule above.

The twin conjunct is the whole difference from ours, and it is why ours needed
the worry-back gate. lonelybot's documentation is explicit that `r - 2` is the
classical condition *for worry-back games*, not a relaxation.

**Why this was missed.** `DESIGN.md` has cited Solvitaire since day one, for the
81.945% figure and for the warning that worry-back invalidates safe autoplay. It
took that warning as the end of the story. The same paper contains the repaired
rule. Reading the reference implementation should have come before the third
derivation attempt, not after.

**What is genuinely ours to prove.** Solvitaire switches foundation dominances
off outright for two-deck games — `if (rules.two_decks) return false;` is the
first line of `get_dominance_move` — and lonelybot is single-deck. So the
published proofs are one-deck proofs. The two-deck forms need the same treatment
we already gave our own safe autoplay: each `f(...)` becomes a minimum over that
suit's two foundation piles. That is a port with a template, which is a very
different problem from the open-ended search this project thought it faced.

**A discrepancy in the rule already shipped, to resolve rather than assume
away.** The no-worry-back variant is quoted upstream as `f_opp >= r`; ours uses
`f_opp >= r - 1`. Rank indexing differs between codebases and these may be the
same statement, but ours being one rank more permissive is the direction that
loses wins. Klondike validation contradicted no verdict, which is real evidence
against a bug — it is not a substitute for checking the condition.

**Confirmed independently, and we are ahead on one point.** Solvitaire disables
pile symmetry whenever the stock deals to tableau piles, with the comment "If
the stock deals to the tableau piles, there is no pile symmetry" — our
2026-09-11 finding, reached separately. Their gate is coarser: they disable it
for the whole game, where we gate on the stock being *currently* empty and
recover the symmetry for the endgame, which is where most of the search sits.

**Suit symmetry is dropped as a candidate.** It was proposed as an
already-argued, unimplemented win. Solvitaire classifies it as a *streamliner* —
their word for an unsound speedup — because what it implements is collapsing
suit to colour in the cache key, which conflates positions that per-suit
foundations distinguish. That is the exact situation John raised from play:
needing to expose the particular suit a foundation wants while its same-colour
partner shows. The sound version is a full consistent relabelling, which
lonelybot carries as "twin-swap theorem T" and its own soundness ledger rates
`[~] argued with named gaps`. Unproven at the state of the art, undetectable if
wrong on a deal set that proves nothing unsolvable, and no longer the best rule
available. Not worth the risk.

**Rejected: copying code.** Solvitaire is GPL-2 and this repo is not. Nothing is
lifted from it. What transfers is published rules and their proofs, rewritten
here for two decks, which is what `CLAUDE.md`'s one-implementation constraint
requires anyway.

**Recorded about the environment, because it shaped the survey.** arxiv, JAIR,
Dagstuhl and Semantic Scholar are all blocked by this session's egress proxy;
only GitHub is reachable. Everything above is read out of source and repo
documentation, not the papers. The rule statements should be checked against
Blake & Gent by someone who can open it before either port is called proven.

## 2026-09-15 — The paper answers back: two decks are outside its proof, and a different rule is the one to take

**Status:** firm. Supersedes the recommendation in *The worry-back dominance
exists and is published* (earlier today), which stands as a record of what was
believed from source code alone.

Gizmo retrieved Blake & Gent and answered the questions in
`docs/research/literature-questions.md`; the report is
`docs/reports/literature-brief-20260915T203648Z.md`. Two of this morning's
conclusions were wrong.

**Wrong 1: "port Keller's rule to two decks" was presented as licensed by the
paper. It is not.** The safe-foundation theorem excludes multiple decks in
terms:

> we assume that we do not have multiple decks: i.e. while the theorem applies
> to a single deck with eight different suits of two colours, it does not apply
> to a game with two copies of the standard deck. The occurrence of duplicate
> cards leads to potential edge cases that we do not consider in this proof.

Solvitaire's `if (rules.two_decks) return false;` is that boundary enforced, not
an engineering shortcut as the earlier entry guessed. The worry-back ban is a
corollary of the same theorem and inherits the restriction. **Unproven for two
decks, not disproven** — but a port would be an extension of the paper, not an
application of it, and must be argued as such.

**Wrong 2: the flag raised against our own shipped rule is withdrawn.** The
paper gives two ways to qualify. With worry-back allowed: opposite-colour
foundations within two ranks and the same-colour twin within three. With
worry-back suppressed, *additionally*: opposite-colour foundations within one
rank. Our `never_wanted_in_the_tableau` implements the second, generalised from
two suit foundations to our four slots, against foundations stored as top rank
with 0 for empty — which is the paper's convention. It is the published
condition, not a permissive variant of it. **No correction owed.**

It is however *weaker* than the published rule, because we implement only one of
the two disjuncts. Adding the first would prune more in the restricted arm, and
needs the twin condition proved for four slots the way the current one was.

**What we should take instead: the incomplete-pile dominance (Appendix B.2),
which the earlier survey missed entirely.** Unlike the safe-foundation theorem
it is deliberately generalised past a single deck — the paper's own worked
example is five identical decks. It says an incomplete built pile need only be
moved when the card above it is then built immediately to foundation.

Its hypotheses want an "indistinguishable" build policy, where two cards have
identical or disjoint build destinations and the same policy governs single-card
and group moves. **Gypsy appears to qualify precisely because of the permissive
variant `CLAUDE.md` forbids correcting away:** any alternating-colour sequence
moves as a unit, so group and single-card policies agree. The paper explicitly
excludes standard Spider for failing exactly that test — its singles move by any
suit, its groups only by one.

Two hypotheses are unchecked and are not to be assumed: whether a stock that
deals to every column counts as "a rule invalidating moves by constraints on
their order", and whether the proof hides a dependence on foundations being
irremovable. The first is the same hazard that killed two dominances on
2026-09-11 and deserves the same suspicion.

**Answered, and it closes a `DESIGN.md` open question: no published Gypsy
figure exists.** A clean negative across the paper, the solver presets and a
bibliographic search — no peer-reviewed, preprint or solver-source winnability
figure for Gypsy or Gipsy. The one hit is a 2019 WPI report that generates the
game rather than measuring it, and is explicitly not peer reviewed. The close
relatives are genuinely distinct games and their figures do not transfer.

**Calibration, which is the sobering part.** The paper reports two two-deck
games. Mrs Mop resolves to 0.12% unknown over two million instances, so two
decks alone do not make a game intractable. Spider — two decks *and* a stock
dealing to every column, which is Gypsy's mechanic — is their worst resolved
fraction by a factor of twenty, at 2.69% unknown over ten thousand. Gypsy is in
the harder family and additionally cannot use the foundation dominances.

**Recorded about method, because it cost a day.** The first survey was built
from solver source because the papers were unreachable, and source is precise
about *what* a rule is while silent about *what it assumes*. Both errors above
are of that kind: a threshold read correctly, a hypothesis invisible. Where a
proof boundary matters, read the proof.

## 2026-09-16 — Measured: the incomplete-pile rule would remove 42% of Gypsy's moves

**Status:** firm as a measurement. Its "not implemented" note is superseded by
*A run is not split to expose a dead card* below, which implements the rule
behind both gates proposed here.

Blake & Gent's Theorem 4 (Appendix B.2) restricts moving an incomplete built
pile to the case where the card it exposes is then built to foundation. Unlike
their safe-foundation theorem it is deliberately generalised past a single deck.
Before proving it for Gypsy, the cheap question: would it cut anything?

**Method.** `cli/src/bin/branching.rs`, a diagnostic binary. Positions are
sampled along the search's own first descent — always the first action in
`legal_actions` order reaching an unvisited position — because that is where the
budget goes. Ten seeds, 50,000 steps each. A move counts as *partial* when it
carries a strict suffix of the built run, so whole-run and whole-pile moves are
never affected. *Removable* means the card it would expose has no foundation to
go to, which is the weaker form Solvitaire implements.

| Arm | Positions | Moves | Partial | Removable | + onto-card | + stock-empty | Both gates |
|---|---|---|---|---|---|---|---|
| full (worry-back) | 44,098 | 598,978 | 55.0% | **49.7%** | 46.2% | 42.2% | **42.1%** |
| no-worry-back | 5,544 | 65,245 | 73.5% | 73.4% | 41.5% | 4.7% | 4.2% |

**Half the full arm's move generation is partial-group moves that expose a dead
card**, and 42% survives both conservative gates below. At a mean branching
factor of 13.6 that compounds hard, and it is the first cut of this size this
project has found for the worry-back game.

The restricted arm's row is not comparable: its walks die after 5,544 positions
against the full arm's 44,098, so they rarely reach an empty stock and the
stock-empty column is a sampling artefact rather than a property of the game.

**Two proof obligations, and both have a conservative gate that appears to
discharge them.** Neither is optional — `CLAUDE.md` requires the proof before
the rule, and the two dead dominances of 2026-09-14 died on exactly this kind of
unexamined hypothesis.

1. **Empty columns may break the "indistinguishable build policy" hypothesis.**
   The condition is that any two cards have identical or disjoint sets of places
   they can move to. Gypsy lets *any* card go to an empty column, so every
   card's set overlaps every other's — unless empty columns are read as a
   separate spaces policy rather than part of the build policy. The proof never
   mentions empty piles, and its invariant is phrased entirely in terms of two
   *cards* whose sub-piles are swapped. Evidence that the authors did not face
   this: of the nine Solvitaire presets using the rule, every one restricts
   spaces to kings or auto-fills them, and none is two-deck. **Gate: restrict
   only moves onto a non-empty column.** Then the last non-compliant move the
   proof rewrites always has a card as its destination, which is what the
   invariant needs. Costs 3.5 points of the 49.7%.

2. **The stock may violate "no rules invalidating moves by constraints on their
   order".** Gypsy's stock deals one card to every column, so a deal interleaved
   into the proof's rewritten suffix lands cards on both affected piles and the
   invariant's "everything else identical" is no longer obvious. **Gate: apply
   the rule only once the stock is empty.** The stock never refills, so from a
   stock-empty position the remaining game contains no stock move at all and is
   an instance of exactly the game Theorem 4 covers. Costs a further 4 points.

Both gates together leave **42.1%**, so conservatism is nearly free here. That
is the trade to take: the ungated rule buys 7.6 more points and would rest on
two hypotheses nobody has checked for this game.

**Not yet argued, and the next piece of work.** That both gates *appear* to
discharge their obligations is a sketch, not a proof. The gap is that Theorem 4
is a statement about a whole instance, while we would apply it as a per-position
move-generation filter under a transposition table — the same composition
hazard `lonelybot` spends 569 lines on. The write-up owes: why restricting only
at stock-empty positions preserves solvability from those positions, and why
requiring compliance only of card-destination moves leaves the proof's case
analysis intact.

**Recorded because it is the reason to be careful:** this is the third rule this
project has measured as promising. Two of the previous three were unsound.

## 2026-09-16 — A run is not split to expose a dead card: the full game's first dominance

**Status:** firm.

A tableau move carrying a strict suffix of a built run is not generated when
the card it would expose has no foundation to go to. This is Blake & Gent's
Theorem 4 (JAIR 85, Appendix B.2) in the weaker form Solvitaire implements —
require only that the exposed card *could* be built, not that the next move
builds it.

**It is the first dominance this project has for the worry-back game**, and it
is the largest cut found so far by a wide margin.

**Why it reaches Gypsy when the safe-foundation rule does not.** That theorem
excludes multiple decks in terms. This one is *deliberately* generalised past a
single deck — the paper's worked example is five identical decks. Its condition
is an "indistinguishable" build policy: any two cards have identical or disjoint
sets of cards they can be built on, and one policy governs both single cards and
groups. Alternating colour gives the first. The second is **the permissive
variant `CLAUDE.md` forbids correcting toward the textbook rules** — any
alternating-colour sequence moves as a unit, exactly as a single card does.
Standard Spider fails precisely there and the paper excludes it by name. The
rule is available to us because of the ruleset John plays.

**Two gates, both ours, both load-bearing.**

*Gate 1, the stock must be empty.* The proof rewrites a winning line by
deleting, swapping and redirecting moves, which needs unrelated neighbouring
moves to be swappable. Gypsy's stock deal is never unrelated: it lands a card on
every column, so a tableau move swapped past one finds its run buried. That is
the theorem's move-order hypothesis failing. Gated on an empty stock it cannot,
because nothing returns cards to the stock: from such a position no continuation
holds a deal at all, and the remaining game is an instance of exactly what the
theorem covers. Positions with cards still to deal are left alone.

*Gate 2, the destination must not be an empty column.* Gypsy admits any card to
an empty column. The proof's critical step replaces a move onto one card with
the same move onto another and argues legality from the two cards accepting the
same set — an argument about cards, which an empty column is not. Restricting
only moves that land on a card means the move the proof rewrites always has a
card as its destination, as does the move onto the pile it vacated, since a
partial move leaves that pile non-empty.

Klondike needs neither gate and gets neither: its stock deals to the *waste*, so
a tableau move and a draw really are unrelated, and its empty piles take kings
only, so a king's build destinations are empty and disjoint from every other
card's. Two implementations again, for the same reason as safe autoplay.

**Measured, and the case for the gates is that they are nearly free:** 49.7% of
the full Gypsy arm's generated moves split a run for nothing; 42.1% still do
with both gates. See the entry above.

**Klondike, 50 deals, 3M, table 256 MiB.** Baselines are the recorded
`klondike-probe-3M-50deals` and `klondike-nwb-3M-50deals-autoplay`; the new runs
are `klondike-{full,nwb}-3M-50deals-splitrun`.

| Arm | | Solvable | Unsolvable | Unknown | Nodes on deals decided in both |
|---|---|---|---|---|---|
| full | without | 24 | 9 | 17 | |
| full | with | **31** | **10** | **9** | **-82.1%** |
| restricted | without | 26 | 10 | 14 | |
| restricted | with | **31** | **12** | **7** | **-71.8%** |

**No verdict contradicted in either arm and none regressed.** Eight deals newly
decided in the full arm, seven in the restricted. Most telling:
proven-unsolvable counts went **up**, 9 to 10 and 10 to 12, and not one
existing proof was overturned. That is the direction a wrong dominance fails
in, on the only deal set this project has that refutes anything.

The full arm's unknown bucket falls from 34% to 18% **at 3M** — better than the
22% the old search needed a 48M budget to reach. 81.945% stays inside the
bracket, now 40.6 points wide against 65.8.

**Gypsy, 50 deals, 5M, table 512 MiB**, against `gypsy-both-5M-50deals`:

| Arm | Without | With |
|---|---|---|
| no-worry-back | 12 solvable, 38 unknown | **28 solvable, 22 unknown** |
| full | 12 solvable (all carried), 38 unknown | **29 solvable, 21 unknown** |

No verdict contradicted, none regressed.

**And the result this project has been waiting for: the full arm solved a deal
by itself.** Seed 15, 68,592 nodes, a deal the restricted arm does not solve.
Before this the full Gypsy arm had resolved *nothing* — 0 of 50 at the same
budget. Stated precisely: seed 15 is `unknown` in the restricted arm rather than
proven unsolvable, so this is not yet proof that the deal needs worry-back. It
is the first deal the worry-back search has ever cracked on its own.

**A line-length note, and `DESIGN.md` is amended for it.** Every Klondike line
this rule touched got shorter, one from 1,351 moves to 231. The old bar asked
for identical line lengths, which was written for safe autoplay — a rule that
*forces* a move the ordering already took first, so the descent barely moves.
A rule that *removes* half the move set sends the search elsewhere, and every
line is still replayed from the deal before it is believed. Shorter lines are
evidence the search stopped wandering, which is exactly what the 99,982-move
lines of 2026-09-15 said it was doing.

**What is still owed.** The gates are argued here and in the code, not proved to
the standard of the safe-autoplay entry: each is a reason the theorem's proof
survives, not a re-run of its case analysis. The honest status is that the
measured evidence is strong — two arms, no contradictions, three new unsolvable
proofs — and the argument is a sketch. If a later run contradicts a verdict,
this entry is where to look first.

## 2026-09-16 — Re-swept on the current search: the curve moved down, not round, and the wall is now memory

**Status:** firm as a measurement. Supersedes the extrapolation in *Re-measured:
the search does converge with budget, far too slowly* (2026-09-14), whose 0.804
slope was taken before either dominance existed. `DESIGN.md` asked for this
re-sweep before anyone planned around that slope.

**Method.** The same 50 Klondike deals, full arm, at 3M, 12M and 48M nodes, with
the table held at 5.6 slots per node of budget — 256 MiB, 1 GiB, 4 GiB — so that
what varies between levels is the budget and not table pressure. That protocol
is checked rather than assumed this time: peak fill was **17.9% of capacity at
both upper levels**, so no level was evicting. Raw results in
`docs/results/klondike-sweep-{12M,48M}-50deals-splitrun.jsonl`; the 3M level is
the recorded `klondike-full-3M-50deals-splitrun.jsonl`, which the re-run
reproduced exactly — verdict, node count and line length on all 50, on different
hardware again.

| Budget | Table | Solvable | Unsolvable | Unknown | Bracket |
|---|---|---|---|---|---|
| 3M | 256 MiB | 31 | 10 | 9 (18%) | 40.6 pts |
| 12M | 1 GiB | 32 | 10 | 8 (16%) | 38.6 pts |
| 48M | 4 GiB | 34 | 10 | 6 (12%) | 34.6 pts |

**The slope is unchanged. 0.817 per fourfold step against the old 0.804** —
0.889 then 0.750, and at n=50 those two steps are two and three deals, so the
difference from the old figure is noise. The dominance did not change the rate
at which budget buys verdicts; it lowered the curve. This is the second time a
large improvement has moved the curve down and left the slope alone, the table
rewrite of 2026-09-13 being the first, and it is now the expected shape rather
than a surprise.

**What that is worth, and it is a great deal.** Reaching 5% unknown from 12%
needs 4.3 further fourfold steps, about **400x the 48M budget, 1.9x10^10 nodes
per deal**. The old sweep put the same gate at 4x10^4 times *its* 48M budget,
2x10^12 nodes per deal. Two orders of magnitude closer for one dominance. The 1%
publishing gate needs 2.4x10^7 times the 48M budget and is not worth costing.

**But the wall has changed kind, and this is the finding.** At the 176k nodes
per second measured here, 1.9x10^10 nodes is 33 CPU-hours for one hard deal and
about 4.4 CPU-days for a 50-deal set — extrapolating total nodes, which grow as
budget^0.862 across this sweep, not as the budget. Roughly 88 CPU-days for a
thousand deals, three weeks of wall clock at four workers. That is large but it
is no longer absurd; time is not what rules it out.

**Memory is.** At the protocol this sweep uses, a 1.9x10^10-node budget wants
1.1x10^11 slots, **1.7 TB of table per worker**. Run it at 4 GiB instead and the
level is not on this curve at all: the table holds 2.7x10^8 slots, the search
would be evicting by a factor of seventy, and the sweep would be measuring table
pressure — which is exactly what the protocol exists to exclude, and which
2026-09-15 recorded as costing re-expansions in both directions. **So the
extrapolation breaks its own conditions long before it reaches the gate.** The
honest statement is not "400x the budget reaches 5%"; it is that on affordable
memory nothing on this curve is measurable much past 10^9 nodes per deal, and
the gate sits an order of magnitude beyond that.

**And this sweep was already unaffordable on the box it is meant for.** The 48M
level needs a 4 GiB table per worker; fritter.lol has about 4.9 GiB available in
total. It ran here, on a 15 GiB container, at two workers. A budget-led route to
the gate would need the machine before it needed the patience.

**No verdict contradicted and none regressed**, in either direction, across all
three levels. Every unknown at every level was stopped by the node budget; the
stack guard was never reached. Proven-unsolvable held at 10 of 50 throughout —
still 20%, still above Klondike's true 18.06% ceiling, still inside the Wilson
interval at n=50 and still unable to discriminate until n=1000. That warning
from 2026-09-14 stands unchanged.

**Gypsy, the same three levels, both arms**, table and worker counts as above.
Raw results in `docs/results/gypsy-both-{3M,12M}-50deals-splitrun.jsonl`, with
the recorded 5M run shown for continuity.

| Budget | restricted | full | full arm's own wins |
|---|---|---|---|
| 3M | 25 solvable, 25 unknown | 26, 24 | 1 |
| 5M | 28, 22 | 29, 21 | 1 |
| 12M | 31, 19 | 31, 19 | **0** |

**The full arm still resolves nothing it is asked to search.** The zero at 12M is
not a regression and the ones above it are not progress: under `--both-arms` the
restricted arm goes first and its wins are carried, so the full arm only ever
searches the deals the restricted arm failed. Seed 15 — the one deal the full
arm has ever cracked by itself — is solved by the restricted arm at 12M, so the
full arm never saw it. On the 19 deals it did search at 12M it spent every node
of its budget, 228M in total, and decided none of them. A fourfold budget step
has never yet taken a deal off that list.

**Zero Gypsy deals are proven unsolvable at any budget, in either arm.** That is
the same hole 2026-09-15 recorded: the Gypsy set cannot catch a dominance that
discards a winning line, because it proves nothing in the direction that error
shows up in. Klondike remains the only set with teeth.

**What this settles.** Budget is not a route to the gate, and it is not a route
to the headline figure either. The 0.804 slope should not be quoted again; the
0.817 that replaces it says the same thing, from a curve that is two orders of
magnitude lower and against a ceiling that is now memory rather than time.
Dominances remain the critical path, exactly as 2026-09-12 set it and for the
third time with a different reason.
