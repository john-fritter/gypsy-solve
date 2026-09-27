# Gypsy slope pin — 192M / 128 restarts

**Report timestamp:** 2026-09-24T09:19:57Z  
**Repository:** `/home/seeduser/gypsy-solve`  
**Branch/commit:** `main` at `2d72ec5054d176fb3b003c8ae7a06994880ac0d4` (`Merge pull request #12`)

## Status

The requested third budget point completed over the 125 restricted-arm unknowns from the 48M survey. It produced **250 rows**, covering both arms for all 125 seeds. No thirteen-rank deal was proved unsolvable. The result was copied to the active workspace and validated.

## Exactness canary

Before the batch, three deals from the 48M run with `restarts_used` above 16 were re-solved at 192M / 128 restarts / 256 MiB table / both arms:

| Seed | 48M restricted verdict | 48M nodes | 48M restarts | 192M restricted verdict | 192M nodes | 192M restarts |
|---:|---|---:|---:|---|---:|---:|
| 268 | solvable | 46,110,893 | 31 | solvable | 46,110,893 | 31 |
| 330 | solvable | 45,004,607 | 31 | solvable | 45,004,607 | 31 |
| 562 | solvable | 45,666,665 | 31 | solvable | 45,666,665 | 31 |

The full-arm rows matched as carried wins as well. The canary passed exactly: the first 32 restart slices replayed node-for-node, so the unknowns-only shortcut is valid.

## Run configuration

| Parameter | Value |
|---|---:|
| Input seeds | The 125 restricted-arm unknowns from the 48M survey |
| Total population represented | 1,000 deals |
| Budget | 192,000,000 nodes |
| Restarts | 128 |
| Slice | 1,500,000 nodes per restart |
| Table | 256 MiB per worker |
| Workers | 4 per-deal processes |
| Arms | `--both-arms` |
| Max depth | 100,000 |
| Result rows | 250 |

## Headline result

| Arm | Unknown at 48M | Unknown at 192M | Unknown fraction over 1,000 | Step multiplier |
|---|---:|---:|---:|---:|
| Restricted / no worry-back | 125 | **93** | **9.3%** | **0.744** |
| Full / worry-back | 115 | **82** | **8.2%** | **0.713** |

The previous 12M → 48M restricted multiplier was **0.539** (12.5% → 23.2% when expressed as 48M unknowns / 12M unknowns), and the corresponding full-arm multiplier was **0.545** (11.5% / 21.1%). At 192M the multiplier is higher: **0.744 restricted** and **0.713 full**. The unknown fraction is still falling, but the improvement has slowed substantially relative to the first fourfold step. This third point therefore supports a **flattening slope**, not continued 0.54 scaling.

No unsolvable result appeared. Seed 188 remained unknown, as expected for a 1.5M-slice search.

## Newly resolved deals and restart depth

### Restricted arm

The 32 restricted-arm deals newly resolved at 192M were all solvable. Their `restarts_used` values are:

```text
(32, 33), (64, 81), (85, 38), (103, 43), (104, 66), (151, 80), (170, 128), (233, 54),
(267, 47), (351, 55), (384, 53), (388, 63), (521, 63), (549, 43), (564, 67), (569, 42),
(583, 92), (659, 67), (660, 77), (669, 79), (693, 75), (706, 40), (761, 63), (780, 62),
(806, 50), (882, 81), (905, 96), (930, 66), (941, 123), (943, 41), (970, 48), (979, 86)
```

The detailed rows are extracted from the result file as `(seed, restarts_used)` pairs above. The values range from **33** through **128**; none were resolved in the first 32 slices, which is exactly the work this pin was measuring.

### Full arm

The full arm gained 43 solvable rows within this 125-seed input:

- **32 carried** from newly solved restricted-arm wins (`verdict_from: no-worry-back`), with `restarts_used: 0` in the carried row.
- **11 independently searched** full-arm wins (`verdict_from: search`). Their `(seed, restarts_used)` pairs are:

```text
(113, 13), (185, 8), (270, 83), (361, 25), (376, 104), (395, 4), (570, 10), (640, 24),
(695, 13), (928, 105), (934, 48)
```

## Arm disagreements

| Restricted | Full | Deals |
|---|---|---:|
| solvable | solvable | 32 |
| unknown | solvable | **11** |
| unknown | unknown | 82 |
| solvable | unknown | 0 |
| any unsolvable | any | 0 |

The 11 independent full-arm resolutions are the only new worry-back disagreements. No deal disagreed in the reverse direction, and no unsolvable verdict occurred.

## Remaining unknown seeds

### Restricted / no worry-back — 93 seeds

```text
12, 23, 37, 56, 65, 71, 88, 105, 109, 112, 113, 129, 148, 150,
153, 157, 171, 185, 188, 191, 193, 200, 202, 250, 258, 259, 270, 272,
280, 301, 311, 312, 317, 324, 335, 361, 370, 372, 376, 389, 395, 399,
419, 437, 445, 450, 454, 465, 474, 484, 497, 508, 516, 523, 535, 536,
537, 550, 570, 588, 592, 594, 612, 613, 631, 632, 636, 640, 649, 657,
664, 667, 676, 691, 692, 694, 695, 701, 727, 730, 746, 787, 795, 818,
825, 832, 885, 916, 928, 934, 948, 952, 999
```

### Full / worry-back — 82 seeds

```text
12, 23, 37, 56, 65, 71, 88, 105, 109, 112, 129, 148, 150, 153,
157, 171, 188, 191, 193, 200, 202, 250, 258, 259, 272, 280, 301, 311,
312, 317, 324, 335, 370, 372, 389, 399, 419, 437, 445, 450, 454, 465,
474, 484, 497, 508, 516, 523, 535, 536, 537, 550, 588, 592, 594, 612,
613, 631, 632, 636, 649, 657, 664, 667, 676, 691, 692, 694, 701, 727,
730, 746, 787, 795, 818, 825, 832, 885, 916, 948, 952, 999
```

Both remaining sets are subsets of their corresponding 48M unknown sets. This is the expected monotonicity check and passed.

## Occupancy, timing, and resources

- Peak table fill: **1,500,000 / 16,777,216 = 8.9407%** in both arms.
- Aggregate restricted-arm elapsed time: **53,832.845 seconds**, or **430.662760 seconds per re-solved deal**.
- Aggregate full-arm elapsed time: **38,249.419 seconds**, or **305.995352 seconds per re-solved deal**. Carried full-arm rows contribute zero elapsed time.
- Combined re-solved search work: **736.658112 seconds per input seed**.
- Approximate wall clock: **6h 35m 30.070s**.
- Table occupancy stayed under 10%, as expected.

## Artifacts

Copied to the active workspace:

- `gypsy-both-192M-restart128-48Munknowns.jsonl` — SHA-256 `a49aac641889fda67aaa3b76c592f398b2685f0bf3e4984f3427cc84700dc60f`
- `gypsy-both-192M-restart128-48Munknowns.log` — SHA-256 `9c91880328f9bf256beca23985f77e19ab18c35b2ce868cab62dc2f6a9d784ce`
- `gypsy-both-192M-restart128-48Munknowns.progress.json` — SHA-256 `4431294c20fe5785830abade6e588c1a32780c66c87c338817827af8b6d41a1d`

## Interpretation boundary

This is a solver-budget survey, not a published Gypsy winnability estimate. The unknown bucket is still too large for a population percentage or confidence interval. The useful result is the measured third-point slope: increasing the number of 1.5M restarts continues to resolve deals, but the fourfold budget multiplier is now about **0.74 restricted / 0.71 full**, not the earlier **0.54**.
