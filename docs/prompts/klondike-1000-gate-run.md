# Gizmo request: settle the 5% gate, 2026-09-18

The thousand-deal budget sweep landed at **5.1% unknown at 48M** — fifty-one
unresolved deals where the gate that opens Gypsy batch work wants fifty or
fewer. One deal. This run is the single point that settles it.

The choice worth recording is the budget. The fitted curve puts 5% at about 52M
a deal, so 64M is the smallest budget with real margin (it predicts ~47 unknown)
that also keeps table occupancy at a level where the run is still measuring
budget rather than displacement. 96M would predict ~41 but sits at 72% of the
table, and displacement costs resolution — which is the thing being measured.

The request as sent follows.

---

**Settle the 5% gate for `gypsy-solve`: one thousand-deal Klondike run at 64M.**

Build `main` and run `gypsy batch --game klondike` over **seeds 0–999** at a
**64,000,000**-node budget, full ruleset, `--restarts 1`, **2,048 MiB** of table
per worker, two workers.

Everything is the same as the 48M point of the last sweep except the budget.
That run left **5.1% unknown** — 51 deals — against a gate of 5%. This run is
one number: the unknown fraction at 64M.

## Why these parameters

**64M and not higher.** The fitted curve from the sweep puts 5% at about 52M a
deal and predicts roughly **47 unknown deals at 64M**, which clears the gate
with a few deals of margin rather than one. Going further does not help: at 64M
the table sits at about **48% occupancy**, at 96M about 72%, and past about half
full the eight-slot probe window starts displacing entries often enough to
matter. A displaced entry costs a re-expansion, and re-expansions cost
resolution — so a bigger budget on this table would partly spend itself
undoing its own gain.

**Same table for the same reason as last time.** 2,048 MiB per worker, and if
the run does not fit, reduce workers rather than the table. It keeps this point
comparable with the three already measured.

**Full arm, restarts off.** As before: the full arm is the one with a published
figure, and restarts are a measured loss on Klondike.

**The commit does not matter much here, but build `main`.** A dominance called
safe autoplay was corrected recently on a branch. It fires only with worry-back
suppressed, so this arm is untouched by it and was verified identical to the
node across that change. Nothing about this run needs to wait for it.

## Sizing

The 48M run took **1h26m of wall clock** on two workers and 2.465 hours of
aggregate per-deal time. 64M is about a third more work, so expect **under two
hours**. If it is tracking past four, something is wrong with the configuration
— say so rather than letting it finish.

## What to report

Markdown, timestamped, plus the results file.

- **The unknown fraction, and whether it is at or below 5%.** That is the whole
  run. Fifty deals or fewer clears it.
- Solvable / unsolvable / unknown counts, and the validation bracket against the
  published figure from the repo's analysis script.
- **Peak table occupancy.** At roughly half full this is the first run where
  displacement is worth watching, and it is the evidence the number means what
  it says.
- **The unresolved seed list, taken from the results file rather than
  transcribed.** The 48M report gave a count of 51 and a list of 50; the file
  was right and the list was short. This list is the working set for the next
  round of solver work, so it matters that it is complete.
- Whether the 64M unknowns are a **subset of the 48M unknowns**. More budget can
  only resolve more deals, so anything outside that set means something moved
  that should not have.
- The measured unknown fraction against the **4.69% the fit predicted**. A large
  miss is more interesting than a small one and is worth saying either way.
- Wall clock, aggregate per-deal time, workers and table actually used.

Name the file `klondike-full-64M-1000deals-restart1.jsonl`. Get it off that box;
the report comes back either way.

## If it does not clear

**Stop and report.** If the unknown fraction comes back above 5%, the next step
is a judgement about the table and the machine rather than more budget on this
one, and that is a decision to bring back rather than to make here.

## Not part of this

- **Do not start a Gypsy batch**, even if this clears the gate. What that batch
  should run — budget, restarts, arms, how many deals — is a separate design
  question that is not settled yet.
- Do not tune anything to get under 5%. A run that misses honestly is the
  useful answer; a run that clears because something was adjusted is worse than
  no run at all.
