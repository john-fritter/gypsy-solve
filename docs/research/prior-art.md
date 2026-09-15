# Prior art: what other solvers already know

Surveyed 2026-09-15, after the Gypsy full arm came back 0 of 50 and the plan
called for more dominances with no candidate to offer.

**The headline: `DESIGN.md` was wrong.** It said the worry-back game "still has
no dominance at all" and that the next rule would need "a different shape of
argument". Both of the rules it wanted are published, proven, and shipping in
two independent solvers. We were re-deriving from scratch what the field
settled years ago.

Only source was reachable from this environment — arxiv, JAIR, Dagstuhl and
Semantic Scholar are all blocked by the egress proxy, so everything below is
read out of code and repo documentation rather than the papers themselves.
Statements attributed to Blake & Gent are quoted from those repos and should be
checked against the paper by someone who can open it. The questions that need
answering are written out in `literature-questions.md`, as sent to Gizmo.

## The two solvers

| | [Solvitaire](https://github.com/thecharlieblake/Solvitaire) | [lonelybot](https://github.com/vuonghy2442/lonelybot) |
|---|---|---|
| Authors | Charles Blake, Ian Gent (St Andrews) | Hy Vuong |
| Language | C++ | Rust |
| Scope | 73 variants of 35 games | Klondike only |
| Licence | **GPL-2** | MIT |
| Standing | the published 81.945% | claims state of the art for Klondike since 2024 |

**Licence note.** Solvitaire is GPL-2, so its *code* cannot be copied into this
repo. Nothing here proposes to. What is being taken is published rules and
their proofs, which are ideas in a paper, plus our own reading of what the
conditions are. lonelybot is MIT and its documentation is unusually good.

## 1. There is a safe-autoplay rule that survives worry-back

This is the finding that matters. It is **Keller's rule**, proven for the
standard game in Blake & Gent's Appendix B.1.

With red-black building and worry-back **allowed**, a card `q` of rank `r` and
colour `c` may be forced to the foundation when it is playable and

```
f(both opposite-colour suits) >= r - 1
f(the other suit of colour c, the "twin")  >= r - 2
```

The second conjunct is the whole difference. lonelybot's documentation is
explicit that it is not a fudge:

> the `r−2` twin condition is **not an engine relaxation**, it *is* the
> classical condition for games with worry-back. (The genuinely stronger
> classical variant — stackable when `f_opp ≥ r` — applies only to games
> *without* worry-back.)

Their worked example: foundations 8♣ 7♦ 9♥ 8♠ make 10♥ safe and J♥ unsafe.

**What this means for us.** Our safe autoplay checks only the opposite-colour
piles and is gated on worry-back being off. The published rule adds the twin
condition and then needs no gate. Porting it to two decks is the same exercise
we already did once — every suit has two foundation piles, so each `f(...)`
becomes a minimum over two piles, and the twin conjunct covers two more.

**And a thing to check about the rule we already shipped.** The no-worry-back
variant is quoted above as `f_opp >= r`. Ours uses `f_opp >= r - 1`. That may
be nothing — rank indexing differs between these codebases and the two might
be the same statement — but our rule being one rank *more permissive* than the
published one is exactly the direction that loses wins. It was validated
against Klondike without contradicting a verdict, which is real evidence, but
the discrepancy should be resolved against the paper rather than left.

## 2. There is a published worry-back dominance too

Never worry a card back while it is currently safe-automovable — "a pointless
loop". In lonelybot that is one mask operation (`stack_pile &= !dom_sm`); in
Solvitaire it is `dominance_blocks_foundation_move`.

It generalises Bjarnason, Tadepalli & Fern (2007), and lonelybot notes it needs
no separate compatibility proof with the safe-autoplay rule, because it falls
out of the same theorem: a solution that always plays a safely-playable card up
never worries one back, so both rules merely select among solutions that
theorem already guarantees exist.

This is the "delay a worry-back until it is needed" idea from the planning
discussion, in a sharper and already-proven form.

## 3. Neither solver has these rules for two decks

`game_state.dominance_moves.cpp`, first line of the function:

```cpp
if (rules.foundations_only_comp_piles || rules.two_decks)
    return false;
```

Solvitaire **switches auto-foundations off entirely for two-deck games**, and
the worry-back ban is built on top of that predicate, so it switches off too.
lonelybot is single-deck Klondike and does not face the question.

So the published proofs are for one deck. The two-deck versions are not
covered, and the four-pile correction we derived for our own safe autoplay has
no counterpart in either solver. That is the gap this project sits in — and it
means the ports need their own proofs, not citations.

## 4. Independent confirmation of our stock-order finding, and we are ahead

`game_state.cpp`:

```cpp
// If the stock deals to the tableau piles, there is no pile symmetry
if (rules.stock_size == 0 || rules.stock_deal_t != sdt::TABLEAU_PILES) {
    eval_pile_order(pr, true);
}
```

That is our 2026-09-11 finding, reached independently: a stock that deals card
*i* to column *i* destroys pile interchangeability. It killed two of our
dominances and gated our transposition key.

Their gate is coarser than ours. They disable pile symmetry for the **whole
game** when the stock deals to tableau piles; we gate on the stock being
*currently* empty and recover the symmetry for the endgame, which is where most
of the search sits. On this one point our implementation is strictly stronger
than the published solver.

## 5. Suit symmetry is not the free win it looks like

Worth recording because it was proposed and is now deprioritised.

Solvitaire classifies suit symmetry as a **streamliner**, which in their
vocabulary means an *unsound* speedup used to find wins faster, not a
dominance. What it actually does (`global_cache.cpp`) is replace a card's suit
with its **colour** in the cache key, so 5♥ and 5♦ hash alike. That is lossy:
foundations are per-suit, and two positions differing in which red suit sits
where are not the same position.

That is precisely the situation John described from play — needing to expose
the *specific* suit a foundation wants when its same-colour partner is the one
showing. Under colour-collapsed hashing the solver cannot tell those apart.

A full consistent relabelling (swap hearts↔diamonds *everywhere*, foundations
included) is a different and genuinely sound-looking construction, and
lonelybot has it as "twin-swap theorem T" — but its own soundness ledger rates
it `[~] argued with named gaps`, still awaiting proof at the state of the art.

**Conclusion: do not do this next.** The unsound version is a trap, the sound
version is unproven by people further along than us, and our Gypsy set proves
*zero* deals unsolvable — so a wrong state merge would be undetectable. Bad
risk, bad verifiability, and it is no longer the best rule available anyway.

## 6. The rest of the cascade, for later

Rules these solvers have that we do not, roughly in order of how well argued
they are:

| Rule | Status upstream |
|---|---|
| Forced safe stack (Keller) | proven, B&G App. B.1 |
| Worry-back ban | proven, corollary of their Theorem 1 |
| Incomplete-pile dominance | proven, B&G App. B.2 (Wolter 2014, Birrell 2018) |
| ≥3 redundant stackables → keep the lowest | argued, reduces to the two above |
| Deck/stock dominance | published for draw-1; draw-≥2 form is open |
| Twin-pair collapse | rests on the unproven twin-swap theorem |
| Least-stack cascade | lonelybot's own docs call it the least argued rule they have |
| King / empty-pile rules | argued |

lonelybot also runs **path-dependent pruners** separately from dominances — a
cycle pruner that forbids immediately reversing the last move, and rules keyed
on the last draw. Its `docs/pruning_dominance_interaction.md` is 569 lines on
how to compose those with dominances *and* a transposition table without
unsoundness. That is the same hazard we recorded when the capped-worry-back
design ran into path-state-versus-position-state, and it is worth reading
before building anything path-dependent here.

## 7. Nobody has published a Gypsy figure

Solvitaire's preset list has no Gypsy or Gipsy under any spelling. Its nearest
relatives:

- **Gargantua** — two decks, 9 piles, red-black, group moves, foundations
  removable. Gypsy with kings-only spaces and a different stock.
- **Spider** — two decks, and a stock that deals to the tableau piles, which is
  Gypsy's mechanic exactly.

So the closest published work covers games adjacent to Gypsy but not Gypsy, and
covers them *without* the foundation dominances, because they are two-deck.
`DESIGN.md`'s open question — whether a Gypsy winnability figure would be novel
— is still unanswered by a literature search proper, but it survives contact
with the reference work.

## What to do with this

In order:

1. **Port Keller's rule to two decks and ungate safe autoplay.** This is the
   first dominance the *full* Gypsy game has ever had. Prove the two-deck form,
   validate on the full Klondike arm where it fires and the 81.945% bracket
   applies.
2. **Add the worry-back ban** on top of it, which is nearly free once (1) is in.
3. **Resolve the `f_opp >= r` versus `>= r - 1` discrepancy** in the rule we
   already shipped.
4. Then reconsider the capped worry-back arm — it may not be needed.

Sources: [Solvitaire](https://github.com/thecharlieblake/Solvitaire),
[lonelybot](https://github.com/vuonghy2442/lonelybot), and through them Blake &
Gent, *The Winnability of Klondike Solitaire and Many Other Patience Games*,
JAIR 85 Article 21, 2026, doi:10.1613/jair.1.17167.
