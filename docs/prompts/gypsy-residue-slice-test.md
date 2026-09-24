# Gizmo request: bigger slices on the Gypsy residue, 2026-09-24

**Re-solve the 93 deals still unknown after the 192M / 128-restart run. Same
192M total budget, but split into 32 slices of 6M instead of 128 slices of
1.5M. Both arms.**

## Why

The slope pin showed more restarts at 1.5M nodes no longer help. Over restarts
97–128 the chance that one more restart solves a still-unknown deal was about
0.07%. The 93 deals left are ones where a 1.5M search almost never finds a
win. They are a mix of two kinds of deal, and the next step on the solver
depends on which kind dominates:

- **Winnable, but the win needs a longer search than 1.5M nodes.** Bigger
  slices or a better move ordering would solve these.
- **Unwinnable.** Nothing short of an exhaustive search settles these, which
  means new pruning rules. Restarts can never prove a deal unwinnable.

This run tells them apart without any code change. Every one of these deals
has already failed 192M spent as 1.5M slices. **Any win found here came from
using the same budget in deeper slices.** If a lot of deals are solved, the
residue is mostly deep wins. If few are, it is mostly jammed.

This is a new search, not an extension of the last one. A 6M slice under a
given ordering is not the 1.5M slice under that ordering carried on, so no
earlier result replays here. Compare results, not node counts.

## The run

Build `main`. Its solver, core and CLI source are identical to `2d72ec5`, the
build every earlier Gypsy run used; only tests and documentation have been
added since. If `main` has moved on, build it anyway and say what it is.

`gypsy batch --game gypsy --both-arms`, `--budget 192000000`,
`--restarts 32`, `--table-mib 256`, four workers, default `--max-depth`. The
seeds are the restricted arm's 93 unknowns from
`gypsy-both-192M-restart128-48Munknowns.jsonl`:

```text
12, 23, 37, 56, 65, 71, 88, 105, 109, 112, 113, 129, 148, 150,
153, 157, 171, 185, 188, 191, 193, 200, 202, 250, 258, 259, 270, 272,
280, 301, 311, 312, 317, 324, 335, 361, 370, 372, 376, 389, 395, 399,
419, 437, 445, 450, 454, 465, 474, 484, 497, 508, 516, 523, 535, 536,
537, 550, 570, 588, 592, 594, 612, 613, 631, 632, 636, 640, 649, 657,
664, 667, 676, 691, 692, 694, 695, 701, 727, 730, 746, 787, 795, 818,
825, 832, 885, 916, 928, 934, 948, 952, 999
```

Check the list against the results file rather than trusting this copy. The
full arm's 82 unknowns are a subset of these 93. `--both-arms` searches the
full game only where the restricted arm fails, so eleven of these deals
(already solved in the full arm, not in the restricted arm) will be solved
again in the full arm. That is expected.

**The table stays at 256 MiB, as in every earlier survey.** A 6M slice fills
at most about 36% of it, which is well under the point where displacement
starts to matter. Don't raise it. Keeping memory per worker unchanged means
the slice size is the only thing that differs.

As before, `gypsy batch` takes a contiguous range, not a list. Run it however
you like, but **do not change the solver or the CLI to do it.**

## Canary

**Seed 188, restricted arm**, before the batch. It is proved unwinnable, and
its restricted search exhausts in 4,203,474 nodes, inside one 6M slice. It
must come back **`unsolvable` on restart 1**. If it comes back `solvable`,
stop and report: that means a pruning rule is discarding winning lines. If it
comes back `unknown`, stop and report, because something differs from the
build earlier runs used. The 4,203,474 figure was measured on a 1,024 MiB
table, so on 256 MiB a slightly different node count is possible. If
`unsolvable` comes back with a different count, note it and go ahead.

Seed 188's full arm needs 19.8M nodes to exhaust, more than one slice holds,
so it will come back `unknown` in the full arm. That is correct.

## What to report

- **Deals solved, per arm, out of 93 restricted and 82 full.** This is the
  headline.
- **For each win: `restarts_used`, and the nodes spent in the winning
  slice**, which is `nodes − (restarts_used − 1) × 6,000,000`. This is the
  measurement that matters most. A win found after more than 1.5M nodes into
  its slice is one no 1.5M slice could have reached from that ordering.
  Please give the distribution: how many wins came before 1.5M into the slice,
  how many between 1.5M and 3M, and how many after 3M.
- Any `unsolvable` besides seed 188's restricted arm. A 6M slice can exhaust a
  small unwinnable deal, but the earlier 12M contiguous run on these seeds
  refuted only 188, so another would be a surprise and needs verifying the
  usual way. Re-solve with one pruning rule removed at a time. Only
  `solvable` shows a rule failing; `unknown` means raise the budget.
- Any deal where the arms disagree, in either direction.
- The seeds still unknown, per arm, taken from the results file.
- Peak table occupancy (expected at most about 36%), wall clock, aggregate
  per-deal time, workers.

Name the results file `gypsy-both-192M-restart32-slice6M-residue93.jsonl`, and
get it off that box.

## Sizing

The slope-pin run took 6h35m for 125 deals at the same total budget, and most
of these 93 will use their whole budget. Expect five to six hours. **If it
passes twenty-four hours, stop and report what is done.** `--resume` makes a
kill cost one deal.

## Not part of this

- No figure is published from this, and don't quote one.
- Don't try other slice sizes or restart counts in the same run. One change
  from the last run, so the result has one explanation. If this one points
  somewhere, the next slice size is a separate request.
