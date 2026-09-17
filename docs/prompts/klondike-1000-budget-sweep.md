# Gizmo request: the thousand-deal budget sweep, 2026-09-17

The thousand-deal validation gave one point on the budget curve — 8.1% unknown
at 12M — against a slope measured on a fifty-deal sample that turned out to be
twice as hard as the population. The extrapolated distance to the 5% gate rests
entirely on that borrowed slope. This run replaces it with three measured points
at the real sample size.

It is affordable because the first run was: fourteen minutes of wall clock for a
thousand deals at 12M, where the plan had assumed days.

The request as sent follows.

---

**Sweep the node budget over a thousand Klondike deals for `gypsy-solve`.**

Build `main` and run `gypsy batch --game klondike` over **seeds 0–999** three
times, at **3M, 12M and 48M** nodes per deal. Everything else identical.

The output is a curve: the unknown fraction at each budget, and the factor it
falls by per fourfold step. That factor is what says how far the 5% gate is, and
right now it is borrowed from a fifty-deal sample rather than measured.

## The parameters, and why each one

**Full ruleset, `--restarts 1`.** The full arm is the one with a published
figure to check against. Restarts are measured as a loss on Klondike — at a
thousand deals they cost 29 refutations and pushed 8.1% unknown up to 12.6% —
so the sweep runs the policy that won.

**One table size for all three budgets: 2048 MiB per worker.** Not the
budget-scaled table the earlier sweeps used. Two reasons, and they pull the same
way. A table that grows with the budget makes each point a measurement of two
things at once, and the curve is supposed to isolate one. And it is not needed:
the 12M thousand-deal run peaked at **18% occupancy of a 1,024 MiB table**, so
nothing was evicting; 2,048 MiB holds 48M budget's worth of positions at about a
third full, so none of the three points evict either.

**If it does not fit, reduce workers, not the table.** A smaller table would
make the top of the sweep a measurement of eviction, which is the one thing this
run exists to avoid. Two workers at this table is expected to fit; the runner
refuses rather than starting a run that will not.

**Seeds 0–999, and the 12M point is a check on the other two.** That run already
exists — `docs/results/klondike-full-12M-1000deals-restart1.jsonl` — and the
full Klondike arm has not changed since, so the 12M point must come back with
**the same verdict on every seed**. Node counts should match too: neither table
size evicts, so the search is the same search. **If any verdict differs, stop
and report** rather than running the rest.

One thing has changed in the repo since that file: a dominance called safe
autoplay was corrected, which made the search slightly weaker. It fires **only
with worry-back suppressed**, so this run's arm is untouched — the Klondike full
arm reproduces its recorded results to the node. Nothing here needs adjusting
for it; it is noted so a difference, if one appears, is not mistaken for noise.

## Sizing

The 12M thousand-deal run cost about 3,780 seconds of aggregate per-deal time
and fourteen minutes of wall clock on four workers. 3M is cheaper, 48M is
roughly four times 12M in the worst case, and most deals finish far under
budget. At two workers expect **a few hours in total, not a day**. If 48M alone
is tracking past about six hours, say so rather than letting it run.

## What to report

Markdown, timestamped, plus the three results files.

- **The curve.** Unknown fraction at 3M, 12M and 48M, and the ratio between
  consecutive points. The fifty-deal sample gave 0.817 per fourfold step; the
  question is whether a thousand deals agree.
- From that ratio, the budget the curve puts the **5%** and **1%** thresholds
  at, stated as an extrapolation rather than a measurement.
- Solvable / unsolvable / unknown at each budget, and the validation bracket
  each file produces against the published figure. The repo's analysis script
  does the bracket; run it per file, never pooled.
- **Peak table occupancy at each budget**, as a fraction of capacity. It is the
  evidence that the curve measures budget rather than eviction, and if 48M comes
  back much above half full, say so — the top point is then suspect.
- Wall clock and aggregate per-deal time at each budget, and the worker count
  and table size actually used.
- **The seeds still unknown at 48M** — the list. That is the hard core, and it
  is the working set for whatever comes next.
- Anything that had to differ, and why.

Name the files `klondike-full-{3M,12M,48M}-1000deals-restart1.jsonl`. That box's
root filesystem has been near full and is RAID0 with no redundancy, so get the
results off it; the report comes back either way.

## Not part of this

- **Do not start a Gypsy batch.** It is still gated on the Klondike unknown
  bucket, and 8.1% is not 5%.
- Do not tune anything to improve the curve. A slope that says the gate is far
  away is the useful answer if it is the true one.
