# Gypsy refutation hunt — Run 1 closeout

**Report timestamp:** 2026-09-20T03:26:23Z  
**Repository:** `/home/seeduser/gypsy-solve`  
**Branch/commit:** `main` at `2d72ec5054d176fb3b003c8ae7a06994880ac0d4` (`Merge pull request #12`)

## Status

Run 1 completed, and a candidate refutation was found. The mandatory dominance-removal verification then showed that the candidate is **not a valid refutation**. The failing rule is safe autoplay. **Run 2 was not started.**

## Build and run configuration

`cargo build --release --locked --bin gypsy` passed before the run.

| Parameter | Value |
|---|---:|
| Game | Gypsy |
| Seeds | 0–999 inclusive |
| Budget | 12,000,000 nodes per deal |
| Restarts | 1 contiguous search |
| Arms | `--both-arms` |
| Table | 1,024 MiB per worker |
| Workers | 4 |
| Max depth | 100,000 |
| Output rows | 2,000 |

The runner preflight reported **1,415 MiB per worker** and the run used four workers. The output contains exactly one row for every seed and arm.

## Run 1 results

| Arm | Solvable | Unsolvable | Unknown | Unknown fraction |
|---|---:|---:|---:|---:|
| Restricted / no worry-back | 516 | **1** | 483 | **48.3%** |
| Full / worry-back | 524 | 0 | 476 | **47.6%** |

The candidate refutation was:

```text
seed       188
arm        no-worry-back
verdict    unsolvable
nodes      4,203,474
limit      none
```

No full-arm verdict was unsolvable. No full-arm unsolvable verdict was carried to the restricted arm.

### Arm disagreement and provenance

Verdict pairs by seed:

| Restricted | Full | Deals |
|---|---|---:|
| solvable | solvable | 516 |
| unknown | solvable | 8 |
| unknown | unknown | 475 |
| unsolvable | unknown | 1 |
| solvable | unknown | 0 |
| any other pair | any | 0 |

The eight worry-back resolutions were:

```text
110, 216, 243, 271, 388, 415, 756, 984
```

Full-arm provenance was **516 carried from the restricted arm** and **484 independently searched**.

### Comparison with the restart survey

At the same 12M budget, the earlier restart-8 survey produced 232 restricted unknowns and 211 full unknowns. The contiguous run produced 483 and 476 respectively. The unknown fraction is therefore much worse, as expected, but this configuration is the one that can exhaust a search and prove unsolvability.

## Resource and timing data

- Peak table fill: **11,999,952 / 67,108,864 = 17.8813%**.
- Aggregate restricted-arm elapsed time: **20,833.670 seconds** (**20.833670 seconds/deal**).
- Aggregate full-arm elapsed time: **16,523.250 seconds** (**16.523250 seconds/deal of independent full-arm search**; carried rows contribute zero).
- Combined search work: **37.356920 seconds/deal**.
- Approximate wall clock from result-file timestamps: **2h 41m 46.988s**.
- Table occupancy is below half; the run is not primarily measuring displacement.

## Refutation verification

The candidate was re-solved at seed 188 in three fresh detached worktrees. Each variant was built successfully with `cargo build --release --locked --bin gypsy`, then run with one worker, a 1,024 MiB table, 12M budget, one contiguous search, and `--both-arms`.

| Variant | Restricted verdict | Restricted nodes | Full verdict | Interpretation |
|---|---|---:|---|---|
| Baseline | unsolvable | 4,203,474 | unknown | Candidate result |
| Safe autoplay removed | **unknown** | 12,000,000 | unknown | **Dominance fails** |
| Safe-foundation forcing removed | unsolvable | 4,203,474 | unknown | Candidate survives this removal |
| Split-run filter removed | unsolvable | 4,671,078 | unknown | Candidate survives this removal |

The safe-autoplay removal is the decisive result: it changes the restricted arm from an exhaustive refutation to a budget-limited `unknown`. Therefore seed 188 must **not** be reported as an unsolvable Gypsy deal. The result instead exposes a false-refutation bug in the restricted-arm safe-autoplay dominance, which is more important than the candidate refutation and is exactly why this verification gate was required.

The edits were confined to detached worktrees under `/tmp/gypsy-refutation-verify/`; `main` was not modified.

## Unresolved seed lists

These are extracted from the Run 1 result file.

### Restricted / no worry-back — 483 seeds

```text
0, 3, 5, 12, 14, 19, 23, 24, 25, 29, 31, 32, 37, 38,
39, 41, 43, 45, 46, 51, 52, 53, 57, 59, 61, 62, 64, 65,
67, 71, 75, 78, 79, 80, 83, 85, 86, 88, 93, 95, 97, 98,
100, 101, 103, 104, 105, 109, 110, 111, 112, 113, 114, 120, 123, 124,
125, 126, 127, 128, 129, 130, 133, 135, 136, 137, 142, 144, 145, 148,
150, 151, 152, 153, 154, 155, 157, 158, 159, 160, 161, 163, 166, 170,
171, 172, 177, 180, 182, 185, 187, 189, 190, 191, 192, 193, 194, 198,
200, 202, 205, 207, 210, 214, 215, 216, 220, 221, 222, 225, 226, 227,
228, 230, 233, 234, 235, 236, 241, 242, 243, 244, 245, 246, 247, 249,
250, 253, 254, 256, 258, 259, 262, 263, 265, 267, 268, 270, 271, 275,
279, 280, 281, 285, 288, 290, 293, 294, 295, 301, 304, 305, 306, 307,
310, 311, 312, 313, 315, 317, 320, 322, 324, 326, 330, 332, 333, 335,
340, 341, 344, 347, 350, 351, 352, 357, 358, 360, 361, 363, 364, 370,
371, 372, 373, 375, 376, 379, 380, 381, 382, 383, 384, 388, 389, 392,
395, 396, 399, 405, 406, 408, 411, 413, 414, 415, 418, 419, 420, 424,
425, 426, 427, 430, 431, 432, 433, 434, 437, 438, 440, 441, 442, 443,
444, 445, 450, 451, 452, 453, 454, 455, 461, 463, 465, 466, 467, 468,
469, 470, 474, 484, 489, 490, 491, 494, 497, 498, 499, 502, 508, 512,
516, 517, 521, 523, 524, 526, 527, 529, 531, 535, 536, 537, 539, 542,
543, 545, 547, 549, 550, 553, 555, 557, 560, 561, 562, 564, 567, 569,
572, 574, 575, 577, 580, 582, 583, 584, 585, 587, 588, 590, 591, 592,
593, 594, 597, 606, 610, 612, 613, 614, 616, 619, 621, 624, 627, 628,
631, 632, 636, 637, 639, 640, 642, 643, 647, 649, 650, 651, 652, 657,
659, 660, 662, 663, 664, 666, 667, 668, 669, 672, 673, 676, 679, 680,
690, 691, 692, 693, 694, 695, 697, 699, 700, 701, 706, 708, 710, 713,
718, 719, 725, 727, 728, 729, 730, 734, 736, 737, 742, 743, 744, 745,
746, 749, 752, 753, 756, 760, 761, 763, 764, 767, 771, 780, 782, 783,
784, 787, 789, 790, 791, 793, 795, 796, 799, 800, 801, 804, 805, 806,
808, 810, 812, 813, 815, 816, 818, 820, 822, 824, 825, 827, 829, 830,
831, 832, 833, 837, 838, 839, 841, 848, 850, 854, 855, 862, 865, 874,
880, 882, 883, 885, 888, 889, 891, 892, 893, 898, 900, 902, 905, 907,
908, 909, 910, 911, 913, 916, 920, 922, 926, 927, 928, 930, 934, 940,
941, 943, 946, 947, 948, 951, 952, 954, 960, 961, 962, 964, 965, 966,
967, 968, 970, 971, 973, 976, 979, 981, 983, 984, 986, 987, 988, 989,
990, 992, 994, 995, 996, 998, 999
```

### Full / worry-back — 476 seeds

```text
0, 3, 5, 12, 14, 19, 23, 24, 25, 29, 31, 32, 37, 38,
39, 41, 43, 45, 46, 51, 52, 53, 57, 59, 61, 62, 64, 65,
67, 71, 75, 78, 79, 80, 83, 85, 86, 88, 93, 95, 97, 98,
100, 101, 103, 104, 105, 109, 111, 112, 113, 114, 120, 123, 124, 125,
126, 127, 128, 129, 130, 133, 135, 136, 137, 142, 144, 145, 148, 150,
151, 152, 153, 154, 155, 157, 158, 159, 160, 161, 163, 166, 170, 171,
172, 177, 180, 182, 185, 187, 188, 189, 190, 191, 192, 193, 194, 198,
200, 202, 205, 207, 210, 214, 215, 220, 221, 222, 225, 226, 227, 228,
230, 233, 234, 235, 236, 241, 242, 244, 245, 246, 247, 249, 250, 253,
254, 256, 258, 259, 262, 263, 265, 267, 268, 270, 275, 279, 280, 281,
285, 288, 290, 293, 294, 295, 301, 304, 305, 306, 307, 310, 311, 312,
313, 315, 317, 320, 322, 324, 326, 330, 332, 333, 335, 340, 341, 344,
347, 350, 351, 352, 357, 358, 360, 361, 363, 364, 370, 371, 372, 373,
375, 376, 379, 380, 381, 382, 383, 384, 389, 392, 395, 396, 399, 405,
406, 408, 411, 413, 414, 418, 419, 420, 424, 425, 426, 427, 430, 431,
432, 433, 434, 437, 438, 440, 441, 442, 443, 444, 445, 450, 451, 452,
453, 454, 455, 461, 463, 465, 466, 467, 468, 469, 470, 474, 484, 489,
490, 491, 494, 497, 498, 499, 502, 508, 512, 516, 517, 521, 523, 524,
526, 527, 529, 531, 535, 536, 537, 539, 542, 543, 545, 547, 549, 550,
553, 555, 557, 560, 561, 562, 564, 567, 569, 572, 574, 575, 577, 580,
582, 583, 584, 585, 587, 588, 590, 591, 592, 593, 594, 597, 606, 610,
612, 613, 614, 616, 619, 621, 624, 627, 628, 631, 632, 636, 637, 639,
640, 642, 643, 647, 649, 650, 651, 652, 657, 659, 660, 662, 663, 664,
666, 667, 668, 669, 672, 673, 676, 679, 680, 690, 691, 692, 693, 694,
695, 697, 699, 700, 701, 706, 708, 710, 713, 718, 719, 725, 727, 728,
729, 730, 734, 736, 737, 742, 743, 744, 745, 746, 749, 752, 753, 760,
761, 763, 764, 767, 771, 780, 782, 783, 784, 787, 789, 790, 791, 793,
795, 796, 799, 800, 801, 804, 805, 806, 808, 810, 812, 813, 815, 816,
818, 820, 822, 824, 825, 827, 829, 830, 831, 832, 833, 837, 838, 839,
841, 848, 850, 854, 855, 862, 865, 874, 880, 882, 883, 885, 888, 889,
891, 892, 893, 898, 900, 902, 905, 907, 908, 909, 910, 911, 913, 916,
920, 922, 926, 927, 928, 930, 934, 940, 941, 943, 946, 947, 948, 951,
952, 954, 960, 961, 962, 964, 965, 966, 967, 968, 970, 971, 973, 976,
979, 981, 983, 986, 987, 988, 989, 990, 992, 994, 995, 996, 998, 999
```

There is one restricted-arm `unsolvable` row in the raw result file, seed 188, but it is invalidated by the safe-autoplay verification above and must not be treated as a refutation.

## Artifacts

The Run 1 JSONL and log, plus the three seed-188 verification outputs, were copied into the active workspace and hashed.

```text
cae6d049dcdba1b816e28ff30dad000b8cbed0eafea37a96aa38788f92282cee  gypsy-both-12M-1000deals-contiguous.jsonl
de7efbdef69e19d65820bf4ecadbb8892406e60808f214ff481b7f8d47fc5eb2  gypsy-both-12M-1000deals-contiguous.log
00815e8149fb8b16746a53d312b12b1919f6c8eda3e6658f0c6e939e1d4ed642  gypsy-seed188-autoplay-off.jsonl
06560cc3786c35b2573b13c20321b70ea788ade358d5e4ca9eb7787d79f091e4  gypsy-seed188-foundation-off.jsonl
cfeac132e510c5f7a2cdaedbbb4f0dd284dead13ea495d73af6ac51b65e15ee8  gypsy-seed188-split-off.jsonl
```

## Next step

The task explicitly says to stop and report when a supposed refutation changes under dominance removal. No 48M contiguous run was started. The next solver task should address and re-verify the restricted-arm safe-autoplay rule before any further refutation hunt or survey number is trusted.
