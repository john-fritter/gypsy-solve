# Gizmo request: hunt for the first full-size Gypsy refutation, 2026-09-19

Every thousand-deal Gypsy run so far used restarts, and a restart slice of 1.5M
nodes cannot exhaust a full-size game. **So no Gypsy run this project has ever
done was capable of returning `unsolvable`**, and the fact that none ever has
says nothing yet. Contiguous-budget Gypsy runs exist only at fifty deals, where
zero refutations is the expected result even if the game has them at a few per
cent.

This is the run that can actually return one. It deliberately resolves fewer
deals than the restart configuration — that is the trade, and it is the point.

The sizing note that matters: **the table rule reverses when restarts are off.**
The last Gypsy task said a small table was right, because each restart slice is
its own search with its own table. A contiguous search holds every position it
expands, so the table has to be sized to the budget.

The request as sent follows.

---

**Look for a Gypsy deal that can be proved unsolvable.**

Build `main` and run `gypsy batch --game gypsy` over **seeds 0–999** with
**`--restarts 1`** and `--both-arms`, at two budgets, the smaller first.

| Run | Budget | Table per worker | Occupancy at full budget |
|---|---:|---:|---:|
| 1 | 12,000,000 | 1,024 MiB | ~18% |
| 2 | 48,000,000 | 2,048 MiB | ~36% |

Report after run 1 before starting run 2.

## What this is for

No Gypsy deal has ever been proved unsolvable at thirteen ranks. That is the
single largest gap in what this project can say: without a refutation the answer
can only ever be "at least X% winnable", and no dominance can be tested in the
direction where a wrong one fails.

**Restarts are why, and it is structural rather than bad luck.** `unsolvable` is
claimed only by a search that exhausts the whole reachable game without touching
a limit. Under `--restarts 32` each search gets 1.5M nodes. Nothing full-size
exhausts in 1.5M nodes, so those runs could not have returned a refutation
whatever the deals looked like.

**One run at a time, contiguous, is the only shape that can.** It will resolve
fewer deals — the recorded fifty-deal comparison is 12 unknown per arm
contiguous at 48M against 4 with restarts — and that is accepted here.

## Why the table must grow, having just been told to keep it small

The previous Gypsy task specified 256 MiB and said a larger table bought
nothing. That was correct **for restarts**: each slice is a separate `solve`
with its own table, so it never holds more than one slice's 1.5M positions.

With `--restarts 1` the search holds everything it expands. A 12M contiguous
search fills about 12M entries and a 48M one about 48M, so the tables above are
sized to keep occupancy well under half. **Do not carry the 256 MiB figure over
from the last task** — at 48M contiguous it would be oversubscribed by a factor
of three, and most of the budget would go on re-expanding positions it had
already seen rather than on new ground.

Workers to fit: roughly 1,415 MiB each at run 1 and 2,439 MiB at run 2,
including the search stack. Reduce workers rather than the table.

## The headline, and what to do if it happens

**Any `unsolvable` verdict at all is the result.** For each one, report the
seed, the arm, and **the node count it exhausted in** — that number says how
expensive a Gypsy refutation is, which nobody knows.

**If any deal comes back `unsolvable`, verify it before reporting it as one.**
This project has had a false refutation before: a dominance that discarded
winning lines made winnable deals look unsolvable, and it went unnoticed for a
week. Re-solve each refuted seed three more times, each with **one** of the
three dominances removed from `legal_actions` — safe autoplay, the
safe-foundation forcing rule, and the split-run filter. A genuine refutation
survives all three; one that changes to `solvable` or `unknown` with a rule
removed is that rule failing, which is a far more important finding than the
refutation and should be reported as such.

## What else to report

- Solvable / unsolvable / unknown per arm per budget, and the unknown fraction.
- **Against the restart runs at the same budget**: 12M with `--restarts 8` gave
  23.2% restricted unknown and 21.1% full. Contiguous will be worse; by how much
  is the price of being able to refute at all, and worth having as a number.
- Whether any full-arm `unsolvable` carried to the restricted arm, and any
  disagreement between the arms in either direction.
- The unresolved seed lists, taken from the results files.
- Peak table occupancy per run. If it is near or above half, say so — the run is
  then partly measuring displacement rather than budget.
- Wall clock, aggregate per-deal time, workers and table used.

Name the files `gypsy-both-{12M,48M}-1000deals-contiguous.jsonl`. Get them off
that box.

## Sizing

The recorded fifty-deal contiguous runs cost 0.58 aggregate hours at 12M and
1.58 at 48M, so a thousand deals is roughly 12 and 32 aggregate hours. Run 1
should be a few hours; run 2 is an overnight job. Seeds 0–49 are easier than the
population on Gypsy — the thousand-deal survey came back at roughly double their
unknown rate — so treat those figures as a floor. **If run 1 is tracking past
twelve hours or run 2 past two days, stop and report.**

## Not part of this

- No figure is published from this and none should be quoted as one.
- Do not switch restarts back on to improve the unknown fraction. A worse
  unknown fraction is the cost of the only configuration that can answer the
  question being asked.
