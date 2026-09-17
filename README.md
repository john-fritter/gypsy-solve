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
cli/        deal, list moves, replay, solve, batch        (Rust)
analysis/   summarising results files                     (Python)
```

The Rust/Python boundary is the results files under `docs/results/`. Nothing
calls across it.

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
gypsy klondike --seed 0 --deals 50 --no-worry-back
gypsy batch  --game klondike --deals 1000 --workers 3 --out results.jsonl
gypsy batch  --game gypsy --deals 50 --both-arms --out both.jsonl
gypsy batch  --game gypsy --top-rank 4 --deals 20000 --both-arms --out small.jsonl
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
hitting it means something is wrong rather than that it needs raising.

`--json` prints one flat object per deal, naming the game and ruleset that
produced it; `--trace
PATH` writes the winning line in a form `gypsy replay --moves-file PATH --step`
will walk. `gypsy klondike --json` carries the whole line too, because what a
winning line contains is a measurement in its own right — see
`analysis/worry_back_usage.py`, which counts how much worry-back wins actually
use.

The solver applies three dominances, each with a proof that it cannot discard a
winning line. All are implemented once per game rather than shared, because the
proofs differ.

**Safe autoplay, restricted game only.** With `--no-worry-back`, a card that can
never be wanted in the tableau again is played up and nothing else is considered
at that position. The gate is the proof: worry-back lets the cards the rule
checks come back down, and then it proves nothing. Gypsy checks four
opposite-colour foundation piles because two decks give each suit two; Klondike
checks two, and additionally excludes the waste, because playing a card off it
re-aligns every later draw-three.

An ace qualifies with the foundations empty, because nothing stacks on an ace. A
two does not: until every opposite-colour ace is up, one may still want to sit on
it. Both games waved twos through until 2026-09-17, when the rank-4 deal set
produced deals the restricted arm wrongly proved unsolvable — see
`DECISIONS.md`.

**Safe foundation plays with worry-back legal**, in both games. A card whose
opposite-colour foundations are within a rank, whose same-colour twin suit is
within two, and — for Gypsy, where duplicate cards force it — whose own suit's
second slot is level with the first, is played up and nothing else is
considered. This is Keller's rule; the paper proves it for one deck and excludes
duplicate cards in terms, so the two-deck form is proved on
`Gypsy::legal_actions` rather than inherited. It is the first rule the *full*
game has that forces a move.

**Never split a built run to expose a dead card**, in both games. A move
carrying part of a built run is not offered when the card it would uncover has
no foundation to go to: splitting a run frees the card beneath it, and splitting
it to free a dead card is a shuffle. This is Blake & Gent's Theorem 4, which
unlike their safe-foundation rule is proved for games with duplicate cards, and
which reaches Gypsy because any alternating-colour sequence moves as a unit —
the theorem needs one policy for single cards and for groups. Gypsy gates it on
an exhausted stock and a non-empty destination, both proved on
`Gypsy::legal_actions` rather than inherited; Klondike needs neither gate.

The second rule is the only one the **full** game has, and it is what took the
Gypsy worry-back arm from resolving nothing to resolving most of a 50-deal
sample. Hard deals still return `unknown`.

Both rules apply at every position, so the pair needs an argument of its own and
not just one per rule. It is on `Gypsy::legal_actions`: an induction on line
length, which works because exactly one of the two rules governs any one
position and neither rewrite lengthens a line.

`cargo run --release --bin branching` reports how much of move generation each
kind of move accounts for, which is how the split-run rule was sized before it
was built.

`cargo run --release --bin compose` runs that composition argument instead of
restating it. Given a recorded winning line it applies whichever rewrite each
position calls for, and checks that what comes out replays to a win, is offered
by the generator at every position, and is no longer than the line that went in:

```
compose --results docs/results/gypsy-nwb-5M-depth-sweep.jsonl --arm both
compose --seed 15 --moves-file line.txt --arm full --delay-foundations
```

`--delay-foundations` pushes every foundation play as late as it will commute
before the construction runs. A line from the solver already forces its safe
cards up, so without it the forcing rule's rewrite is never exercised. Set
`COMPOSE_TRACE=1` for the step-by-step. `docs/results/gypsy-composition-construction.jsonl`
is the recorded run.

## Restarts

`--restarts k` splits the budget into *k* searches, each under a different move
ordering, and stops at the first one that decides the deal. It is on `solve`,
`klondike` and `batch`, and defaults to 1, which is the search as it was before
restarts existed.

Nothing about the three-valued verdict changes. Each restart is a whole search:
a win is replayed from the deal before it is believed, and `unsolvable` is still
claimed only by a search that exhausted the reachable game without touching a
limit — a proof whatever ordering produced it. **A restart can turn `unknown`
into a decision and can never turn a decision into anything else.** The first
slice always runs the game's own ordering, so a restart run begins with the
deterministic one.

The ordering is shuffled *within* the bands the game already sorts into, seeded
from the position, so two routes to the same position still generate the same
children in the same order and a verdict is still reproducible from its seed.

**It is worth a great deal on Gypsy and nothing on Klondike**, and the reason is
line length. Gypsy wins run 1,301 to 99,982 moves and are walked into or missed;
Klondike wins run 136 to 467 and its unknown deals are simply large. See
`DECISIONS.md`.

## Batch runs

`gypsy batch` solves many deals at once and is what the long runs use. It
appends one JSON object per deal as that deal finishes, so a run that is killed
keeps everything it had already proved, and `--resume` skips what is already
recorded — the answer to a kill is to run the same command again. A record left
torn by a kill is dropped on resume rather than appended to, which would
destroy the complete record after it.

`--game klondike --no-worry-back` runs the restricted validation arm; the
`ruleset` field in each record says which arm produced it.

`--both-arms` solves each deal in *both* rulesets and writes a record for each,
which is the experiment `DESIGN.md` describes — every deal solved twice — and
is cheaper than two separate runs, because each arm's proof is carried to the
other wherever that is sound:

- a restricted `solvable` is a full `solvable`, since the restricted game's
  moves are a subset, so the line replays in the full game move for move — and
  it *is* replayed there, not assumed;
- a full `unsolvable` is a restricted `unsolvable`, since the exhausted larger
  search covered every position the subset could reach.

Neither carries back. A full win may have used worry-back, and a restricted
refutation says nothing about a game with more moves in it — that gap is the
worry-back delta itself. The `verdict_from` field on every record names the arm
that established the verdict, `search` or the ruleset it came from; a carried
verdict reports zero nodes because it spent none. Measured on 50 Gypsy deals at
5M, the carry takes the full arm from 0 resolved to 12.

Both records for a deal are written together, and resume treats a deal as done
only when every ruleset the run writes is present, so a kill between them costs
that one deal rather than silently dropping an arm.

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

`analysis/klondike_validation.py results.jsonl` turns a run into that bracket.
It summarises each ruleset in a file separately and never pools them, and it
refuses any run that does not say `"game":"klondike"` — an unlabelled one
included. Comparing another game against a Klondike figure would manufacture a
result, and the dangerous case is that it manufactures a passing one.

`--no-worry-back` runs the same deals with foundation-to-tableau moves
suppressed. That arm matches no published figure and is not offered as one; it
exists because a dominance provable only with worry-back off never fires in the
published variant, so that variant cannot check it. It is where the
safe-autoplay rule is tested, and it is the only set here that proves deals
unsolvable in numbers — which is the direction a wrong dominance fails in.

## Move notation

```
S          deal one card from the stock onto every column
T0>T3:2    move two cards from column 0 onto column 3
T4>F1      play column 4's top card to foundation slot 1
F1>T4      worry back: foundation slot 1's top card onto column 4
```

Foundation slots run 0-7, two per suit: 0-1 spades, 2-3 hearts, 4-5 clubs,
6-7 diamonds. The two slots of a suit are interchangeable.

## Reduced deals

`--top-rank N` deals from a deck of ranks `A..=N` instead of `A..=K`. It is on
`deal`, `moves`, `replay`, `solve` and `batch`, and defaults to 13, which is the
game.

Everything else is untouched: two decks, four suits, eight columns, eight
foundation slots, alternating-colour building, any sequence moving as a unit,
worry-back, and a stock still dealing one card to every column — `8N - 24`
divides by 8 for every cap, so the stock never deals a short row. A capped deal
is a smaller instance of this game, not a variant of it, and `--top-rank 13`
reproduces the frozen deck card for card.

**It exists because the Gypsy arm proves nothing unsolvable.** Every Gypsy
verdict the project has is `solvable` or `unknown`, and a win replayed from the
deal cannot catch the error a bad dominance makes: discarding the only winning
line turns a solvable deal into `unsolvable`, and with no deal proved unsolvable
there is nothing to notice it on. Klondike supplies refutations but is
single-deck, so it never exercises duplicate cards, group moves as a unit, or
the deal-to-every-column stock — the three things every two-deck proof here had
to extend past.

At `--top-rank 4` the search exhausts every deal. Over 200,000 deals, 23 are
genuinely unsolvable — one in about 8,700, the same 23 in both arms. Those are
the only positions that test a Gypsy dominance in the direction it fails.

**The first thing they found was a bug in a shipped dominance.** Safe autoplay
proved four of those 200,000 deals `unsolvable` when they are winnable, because
it waved every two straight to the foundation. Fixed the same day, in both
games; the four now come back solvable and the 23 genuine refutations stand. See
`DECISIONS.md`.

`--top-rank 5` and above resolve too, but the refutations disappear: more ranks
make the game *more* winnable, not less, because the tableau stays eight columns
wide while the deck grows. So the cap cannot be walked up toward the real game
and keep its teeth, and cap 4 has runs of at most four cards and a single stock
deal, which exercises the split-run rule weakly. Broken on purpose, the set
catches a rule that forces everything (6 verdicts in 20,000) and misses a
one-rank error entirely (0 in 20,000). It is a smoke test, not validation.

## Seeds

A seed fixes the whole deal, stock order included. The deck order and the
shuffle (SplitMix64, in `core/src/rng.rs`) are frozen: changing either changes
what every published seed means, and a test pins one deal to catch that.
