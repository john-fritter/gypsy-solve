# Gizmo request: re-prove the unwinnable Gypsy deals with no shortcuts, 2026-10-01

**Run four long, single-deal searches of the `gypsy-solve` solver, one at a
time, and commit the four result lines. Expect hours per run.**

## Why

Five Gypsy deals have been proved unwinnable: seeds 188, 3796, 3966, 4260 and
4617. The solver proves that faster by skipping moves that its pruning rules
("dominances") say can never matter. A wrong rule would make a winnable deal
look unwinnable, so each of these five verdicts was re-checked with each rule
removed on its own. The writeup now claims more than that: the verdicts hold
with **every rule removed at once**, on the bare rules of the game. Three of
the five have been run that way. The remaining runs are too long for the
environment they were started in, because it restarts and kills them.

Already done, so don't repeat them: seed 4617 and seed 3796 with worry-back,
and seed 3966 without worry-back.

## The build

Build the `rule_free` binary (`cli/src/bin/rule_free.rs`) from `main`. If
`main` doesn't have that file yet, build branch `claude/adoring-fermat-fidy7o`
instead, and say which commit you built.

`rule_free` solves one deal using no pruning rule at all. It prints one JSON
line when it finishes and nothing before then, so a long run gives no
progress output. That is expected.

## The runs, in this order, one at a time

| # | Seed | `--arm` | Known so far |
|---|---:|---|---|
| 1 | 4260 | `no-worry-back` | ran past 18 minutes on a 4 GiB table before a restart killed it |
| 2 | 4260 | `full` | ran past 1h45m on an 8 GiB table before a restart killed it |
| 3 | 3966 | `full` | not tried; with the rules on it is the largest of the five |
| 4 | 188 | `full` | done once on 2026-09-20: `unsolvable` in 1,076,602,384 nodes, using a temporary patched build that left no result file. This run puts it on record with the committed tool |

Every run: `--budget 40000000000` and `--table-mib` set as below.

**Table size.** Use the largest power of two in MiB that fits in memory the
box can spare beside its production services, and use the same size for all
four runs. A non-power of two is silently rounded up, so 6,000 MiB would
allocate 8 GiB. The table will fill up completely. That is expected and
doesn't affect correctness: when the table is full, the search expands a
displaced position again instead of skipping it. A smaller table makes the run
slower, never wrong. For scale, seed 3796 with worry-back took 1.18 billion
nodes in 41 minutes on an 8 GiB table, and the table was full.

**One run at a time.** Each run uses one core and the whole table, so two at
once would halve the table each one gets.

## Reading the result

- `"verdict":"unsolvable"` with `"limit":"none"`: **confirmed.** This is the
  expected outcome.
- `"verdict":"solvable"`: **stop everything and report at once.** That means
  one of the solver's rules was discarding a winning line, which would
  invalidate published results. The run is deterministic, so the line can be
  regenerated here; you don't need to capture it.
- `"verdict":"unknown"` with `"limit":"budget"`: inconclusive, not a failure.
  Report it with the node count, and go on to the next run.
- **Any single run past 36 hours**: kill it, record it as stopped by wall
  clock with the elapsed time, and go on. If the whole job passes four days,
  stop and report what is done.

## Where results go

Append each run's JSON line, exactly as printed, to
`docs/results/gypsy-rule-free-refutations-fritter.jsonl` as soon as it
finishes, so a kill loses at most one run. When all four are done (or stopped),
commit that file on a new branch, `gizmo/rule-free-refutations`, and push.
Don't push to `main`. Plain git; the file is four lines.

## What to report

For each run: verdict, limit, nodes, elapsed time, table size and how full it
got. Also the build commit, and anything that stopped a run early.

## Not part of this

- Don't change the solver or `rule_free`.
- No other seeds and no other arms. One question per run.
- Don't interpret the results beyond the reading above.
