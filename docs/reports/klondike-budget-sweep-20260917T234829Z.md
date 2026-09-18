# Klondike budget sweep — 3M, 12M, and 48M nodes

**Run timestamp:** 2026-09-17T23:48:29Z  
**Repository:** `gypsy-solve`  
**Commit:** `51a6bf0cd90884aedbe02da56da8f38b42b63a69` (`main`)  
**Worktree:** `/home/seeduser/gypsy-solve-validation`

## Executive findings

- The requested 1,000-deal sweep is complete at 3M and 48M nodes. The 12M
  point was retained from the earlier 2,048 MiB-table control; it matched the
  accepted 1,024 MiB-table run on every verdict.
- Full Klondike, restart-1, seeds 0–999, and a fixed 2,048 MiB table were used
  at all three budgets. Two workers were used throughout the sweep points.
- Unknown fraction fell from **12.3% → 8.1% → 5.1%**:

  | Budget | Solvable | Unsolvable | Unknown | Unknown fraction |
  |---:|---:|---:|---:|---:|
  | 3M | 755 | 122 | 123 | **12.3%** |
  | 12M | 777 | 142 | 81 | **8.1%** |
  | 48M | 791 | 158 | 51 | **5.1%** |

- The unknown fraction is multiplied by **0.6585** on the 3M→12M step and by
  **0.6296** on the 12M→48M step. Equivalently, each fourfold budget increase
  reduced the unknown bucket by factors of **1.52×** and **1.59×**.
- A log-linear fit through all three measured points gives a decay exponent of
  **−0.3175** with budget. Extrapolating that fit puts the 5% threshold at
  approximately **52.3M nodes per deal** and the 1% threshold at approximately
  **8.32B nodes per deal**. These are extrapolations far beyond the measured
  range, not measurements.
- The 48M point is still just above the project's 5% gate: **5.1%**, or one
  unknown deal above the exact 5% sample count. No Gypsy batch was started.
- All three files are consistent with the published **81.945%** Klondike figure.
  The 95% Wilson brackets narrow as the budget rises:
  **72.7%–89.7%**, **75.0%–87.8%**, and **76.5%–86.3%**.
- Peak table occupancy was **2.24%**, **8.94%**, and **35.74%** at 3M, 12M,
  and 48M respectively, all below half full. The curve is therefore not showing
  obvious table eviction at the top point.

## Protocol

All three points used the full published Klondike variant: 24-card stock, draw
three, unlimited redeals, and worry-back permitted. The search policy was
restart-1. The 12M point was not rerun during this resumed task; the existing
2,048 MiB-table control was used as the middle point after it passed the revised
verdict-only reproducibility check against the earlier 1,024 MiB run.

| Parameter | Value |
|---|---:|
| commit | `51a6bf0` |
| game | `klondike` |
| seeds | 0–999 inclusive |
| ruleset | full |
| restarts | 1 |
| max depth | 100,000 |
| table | 2,048 MiB per worker |
| workers | 2 |
| disk floor | 512 MiB (runner default) |
| budgets | 3,000,000; 12,000,000; 48,000,000 |

`cargo test` passed all CLI, core, solver, and Klondike tests; doc tests also
passed. `cargo build --release` passed.

## Results and validation brackets

The analysis script was run separately on each result file, never pooled.

| Budget | Solvable | Unsolvable | Unknown | Sample lower–upper bound | 95% Wilson bracket | Published figure inside? |
|---:|---:|---:|---:|---:|---:|:---:|
| 3M | 755 (75.5%) | 122 (12.2%) | 123 (12.3%) | 75.5%–87.8% | **72.7%–89.7%** | yes |
| 12M | 777 (77.7%) | 142 (14.2%) | 81 (8.1%) | 77.7%–85.8% | **75.0%–87.8%** | yes |
| 48M | 791 (79.1%) | 158 (15.8%) | 51 (5.1%) | 79.1%–84.2% | **76.5%–86.3%** | yes |

Every unknown in every file has `limit: "budget"`. No deal stopped on the
stack-depth limit or another limit.

The analysis outputs were:

```text
3M:  CONSISTENT: the published figure is inside the bracket (16.9 points wide).
12M: CONSISTENT: the published figure is inside the bracket (12.8 points wide).
48M: CONSISTENT: the published figure is inside the bracket (9.9 points wide).
```

## Budget curve

Let `u(B)` be the unknown fraction at budget `B`.

| Step | Unknown fraction change | Multiplier `u(next)/u(previous)` | Reduction factor |
|---|---:|---:|---:|
| 3M → 12M | 12.3% → 8.1% | **0.6585** | **1.52×** |
| 12M → 48M | 8.1% → 5.1% | **0.6296** | **1.59×** |

The geometric mean multiplier per fourfold step is **0.6439**. A log-linear fit
through all three points has exponent **−0.3175** in natural-log budget units,
so approximately:

```text
unknown_fraction(B) ≈ exp(2.64748) × B^(-0.31752)
```

Using that fit:

| Target unknown fraction | Extrapolated budget |
|---:|---:|
| 5% | **52.3M nodes/deal** |
| 1% | **8.32B nodes/deal** |

The 5% estimate is close to the measured range and should be read as a rough
projection. The 1% estimate is more than two orders of magnitude above the
largest measured point and is correspondingly speculative. If instead the
geometric-mean multiplier is anchored exactly at the measured 12M point, the
same thresholds are approximately **54.8M** and **8.72B** nodes; this gives a
reasonable sensitivity range for the extrapolation rather than false precision.

## Peak table occupancy

All three result files use table capacity **134,217,728 entries**.

| Budget | Peak `table_filled` | Peak occupancy | Peak seed |
|---:|---:|---:|---:|
| 3M | 3,000,000 | **2.235%** | 4 |
| 12M | 12,000,000 | **8.941%** | 504 |
| 48M | 47,969,712 | **35.740%** | 332 |

The 48M table did not approach half full. This supports interpreting the curve
primarily as a node-budget curve rather than a table-capacity/eviction curve.
The 12M and 48M maxima are exact or nearly exact budget-sized fills because the
hard deals reached their per-deal node limits.

## Runtime and resources

The result records' `elapsed_ms` values are aggregate per-deal elapsed time,
not OS CPU-time measurements. Wall intervals below are taken from each progress
log's filesystem creation time to its final modification time.

| Budget | Workers | Table/worker | Wall clock | Aggregate per-deal elapsed | Preflight memory estimate |
|---:|---:|---:|---:|---:|---:|
| 3M | 2 | 2,048 MiB | 808.148 s (13m 28.148s) | 1,490.773 s (0.414 h) | 2,439 MiB/worker |
| 12M | 2 | 2,048 MiB | 1,968.571 s (32m 48.571s) | 3,511.615 s (0.975 h) | 2,439 MiB/worker |
| 48M | 2 | 2,048 MiB | 5,170.815 s (1h 26m 10.815s) | 8,874.256 s (2.465 h) | 2,439 MiB/worker |

The runner charged 2,048 MiB for the table plus approximately 391 MiB for the
search stack per worker, or approximately 4,878 MiB for two workers. Peak RSS
was not captured by the runner and is not claimed here. The 48M run completed
well inside the six-hour overrun warning in the protocol.

## 48M unresolved working set

All 51 unresolved seeds stopped on the 48,000,000-node budget:

```text
8, 30, 41, 44, 48, 65, 125, 130, 166, 195, 197, 221, 233, 238, 241,
254, 273, 295, 298, 306, 312, 321, 323, 332, 338, 348, 366, 408, 425,
479, 501, 504, 615, 641, 647, 670, 679, 680, 700, 703, 711, 783, 788,
805, 815, 822, 882, 923, 952, 960
```

## 12M control check

The resumed task's accepted 12M point was not required to match node counts
against the earlier 1,024 MiB-table file. The corrected protocol treats the
transposition-table displacement effect as expected: the larger table can
avoid a re-expansion and therefore reduce node counts slightly, without
changing a verdict.

The 2,048 MiB control matched the earlier file's verdict on all 1,000 seeds.
Its node counts differed on 9 seeds, all downward, by a total of 13 nodes in
1,380,690,815 reference nodes. There were no verdict differences. That is the
control retained here.

## Artifacts

Results:

- `klondike-full-3M-1000deals-restart1.jsonl`
- `klondike-full-12M-1000deals-restart1.jsonl`
- `klondike-full-48M-1000deals-restart1.jsonl`

Progress logs with the same stems were also preserved. Each result file has
exactly 1,000 complete JSONL records and exactly one record for each seed from
0 through 999.

No Gypsy batch was started. The 48M unknown fraction is **5.1%**, still above
the project's approximately 5% opening gate.
