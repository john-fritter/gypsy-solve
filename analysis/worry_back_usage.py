"""How much worry-back a winning line actually uses.

Worry-back is what makes the full game expensive: it adds an action for every
foundation-and-pile pair, it makes the state graph cyclic, and it is the reason
the only dominance this project has is gated off in the published arm.
`DESIGN.md` sanctions capping the count as a valid lower bound. Before building
a cap, this asks whether one would bind at all.

Two questions, and the second is the stronger one:

1. **How many worry-backs does each winning line contain?** Both games print a
   worry-back as `F<slot>>T<pile>`, so one counter serves both. Read this as an
   upper bound on what a cap must allow, not as what the win required: the move
   ordering tries worry-back second to last, so these lines are biased toward
   few. The bias runs the right way — a line using k worry-backs witnesses that
   a win exists within k.

2. **Is the deal winnable with worry-back off altogether?** Two independent
   witnesses, and either settles it. A restricted-arm run that returns
   `solvable` is one. A full-arm line containing no worry-back at all is the
   other, and it is worth having because the restricted arm runs at a smaller
   budget and returns `unknown` on deals it simply could not afford: suppressing
   worry-back removes the foundation-to-tableau actions and nothing else, so a
   line that uses none is already a line of the restricted game.

Usage:

    worry_back_usage.py WINS.jsonl [--restricted RESTRICTED.jsonl]

Both inputs are the JSON records `gypsy klondike --json` and `gypsy solve
--json` write. `WINS.jsonl` must carry the `line` field; the restricted arm is
read for its verdicts only.
"""

import json
import sys
from collections import Counter


def worry_backs(line):
    """Worry-back moves in a line. Both games print them as `F..>T..`."""
    return sum(1 for move in line.split() if move.startswith("F") and ">T" in move)


def load(path):
    with open(path) as handle:
        return {
            record["seed"]: record
            for record in (json.loads(t) for t in handle if t.strip())
        }


def main(wins_path, restricted_path):
    wins = {
        seed: record
        for seed, record in load(wins_path).items()
        if record["verdict"] == "solvable" and record.get("line")
    }
    if not wins:
        print("no winning lines in the input", file=sys.stderr)
        return 1
    restricted = load(restricted_path) if restricted_path else {}

    counts = Counter()
    without = {}
    header = f"{'seed':>6}  {'moves':>6}  {'worry-backs':>11}  {'nodes':>10}"
    if restricted:
        header += "  wins without worry-back"
    print(header)

    for seed, record in sorted(wins.items()):
        used = worry_backs(record["line"])
        counts[used] += 1
        row = (
            f"{seed:>6}  {record['line_length']:>6}  "
            f"{used:>11}  {record['nodes']:>10}"
        )
        if restricted:
            # Either witness settles it; the line is checked first because it
            # is the one that does not depend on the restricted arm's budget.
            if used == 0:
                witness, without[seed] = "yes (its line uses none)", True
            elif restricted.get(seed, {}).get("verdict") == "solvable":
                witness, without[seed] = "yes (restricted arm)", True
            else:
                witness, without[seed] = "not established", False
            row += f"  {witness}"
        print(row)

    total = len(wins)
    print(f"\n{total} winning lines")
    print(f"{'worry-backs':>11}  {'lines':>5}  {'cumulative':>10}")
    seen = 0
    for used in sorted(counts):
        seen += counts[used]
        print(f"{used:>11}  {counts[used]:>5}  {seen / total:>9.1%}")
    print(f"most in any one line: {max(counts)}")

    if restricted:
        settled = sum(without.values())
        print(
            f"\n{settled} of {total} are winnable with worry-back off, proven."
            f" {total - settled} not established either way:"
            f" {sorted(s for s, ok in without.items() if not ok)}"
        )
        print("An unestablished deal is not a deal that needs worry-back.")
    return 0


if __name__ == "__main__":
    argv = sys.argv[1:]
    restricted = None
    if "--restricted" in argv:
        at = argv.index("--restricted")
        restricted = argv[at + 1]
        argv = argv[:at] + argv[at + 2 :]
    if len(argv) != 1:
        print(__doc__, file=sys.stderr)
        raise SystemExit(2)
    raise SystemExit(main(argv[0], restricted))
