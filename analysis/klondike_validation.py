"""Summarise a Klondike validation run.

Reads the JSON lines written by `gypsy klondike --json` and reports what the
run actually establishes about winnability.

The point is the bracket, not a point estimate. Every `solvable` verdict was
replayed from the deal before the solver returned it, so the solvable count is
a lower bound on the truth that cannot be inflated by a search bug. Every
`unsolvable` verdict is an exhaustive refutation, so the unsolvable count
bounds the truth from above. The deals that came back `unknown` could go either
way. So:

    solvable / n  <=  true winnability  <=  1 - unsolvable / n

Those are bounds on this sample, so the comparison has to allow for sampling
error too: the bracket actually tested is the Wilson lower bound of the
solvable rate to the Wilson upper bound of the not-proven-unsolvable rate.
Validation passes when the published figure lands inside it. It fails, and
means the solver is wrong rather than slow, when the figure falls outside —
a small sample gives a wide bracket and cannot fail this test by accident.

A restricted run (`--no-worry-back`) is summarised the same way but the
comparison is withheld: the published figure is the worry-back variant, so a
restricted run neither passes nor fails this test. It is measuring a different
game and would fail a comparison it was never making.

A `--both-arms` file holds both rulesets, and they are summarised one after the
other, never pooled. Pooling two different games would manufacture a figure
belonging to neither, and the risk is that it manufactures a *passing* one.

Usage: python3 analysis/klondike_validation.py results.jsonl [RULESET]
"""

import json
import math
import sys
from collections import Counter

# Solvitaire (Blake & Gent), thoughtful Klondike: 24-card stock drawn three at
# a time, unlimited redeals, worry-back permitted.
PUBLISHED = 0.81945
PUBLISHED_INTERVAL = 0.00084


def wilson(successes, total, z=1.96):
    """Wilson score interval, which behaves near 0 and 1 where the normal
    approximation does not."""
    if total == 0:
        return (0.0, 1.0)
    phat = successes / total
    denominator = 1 + z * z / total
    centre = (phat + z * z / (2 * total)) / denominator
    spread = z * math.sqrt(phat * (1 - phat) / total + z * z / (4 * total * total)) / denominator
    return (max(0.0, centre - spread), min(1.0, centre + spread))


def main(path, wanted=None):
    with open(path) as handle:
        rows = [json.loads(line) for line in handle if line.strip().startswith("{")]

    if not rows:
        sys.exit(f"no results in {path}")

    # Older files predate the field; those runs are all the full variant.
    for row in rows:
        row.setdefault("ruleset", "full")
    rulesets = sorted({row["ruleset"] for row in rows})
    if wanted is not None:
        if wanted not in rulesets:
            sys.exit(f"{path} holds {rulesets}, not {wanted!r}")
        rulesets = [wanted]

    for at, ruleset in enumerate(rulesets):
        if at:
            print()
            print("-" * 60)
            print()
        summarise([row for row in rows if row["ruleset"] == ruleset], ruleset)


def summarise(rows, ruleset):
    """One arm. Never called with two rulesets mixed together."""
    counts = Counter(row["verdict"] for row in rows)
    n = len(rows)
    solvable = counts["solvable"]
    unsolvable = counts["unsolvable"]
    unknown = counts["unknown"]

    low = solvable / n
    high = 1 - unsolvable / n

    print(f"ruleset          {ruleset}")
    print(f"deals            {n}")
    print(f"  solvable       {solvable:5}  {100 * solvable / n:5.1f}%   (each replayed to a win)")
    print(f"  unsolvable     {unsolvable:5}  {100 * unsolvable / n:5.1f}%   (each an exhaustive refutation)")
    print(f"  unknown        {unknown:5}  {100 * unknown / n:5.1f}%   (a limit stopped the search)")

    limits = Counter(row["limit"] for row in rows if row["verdict"] == "unknown")
    for limit, count in sorted(limits.items()):
        print(f"    {limit:12} {count:5}")

    # Allowing for sampling error: the loosest defensible claim about the
    # population, given what this sample proved about itself.
    floor = wilson(solvable, n)[0]
    ceiling = wilson(n - unsolvable, n)[1]

    print()
    print(f"in this sample, winnability is between {100 * low:.1f}% and {100 * high:.1f}%")
    print(f"allowing for sampling error, 95%:      {100 * floor:.1f}% to {100 * ceiling:.1f}%")

    print()
    if ruleset != "full":
        print(f"published  {100 * PUBLISHED:.3f}% +/- {100 * PUBLISHED_INTERVAL:.3f}%  "
              f"(the worry-back variant)")
        print("NOT COMPARED: this run suppressed worry-back, so it is a different game.")
        print("The restricted arm validates against no published figure. It exists to")
        print("exercise a dominance the published variant cannot reach; what it checks")
        print("is agreement with the same run made without that dominance.")
        return

    print(f"published  {100 * PUBLISHED:.3f}% +/- {100 * PUBLISHED_INTERVAL:.3f}%")
    if floor <= PUBLISHED <= ceiling:
        print(f"CONSISTENT: the published figure is inside the bracket "
              f"({100 * (ceiling - floor):.1f} points wide).")
        print("Narrowing it is a matter of search strength, not correctness.")
    else:
        print("INCONSISTENT: the published figure is outside what this run establishes.")
        print("The solver is wrong, not slow. Suspect the rules before the search:")
        print("  - king-only on an empty pile, and suffix moves off a pile")
        print("  - draw three, and redeals without limit")
        print("  - worry-back from the foundations")


if __name__ == "__main__":
    main(
        sys.argv[1] if len(sys.argv) > 1 else "-",
        sys.argv[2] if len(sys.argv) > 2 else None,
    )
