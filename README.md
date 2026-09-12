# gypsy-solve

How often is two-deck Gypsy solitaire winnable, and how much of that depends on
worrying back? `DESIGN.md` has the exact ruleset, the questions, and the build
order; `DECISIONS.md` records what the implementation settled on and why;
`CLAUDE.md` has the working constraints. Reports from runs on the server are
kept under `docs/reports/`.

## Layout

```
core/     rules, state, move generation, seeded deals   (Rust)
solver/   search, transposition table                   (Rust)
cli/      deal, list moves, replay, solve               (Rust)
```

## Build

```
cargo test
cargo build --release
```

## Use

```
gypsy deal   --seed 42
gypsy moves  --seed 42 --moves "T7>F2"
gypsy replay --seed 42 --moves-file line.txt --step
gypsy solve  --seed 42 --budget 10000000 --trace line.txt
```

`--moves-file -` reads from stdin. In a move file, `#` starts a comment.

## Solving

`gypsy solve` searches a deal and reports one of three verdicts:

| Verdict | Means |
|---|---|
| `solvable` | a winning line was found, and replayed from the deal to check it |
| `unsolvable` | the whole reachable game was searched; there is no win |
| `unknown` | a limit stopped the search; nothing is proven either way |

`unknown` is never rounded to `unsolvable`. Both limits are deterministic —
`--budget` counts states expanded and `--max-depth` counts moves, and neither
is a clock — so a verdict is reproducible on any machine. `--json` prints one
flat object per deal; `--trace PATH` writes the winning line in a form
`gypsy replay --moves-file PATH --step` will walk.

The solver applies no dominances. It is the baseline that later cuts get
measured against, and on a real deal it mostly returns `unknown`.

## Move notation

```
S          deal one card from the stock onto every column
T0>T3:2    move two cards from column 0 onto column 3
T4>F1      play column 4's top card to foundation slot 1
F1>T4      worry back: foundation slot 1's top card onto column 4
```

Foundation slots run 0-7, two per suit: 0-1 spades, 2-3 hearts, 4-5 clubs,
6-7 diamonds. The two slots of a suit are interchangeable.

## Seeds

A seed fixes the whole deal, stock order included. The deck order and the
shuffle (SplitMix64, in `core/src/rng.rs`) are frozen: changing either changes
what every published seed means, and a test pins one deal to catch that.
