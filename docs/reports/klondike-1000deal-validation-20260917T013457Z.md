# Klondike validation: 1,000 deals at 12M nodes

**Run timestamp:** 2026-09-17T01:34:57Z  
**Repository:** `gypsy-solve`  
**Commit:** `51a6bf0cd90884aedbe02da56da8f38b42b63a69` (`main`)  
**Worktree:** `/home/seeduser/gypsy-solve-validation`

## Executive findings

- The required 50-deal reproducibility gate passed exactly for seeds 0–49:
  every verdict and node count matched the checked-in 1,024 MiB-table reference
  `docs/results/klondike-full-12M-50deals-safefoundation.jsonl`.
- Both 1,000-deal files are complete and cover each seed 0–999 exactly once.
- Restart 1: **777 solvable, 142 unsolvable, 81 unknown**; unknown fraction
  **8.1%**.
- Restart 8: **761 solvable, 113 unsolvable, 126 unknown**; unknown fraction
  **12.6%**.
- Every unknown in both runs stopped on the **12,000,000-node budget**. No deal
  stopped on another limit.
- The 95% Wilson brackets both contain the published **81.945%** Klondike
  figure, so both runs are **CONSISTENT** with it:
  - restart 1: **75.0%–87.8%**, width **12.8 percentage points**;
  - restart 8: **73.4%–90.5%**, width **17.2 percentage points**.
- Neither policy clears the project's roughly **5%** unknown threshold for
  opening Gypsy batch work, and neither is near the roughly **1%** threshold
  needed to make a ±0.5-point figure reachable. Restart 8 resolved fewer deals,
  as expected for the measured Klondike behavior.
- **No Gypsy batch was started.**

## Protocol

Both runs used the full published Klondike ruleset: 24-card stock, draw three,
unlimited redeals, and worry-back permitted. The only changed parameter between
the two runs was `--restarts`.

| Parameter | Value |
|---|---:|
| commit | `51a6bf0` |
| game | `klondike` |
| seeds | 0–999 inclusive |
| node budget per deal | 12,000,000 |
| max depth | 100,000 |
| table | 1,024 MiB per worker |
| workers | 4 |
| disk floor | 512 MiB (runner default) |
| restart policies | 1 and 8 |
| restricted arm | not run |

The runner's memory guard charged **1,415 MiB per worker** at this table and
max-depth setting: 1,024 MiB for the table plus an estimated 391 MiB for the
search stack. Four workers therefore required an estimated 5,660 MiB and fit
the available memory at startup. The worker count and table size were identical
for both runs.

## Reproducibility gate

The gate command used the same full ruleset, budget, table, depth, and restart-1
policy over seeds 0–49. Its output was compared programmatically against the
checked-in 1,024 MiB-table reference.

- Reference rows: 50
- Gate rows: 50
- Missing seeds: none
- Extra seeds: none
- Verdict/node-count differences: **none**

The older legacy `klondike-12M-50deals.jsonl` was not used as the gate reference:
it does not carry the current configuration metadata and its tuples differ from
the required 1,024 MiB-table run. The matching reference is the checked-in
`safefoundation` file named above.

## Results summary

| Policy | Solvable | Unsolvable | Unknown | Unknown fraction | Sample bracket | 95% Wilson bracket | Published inside? |
|---|---:|---:|---:|---:|---:|---:|---|
| restart 1 | 777 (77.7%) | 142 (14.2%) | 81 (8.1%) | 8.1% | 77.7%–85.8% | 75.0%–87.8% | yes |
| restart 8 | 761 (76.1%) | 113 (11.3%) | 126 (12.6%) | 12.6% | 76.1%–88.7% | 73.4%–90.5% | yes |

The analysis script was run separately on each file:

```text
python3 analysis/klondike_validation.py \
  docs/results/klondike-full-12M-1000deals-restart1.jsonl

python3 analysis/klondike_validation.py \
  docs/results/klondike-full-12M-1000deals-restart8.jsonl
```

Both returned `CONSISTENT`.

## Limits and unresolved seeds

All unknowns had `limit: "budget"`; the count for every other limit is zero.

### Restart 1 — 81 unknowns

```text
8, 21, 30, 35, 36, 41, 44, 48, 61, 65, 66, 81, 125, 130, 166, 195,
197, 201, 221, 233, 236, 238, 241, 244, 254, 273, 295, 298, 306, 312,
317, 320, 321, 323, 332, 334, 338, 348, 357, 366, 408, 425, 435, 441,
452, 462, 479, 501, 504, 558, 592, 615, 641, 647, 655, 670, 679, 680,
690, 700, 703, 711, 728, 731, 750, 783, 788, 800, 805, 811, 815, 822,
882, 889, 890, 918, 923, 952, 960, 964, 989
```

### Restart 8 — 126 unknowns

```text
4, 8, 13, 21, 30, 35, 36, 41, 44, 48, 52, 61, 65, 66, 70, 80, 96,
109, 112, 130, 131, 149, 157, 166, 182, 195, 197, 201, 211, 221, 233,
236, 238, 241, 244, 254, 257, 266, 278, 304, 306, 309, 317, 320, 321,
323, 328, 332, 333, 334, 348, 354, 357, 366, 379, 382, 408, 417, 423,
425, 435, 437, 441, 452, 454, 458, 460, 462, 479, 501, 504, 505, 512,
513, 537, 558, 560, 592, 600, 604, 608, 619, 641, 647, 655, 670, 679,
680, 690, 700, 703, 711, 717, 728, 731, 734, 738, 750, 751, 763, 769,
776, 778, 782, 783, 788, 800, 805, 811, 815, 822, 840, 849, 860, 869,
882, 889, 890, 899, 907, 918, 923, 936, 960, 964, 989
```

## Runtime and resource observations

The result files record `elapsed_ms` for each deal. Summing those fields gives
aggregate per-deal elapsed time, useful as a worker-time proxy but **not an OS
CPU-time measurement**:

| Policy | Wall interval from run log file creation to final update | Aggregate per-deal elapsed | Max `table_filled` | Table capacity |
|---|---:|---:|---:|---:|
| restart 1 | 883.959 s (14m 43.959s) | 3,779.939 s (1.050 h) | 11,999,944 | 67,108,864 entries |
| restart 8 | 1,520.747 s (25m 20.747s) | 5,930.050 s (1.647 h) | 1,500,000 | 67,108,864 entries |

The restart-8 table-fill maximum is lower because its 12M total-node budget is
split across eight 1.5M-node searches. The measured table occupancy is not a
peak RSS measurement.

The batch runner does not emit OS CPU time or per-worker RSS, and these runs were
not wrapped with an external resource-accounting command. Therefore **peak RSS
per worker is not available from this run**. The defensible memory figure is the
runner's preflight estimate of 1,415 MiB per worker, not a claimed measured RSS.
No parameter was silently reduced to fit memory.

## Artifacts

Results:

- `docs/results/klondike-full-12M-1000deals-restart1.jsonl`
- `docs/results/klondike-full-12M-1000deals-restart8.jsonl`

Progress logs:

- `docs/results/klondike-full-12M-1000deals-restart1.log`
- `docs/results/klondike-full-12M-1000deals-restart8.log`

Gate output:

- `docs/results/klondike-full-12M-50deals-gate-restart1.jsonl`
- `docs/results/klondike-full-12M-50deals-gate-restart1.log`

Build verification on the target commit:

- `cargo test`: passed — 12 CLI, 34 core, 50 solver, and 36 Klondike tests;
  doc tests also passed.
- `cargo build --release`: passed.
- Both result files: 1,000 rows, 1,000 unique seeds, exact range 0–999, and
  one consistent full-Klondike configuration.

## Evidence boundary

This report distinguishes:

- **solvable:** a winning line was found and replayed;
- **unsolvable:** the reachable game was exhausted without a win;
- **unknown:** the node budget stopped the search, so the deal may be either.

The brackets are therefore the appropriate validation result; the raw solvable
percentage is not a claimed exact Klondike winnability estimate.
