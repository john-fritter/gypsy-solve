# Gypsy residue slice test — 192M / 32 restarts / 6M slices

**Timestamp:** 2026-09-25T04:08:49Z  
**Repository:** `/home/seeduser/gypsy-solve`  
**Build:** `main` at `2d72ec5054d176fb3b003c8ae7a06994880ac0d4`

## Result

The 93 restricted-arm deals still unknown after the 192M/128-restart slope pin were re-solved at the same 192M total budget using 32 slices of 6M. The run produced 186 rows, two arms per seed, with complete coverage.

The 6M-slice canary, seed 188 restricted arm, passed: `unsolvable`, 4,203,474 nodes, restart 1.

| Arm | Input population | Solvable | Unsolvable | Unknown |
|---|---:|---:|---:|---:|
| Restricted | 93 | **10** | **1** | 82 |
| Full previous residue | 82 | **6** | 0 | 76 |

The complete result file contains 93 full-arm rows: 14 solvable and 79 unknown, because 11 full-arm deals had already been solved at the previous point and were rerun as the expected `--both-arms` side effect.

No new unsolvable result appeared. Seed 188 was the expected restricted-arm refutation; its full arm remained unknown.

## What the larger slices found

Restricted-arm wins, as `(seed, restarts_used, nodes_in_winning_6M_slice)`:

```text
[(56, 1, 2858306), (113, 2, 2315001), (185, 10, 5935413), (191, 2, 2440098), (193, 14, 4729149), (395, 4, 2782381), (570, 1, 3789336), (691, 7, 3536321), (692, 26, 2345540), (928, 18, 1812629)]
```

The winning-slice distribution is:

| Slice depth | Wins |
|---|---:|
| ≤1.5M | 0 |
| 1.5M–3M | **6** |
| >3M | **4** |

None of the ten restricted wins would have been reachable within a 1.5M slice from its successful ordering. This is direct evidence that the residue contains deeper wins, not merely deals waiting for more random restarts.

For the 82-deal full-arm residue, five wins were carried from restricted wins and one was independently found:

```text
carried: [56, 191, 193, 691, 692]
independent: [(508, 3, 3147130)]
```

The independent full-arm resolution is seed 508, restart 3, with 3,147,130 nodes in its winning slice.

## Arm comparison

| Restricted | Full | Deals |
|---|---|---:|
| solvable | solvable | 10 |
| unknown | solvable | **4** |
| unknown | unknown | 78 |
| unsolvable | unknown | 1 |
| solvable | unknown | 0 |

The four new worry-back resolutions are seeds **361, 508, 640, 695**. No reverse disagreement occurred.

## Remaining unknown seeds

### Restricted — 82

```text
12, 23, 37, 65, 71, 88, 105, 109, 112, 129, 148, 150, 153, 157,
171, 200, 202, 250, 258, 259, 270, 272, 280, 301, 311, 312, 317, 324,
335, 361, 370, 372, 376, 389, 399, 419, 437, 445, 450, 454, 465, 474,
484, 497, 508, 516, 523, 535, 536, 537, 550, 588, 592, 594, 612, 613,
631, 632, 636, 640, 649, 657, 664, 667, 676, 694, 695, 701, 727, 730,
746, 787, 795, 818, 825, 832, 885, 916, 934, 948, 952, 999
```

### Full — 76 of the previous full-arm residue

```text
12, 23, 37, 65, 71, 88, 105, 109, 112, 129, 148, 150, 153, 157,
171, 188, 200, 202, 250, 258, 259, 270, 272, 280, 301, 311, 312, 317,
324, 335, 370, 372, 389, 399, 419, 437, 445, 450, 454, 465, 474, 484,
497, 516, 523, 535, 536, 537, 550, 588, 592, 594, 612, 613, 631, 632,
636, 649, 657, 664, 667, 676, 694, 701, 727, 730, 746, 787, 795, 818,
825, 832, 885, 916, 948, 952, 999
```

Both sets are subsets of the previous corresponding unknown sets.

## Resources and timing

- Budget: 192M total, 32 restarts, 6M per slice
- Table: 256 MiB per worker
- Workers: 4
- Peak fill: **5,996,357 / 16,777,216 = 35.7411%**
- Restricted aggregate elapsed: **42,359.055 seconds**
- Full independent-search elapsed: **35,865.758 seconds**; carried rows contribute zero
- Combined search work: **840.662505 seconds/input seed**
- Approximate wall clock: **5.6 hours**

## Artifacts

Copied and verified in the active workspace:

- `gypsy-both-192M-restart32-slice6M-residue93.jsonl` — `7239f9733817bf34635ee138e7d2eeca08d811a50a0f6728f20a3f44c716eff3`
- `gypsy-both-192M-restart32-slice6M-residue93.log` — `c1559e08a1d8a9d3d8dea6bb7ed692004b5d8d5b2b17d4903b1f9e8faf0cd741`
- `gypsy-both-192M-restart32-slice6M-residue93.progress.json` — `4f97c3f48e9cc462945ea22cb969b98e34edb345b2f869366707c3038dacce00`

This remains a solver-budget survey, not a publishable winnability estimate.
