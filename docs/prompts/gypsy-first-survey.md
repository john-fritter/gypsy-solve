# Gizmo request: the first Gypsy survey, 2026-09-18

The Klondike gate cleared at 64M — 4.6% unknown on a thousand deals — so Gypsy
batch work can start. This is the first one, and it is a survey rather than the
production run.

Two things shape it. **Every Gypsy number this project has is measured on fifty
deals**, and the Klondike fifty-deal set turned out to be wrong about both the
height of its curve and its slope, in the same direction both times. And a
dominance that fires in the arm every Gypsy verdict comes from was found unsound
yesterday, so the run is blocked on that fix being merged.

One sizing fact that is not obvious and is easy to get backwards: a restart
run's table is sized by the **slice**, not the total budget, because each
restart is a fresh `solve` with its own table. Gypsy wants a small table and
gains nothing from a large one — the opposite of the Klondike sweep.

The request as sent follows.

---

**Run the first thousand-deal Gypsy survey for `gypsy-solve`.**

Three runs, in this order, each reported before the next begins.

## Before anything: check the build

`main` **must contain the commit titled "Drop safe autoplay's shortcut for
twos"**. If it does not, **stop and say so** — nothing below is worth running
without it.

That commit fixes a dominance that discards winning lines. It fires in the
restricted (no-worry-back) arm, which is the arm essentially every Gypsy verdict
comes from, because the full arm's results are carried from it. On the older
code the loss shows up as `unknown` rather than as a wrong answer, so a survey
run against it would understate Gypsy's winnability by a margin nobody can put a
number on.

## Run 1 — confirm the fix on Gypsy, 50 deals

Gypsy, **seeds 0–49**, 12M nodes, `--restarts 8`, `--both-arms`, **256 MiB**
table. Diff it against the recorded
`docs/results/gypsy-both-12M-50deals-restart8-safefoundation.jsonl`.

The bar is **no verdict contradicted**: nothing that was `solvable` may become
`unknown` or `unsolvable`, and nothing that was `unknown` may become
`unsolvable`. An `unknown` becoming `solvable` is a gain, not a failure. Node
counts will move and that is expected — the fix removes a rule that fired
constantly.

**If any verdict is contradicted, stop and report.** That is a much more
interesting result than the survey and it changes what should be run next.

This costs about twenty minutes. The same comparison at 3M found nothing changed
and nodes up 1.0%; 12M with restarts resolves far more deals, so it has more
room to differ, which is why it is worth doing before the long runs.

## Runs 2 and 3 — the survey, 1,000 deals

Gypsy, **seeds 0–999**, `--both-arms`, 256 MiB table, at both of the tuned
configurations:

| Run | Budget | Restarts | Slice |
|---|---:|---:|---:|
| 2 | 12,000,000 | 8 | 1.5M |
| 3 | 48,000,000 | 32 | 1.5M |

**Both arms, in one run each, not two runs per budget.** `--both-arms` solves
each deal in both rulesets and carries each arm's proof to the other where that
is sound, which is cheaper than two runs and never weaker.

**Why two budgets rather than one.** The only Gypsy numbers that exist are on
fifty deals, and the equivalent Klondike sample was twice as hard as its
population — it got the height of the curve wrong, and then the slope wrong too.
Two points give a height and a first slope together, and the slice size is 1.5M
in both, so the only thing changing between them is how many slices.

**Why 256 MiB and not more.** Each restart is a separate search with its own
table, so the table only ever holds one slice's worth of positions — about 1.5M
against 16.8M entries, under 10% full, whatever the total budget. A larger table
would change nothing. This is the reverse of the Klondike sweep, where the table
was the binding constraint.

**Workers: four.** At this table size a worker costs roughly 650 MiB including
its search stack, so four fit comfortably. The runner refuses to start a run
that will not fit.

## Sizing

On the recorded fifty-deal throughput, a thousand deals should cost roughly 7.4
aggregate hours at 12M and 15.4 at 48M — **under a day on four workers for
both**. That assumes the population resembles seeds 0–49, which is the
assumption the run exists to test, so treat it as a floor rather than an
estimate. **If either run is tracking past two days, stop and report.**

The runner writes each deal as it finishes and `--resume` skips what is already
recorded, so a kill costs one deal.

## What to report

Markdown, timestamped, plus the results files.

- **Unknown fraction per arm, per budget.** The headline. Compare it against the
  fifty-deal figures — 6 of 50 unknown at 12M and 4 of 50 at 48M — and say
  plainly whether the small sample was representative.
- **The worry-back delta, which is the finding this project exists for after
  winnability itself.** Count the deals where the two arms disagree, in both
  directions: restricted `unknown` with full `solvable`, and anything else. On
  fifty deals this was one deal at 12M and none at 48M. A thousand deals is the
  first honest look at it.
- **How many full-arm verdicts came from `search` and how many were carried.**
  The `verdict_from` field says. A full arm that still resolves nothing of its
  own is a different situation from one that has started to.
- Solvable / unsolvable / unknown counts per arm, and whether any Gypsy deal at
  thirteen ranks is proved **unsolvable** — none ever has been, and the first
  one would matter.
- The unresolved seed lists, **taken from the results files**, per arm per
  budget. Those are the working set.
- Peak table occupancy — expected under 10%, and worth saying if it is not.
- Wall clock, aggregate per-deal time, workers and table used.

Name the files `gypsy-both-{12M-restart8,48M-restart32}-1000deals.jsonl`, and
run 1's `gypsy-both-12M-50deals-restart8-twofix.jsonl`. Get them off that box.

## Not part of this

- **No figure is published from this run and none should be quoted as one.**
  It is a survey to find out where Gypsy stands; the unknown bucket will be far
  wider than any interval worth publishing.
- Do not tune the solver, the budgets or the restart counts to improve the
  numbers. The point is to learn the size of the problem.
