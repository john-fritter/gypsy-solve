# Gizmo request: pin the Gypsy budget slope at 192M, 2026-09-24

**Re-solve the 125 deals the 48M survey left unknown, at 192M nodes as 128
restarts. Both arms. Nothing else changes.**

## Why

The 2026-09-18 survey (seeds 0–999, `--both-arms`, 256 MiB table) gave 23.2%
restricted unknown at 12M / 8 restarts and 12.5% at 48M / 32 restarts. That is
a multiplier of about 0.54 per fourfold budget step. On that slope, getting to
1% unknown takes about four more steps, which is days of compute rather than a
research problem. But the slope comes from two points, and two points are
exactly what got the Klondike slope wrong earlier. This run is the third point.

The answer we want is **whether the unknown fraction keeps falling at about
0.54 or levels off.** Levelling off would mean the residue is deals that
restarts cannot settle. Restarts slice the budget into 1.5M searches and
cannot prove a full-size deal unwinnable, so a residue of large unwinnable
deals would show up as exactly that plateau.

## Why only the unknowns, and why the answer is still exact

`solve_restarting` splits the budget evenly and derives each restart's ordering
from the attempt index alone. So 192M / 128 is 1.5M per restart, the same as
48M / 32, and **restarts 1–32 of this run are the 48M run's search, node for
node**, on the same table size. A deal the 48M run resolved would be resolved
again on the same restart with the same node count, so it does not need
re-running. Re-running only the unknowns gives the same 1,000-deal result as
re-running everything.

That only holds with **the same table size**. Node counts depend on the table,
so a different `--table-mib` breaks the argument. Use 256 MiB, as the survey
did.

**It does not make the run cheap.** The unknowns were already the expensive
part of the survey, because each one used its whole budget. Skipping the
resolved deals saves a little. Expect roughly four times the survey's
unknown-deal cost: something like twelve hours on four workers. **If it passes
thirty-six hours, stop and report what is done.** `--resume` makes a kill cost
one deal.

## The run

Build `main`. Its solver, core and CLI source are identical to `2d72ec5`, the
survey's build; only tests have been added since. If `main` has moved on, build
it anyway and say what it is.

`gypsy batch --game gypsy --both-arms`, `--budget 192000000`, `--restarts 128`,
`--table-mib 256`, four workers, default `--max-depth`, over these seeds, which
are the restricted arm's 125 unknowns from `gypsy-both-48M-restart32-1000deals.jsonl`:

```text
12, 23, 32, 37, 56, 64, 65, 71, 85, 88, 103, 104, 105, 109,
112, 113, 129, 148, 150, 151, 153, 157, 170, 171, 185, 188, 191, 193,
200, 202, 233, 250, 258, 259, 267, 270, 272, 280, 301, 311, 312, 317,
324, 335, 351, 361, 370, 372, 376, 384, 388, 389, 395, 399, 419, 437,
445, 450, 454, 465, 474, 484, 497, 508, 516, 521, 523, 535, 536, 537,
549, 550, 564, 569, 570, 583, 588, 592, 594, 612, 613, 631, 632, 636,
640, 649, 657, 659, 660, 664, 667, 669, 676, 691, 692, 693, 694, 695,
701, 706, 727, 730, 746, 761, 780, 787, 795, 806, 818, 825, 832, 882,
885, 905, 916, 928, 930, 934, 941, 943, 948, 952, 970, 979, 999
```

Check the list against the results file rather than trusting this copy.

`gypsy batch` takes a contiguous range, not a list. How you get it to run
these seeds is up to you; one deal per invocation is fine. **Do not change the
solver or the CLI to do it.** The result must come from the same code as the
survey.

The full arm's 115 unknowns are a subset of these 125. `--both-arms` searches
the full game only where the restricted arm fails, and it carries restricted
wins across, so the ten deals the 48M run solved in the full arm alone will be
re-solved as a side effect.

## First, the check that the exactness argument holds

Before the batch, pick **three deals the 48M run solved with `restarts_used`
above 1**, preferably one above 16. Solve each at 192M / 128 / 256 MiB, both
arms. **Verdict, `nodes` and `restarts_used` must match the 48M record
exactly.** If they don't, the claim that restarts 1–32 replay the 48M run is
false, the unknowns-only shortcut is invalid, and the batch should not start.
Report what differed.

## What to report

- **Unknown count per arm at 192M, out of the 1,000 deals.** For each arm, that
  is the unknowns left from this run; everything else was resolved at 48M or
  now.
- **The step multiplier, 192M unknown ÷ 48M unknown, per arm**, next to the
  previous step's 0.539 (12.5% ÷ 23.2%). This is the headline.
- Confirm that every deal still unknown at 192M was unknown at 48M. That must
  hold by the argument above, so a violation is a finding in its own right.
- **For each newly resolved deal, `restarts_used`.** This shows whether wins
  show up evenly across restarts 33–128 or bunch up early. Even spacing means
  more restarts are what help. Early bunching with a long dry tail means more
  restarts have stopped helping.
- Any `unsolvable`. None is expected, because a 1.5M slice is far too small to
  exhaust a full-size deal. Seed 188, which is known to be unwinnable, is in the
  list and should come back `unknown`.
- Any deal where the arms disagree, in either direction.
- The remaining unknown seed lists per arm, taken from the results file.
- Peak table occupancy (expected about 8.9%, as before), wall clock, aggregate
  per-deal time, workers.

Name the results file `gypsy-both-192M-restart128-48Munknowns.jsonl`. Get it
off that box.

## Not part of this

- No figure is published from this, and don't quote the result as one.
- Don't tune budgets, restart counts or table size to improve the numbers.
  This is a measurement of the curve, and it only means something if it sits on
  the same curve as the first two points.
