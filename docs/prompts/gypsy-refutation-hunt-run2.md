# Gizmo request: widen the refutation hunt, 2026-09-21

Run 1 worked. Seed 188 is unwinnable — verified with every dominance removed in
turn and with all of them removed at once, in both arms — and it is the first
thing this project has ever proved about Gypsy in the losing direction.

**Run 2 is not the 48M run the original task named**, and the reason is Run 1's
own cost data. Refutations come from jammed deals with small reachable games:
seed 188's restricted arm exhausted in 4.2M nodes against a 12M budget. Raising
the budget mostly buys *wins* on large winnable deals, which is expensive and is
not what this hunt is for. On the numbers, four thousand more deals at 12M costs
about ten hours and should yield about four more refutations; the same ten
hours spent quadrupling the budget on the deals already done buys an unknown and
probably smaller number. More deals, same budget.

It also gives the thing a single refutation cannot: a **rate**, over five
thousand deals, which is what turns "below 100%" into an upper bound.

The request as sent follows.

---

**Widen the Gypsy refutation hunt: four thousand more deals at the same budget.**

Build `main` and run `gypsy batch --game gypsy` over **seeds 1000–4999** with
**`--restarts 1`**, `--both-arms`, **1,024 MiB** table per worker, four workers,
12,000,000 nodes per deal.

Every parameter except the seed range is exactly Run 1's, so the two runs
combine into one five-thousand-deal sample.

**The build:** `main` is the right thing to build. There is an unmerged branch
carrying a regression test and documentation from the Run 1 verification; it
changes no solver, core or CLI source, so it cannot affect a result either way.
If `main` has moved on since, build it anyway and say what it is.

## First, a thirty-second canary

Before the batch, solve **seed 188** alone: restricted arm, `--restarts 1`, 12M
budget, 1,024 MiB table.

It must come back **`unsolvable` in 4,203,474 nodes**. That deal has been proved
unwinnable four independent ways and its verdict is now a fixed point.

**If it comes back `solvable`, stop immediately and report.** That would mean a
dominance is discarding winning lines — the most serious failure this solver can
have, and the reason the deal is worth keeping. If it comes back `unknown`, or
with a different node count, something about the build or the machine differs
from Run 1 and the batch is not worth starting until that is understood.

## What is being hunted

**More refutations, to get a rate.** One in a thousand is a fact about one deal.
Five thousand deals gives a denominator, and the count is what bounds Gypsy's
winnability from above for the first time.

**And the worry-back delta, which is now findable.** A restricted `unsolvable`
does **not** carry to the full arm — only a full `unsolvable` carries down — so
every deal the restricted arm refutes is searched independently with worry-back
legal. **A deal that is restricted-`unsolvable` and full-`solvable` is a proof
that worry-back changes winnability**, which is this project's second question
and which every measurement so far has failed to produce. Seed 188 is unwinnable
both ways and so is not one. Report any such pair prominently; it is the more
valuable of the two findings.

## Verifying a refutation — read this before concluding anything

Re-solve each refuted seed with **one** dominance removed at a time: safe
autoplay, the safe-foundation forcing rule, the split-run filter.

**Two of the three outcomes are not failures.**

- Still `unsolvable` → survives that removal. Good.
- **`solvable`** → that rule discarded a winning line. **The only outcome that
  shows a rule failing**, and it matters more than the refutation.
- **`unknown` → inconclusive, not a failure.** Removing a dominance makes the
  tree bigger, and a search that exhausted the smaller tree will not
  necessarily exhaust the larger one on the same budget. **Raise the budget and
  re-run that variant until it decides.**

This is exactly where Run 1 went wrong, on an instruction that was ours rather
than yours. Removing safe autoplay took seed 188 from 4.2M nodes to **226M** — a
factor of 54 — so at 12M it returned a budget-limited `unknown`, and the task as
written called that a failed dominance and a void refutation. Both were wrong.
Expect to need one to two orders of magnitude more budget on these re-runs, and
pass **powers of two** to `--table-mib`: the table rounds its entry count up to
a power of two, so 10,240 MiB asks the allocator for 16 GiB.

## What to report

- **Every `unsolvable`**: seed, arm, and the node count it exhausted in. The
  node counts are a distribution nobody has — they say how expensive a Gypsy
  refutation is, and Run 1 has exactly one sample.
- **The refutation rate over the combined five thousand deals**, restricted and
  full separately.
- Any restricted-`unsolvable` with full-`solvable`, per above.
- Solvable / unsolvable / unknown per arm, and the unknown fraction against Run
  1's 48.3% restricted and 47.6% full — these seeds are new, so a large
  difference is itself worth knowing.
- The unresolved seed lists, taken from the results file.
- Peak table occupancy, expected around 18%.
- Wall clock, aggregate per-deal time, workers and table used.

Name the file `gypsy-both-12M-seeds1000-4999-contiguous.jsonl`. Get it off that
box.

## Sizing

Run 1 was a thousand deals in **2h42m** on four workers at these settings. Four
thousand deals is about **eleven hours**. **If it passes twenty-four, stop and
report.**

The runner writes each deal as it finishes and `--resume` skips what is already
recorded, so a kill costs one deal.

## Not part of this

- **The 48M run is deferred, not cancelled.** It is the right way to resolve
  more deals; it is the wrong way to find more refutations, which is what this
  round is for.
- No figure is published from this and none should be quoted as one. The unknown
  bucket is near half.
