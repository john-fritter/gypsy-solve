# Gizmo request: the 1,000-deal Klondike validation, 2026-09-16

Every winnability number in this repo is measured on **50 deals**. The gate that
decides when Gypsy batch work may start is defined on **1,000**, and nobody has
run it. This asks for that run.

Kept here because the sizing and the non-obvious choices — why 12M and not 48M,
why the table is not the knob to shrink, why both restart policies — are the
part worth recording, and because the next run of this kind should start from
this one rather than re-deriving it.

The request as sent follows.

---

**Run the 1,000-deal Klondike validation for `gypsy-solve`.**

Build `main` at `51a6bf0` and run `gypsy batch --game klondike` over **seeds
0–999**, twice: once at `--restarts 1`, once at `--restarts 8`. Everything else
identical between the two.

This is the run that tells us how far the solver is from the bar that gates the
whole project. It is not a tuning exercise and nothing about the solver should
be changed to improve the number.

## The parameters, and why each one

**Full ruleset only — not `--no-worry-back`.** The full arm is the one with a
published figure to check against: 81.945% ± 0.084% (Blake & Gent, JAIR 85), on
exactly the variant implemented — 24-card stock drawn three at a time, unlimited
redeals, worry-back permitted. The restricted arm matches no published number,
so running it here would double the compute for nothing.

**Budget 12M nodes per deal.** Higher resolves a little more — 48M leaves 12%
unknown against 16% at 12M on the 50-deal sample — but the protocol those sweeps
used scales the transposition table with the budget, and a table sized for 48M
does not fit on the box beside enough workers to be worth it. 12M is the largest
budget that runs at a table size the box can hold.

**Both restart policies, reported side by side.** Restarts are measured as worth
roughly four times the budget on Gypsy and worth nothing on Klondike. That means
Gypsy production will use them and Klondike validation would not — which would
leave us validating a configuration other than the one that publishes. Running
both closes that. Expect the restarts-on bucket to come out slightly larger;
that is the known cost and not a fault.

**Table 1024 MiB per worker. This is not the knob to turn.** If the run does not
fit, **reduce workers, not the table.** Table size changes which deals come back
`unknown`, so shrinking it to fit more workers in parallel would silently make
this run incomparable to everything already recorded — and the incomparability
would not show up anywhere in the output. If memory genuinely forces the issue,
512 MiB is the floor worth considering: deals at this budget filled only 18% of
a 1024 MiB table, so there is real headroom. A run at 512 MiB still has to clear
the check below before its numbers are used for anything.

**Seeds 0–999, and the first fifty are a built-in check.** The recorded 50-deal
runs used seeds 0–49 at this budget and table size, so under `--restarts 1`
those fifty must come back with **identical verdicts and identical node counts**.
If any of them differ, **stop and report it** rather than running the other 950.
A difference there means the box or the build differs from where those numbers
were measured, and everything downstream would inherit that.

The runner appends per deal and `--resume` skips what is already recorded, so
the answer to a kill is the same command again. It refuses to start when the run
would not fit in memory or on disk, and it has a disk floor it checks as it goes
— worth pointing the output somewhere with room rather than relying on that.

## Sizing, so an overrun is recognisable

Fifty deals at this budget cost about **0.14 CPU-hours** and 1.1×10⁸ nodes. So
1,000 deals is roughly **3 CPU-hours** and 2.2×10⁹ nodes at `--restarts 1`, and
perhaps twice that at `--restarts 8`. **This is hours, not days.** If it is
tracking toward days, something is wrong with the configuration — say so rather
than letting it run to the end.

## What to report

Markdown, timestamped, plus the two results files.

- The validation summary for each run. The repo's analysis script turns a
  results file into the bracket against the published figure; run it per file
  rather than pooling the two.
- **The unknown fraction for each policy.** This is the number the whole run
  exists to produce. It is read against two thresholds: below about **5%** opens
  Gypsy batch work, below about **1%** makes the ±0.5% figure this project is
  for reachable at all.
- Solvable / unsolvable / unknown counts, and for the unknowns, how many stopped
  on the node budget versus any other limit.
- Whether the bracket contains 81.945%, and how wide it is.
- Wall clock, CPU time, peak RSS per worker, and the worker count and table size
  actually used. The Gypsy run that eventually follows is sized off these.
- **The seeds still unknown — the list, not a count.** Those deals are the
  working set for the next round of solver work.
- Anything that had to differ from the above, and why.

Name the files `klondike-full-12M-1000deals-restart1.jsonl` and
`klondike-full-12M-1000deals-restart8.jsonl`. That box's root filesystem has
been near full and is RAID0 with no redundancy, so the results are worth getting
off it; the report comes back either way.

## Not part of this

- **Do not start a Gypsy batch.** Gypsy batch work is gated on this run's
  result. Starting it early spends days of box time producing a number that
  could not be published.
- Do not adjust the solver, the budget, or the dominance rules to improve the
  result. A worse number, honestly measured, is the useful output — the point is
  to find out where we actually are.
