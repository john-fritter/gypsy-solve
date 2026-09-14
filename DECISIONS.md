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
