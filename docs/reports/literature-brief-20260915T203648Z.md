# gypsy-solve literature brief: exact patience-solver dominance conditions

**Research timestamp:** 2026-09-15T20:36:48Z  
**Project:** `gypsy-solve`  
**Primary source:** Charlie Blake and Ian P. Gent, *The Winnability of Klondike Solitaire and Many Other Patience Games*, JAIR 85, Article 21 (2026), DOI 10.1613/jair.1.17167.[1]

## Executive findings

1. The paper's safe-foundation rule is the familiar **opposite-colour within two ranks, other same-colour suit within three ranks** rule when worry-back is allowed.[7] The alternative wording “opposite within two, same within three” matches the paper.[10] The `r−1`/`r−2` wording is the same rule expressed with a zero-based candidate rank.[10]
2. Without worry-back, the paper adds the stronger condition **opposite-colour foundations within one rank**.[7]
3. The worry-back ban is a corollary of the safe-foundation theorem, not a separately proved theorem.[7] It forbids worrying a card back when it would be immediately safely movable to a foundation again.[3]
4. The incomplete-pile dominance is proved under a substantially more general condition and is not explicitly restricted to one deck.[3] Its stronger form requires the card above a moved partial pile to be built to foundation **immediately**.[3][7]
5. The published compatibility theorem concerns **safe foundation moves + immediate building after incomplete-pile moves**.[7] In the journal PDF it is **Theorem 5**.[7]
   The current arXiv v6 HTML renumbers the corresponding results as Theorems 2, 5, and 6; this is a numbering discrepancy, not a different compatibility claim.[2][3][4]
6. The foundation-dominance proof explicitly excludes multiple decks.[3] Therefore it does **not** justify enabling that dominance for two-deck Gypsy. The paper says the multi-deck adjustment is still needed; it does not say the rule is false for two decks.[7]
7. Of the six named two-deck presets in the brief, **only Spider is actually reported in Blake & Gent**.[3] The paper's other reported two-deck experiment is **Mrs Mop**.[7]
   There are no paper figures or confidence intervals for Gargantua, Gargantua with redeal, Forty Thieves, Ultra Klondike 2-deck, or Klondike two-deck 9.[3][4][26]
8. I found no peer-reviewed, preprint, or solver-scale numerical winnability figure for Gypsy/Gipsy in the searched corpus. There is a rules/code-generation report for Gypsy, but it does not supply a winnability estimate.[3][4][14]

## 1. Safe automove (“Keller's rule”): exact condition

### What the paper states

The journal PDF states in §5.4.1:

> In such games we can automatically move a card to the foundation if it is at most two more than the current card on foundations of the opposite colour and at most three more than the current card on foundation of the other suit of the same colour.[7]

The paper's example is foundations at `8♣, 7♦, 9♥, 8♠`; `10♥` is safe, while `J♥` is not.[7] This makes “current card” the **top card already on the foundation**, not the next card the foundation accepts.[3][7]

The paper first defines a card as “potentially safely buildable” when its same-suit predecessor is already on foundation (or it is the first card, normally an Ace), and when every card that could be moved onto it is itself potentially safely buildable. It then calls the card safely buildable when the foundation move is legal as well.[3][7]

### Rank convention

The paper does not declare a programming index such as zero-based or one-based.[3] Its ordinary card order is the usual foundation order `A, 2, ..., K`; elsewhere it describes foundation building as `A` to `K` and tableau building as one rank lower.[3]

For an exact implementation translation, use this notation:

- `r = 1..13` is the candidate card's ordinal rank, with `A=1` and `K=13`.
- `F_s` is the **top rank currently on** foundation suit `s`; for a formula-only representation, an empty foundation can be represented as `0`.
- The candidate must already be the legal next card for its own suit foundation.

With worry-back allowed, the paper's condition is:

```text
F_opp1 >= r - 2
F_opp2 >= r - 2
F_twin  >= r - 3
```

where `opp1` and `opp2` are the two opposite-colour suits and `twin` is the other suit of the candidate's colour.

Equivalently, if the candidate is represented by a zero-based rank `q=r−1` and foundation tops by zero-based heights `g=F−1` (empty = `−1`), the same condition is:

```text
g_opp1 >= q - 1
 g_opp2 >= q - 1
g_twin  >= q - 2
```

That is the wording recorded by the independent `lonelybot` audit: “Forced safe stack ... Keller's rule, opp ≥ r−1, twin ≥ r−2.”[10] The two formulations are not a threshold disagreement; they use different rank bases.[10] `Solvitaire` implements the same “opposite within 2, same within 3” test.[8]

### Without worry-back

The paper says:

> If a game does not allow worrying back, then we can use a slightly stronger rule. The rule as above applies but we can also move to foundation unconditionally if the card is no more than one higher ranked than the foundations of the opposite colour.[7]

Thus the no-worry-back condition is the previous condition **OR**:

```text
F_opp1 >= r - 1
F_opp2 >= r - 1
```

In zero-based candidate/top notation, the additional condition is `g_opp1 >= q` and `g_opp2 >= q`.

### Consequence for gypsy-solve

The current gypsy-solve checkout stores card ranks as `1..13`, stores foundation slots as their current top rank with `0` for empty, and currently generates rules-legal moves without applying a dominance. That representation matches the paper's top-rank interpretation.[3] The literature does **not** yet warrant turning this automove rule on for two-deck Gypsy, because the proof explicitly excludes duplicate decks; see §7 below.[7]

## 2. Worry-back ban and its theorem status

Section 5.4.1 says that when worry-back is allowed:

> We can ban worrying back from foundations to tableau if the card replaced on the tableau would be eligible for automatic movement to the tableau under the first dominance: such a move would lead to a pointless loop.[7]

The formal statement is Corollary 2 in the journal numbering / Corollary 3 in the current arXiv v6 HTML:

> We can correctly add a dominance that disallows worrying back a card from foundation if it would be immediately safely buildable after being worried back.[3][7]

This follows from the safe-move theorem: a compliant continuation would immediately put that card back on foundation, so the worry-back and rebuild cancel.[3] It is not a separate independent theorem about all possible worry-back moves. The ban applies to the specific case where the worried-back card is immediately safely rebuildable; it is not a blanket “never worry back” rule.[7]

The independent `lonelybot` ledger reaches the same reading and explicitly distinguishes the published worry-back ban from its own stock-side analogue.[10]

## 3. The compatibility theorem

The journal PDF's Theorem 5 states:

> If the conditions of Theorem 1 and Theorem 4 both apply, then any winnable instance has a winning sequence in which all moves are compliant with both dominances.[7]

The compatible pair is therefore:[7]

- **Theorem 1:** safe moves to foundations; and
- **Theorem 4:** immediate building to foundation after an incomplete tableau-pile move.

The proof shape is:[3][7]

1. Start with Theorem 4's result: choose a winning sequence in which every moved partial pile is followed immediately by foundation-building of the card above it.
2. Apply the safe-foundation proof's move-reordering process to that sequence.
3. If the last non-compliant move moved a card that was already safely buildable, delete that move and use the later first safe-foundation move for that card.
4. If the moved card was not safely buildable, swap the non-compliant move with the next safe-foundation move.
5. Check that the rewrite has not introduced a new incomplete-pile violation. The paper's case split shows either that the suspect partial-pile move was deleted or that the combination is impossible because the next move is a safe-foundation move.
6. Repeat using the decreasing measure “number of non-compliant moves, then the position of the last one.”

The arXiv v6 HTML calls the safe-foundation result Theorem 2, the incomplete-pile result Theorem 5, and the compatibility result Theorem 6.[3] The arXiv v5 HTML uses Theorem 1, Theorem 4, and Theorem 5 and places the proofs in Appendix C rather than Appendix B.[4] For the requested journal citation, use the journal PDF's numbering: §5.4.1, Theorem 5, Appendix B.1/B.2, approximately article pages 15–16 and 31–36.

## 4. Incomplete-pile dominance: full statement and conditions

Section 5.4.2 describes the weaker form implemented by Solvitaire: a tableau move of a built pile is allowed if either the **entire pile** is moved, or a **partial pile** is moved and the card immediately above the moved part can be built to foundation.[3][7] The proof establishes the stronger immediate form.[3][7]

The journal's Appendix B.2 theorem is:

> We consider any patience or solitaire game which: has a tableau which builds down according to an ‘indistinguishable’ build policy (as defined above); allows moves of complete or incomplete built piles as a single move according to the same policy as for individual cards; the only place a card can move from the tableau is to another tableau pile or to a foundation; is won by moving all cards to the foundations; and contains no rules invalidating moves by constraints on their order in the move sequence. For any instance of such a game, if the instance is winnable with the original rules, then it is also winnable with the restriction that an incomplete built pile may only be moved if the card above the moved partial pile is then built immediately to foundation.[3][7]

“Indistinguishable” means, in substance, that two cards have either identical or disjoint sets of possible build destinations, and that the same policy controls individual-card moves and group moves.[3] The paper explicitly excludes `different-suit` as a counterexample policy and notes that standard Spider is not covered because individual cards can move by any suit while groups can move only when they are one suit.[3]

This theorem is the one that generalizes beyond the standard four-suit/single-deck setting.[3] The paper gives as an example five identical decks with three suits when the build policy is same-suit.[3] That generality does **not** transfer to the safe-foundation theorem in Appendix B.1.[7]

## 5. Explicit assumptions: deck count and foundation multiplicity

| Rule/result | Single-deck assumption | Exactly one foundation pile per suit stated? |
|---|---|---|
| Safe-foundation definition and theorem (B.1) | **Yes, explicit.** The paper says it does not have multiple decks and that duplicate cards create edge cases not considered.[3] | **No explicit cardinality hypothesis found.** The prose assumes identifiable suit foundations and refers to “the other suit” and foundation tops, but does not state or prove a version with multiple foundation piles per suit. The paper is silent on that extension.[7] |
| Worry-back ban | **Inherits B.1's limit**, because it is the safe-foundation corollary.[3][7] | **Silent**, for the same reason. |
| Immediate incomplete-pile theorem (B.2) | **No single-deck restriction in the theorem.** It is deliberately generalized and gives a multiple-identical-deck example.[3] | **No explicit one-pile-per-suit hypothesis found.** The theorem requires a foundation goal and the stated move structure; it does not discuss multiple foundations per suit.[3] |
| Compatibility theorem | Requires both preceding theorems, so the safe-foundation side brings back the single-deck restriction.[7] | **Silent** on multiple foundation piles per suit. |

The safe-foundation theorem also assumes that move order does not itself change legality, because its proof permutes moves.[3][7] That condition matters independently of the deck-count issue.

## 6. Two-deck figures: what Blake & Gent actually report

The paper's Appendix A rule table contains two two-deck games: **Mrs Mop** and **Spider**.[3] Appendix D reports the corresponding experiments.[7]
The six names in the brief are not six two-deck results from this paper.[3][4]

| Requested game/preset | Present in Blake & Gent's reported two-deck tables? | Figure / 95% CI | Raw sample and unresolved fraction |
|---|---:|---:|---:|
| Gargantua | No | **N/A — no paper figure** | **N/A — no paper sample** |
| Gargantua with redeal | No | **N/A — no paper figure** | **N/A — no paper sample** |
| Spider | Yes | Solvitaire: **98.487% ± 1.513%**; best other: **99.9886% ± 0.0114%**[3] | Solvitaire: `9,731` winnable, `0` unwinnable, `269` unknown out of `10,000` = **2.69% unresolved**. Best-other raw data: `31,998` winnable, `0` unwinnable, `2` unknown out of `32,000` = **0.00625% unresolved**.[7] |
| Forty Thieves | No | **N/A — no paper figure** | **N/A — no paper sample** |
| Ultra Klondike 2-deck | No | **N/A — no paper figure** | **N/A — no paper sample** |
| Klondike two-deck 9 | No | **N/A — no paper figure** | **N/A — no paper sample** |

For completeness, the paper's other two-deck result is **Mrs Mop**: **97.992% ± 0.079%**, from `1,958,661` winnable, `38,969` unwinnable, and `2,370` unknown out of `2,000,000`; its unresolved fraction is **0.1185%**.[3][7]

The associated Figshare release is real and current, but it is a 6.64-GB tarred/gzipped archive of many CSV files; its landing page says individual CSV lines can be inconclusive run results.[12] I did not download that archive. The absence claims above are based on the paper's published rule/result tables and full extracted v5/v6 manuscript text, not on assuming that a named Solvitaire preset must have a published experiment.

## 7. Why foundation dominances are disabled for two decks

The paper gives the reason directly:

> We restrict consideration to games which involve building to foundation and moving all cards there to win. We also assume that we do not have multiple decks: i.e. while the theorem applies to a single deck with eight different suits of two colours, it does not apply to a game with two copies of the standard deck. The occurrence of duplicate cards leads to potential edge cases that we do not consider in this proof.[3][7]

Section 5.4.1 then says:

> All of the preceding discussion concerns single-deck games, since that is what our proof covers: some adjustment to the dominance would be necessary for multiple-deck games.[3][4][7]

The Solvitaire implementation matches that boundary: its foundation-dominance predicate returns false when `rules.two_decks` is true, and the general dominance generator also exits for two-deck rules.[8]

Therefore the correct interpretation is **unproven for two decks**, not “proven unsafe” and not “known false.” For two-deck Gypsy, enabling the rule would require a new duplicate-card proof or a verified game-specific replacement. The paper does not provide that proof.

## 8. Gypsy novelty check and close relatives

### Search result

I found no numerical winnability study for Gypsy or Gipsy in:

- Blake & Gent v5 and v6, including their rules, result, and literature-summary tables.[3][4]
- The named Solvitaire presets and source implementation.[8][26]
- The WPI 2019 report that models Gypsy as one of its solitaire families. That report is explicitly not editorially or peer reviewed and describes game construction rather than a winnability experiment.[14]
- The searched bibliographic/index results for `Gypsy solitaire`, `Gipsy solitaire`, and close relatives.

This is a **clean negative within that search scope**, not a claim that no unindexed private experiment or consumer-site statistic exists. I found no peer-reviewed, preprint, or solver-source figure with a defensible confidence interval for Gypsy.

### These are distinct rule sets

The names are close relatives, but they are not interchangeable games:

| Game | Rule-level identity relevant to Gypsy |
|---|---|
| **Gypsy** | Two decks; eight tableau piles of three, with the top card face up and the others face down; alternating-colour tableau building; alternating-colour groups may move; eight suit foundations; worry-back allowed; one card is dealt to each tableau pile from the stock.[18] |
| **Gargantua** | A two-deck Klondike relative with a nine-pile triangular tableau, one-card stock deals, two passes, and no worry-back in the cited rules description.[22] It is not Gypsy's eight piles of three with stock deals to all columns. |
| **Gargantua with redeal** | In the Solvitaire source this is a separate preset named `gargantua-redeal`, with the Gargantua layout and redeal enabled. It is a code-defined variant, not a result listed in Blake & Gent's paper.[26] |
| **Irmgard** | A Gypsy variant with an extra tableau pile and kings-only empty spaces; nine piles of three.[19] |
| **Blockade** | Two decks; twelve one-card tableau piles; same-suit building; empty spaces are automatically filled from stock before the stock is exhausted.[20] |
| **Miss Milligan** | Two decks; eight face-up one-card starting piles; alternating-colour building; stock deals to all columns; a pocket appears after the stock is exhausted for “waiving” a card or stack.[21] |
| **Forty Thieves** | Two decks; ten face-up piles of four; same-suit building; only single-card moves; one pass through the stock.[23] |
| **Spider** | Two decks; ten tableau piles; cards build down regardless of suit, but only same-suit sequences move as groups; one stock card is dealt to each tableau pile; complete suit sequences are removed rather than individual cards being built to ordinary suit foundations.[24] |
| **Ultra Klondike 2-deck** | A Solvitaire preset with 12 tableau piles, red-black building, a three-card deal, unlimited redeal, and two decks.[26] It is a solver preset, not a named game/result in Blake & Gent's paper. |
| **Klondike two-deck 9** | A Solvitaire preset with 9 tableau piles, red-black building, a three-card deal, unlimited redeal, and two decks.[26] It is a solver preset, not a named game/result in Blake & Gent's paper. |

The rules distinctions matter scientifically: a figure for Gargantua, Irmgard, or Blockade cannot be transferred to Gypsy merely because the game uses two decks.[18][19][20] A figure for Miss Milligan, Forty Thieves, or Spider cannot be transferred for the same reason.[21][23][24]

## 9. Has Spider solvability been studied specifically?

Yes, in several different senses and with very different evidentiary strength.

### Numeric winnability studies

- **Weisser, 2012 thesis.** The thesis analyzes the number of winning initial arrangements and reports a standard hard Spider simulation with `3,199/3,200 = 99.969%` found winnable.[15] Its abstract explicitly says: “A definitive answer is not reached.”[15] The extracted thesis material reports counts and percentages, but not a defensible 95% confidence interval for the standard-game result. Its standard hard-game sample therefore should be treated as an exploratory simulation, not as a final population estimate.
- **Robinson / Tranzoa solver page.** The current `plspider` page says: “Out of around 131,000+ games, 3 are currently unsolved,” and calls one-in-40,000 a “reasonable, round-numbers estimate,” while also warning readers not to rely on it.[13] This is useful solver evidence and the source behind the older result compared by Blake & Gent, but it is a non-peer-reviewed program page, not a confidence-interval study.
- **Blake & Gent, JAIR 2026.** This is the strongest directly relevant published numerical comparison found here. Their own Solvitaire experiment resolved `9,731/10,000`, with `269` unknown, giving the conservative interval **98.487% ± 1.513%**. Their “best other” comparison is **99.9886% ± 0.0114%**, based on `31,998/32,000` winnable and `2` unresolved.[3][7]

### Complexity result

Jesse Stern's *Spider Solitaire is NP-Complete* proves NP-completeness for a generalized SpiderSolitaire problem and begins from the standard two-deck/104-card game description.[17] That is a result about computational complexity of generalized instances, not a population winnability percentage for random standard Spider deals.[17]

So the answer to the brief's analogue question is **yes**: Spider has a dedicated thesis, dedicated solver studies, a complexity paper, and the 2026 JAIR comparison. Those sources are methodologically different and must not be collapsed into one “Spider win rate.”

## 10. Direct implications for gypsy-solve

- Do **not** enable the Blake–Gent/Keller safe-foundation dominance for two-deck Gypsy on the strength of Appendix B.1. The paper explicitly excludes that case.[3][7]
- The Appendix B.2 incomplete-pile theorem is the plausible literature starting point for a Gypsy port, because it is generalized beyond one deck; its exact hypotheses must be checked against Gypsy's move and group-build rules.[3][7]
- The worry-back ban is not independently licensed for two-deck Gypsy either: as a corollary it inherits the safe-foundation theorem's single-deck proof boundary.[3][7]
- The six named two-deck Solvitaire presets are not interchangeable evidence. Only Spider and Mrs Mop have the requested kind of result in Blake & Gent's paper; a Gypsy figure still needs to be generated from the Gypsy ruleset itself.[3][7][26]

## Sources

[1] https://doi.org/10.1613/jair.1.17167
    > "Published:
Feb 27, 2026"
[2] https://arxiv.org/abs/1906.12314
    > "Submitted on 28 Jun 2019 (v1), last revised 3 Mar 2026 (this version, v6)"
[3] https://arxiv.org/html/1906.12314v6
    > "All of the preceding discussion concerns single-deck games, since that is what our proof covers: some adjustment to the dominance would be necessary for multiple-deck games."
    > "Spider \[Th.\]\[Th.\] | 98.487±1.513%98.487\\pm 1.513\\% | 99.9886±0.0114%"
    > "We do automate the use of the dominance of"
[4] https://arxiv.org/html/1906.12314v5
    > "All of the preceding discussion concerns single-deck games, since that is what our proof covers: some adjustment to the dominance would be necessary for multiple-deck games."
[7] https://arxiv.org/pdf/1906.12314 — Blake & Gent, published manuscript PDF
    > "In such games we can automatically move a card to the foundation if it is at mosttwomore than the current card on foundations of the opposite colour and at mostthreemore than the current card on foundation of the other suit of the same colour"
    > "Theorem 5.If the conditions of Theorem1and Theorem4both apply, then any winnable instance has a winning sequence in which all moves are compliant with both dominances."
[8] https://raw.githubusercontent.com/thecharlieblake/Solvitaire/master/src/main/game/search-state/game_state.dominance_moves.cpp — Solvitaire foundation dominance implementation
    > "if (rules.foundations_only_comp_piles || rules.two_decks)
        return false;"
[10] https://raw.githubusercontent.com/vuonghy2442/lonelybot/main/docs/soundness_ledger.md — lonelybot soundness ledger
    > "Forced safe stack (5.1) — Keller's rule, opp ≥ r−1, twin ≥ r−2"
    > "Worry-back ban (5.4), incl. compatibility with C1"
[12] https://doi.org/10.6084/m9.figshare.8311070 — Patience Experimental Results dataset
    > "File is a tarred and gzipped archive of many files in a number of directories, the most important being many .csv files. Each line of a csv file contains an experimental result (not necessarily conclusive) of a run of Solvitaire on a randomly generated layout of a particular game of patience."
[13] https://www.tranzoa.net/~alex/plspider.htm — Tranzoa Winnable Spider Solitaire Games
    > "Out of around 131,000+ games, 3 are currently unsolved."
[14] https://digital.wpi.edu/downloads/cj82k8579 — Ciccarelli, MacGregor & Redding, Generating Solitaire Games
    > "WPI routinely publishes these reports on its web site without editorial or peer review."
    > "Gypsy was the last base solitaire game we developed."
[15] https://www.weissersolutions.com/media/cc362089536c6195ffff85e9ffffe905.pdf — Weisser, How Many Games of Spider Solitaire are Winnable?
    > "A definitive answer is not reached."
    > "Count=3199 of 3200 (99.969%)"
[17] https://arxiv.org/pdf/1110.1052 — Stern, Spider Solitaire is NP-Complete
    > "This paper will prove that a generalized version of the popular solitaire variant SpiderSolitaire is NP-Complete."
[18] https://politaire.com/help/gypsy — Politaire Gypsy rules
    > "A cross between Spider and Klondike."
[19] https://politaire.com/help/irmgard — Politaire Irmgard rules
    > "A variant of Gypsy where you have an extra tableau pile, but you can only fill spaces with kings."
[20] https://politaire.com/help/blockade — Politaire Blockade rules
    > "Twelve tableau piles of one card each, splayed downward."
[21] https://politaire.com/help/missmilligan — Politaire Miss Milligan rules
    > "A popular English game featuring a tableau that starts with just one card in each column where you have the option to "waive" stacks of cards off into a pocket after the stock runs out."
[22] https://politaire.com/help/gargantua — Politaire Gargantua rules
    > "A two-deck version of Klondike invented by Albert Morehead and Geoffrey Mott-Smith. You get two passes through the deck, dealing cards one at a time."
[23] https://politaire.com/help/fortythieves — Politaire Forty Thieves rules
    > "Similar to Forty and Eight, but the tableau has forty cards in ten stacks of four, and we only allow one pass through the deck."
[24] https://politaire.com/help/spider — Politaire Spider rules
    > "On the 10 tableau piles you can build down regardless of suit, but you can only move single suit sequences. When you click on the stock, one card will be dealt to each tableau pile."
[26] https://raw.githubusercontent.com/thecharlieblake/Solvitaire/master/src/main/input-output/input/sol_preset_types.cpp — Solvitaire named game presets
    > ""gargantua",

                R"(
{
  "tableau piles": {
    "count": 9,
    "build policy": "red-black","
    > ""ultra-klondike-2-deck",

                R"(
{
  "tableau piles": {
    "count": 12,
    "build policy": "red-black","
    > ""klondike-two-deck-9",

                R"(
{
  "tableau piles": {
    "count": 9,
    "build policy": "red-black","
