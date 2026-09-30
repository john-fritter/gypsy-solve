"""Test that seeded deals are a uniform sample of deals.

Every Wilson interval this project quotes assumes seeds `0..n` give i.i.d.
uniform deals. They are SplitMix64 Fisher-Yates shuffles of *sequential*
seeds, so that is a claim about the generator, and nothing had tested it
directly until this. It reads the counts written by

    cargo run --release --bin deal_stats -- --from A --to B > counts.json

and compares each table with its exact expectation under a uniform shuffle.

- **position** (104 slots x 52 cards): each card should be equally likely at
  every slot.
- **consecutive** (52 x 52): the card at a slot in deal `s` against the card at
  the same slot in deal `s + 1`. This is the test aimed at sequential seeds:
  independent deals give the product of the marginals.
- **aces face up** and **aces face down** in the opening deal: 8 aces among
  104 cards, 16 face up and 8 face down, so both are hypergeometric. These are
  the statistics the game actually sees.

The first two tables are sums of per-deal tables whose margins are fixed (each
slot holds one card, each card appears twice), so Pearson's statistic runs
high by N/(N-1) with N = 104. It is scaled by (N-1)/N before the chi-square
tail is taken; unscaled it would drift about a third of a standard deviation
high on a correct shuffle.

**Pass means every p-value is at least 0.001.** Four tests per file and a few
files per run, so that threshold keeps a false alarm on a correct shuffle
below about one per cent. A pass shows the deals match a uniform shuffle on
these statistics. It is necessary for the i.i.d. assumption, not a proof of
it.

Run the `--control naive` counts too: that shuffle is known to be biased and
must fail, or the test cannot see what it is looking for.

Usage: python3 analysis/shuffle_uniformity.py counts.json [counts.json ...]
"""

import json
import math
import sys

THRESHOLD = 0.001


def chi2_sf(x, df):
    """P(chi-square with df degrees of freedom > x): the regularised upper
    incomplete gamma Q(df/2, x/2), by series or continued fraction."""
    a, x = df / 2.0, x / 2.0
    if x <= 0:
        return 1.0
    log_front = a * math.log(x) - x - math.lgamma(a)
    if x < a + 1:
        term = total = 1.0 / a
        n = a
        while abs(term) > abs(total) * 1e-15:
            n += 1
            term *= x / n
            total += term
        return max(0.0, 1.0 - total * math.exp(log_front))
    # Lentz's method for the continued fraction.
    tiny = 1e-300
    b = x + 1 - a
    c = 1 / tiny
    d = 1 / b
    h = d
    i = 0
    while True:
        i += 1
        an = -i * (i - a)
        b += 2
        d = an * d + b
        d = tiny if abs(d) < tiny else d
        c = b + an / c
        c = tiny if abs(c) < tiny else c
        d = 1 / d
        delta = d * c
        h *= delta
        if abs(delta - 1) < 1e-15:
            break
    return math.exp(log_front) * h


def uniform_table(table, deck):
    """Scaled Pearson statistic of a table against the product of its margins.
    Both tables here have margins fixed by construction (every slot holds one
    card per deal, every card appears twice), so that product is the exact
    uniform expectation rather than an estimate of it."""
    rows = [sum(row) for row in table]
    cols = [sum(col) for col in zip(*table)]
    total = sum(rows)
    stat = 0.0
    for r, row in zip(rows, table):
        for c, observed in zip(cols, row):
            expected = r * c / total
            stat += (observed - expected) ** 2 / expected
    df = (len(rows) - 1) * (len(cols) - 1)
    return stat * (deck - 1) / deck, df


def hypergeometric(counts, deck, aces, drawn):
    """Goodness of fit of an ace-count histogram, with tail bins pooled until
    every expected count is at least five."""
    deals = sum(counts)
    pmf = [math.comb(aces, k) * math.comb(deck - aces, drawn - k) / math.comb(deck, drawn)
           for k in range(len(counts))]
    bins = []  # (observed, expected), pooled from the upper tail down
    obs = exp = 0.0
    for k in reversed(range(len(counts))):
        obs += counts[k]
        exp += pmf[k] * deals
        if exp >= 5:
            bins.append((obs, exp))
            obs = exp = 0.0
    if exp > 0:  # anything left over at the bottom joins the last bin formed
        o, e = bins.pop()
        bins.append((o + obs, e + exp))
    stat = sum((o - e) ** 2 / e for o, e in bins)
    return stat, len(bins) - 1


def main(paths):
    failed = False
    for path in paths:
        with open(path) as f:
            data = json.load(f)
        deck = data["deck"]
        deals = data["to"] - data["from"]
        label = f"seeds {data['from']}..{data['to']} ({deals:,} deals)"
        if data["control"]:
            label += f", CONTROL {data['control']}"
        print(label)
        tests = [
            ("card position", uniform_table(data["position"], deck)),
            ("consecutive seeds", uniform_table(data["consecutive"], deck)),
            ("aces face up", hypergeometric(data["aces_face_up"], deck, 8, data["face_up"])),
            ("aces face down", hypergeometric(data["aces_face_down"], deck, 8, data["face_down"])),
        ]
        worst = 1.0
        for name, (stat, df) in tests:
            p = chi2_sf(stat, df)
            worst = min(worst, p)
            shown = f"{p:.4f}" if p >= 1e-4 else f"{p:.1e}" if p > 0 else "<1e-300"
            print(f"  {name:<18} chi2 {stat:>12.1f}  df {df:>5}  p {shown}")
        verdict = "pass" if worst >= THRESHOLD else "FAIL"
        if data["control"]:
            verdict += " (a control must fail)" if worst >= THRESHOLD else " (as a control must)"
            failed |= worst >= THRESHOLD
        else:
            failed |= worst < THRESHOLD
        print(f"  {verdict}\n")
    return 1 if failed else 0


if __name__ == "__main__":
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    sys.exit(main(sys.argv[1:]))
