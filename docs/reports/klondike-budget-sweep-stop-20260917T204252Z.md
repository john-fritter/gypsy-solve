# Klondike budget sweep — stopped at reproducibility gate

**Timestamp:** 2026-09-17T20:42:52Z  
**Repository:** `gypsy-solve`  
**Commit:** `51a6bf0cd90884aedbe02da56da8f38b42b63a69` (`main`)  
**Worktree:** `/home/seeduser/gypsy-solve-validation`

## Status

The requested sweep was **not completed**. The protocol requires a 12M-node
control at the fixed 2,048 MiB table size before running the 3M and 48M points.
That control produced identical verdicts but non-identical node counts on 9 of
1,000 seeds compared with the existing 12M run. I therefore stopped and did not
start either remaining budget.

This is a reproducibility failure, not a sweep result. No 3M or 48M curve,
threshold extrapolation, or 48M unknown-seed working set can honestly be
reported from this run.

**No Gypsy batch was started.**

## Requested protocol versus executed work

| Parameter | Requested value | Executed |
|---|---:|---:|
| game | Klondike | Klondike |
| ruleset | full, worry-back permitted | full, worry-back permitted |
| seeds | 0–999 | 0–999 |
| budgets | 3M, 12M, 48M | 12M control only |
| restarts | 1 | 1 |
| max depth | unchanged | 100,000 |
| table | 2,048 MiB per worker | 2,048 MiB per worker for control |
| workers | reduce if needed | 2 |

The checkout was verified at `origin/main == HEAD == 51a6bf0`. `cargo test`
passed all CLI, core, solver, and Klondike tests; `cargo build --release`
passed.

## Control comparison

The existing reference was preserved as:

```text
docs/results/klondike-full-12M-1000deals-restart1-table1024-reference.jsonl
```

The newly executed control is:

```text
docs/results/klondike-full-12M-1000deals-restart1.jsonl
```

Both files contain exactly one row for every seed 0–999.

### Verdicts

- Verdict differences: **0**
- Reference: 777 solvable, 142 unsolvable, 81 unknown
- Control: 777 solvable, 142 unsolvable, 81 unknown
- Unknowns in both: 81, all stopped on the node budget
- Unknown fraction: **8.1%**

The control alone is consistent with the published Klondike figure. Running the
analysis script on it produced:

```text
in this sample, winnability is between 77.7% and 85.8%
allowing for sampling error, 95%:      75.0% to 87.8%
published  81.945% +/- 0.084%
CONSISTENT: the published figure is inside the bracket (12.8 points wide).
```

### Node counts

Node counts did **not** match:

| Seed | 1,024 MiB reference | 2,048 MiB control | Difference |
|---:|---:|---:|---:|
| 182 | 11,583,595 | 11,583,592 | -3 |
| 304 | 8,442,248 | 8,442,247 | -1 |
| 512 | 11,188,439 | 11,188,438 | -1 |
| 557 | 11,370,183 | 11,370,182 | -1 |
| 560 | 7,709,498 | 7,709,497 | -1 |
| 600 | 10,609,439 | 10,609,438 | -1 |
| 734 | 11,855,635 | 11,855,633 | -2 |
| 782 | 10,654,173 | 10,654,172 | -1 |
| 869 | 11,488,159 | 11,488,157 | -2 |

- Node-count differences: **9 / 1,000**
- Reference total nodes: 1,380,690,815
- Control total nodes: 1,380,690,802
- Total difference: **-13 nodes**
- Verdict differences: none

The reference used a 1,024 MiB table with capacity 67,108,864 entries. The
control used the requested 2,048 MiB table with capacity 134,217,728 entries.
Although neither run approached table capacity in ordinary occupancy terms, the
observed node-count differences mean the control is not identical under the
project's stated reproducibility criterion. I did not reinterpret those small
differences as harmless noise.

## Control resource measurements

| Measurement | 2,048 MiB control |
|---|---:|
| workers | 2 |
| table capacity | 134,217,728 entries |
| peak `table_filled` | 12,000,000 |
| peak occupancy | **8.94%** |
| aggregate per-deal elapsed | 3,511.615 s (0.975 h) |
| wall interval from log creation to final update | approximately 32m49s |
| result rows | 1,000 |
| unique seeds | 1,000 |
| seed range | 0–999 |

The runner's preflight memory estimate for this configuration was 2,439 MiB per
worker (2,048 MiB table plus approximately 391 MiB stack), or approximately
4,878 MiB for two workers. No table reduction was used.

## Not run

Because the 12M fixed-table control failed the node-count identity check:

- `klondike-full-3M-1000deals-restart1.jsonl` was **not produced**;
- `klondike-full-48M-1000deals-restart1.jsonl` was **not produced**;
- no consecutive-budget unknown-fraction ratio was computed;
- no 5% or 1% budget extrapolation was made;
- no 48M unknown-seed list exists.

Producing those values would violate the attached protocol's stop condition.

## Available artifacts

Control result and progress log:

```text
/home/seeduser/gypsy-solve-validation/docs/results/klondike-full-12M-1000deals-restart1.jsonl
/home/seeduser/gypsy-solve-validation/docs/results/klondike-full-12M-1000deals-restart1.log
```

Preserved comparison reference and log:

```text
/home/seeduser/gypsy-solve-validation/docs/results/klondike-full-12M-1000deals-restart1-table1024-reference.jsonl
/home/seeduser/gypsy-solve-validation/docs/results/klondike-full-12M-1000deals-restart1-table1024-reference.log
```

The 12M control result is complete and was written incrementally by the batch
runner. The two lower/higher budget result files are intentionally absent.
