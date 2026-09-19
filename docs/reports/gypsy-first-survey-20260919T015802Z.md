# Gypsy first thousand-deal survey

**Report timestamp:** 2026-09-19T01:58:02Z  
**Repository:** `/home/seeduser/gypsy-solve`  
**Branch/commit:** `main` at `2d72ec5054d176fb3b003c8ae7a06994880ac0d4` (`Merge pull request #12`)

## Build gate

Passed before any survey run.

- `origin/main` was fetched and advanced to `2d72ec5` (`Merge pull request #12`).
- `b27f65f4328b21325ef14c0400e223088ea7b549`, titled `Drop safe autoplay's shortcut for twos, in both games`, is an ancestor of `main`.
- The exact shortcut line is absent from executable code in both `solver/src/gypsy.rs` and `klondike/src/game.rs`; the text remains only in explanatory comments.

## Run 1 — 50-deal fix confirmation

Configuration: seeds 0–49, 12,000,000 nodes, 8 restarts, `--both-arms`, 256 MiB table per worker, 4 workers, max depth 100,000.

The output has exactly 100 rows, one per arm per seed. Compared with `gypsy-both-12M-50deals-restart8-safefoundation.jsonl`, there were **zero contradicted verdicts** and **zero verdict changes**.

| Arm | Solvable | Unsolvable | Unknown |
|---|---:|---:|---:|
| Restricted / no worry-back | 44 | 0 | 6 |
| Full / worry-back | 45 | 0 | 5 |

The full arm searched 6 deals independently and carried 44 verdicts from the restricted arm. Peak table occupancy was **1,500,000 / 16,777,216 = 8.9407%**. Approximate wall clock was **0h 3m 17.548s**; aggregate restricted-arm elapsed time was **526.091 seconds**, or **10.52182 seconds/deal**.

Run 1 cleared the required gate, so Runs 2 and 3 proceeded.

## Run 2 — 12M nodes, 1,000 deals

Configuration: seeds 0–999, 12,000,000 nodes, 8 restarts, `--both-arms`, 256 MiB table per worker, 4 workers, max depth 100,000. The runner preflight reported 2,588 MiB for four workers. The output has exactly 2,000 rows covering both arms for every seed.

| Arm | Solvable | Unsolvable | Unknown | Unknown fraction |
|---|---:|---:|---:|---:|
| Restricted / no worry-back | 768 | 0 | 232 | 23.2% |
| Full / worry-back | 789 | 0 | 211 | 21.1% |

No thirteen-rank Gypsy deal was proved unsolvable.

The 50-deal baseline had 6 restricted unknowns (12.0%) and 5 full unknowns (10.0%). The thousand-deal result is plainly **not representative** of that small sample: 23.2% restricted unknown and 21.1% full unknown, higher by 11.2 and 11.1 percentage points.

### Worry-back delta

| Restricted arm | Full arm | Deals |
|---|---|---:|
| solvable | solvable | 768 |
| unknown | solvable | **21** |
| unknown | unknown | 211 |
| solvable | unknown | 0 |
| any unsolvable result | any | 0 |

There are **21 / 1,000 (2.1%)** disagreements, all restricted `unknown` → full `solvable`; none go in the opposite direction. The resolved seeds are:

```text
0, 52, 177, 185, 210, 216, 288, 292, 313, 360, 395, 489, 549, 665,
792, 793, 806, 820, 829, 851, 959
```

Full-arm provenance: **232 from `search`**, **768 carried** from the restricted arm.

### Unresolved seeds

Restricted / no worry-back — 232:

```text
0, 12, 22, 23, 32, 37, 52, 56, 62, 64, 65, 67, 69, 71,
75, 78, 83, 85, 88, 103, 104, 105, 109, 112, 113, 114, 126, 128,
129, 133, 137, 145, 148, 150, 151, 153, 154, 155, 157, 160, 170, 171,
172, 177, 185, 187, 188, 189, 190, 191, 192, 193, 200, 202, 210, 212,
214, 215, 216, 225, 228, 233, 235, 241, 250, 251, 258, 259, 262, 267,
268, 270, 272, 275, 280, 288, 292, 293, 301, 310, 311, 312, 313, 315,
317, 324, 330, 335, 342, 351, 352, 357, 360, 361, 370, 372, 376, 379,
384, 388, 389, 395, 399, 419, 426, 437, 438, 443, 444, 445, 450, 452,
454, 461, 465, 468, 474, 484, 489, 497, 498, 508, 516, 521, 523, 527,
529, 535, 536, 537, 549, 550, 562, 564, 567, 569, 570, 583, 588, 592,
594, 599, 604, 612, 613, 614, 631, 632, 636, 640, 642, 647, 649, 657,
659, 660, 664, 665, 667, 669, 673, 676, 691, 692, 693, 694, 695, 698,
701, 706, 727, 730, 745, 746, 747, 749, 750, 761, 780, 787, 792, 793,
795, 800, 804, 806, 808, 810, 812, 817, 818, 819, 820, 825, 829, 832,
838, 847, 848, 851, 855, 864, 873, 874, 882, 885, 892, 900, 905, 913,
916, 928, 930, 934, 941, 943, 948, 952, 959, 960, 962, 970, 973, 976,
979, 983, 987, 988, 989, 995, 998, 999
```

Full / worry-back — 211:

```text
12, 22, 23, 32, 37, 56, 62, 64, 65, 67, 69, 71, 75, 78,
83, 85, 88, 103, 104, 105, 109, 112, 113, 114, 126, 128, 129, 133,
137, 145, 148, 150, 151, 153, 154, 155, 157, 160, 170, 171, 172, 187,
188, 189, 190, 191, 192, 193, 200, 202, 212, 214, 215, 225, 228, 233,
235, 241, 250, 251, 258, 259, 262, 267, 268, 270, 272, 275, 280, 293,
301, 310, 311, 312, 315, 317, 324, 330, 335, 342, 351, 352, 357, 361,
370, 372, 376, 379, 384, 388, 389, 399, 419, 426, 437, 438, 443, 444,
445, 450, 452, 454, 461, 465, 468, 474, 484, 497, 498, 508, 516, 521,
523, 527, 529, 535, 536, 537, 550, 562, 564, 567, 569, 570, 583, 588,
592, 594, 599, 604, 612, 613, 614, 631, 632, 636, 640, 642, 647, 649,
657, 659, 660, 664, 667, 669, 673, 676, 691, 692, 693, 694, 695, 698,
701, 706, 727, 730, 745, 746, 747, 749, 750, 761, 780, 787, 795, 800,
804, 808, 810, 812, 817, 818, 819, 825, 832, 838, 847, 848, 855, 864,
873, 874, 882, 885, 892, 900, 905, 913, 916, 928, 930, 934, 941, 943,
948, 952, 960, 962, 970, 973, 976, 979, 983, 987, 988, 989, 995, 998,
999
```

Peak table occupancy was **1,500,000 / 16,777,216 = 8.9407%**, under the expected 10%. Aggregate elapsed time was **13,315.917 seconds restricted** (**13.315917 seconds/deal**) plus **7,081.924 seconds of independent full-arm search** (**7.081924 seconds/deal**; carried rows contribute zero), for **20.397841 seconds/deal combined search work**. Approximate wall clock was **1h 27m 48.028s**.

## Run 3 — 48M nodes, 1,000 deals

Configuration: seeds 0–999, 48,000,000 nodes, 32 restarts, `--both-arms`, 256 MiB table per worker, 4 workers, max depth 100,000. The output has exactly 2,000 rows covering both arms for every seed.

| Arm | Solvable | Unsolvable | Unknown | Unknown fraction |
|---|---:|---:|---:|---:|
| Restricted / no worry-back | 875 | 0 | 125 | 12.5% |
| Full / worry-back | 885 | 0 | 115 | 11.5% |

No thirteen-rank Gypsy deal was proved unsolvable.

The 50-deal 48M baseline had 4 unknowns (8.0%) in each arm. The thousand-deal result is again plainly **not representative** of that small sample: 12.5% restricted unknown and 11.5% full unknown, higher by 4.5 and 3.5 percentage points.

### Worry-back delta

| Restricted arm | Full arm | Deals |
|---|---|---:|
| solvable | solvable | 875 |
| unknown | solvable | **10** |
| unknown | unknown | 115 |
| solvable | unknown | 0 |
| any unsolvable result | any | 0 |

There are **10 / 1,000 (1.0%)** disagreements, all restricted `unknown` → full `solvable`; none go in the opposite direction. The resolved seeds are:

```text
113, 185, 361, 395, 521, 549, 570, 640, 695, 806
```

Full-arm provenance: **125 from `search`**, **875 carried** from the restricted arm.

### Unresolved seeds

Restricted / no worry-back — 125:

```text
12, 23, 32, 37, 56, 64, 65, 71, 85, 88, 103, 104, 105, 109,
112, 113, 129, 148, 150, 151, 153, 157, 170, 171, 185, 188, 191, 193,
200, 202, 233, 250, 258, 259, 267, 270, 272, 280, 301, 311, 312, 317,
324, 335, 351, 361, 370, 372, 376, 384, 388, 389, 395, 399, 419, 437,
445, 450, 454, 465, 474, 484, 497, 508, 516, 521, 523, 535, 536, 537,
549, 550, 564, 569, 570, 583, 588, 592, 594, 612, 613, 631, 632, 636,
640, 649, 657, 659, 660, 664, 667, 669, 676, 691, 692, 693, 694, 695,
701, 706, 727, 730, 746, 761, 780, 787, 795, 806, 818, 825, 832, 882,
885, 905, 916, 928, 930, 934, 941, 943, 948, 952, 970, 979, 999
```

Full / worry-back — 115:

```text
12, 23, 32, 37, 56, 64, 65, 71, 85, 88, 103, 104, 105, 109,
112, 129, 148, 150, 151, 153, 157, 170, 171, 188, 191, 193, 200, 202,
233, 250, 258, 259, 267, 270, 272, 280, 301, 311, 312, 317, 324, 335,
351, 370, 372, 376, 384, 388, 389, 399, 419, 437, 445, 450, 454, 465,
474, 484, 497, 508, 516, 523, 535, 536, 537, 550, 564, 569, 583, 588,
592, 594, 612, 613, 631, 632, 636, 649, 657, 659, 660, 664, 667, 669,
676, 691, 692, 693, 694, 701, 706, 727, 730, 746, 761, 780, 787, 795,
818, 825, 832, 882, 885, 905, 916, 928, 930, 934, 941, 943, 948, 952,
970, 979, 999
```

Peak table occupancy was **1,500,000 / 16,777,216 = 8.9407%**, under the expected 10%. Aggregate elapsed time was **29,878.264 seconds restricted** (**29.878264 seconds/deal**) plus **14,479.174 seconds of independent full-arm search** (**14.479174 seconds/deal**; carried rows contribute zero), for **44.357438 seconds/deal combined search work**. Approximate wall clock was **3h 6m 40.006s**. The run completed in well under the two-day stop threshold.

## Resource summary

Both thousand-deal runs used four workers and a 256 MiB table per worker. The table capacity was 16,777,216 entries; peak fill was 1,500,000 entries in both runs, or **8.9407%**. The table was therefore not close to binding at this slice size.

## Artifacts

The requested JSONL result files and progress logs are copied into the active workspace. Hashes below are SHA-256 hashes of the repository copies; workspace copies were compared byte-for-byte.

### Run 1

- `gypsy-both-12M-50deals-restart8-twofix.jsonl` — `26f379c63672f3b621aea7cfbc6450ea09da40f8c7069efab738872fb0d1cdc5`
- `gypsy-both-12M-50deals-restart8-twofix.log` — `7881f0e9ce352b2e4c822338ea4254d320522efa9ad35acfe191ff6abb7d4578`

Workspace report: `gypsy-both-12M-50deals-run1-20260918T211626Z.md`

### Run 2

- `gypsy-both-12M-restart8-1000deals.jsonl` — `487bf092cf0a3bf2d7b0131b6c0e68b5d804b7eef70e7d7d9b71e32e9d41f5cc`
- `gypsy-both-12M-restart8-1000deals.log` — `cc64a8fcdcea965d25905fe4598dbf55b53e6095b95504e7d87f2319f94aa91e`

Workspace report: `gypsy-both-12M-restart8-1000deals-report-20260918T224714Z.md`

### Run 3

- `gypsy-both-48M-restart32-1000deals.jsonl` — `09a30474d17779ea3d0dcbb9494df0f3be6df85458de41a156de8fdf60354eb1`
- `gypsy-both-48M-restart32-1000deals.log` — `df9ce3cd59bd5e0da02fd078fc1f6b4974a65d9b49db9feef6116b0a95111805`

## Interpretation boundary

This is a survey of solver behavior and unresolved working sets, not a published Gypsy winnability figure. No percentage from these runs should be quoted as the final population estimate or given a confidence interval. The unknown bucket remains too large for that purpose.
