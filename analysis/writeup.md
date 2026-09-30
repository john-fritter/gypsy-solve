# How often can you win Gypsy?

**At least 90.7% of deals, with 95% confidence. Some deals cannot be won at
all. And returning cards from the foundations to the tableau, a move this
version allows, never turned out to be needed to win a deal.**

This is, as far as we could find, the first published winnability figure for
Gypsy. It is a lower bound, not an estimate, and this page says why.

## The short version

| | |
|---|---|
| Deals proved winnable | 925 of 1,000 |
| Deals proved unwinnable | 1 of 1,000, and 5 of 5,000 |
| Deals left undecided | 74 of 1,000 |
| **Winnable, 95% confidence** | **at least 90.7%** |
| Unwinnable, 95% confidence | at least 0.04% |
| Deals where worry-back made the difference | 0 |

Every winnable deal has a winning line that was replayed move by move from
the deal. Every unwinnable one had its entire reachable game searched, and
that search was repeated with all of the solver's shortcuts switched off. None
of the headline figures depends on the solver's shortcuts being correct.

## The game

Gypsy is a two-deck solitaire, and implementations disagree on its rules. **A
winnability figure means nothing without the ruleset, so here is this one
exactly:**

- Two 52-card decks, 104 cards. Eight tableau columns, eight foundations, one
  stock.
- The deal: one row of eight face down, then two rows face up on top. 24 cards
  are dealt and 80 stay in the stock.
- The tableau builds down in alternating colours. **Any alternating-colour
  sequence moves as a unit.** This is more permissive than classic Gypsy,
  which usually moves only same-suit runs. It makes the game easier.
- Any card, or any sequence, may go into an empty column.
- Foundations build up by suit from the ace.
- Each press of the stock deals one card to every column. The stock is never
  redealt.
- **Worry-back is allowed**: a card may come back off a foundation onto the
  tableau.

## What "winnable" means here

The solver sees everything: the face-down cards and the whole order of the
stock. A card still can't move until the rules let it. This is **thoughtful
winnability**, the standard measure in the literature. It is an upper bound on
how often a real player, who can't see the stock, could win.

For comparison, the reference figure for Klondike under the same measure is
81.945% ± 0.084% (Blake & Gent, JAIR 2026). That study covers single-deck games
only, and a search of the literature found no figure for Gypsy.

## How it was measured

**A search, with three possible answers.** The solver is a depth-first search
with a transposition table. It stops at a node budget. It reports
**winnable** (with a line, replayed before it is believed), **unwinnable**
(the whole reachable game searched), or **unknown** (it ran out of budget). An
unknown is never counted as a loss.

**Validated on Klondike first.** The same search, through the same generic
interface, solves Klondike in the variant the published figure uses: draw
three, unlimited redeals, worry-back allowed. On a thousand deals the Klondike
bracket contains the published figure at every budget tried:

| Budget | winnable | unwinnable | unknown | bracket, 95% |
|---:|---:|---:|---:|---|
| 3M nodes | 755 | 122 | 12.3% | 72.7% – 89.7% |
| 12M | 777 | 142 | 8.1% | 75.0% – 87.8% |
| 48M | 791 | 158 | 5.1% | 76.5% – 86.3% |
| 64M | 792 | 162 | 4.6% | 76.6% – 86.0% |

**Shortcuts, each proved for two decks.** A solver is only fast with
*dominances*: rules that skip moves which can never be the only way to win.
This one has three, all from Blake & Gent. Two rest on proofs that stop at a
single deck, and were re-proved for two. The third needed two conditions of its
own for Gypsy's stock, also proved. Testing found a shortcut for twos that was
wrong for two decks, and it was removed. Another candidate was shown to be
false and rejected. The two rules that force a move are also checked position
by position against a search that uses no shortcuts at all.

They matter for speed, not correctness. A win is a line replayed from the
deal, whatever found it. And each of the five unwinnable deals was
re-searched with every shortcut removed.

**Restarts.** Gypsy wins are long plunges that a search either walks into
early or misses. So most of the budget was spent as many short searches, each
of 1.5M nodes, under different move orderings. A short search cannot prove a
deal unwinnable, though, so the refutations come from separate long searches.

**The deals.** Deal *n* is a fixed shuffle of seed *n*, so anyone can recreate
it. Seeds 0–999 got the heaviest search. Seeds 0–4,999 got one long search
each, to hunt for unwinnable deals. Treating those seeds as a random sample of
deals was checked directly: across ten million deals, where each card lands,
how consecutive seeds relate, and how many aces start face up and face down
all match an exact shuffle, and a deliberately biased shuffle fails the same
tests.

## Results

### At least 90.7% winnable

On seeds 0–999, every run merged:

| | deals |
|---|---:|
| winnable | 925 |
| unwinnable | 1 |
| unknown | 74 |

If every unknown deal were winnable the sample would be 99.9% winnable; if
every one were lost, 92.5%. Allowing for sampling error, **the true share of
winnable deals is at least 90.7% (Wilson, 95%)**.

The same thousand deals without worry-back give 918, 1 and 81, so at least 89.9%.

### Why 74 are still unknown

More budget stopped paying:

| Budget per deal | unknown, of 1,000 |
|---:|---:|
| 12M nodes, as 8 searches | 211 |
| 48M, as 32 | 115 |
| 192M, as 128 | 82 |
| every run, merged | 74 |

Each fourfold step first cut the unknown count almost in half, then by less
than a third. By the last step, one more restart solved a remaining deal less
than once in a thousand tries. Deeper searches did no better for the same
effort, and neither did a smarter move ordering.

Where the budget goes explains it. On the hardest deals, 95–98% of the search
happens after the stock runs out, in a crowded endgame where one foundation of
each suit is stuck at or near its ace. The shortcut that plays cards up
automatically needs both copies of every opposite-colour card one rank lower
already on the foundations. In that endgame that almost never happens. A rule that tolerates one lagging foundation was tried,
and it is false: it loses winnable positions.

So the 74 are a mix of winnable deals with very deep wins and unwinnable deals
too large to search to the end, and nothing measured tells how many of each.

### Some deals cannot be won

A long search of each of 5,000 deals proved five unwinnable: seeds **188,
3796, 3966, 4260 and 4617**.

| Seed | nodes to exhaust, with shortcuts | without any |
|---:|---:|---:|
| 4617 | 3,211 | 1,012,261 |
| 3796 | 8,967,671 | pending |
| 4260 | 13,997,443 | pending |
| 188 | 19,801,449 | 1,076,602,384 |
| 3966 | 48,274,858 | pending |

Those counts are with worry-back allowed, and each deal is also unwinnable
without it. Seed 4617 is stuck almost from the deal. The others are large, but
far smaller than the hardest unknown deal examined, which was still finding
almost entirely new positions after 100 million.

Five in five thousand says **at least 0.04% of deals are unwinnable, with 95%
confidence.** That is an upper bound on winnability of 99.96%, a weak one,
because unwinnable deals are rare and each needs its own proof.

### Worry-back never mattered

Every deal was solved twice, with worry-back and without it. **No deal is
unwinnable without worry-back and winnable with it.** Each of the five
unwinnable deals stays unwinnable with worry-back, and there is no deal left
where the question is open.

Across the 5,000 deals, 33 were solved only with worry-back allowed. In each
of them the search without worry-back had simply run out of budget. None was
proved to need it, and more budget kept taking such deals away.
Worry-back is legal and never turned out to be needed. It multiplies the work
of proving a deal lost by 1.0 to 6.3 times, and changes no verdict.

### The opening aces

| Aces face up at the deal | deals | proved winnable |
|---:|---:|---:|
| 0 | 263 | 89.0% |
| 1 | 387 | 94.1% |
| 2 | 275 | 92.7% |
| 3 | 64 | 95.3% |
| 4 or more | 11 | 100% |

These are lower bounds per row, because the 74 unknown deals may not be spread
evenly. A deal with no ace showing is proved winnable less often. Beyond one
ace the pattern is flat. These thousand deals happen to have slightly fewer
face-up aces than an exact shuffle would give. Correcting for that moves the
headline by +0.17 points, so it is left as sampled.

## What this does not say

- **It is a lower bound.** The truth is somewhere from 90.7% to 99.96%, and
  nothing measured says where. Narrowing it needs a solver that can settle the
  74, not more deals.
- **It is this ruleset.** Classic Gypsy, which moves only same-suit runs as a
  group, allows fewer moves, so it can be no easier. Other implementations
  differ in other ways too.
- **It is thoughtful play.** A player who can't see the stock will win less
  often. By how much is a different question.

## Reproduce it

Everything is at
[github.com/john-fritter/gypsy-solve](https://github.com/john-fritter/gypsy-solve).

- Every result file behind these figures is in `docs/results/`, with
  checksums and provenance in `ARCHIVE-20260928.md`.
- `python3 analysis/gypsy_numbers.py` regenerates every number on this page
  from those files. It merges runs as a union of proved verdicts and refuses to
  continue if any two runs disagree. None do.
- Any single deal can be solved directly. Seed 188 without worry-back, for
  example, is proved unwinnable in 4,203,474 nodes:

  ```
  cargo run --release --bin gypsy -- solve --seed 188 --no-worry-back --budget 12000000 --table-mib 1024
  ```

- `cargo run --release --bin rule_free -- --seed 188` repeats a refutation with
  every shortcut removed.
- `cargo run --release --bin deal_stats` and
  `analysis/shuffle_uniformity.py` repeat the shuffle checks.
