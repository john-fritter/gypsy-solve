# gypsy-solve

How often is two-deck Gypsy solitaire winnable, and how much of that depends on
worrying back? `DESIGN.md` has the exact ruleset, the questions, and the build
order; `DECISIONS.md` records what the implementation settled on and why;
`CLAUDE.md` has the working constraints. Reports from runs on the server are
kept under `docs/reports/`.

## Layout

```
core/       rules, state, move generation, seeded deals   (Rust)
solver/     game trait, search, transposition table       (Rust)
klondike/   Klondike, for validating the solver           (Rust)
cli/        deal, list moves, replay, solve               (Rust)
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
gypsy klondike --seed 0 --deals 100 --json
gypsy batch  --game klondike --deals 1000 --workers 3 --out results.jsonl
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
`--budget` counts states expanded and `--max-depth` bounds the search stack,
and neither is a clock — so a verdict is reproducible on any machine.
`--max-depth` is a guard against a pathological descent eating memory rather
than a search parameter: no position is ever re-expanded because of it, and
hitting it means something is wrong rather than that it needs raising. `--json` prints one
flat object per deal; `--trace PATH` writes the winning line in a form
`gypsy replay --moves-file PATH --step` will walk.

The solver applies no dominances. It is the baseline that later cuts get
measured against, and on a real Gypsy deal it mostly returns `unknown`.

## Batch runs

`gypsy batch` solves many deals at once and is what the long runs use. It
appends one JSON object per deal as that deal finishes, so a run that is killed
keeps everything it had already proved, and `--resume` skips what is already
recorded — the answer to a kill is to run the same command again. A record left
torn by a kill is dropped on resume rather than appended to, which would
destroy the complete record after it.

It refuses to start when the run would not fit. Each worker holds its own
transposition table *and* its own search stack, and the stack is not small: at
the default `--max-depth` it can reach around 390 MiB, so four workers at 1 GiB
of table each want about 5.5 GiB rather than 4. `--min-free-mib` does the same
job for disk, checked as the run goes rather than only at the start.

Rows land in completion order, not seed order. Anything that cares sorts.

## Validation

`gypsy klondike` runs the same search over Klondike, which has a published
thoughtful winnability of 81.945% ± 0.084% (Solvitaire, Blake & Gent). The
variant implemented is the one that figure was measured on: 24-card stock drawn
three at a time, redeals without limit, worry-back permitted.

The point is that it is the *same* search — `gypsy_solver::solve` is generic
over a `Game` trait, and Gypsy and Klondike are two implementations of it.
Validating a separate solver would say nothing about the one that publishes.

Because every `solvable` verdict is replayed before it is returned, the measured
rate cannot exceed the truth by accident; it can only fall short when deals come
back `unknown`. So the measured rate is a lower bound, and how close it gets to
81.945% is the measure of the search rather than of Klondike.

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
