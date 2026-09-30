"""Every number the writeup quotes, regenerated from the archived results.

Until now the headline figures were merged across runs by hand in
`DECISIONS.md`, and that produced errors twice: a bound computed from the
restricted arm's wins when the full arm had proved more, and a sample bracket
quoted as a population claim. This script does the merge once, from the files
in `docs/results/`, and prints each figure with its sample and its sampling
allowance.

How runs are merged. A verdict is a fact about a (seed, ruleset) pair, whatever
budget found it: `solvable` comes with a replayed line and `unsolvable` with an
exhaustive search, and `unknown` is only the absence of either. So the merge is
a union of decided verdicts, and two runs that decide the same pair differently
are a contradiction. That stops the script, because it means a solver bug. After
the union, proofs are carried between arms: a win without worry-back is a win
with it, and a full-game refutation is a restricted one. Nothing else is carried.

Wilson intervals assume seeds are an i.i.d. uniform sample of deals. That was
tested on 2026-09-28 (`analysis/shuffle_uniformity.py`).

Usage: python3 analysis/gypsy_numbers.py [RESULTS_DIR]
"""

import json
import math
import os
import sys
from collections import Counter, defaultdict

from klondike_validation import PUBLISHED, wilson

R, F = "no-worry-back", "full"
KLONDIKE = [("3M", "klondike-full-3M-1000deals-restart1.jsonl"),
            ("12M", "klondike-full-12M-1000deals-restart1.jsonl"),
            ("48M", "klondike-full-48M-1000deals-restart1.jsonl"),
            ("64M", "klondike-full-64M-1000deals-restart1.jsonl")]
SURVEY_12M = "gypsy-both-12M-restart8-1000deals.jsonl"
SURVEY_48M = "gypsy-both-48M-restart32-1000deals.jsonl"
SURVEY_192M = "gypsy-both-192M-restart128-48Munknowns.jsonl"
RESIDUE = "gypsy-both-192M-restart32-slice6M-residue93.jsonl"
CONTIGUOUS = ["gypsy-both-12M-1000deals-contiguous.jsonl",
              "gypsy-both-12M-seeds1000-4999-contiguous.jsonl"]
REFUTATIONS = "gypsy-both-200M-refutations-contiguous.jsonl"
ACES = "opening-aces-seeds0-4999.jsonl"


class Contradiction(Exception):
    pass


def load(directory, name):
    with open(os.path.join(directory, name)) as f:
        return [json.loads(line) for line in f if line.strip()]


def merge(*runs):
    """Union of decided verdicts over runs, with the cross-arm carry."""
    decided = defaultdict(set)
    seeds = set()
    for run in runs:
        for record in run:
            seeds.add(record["seed"])
            if record["verdict"] != "unknown":
                decided[record["seed"], record["ruleset"]].add(record["verdict"])
    for seed in seeds:
        if "solvable" in decided[seed, R]:
            decided[seed, F].add("solvable")
        if "unsolvable" in decided[seed, F]:
            decided[seed, R].add("unsolvable")
    verdicts = {}
    for seed in seeds:
        for arm in (R, F):
            found = decided[seed, arm]
            if len(found) > 1:
                raise Contradiction(f"seed {seed} {arm}: {sorted(found)}")
            verdicts[seed, arm] = found.pop() if found else "unknown"
    return verdicts, seeds


def tally(verdicts, seeds, arm):
    counts = Counter(verdicts[s, arm] for s in seeds)
    return counts["solvable"], counts["unsolvable"], counts["unknown"]


def pct(x):
    return f"{100 * x:.1f}%"


def klondike(directory):
    print("## Klondike validation (published 81.945%)\n")
    print("| budget | solvable | unsolvable | unknown | bracket, Wilson 95% | contains |")
    print("|---:|---:|---:|---:|---|---|")
    for budget, name in KLONDIKE:
        verdicts, seeds = merge(load(directory, name))
        solvable, unsolvable, unknown = tally(verdicts, seeds, F)
        n = len(seeds)
        low = wilson(solvable, n)[0]
        high = wilson(n - unsolvable, n)[1]
        inside = "yes" if low <= PUBLISHED <= high else "**NO**"
        print(f"| {budget} | {solvable} | {unsolvable} | {pct(unknown / n)} "
              f"| {pct(low)} – {pct(high)} | {inside} |")
    print()


def survey(directory):
    print("## Gypsy budget curve, seeds 0–999, 1.5M-node restarts\n")
    r12, r48, r192 = (load(directory, n) for n in (SURVEY_12M, SURVEY_48M, SURVEY_192M))
    v12, s12 = merge(r12)
    v48, s48 = merge(r48)
    # The 192M run re-solved only the 48M run's unknowns. Its restarts 1-32
    # replay the 48M run, so taking the 48M verdicts elsewhere is exact.
    v192, s192 = merge(r48, r192)
    print("| budget | restricted unknown | full unknown |")
    print("|---:|---:|---:|")
    for label, (v, s) in [("12M as 8 x 1.5M", (v12, s12)), ("48M as 32 x 1.5M", (v48, s48)),
                          ("192M as 128 x 1.5M", (v192, s192))]:
        print(f"| {label} | {tally(v, s, R)[2]} | {tally(v, s, F)[2]} |")
    print()


def bound(directory):
    print("## The bound: seeds 0–999, every run merged\n")
    runs = [load(directory, n) for n in (SURVEY_12M, SURVEY_48M, SURVEY_192M, RESIDUE, CONTIGUOUS[0])]
    runs.append([r for r in load(directory, REFUTATIONS) if r["seed"] < 1000])
    verdicts, seeds = merge(*runs)
    assert seeds == set(range(1000))
    for arm, name in ((R, "restricted"), (F, "full (the game under study)")):
        solvable, unsolvable, unknown = tally(verdicts, seeds, arm)
        print(f"- {name}: {solvable} solvable, {unsolvable} unsolvable, {unknown} unknown; "
              f"winnable ≥ {pct(wilson(solvable, 1000)[0])} (Wilson 95%)")
    restarts_only = merge(*runs[:4])[0]
    contiguous_only = sorted(
        s for s in seeds if verdicts[s, F] == "solvable" and restarts_only[s, F] == "unknown")
    print(f"- full-arm wins found only by the contiguous run: {contiguous_only or 'none'}")
    print()
    return verdicts


def refutations(directory):
    print("## Refutations: seeds 0–4999, contiguous 12M, plus the re-runs\n")
    contiguous = [r for n in CONTIGUOUS for r in load(directory, n)]
    v, s = merge(contiguous)
    assert s == set(range(5000))
    print(f"- contiguous 12M alone: restricted {tally(v, s, R)[1]} unsolvable, "
          f"full {tally(v, s, F)[1]}")
    v, s = merge(contiguous, load(directory, REFUTATIONS))
    refuted = sorted(seed for seed in s if v[seed, F] == "unsolvable")
    both = [seed for seed in refuted if v[seed, R] == "unsolvable"]
    k, n = len(refuted), len(s)
    low = wilson(k, n)[0]
    print(f"- with the full arm run to exhaustion: {k} refuted in the full game, "
          f"{len(both)} of them in both arms: {refuted}")
    print(f"- unwinnable ≥ {100 * low:.3f}% (Wilson 95%), so winnable ≤ {100 * (1 - low):.2f}%")
    print()
    return v, s


def worry_back(v, s):
    print("## The worry-back delta, seeds 0–4999\n")
    pairs = Counter((v[seed, R], v[seed, F]) for seed in s)
    print(f"- restricted unsolvable, full solvable (a real delta): {pairs['unsolvable', 'solvable']}")
    print(f"- restricted unsolvable, full unknown (open candidates): {pairs['unsolvable', 'unknown']}")
    print(f"- restricted unknown, full solvable (budget, not evidence): {pairs['unknown', 'solvable']}")
    print()


def aces(directory, verdicts):
    print("## The bound by face-up aces, seeds 0–999, full game\n")
    up = {r["seed"]: r["aces_face_up"] for r in load(directory, ACES)}
    pmf = [math.comb(8, k) * math.comb(96, 16 - k) / math.comb(104, 16) for k in range(9)]
    # Four or more face-up aces is 1.9% of deals and 11 of these; pooled.
    top = 4
    stratum = lambda k: min(k, top)
    weight = [sum(pmf[k] for k in range(9) if stratum(k) == j) for j in range(top + 1)]
    deals, wins = Counter(), Counter()
    for seed in range(1000):
        deals[stratum(up[seed])] += 1
        wins[stratum(up[seed])] += verdicts[seed, F] == "solvable"
    print("| face-up aces | deals | expected share | proved winnable |")
    print("|---:|---:|---:|---:|")
    for j in range(top + 1):
        label = f"{j}+" if j == top else str(j)
        print(f"| {label} | {deals[j]} | {pct(weight[j])} | {pct(wins[j] / deals[j])} |")
    raw = sum(wins.values()) / 1000
    reweighted = sum(weight[j] * wins[j] / deals[j] for j in range(top + 1))
    print(f"\n- proved winnable, as sampled: {pct(raw)}; reweighted to the "
          f"hypergeometric: {pct(reweighted)} ({100 * (reweighted - raw):+.2f} points)")
    print()


def main(directory):
    try:
        klondike(directory)
        survey(directory)
        verdicts = bound(directory)
        v, s = refutations(directory)
        worry_back(v, s)
        aces(directory, verdicts)
    except Contradiction as error:
        print(f"CONTRADICTION: {error}. Two runs disagree on a decided verdict.")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1] if len(sys.argv) > 1 else "docs/results"))
