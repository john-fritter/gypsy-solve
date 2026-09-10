# gypsy-solve

How often is two-deck Gypsy solitaire winnable, and how much of that depends on
worrying back? `DESIGN.md` has the exact ruleset, the questions, and the build
order; `DECISIONS.md` records what the implementation settled on and why;
`CLAUDE.md` has the working constraints. Reports from runs on the server are
kept under `docs/reports/`.

## Layout

```
core/   rules, state, move generation, seeded deals   (Rust)
cli/    deal, list moves, replay                      (Rust)
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
```

`--moves-file -` reads from stdin. In a move file, `#` starts a comment.

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
