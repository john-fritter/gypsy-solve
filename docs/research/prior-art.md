# Prior art: what other solvers already know

Surveyed 2026-09-15, after the Gypsy full arm came back 0 of 50 and the plan
called for more dominances with no candidate to offer.

**Revised the same day against the paper itself.** The first version of this
file was read entirely out of solver source code, because arxiv, JAIR and
Dagstuhl are blocked by this environment's egress proxy. Gizmo retrieved the
paper and answered the questions in `literature-questions.md`; the report is
`docs/reports/literature-brief-20260915T203648Z.md` and supersedes this file
wherever they disagree. Two of the first version's conclusions were wrong and
are corrected below.

Sources: Blake & Gent, *The Winnability of Klondike Solitaire and Many Other
Patience Games*, JAIR 85 Article 21 (2026), doi:10.1613/jair.1.17167 (arXiv
1906.12314); [Solvitaire](https://github.com/thecharlieblake/Solvitaire) (C++,
**GPL-2**, the implementation behind the paper);
[lonelybot](https://github.com/vuonghy2442/lonelybot) (Rust, MIT).

**Licence note.** Solvitaire is GPL-2 and this repo is not, so none of its code
can be copied here. What transfers is published rules and their proofs,
rewritten for two decks — which `CLAUDE.md`'s one-implementation constraint
requires anyway.

## 1. The safe-foundation rule, exactly

The paper, §5.4.1, with worry-back **allowed**:

> In such games we can automatically move a card to the foundation if it is at
> most two more than the current card on foundations of the opposite colour and
> at most three more than the current card on foundation of the other suit of
> the same colour.

"Current card" is the top card already on the foundation, not the next card it
accepts — fixed by their worked example, foundations `8♣ 7♦ 9♥ 8♠`, where `10♥`
is safe and `J♥` is not. Writing `r` for the candidate's rank (`A`=1, `K`=13)
and `F_s` for the top rank on suit `s` (empty = 0):

```
F_opp1 >= r - 2      F_opp2 >= r - 2      F_twin >= r - 3
```

With worry-back **suppressed** the paper adds a second, independent way to
qualify — the rule above *or*:

```
F_opp1 >= r - 1      F_opp2 >= r - 1
```

lonelybot's `opp >= r−1, twin >= r−2` is the same worry-back rule written on a
zero-based rank. There was never a threshold disagreement.

### Our shipped rule is sound, and the flag raised against it is withdrawn

`never_wanted_in_the_tableau` requires every opposite-colour slot to be at
`r - 1` or better. That is the paper's no-worry-back disjunct exactly,
generalised from two suit foundations to our four slots, and our foundations
already store the top rank with 0 for empty. The first version of this file
suspected it was one rank too permissive. **It is not.** No correction is owed.

What *is* true is that our rule is **weaker than the published one**: we
implement only the second disjunct, so we miss every card that qualifies under
the first. Adding it is free pruning for the restricted arm — subject to §3.

## 2. The worry-back ban is narrower than it first looked

Corollary 2 in the journal numbering:

> We can correctly add a dominance that disallows worrying back a card from
> foundation if it would be immediately safely buildable after being worried
> back.

Not a blanket ban — it only forbids worry-backs that would immediately undo
themselves, which the paper calls "a pointless loop". It is a corollary of the
safe-foundation theorem rather than an independent result, and so inherits
every restriction that theorem carries.

Our transposition table already catches the literal loop, since worrying a card
back and playing it straight up returns to a position already expanded. The
value here is cutting the move at generation instead of after a child is built.

## 3. The paper explicitly does not cover two decks

This is the correction that matters, and it kills the first version's headline
recommendation.

> We also assume that we do not have multiple decks: i.e. while the theorem
> applies to a single deck with eight different suits of two colours, it does
> not apply to a game with two copies of the standard deck. The occurrence of
> duplicate cards leads to potential edge cases that we do not consider in this
> proof.

and:

> All of the preceding discussion concerns single-deck games, since that is what
> our proof covers: some adjustment to the dominance would be necessary for
> multiple-deck games.

So Solvitaire's `if (rules.two_decks) return false;` is not an oversight or an
engineering shortcut — it is the proof boundary, honestly enforced.

**This is not "proven false for two decks", it is "unproven".** The distinction
matters: a two-deck version may well hold, but the paper does not supply it and
neither does either solver. Anything we adopt from §1 or §2 for Gypsy needs its
own duplicate-card proof, exactly as our existing safe autoplay did.

Note that our existing rule is *not* affected by this. We did not inherit it —
we proved the four-pile form ourselves and validated it on a Klondike deal set
that proves deals unsolvable. That proof stands on its own.

## 4. The incomplete-pile dominance is the one that generalises

Appendix B.2, and the first version of this file missed it entirely. Unlike the
safe-foundation theorem it is **deliberately generalised beyond a single deck**,
and the paper offers five identical decks as a worked example.

> We consider any patience or solitaire game which: has a tableau which builds
> down according to an 'indistinguishable' build policy; allows moves of
> complete or incomplete built piles as a single move according to the same
> policy as for individual cards; the only place a card can move from the
> tableau is to another tableau pile or to a foundation; is won by moving all
> cards to the foundations; and contains no rules invalidating moves by
> constraints on their order in the move sequence. For any instance of such a
> game, if the instance is winnable with the original rules, then it is also
> winnable with the restriction that an incomplete built pile may only be moved
> if the card above the moved partial pile is then built immediately to
> foundation.

"Indistinguishable" means two cards have either identical or disjoint sets of
build destinations, and the same policy governs single-card and group moves.

**Gypsy appears to qualify, and the reason is our permissive variant.** Two red
fives have identical destinations; a red five and a black five have disjoint
ones, so alternating-colour is indistinguishable. Group moves follow the same
policy as single cards, because any alternating-colour sequence moves as a unit
— which is precisely the rule `CLAUDE.md` forbids "correcting" toward the
textbook. The paper explicitly excludes *standard Spider* for failing this test:
its single cards move by any suit while its groups must be one suit. Gypsy has
no such mismatch.

Two hypotheses still need checking against our ruleset before this is claimed:

- **the stock.** Gypsy's stock deals a card to every column. Whether that
  counts as "a rule invalidating moves by constraints on their order" needs an
  argument, not an assumption — the same hazard that killed two dominances on
  2026-09-11.
- **worry-back.** The hypothesis restricts where a card may move *from* the
  tableau. It does not obviously forbid moves *into* the tableau from a
  foundation, but the proof should be read for a hidden dependence before the
  rule is used in the full arm.

If both clear, this is a dominance for the **full, worry-back Gypsy game** with
a published multi-deck-general proof behind it. Nothing else found comes close
to that.

## 5. Suit symmetry: not the free win it looked like

Solvitaire classifies suit symmetry as a **streamliner** — their word for an
*unsound* speedup used to find wins faster, not a dominance. What it implements
is replacing a card's suit with its **colour** in the cache key, so 5♥ and 5♦
hash alike. That is lossy: foundations are per-suit.

That is exactly the situation John raised from play — needing to expose the
specific suit a foundation wants while its same-colour partner is the one
showing. A full consistent relabelling is a different and sounder construction,
and lonelybot carries it as "twin-swap theorem T", but its own soundness ledger
rates that `[~] argued with named gaps`.

**Dropped.** The naive version is known unsound, the sound version is unproven
at the state of the art, and our Gypsy set proves *zero* deals unsolvable, so a
wrong state merge here would be undetectable.

## 6. Independent confirmation of our stock-order finding, and we are ahead

`game_state.cpp`:

```cpp
// If the stock deals to the tableau piles, there is no pile symmetry
if (rules.stock_size == 0 || rules.stock_deal_t != sdt::TABLEAU_PILES) {
    eval_pile_order(pr, true);
}
```

That is our 2026-09-11 finding, reached independently. Their gate is coarser:
they disable pile symmetry for the **whole game** when the stock deals to
tableau piles, where we gate on the stock being *currently* empty and recover
the symmetry for the endgame, which is where most of the search sits. On this
one point our implementation is strictly stronger than the published solver.

## 7. Two-deck results, and what they say about our odds

The paper reports exactly two two-deck games. The other two-deck names in
Solvitaire's preset list — Gargantua, Ultra Klondike 2-deck, Klondike two-deck
9, Forty Thieves — are code-defined presets with no published experiment.

| Game | Figure | Unresolved |
|---|---|---|
| Mrs Mop | 97.992% ± 0.079% | 2,370 of 2,000,000 — 0.12% |
| Spider (their run) | 98.487% ± 1.513% | 269 of 10,000 — 2.69% |
| Spider (best other) | 99.9886% ± 0.0114% | 2 of 32,000 |

Read this as calibration rather than encouragement. Mrs Mop shows a two-deck
game resolved to 0.12% unknown over two million instances, so two decks alone
are not disqualifying. Spider is the closer relative — two decks *and* a stock
dealing to every column, Gypsy's mechanic — and it is their **worst** resolved
fraction of the three, by a factor of twenty. Gypsy sits in the harder family,
with the added burden that the foundation dominances are unproven for it.

## 8. Nobody has published a Gypsy figure

A clean negative within the searched scope: no peer-reviewed, preprint, or
solver-source winnability figure for Gypsy or Gipsy. A 2019 WPI report models
Gypsy among other games but is explicitly not peer reviewed and generates games
rather than measuring winnability.

This answers a `DESIGN.md` open question. The relatives are genuinely different
games and their figures do not transfer: Irmgard adds a pile and restricts empty
spaces to kings; Blockade is twelve piles and same-suit building; Miss Milligan
starts one card per column and adds a pocket; Gargantua is two-deck Klondike
with a nine-pile triangular tableau and one-card deals. Spider is the closest on
stock mechanics and the furthest on building.

## 9. What to do with this

Reordered from the first version, which put a rule the paper does not cover for
two decks at the top.

1. **Test Gypsy against the incomplete-pile theorem's hypotheses** (§4),
   especially the stock and worry-back questions. If it qualifies, implement it:
   it is a published, multi-deck-general dominance and would be the first the
   full game has ever had.
2. **Strengthen the restricted arm** by adding the first disjunct of §1 to our
   safe autoplay, with the twin condition proved for four slots the way we
   proved the current rule. Free pruning where a dominance already runs.
3. **A two-deck proof of the worry-back-legal safe-foundation rule**, if anyone
   wants it. Not licensed by the paper — this would be extending it, and it is
   the same exercise we have already done once.
4. The capped worry-back arm stays the fallback it became, not the main line.

Do not treat (2) or (3) as ported from the paper. They are ours to prove.
