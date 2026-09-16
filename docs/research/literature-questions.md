# Open questions for the literature, 2026-09-15

Written for Gizmo, who can reach the sources this dev environment cannot —
arxiv, JAIR, Dagstuhl and Semantic Scholar are all blocked by the egress proxy
here, so `prior-art.md` is read entirely out of source code and repo
documentation. Everything in it that is attributed to Blake & Gent is quoted
second-hand and needs checking.

Kept in the repo because an unanswered question is worth as much as an answered
one, and because the answers decide whether a rule we already shipped is sound.

The request as sent follows.

---

**Extract exact dominance conditions from the patience-solver literature.**

`gypsy-solve` solves two-deck Gypsy. We have one dominance rule, and we have
just found from reading two open-source solvers that the rules we still need
are already published. We cannot reach the papers from the dev box. You can.

Primary source: Blake & Gent, *The Winnability of Klondike Solitaire and Many
Other Patience Games*, JAIR 85 Article 21 (2026), doi:10.1613/jair.1.17167;
arXiv preprint 1906.12314. Section 5.4 and Appendices B.1 and B.2 are the parts
that matter.

**Why exactness is the whole point.** We are going to rewrite these rules for a
two-deck game. An off-by-one in a rank threshold is the difference between a
sound dominance and one that silently discards winning lines and reports
winnable deals as unwinnable — an error invisible in our output. Paraphrase is
not useful. We need conditions as the paper states them, with its rank
convention made explicit: 0- or 1-indexed, and whether "the foundation is at
*r*" means its top card is *r* or that *r* is the next card it accepts.

Questions, in priority order:

1. **The safe-automove condition ("Keller's rule") in Appendix B.1.** The exact
   statement, in both the worry-back-legal and the worry-back-suppressed form if
   the paper gives both. Two open-source solvers disagree in a way we cannot
   resolve: one states the worry-back form as *both opposite-colour foundations
   at least r−1, and the other suit of the same colour at least r−2*; the other
   implements what reads as *opposite within 2, same within 3*. Which matches
   the paper, and what is the without-worry-back form? This decides whether the
   rule we have already shipped is sound.

2. **The worry-back ban in §5.4** — its statement, and the theorem it follows
   from (we believe Theorem 1). Whether the paper proves it separately or as a
   corollary.

3. **Theorem 5** — what pair of dominances it proves compatible, and the shape
   of the compatibility argument.

4. **The incomplete-pile dominance in Appendix B.2**, attributed to Wolter
   (2014) and Birrell (2018) — full statement and conditions.

5. **For every rule above: does the proof anywhere assume a single deck, or
   that each suit has exactly one foundation pile?** This is the question we
   most need answered and the one least likely to be stated outright. If the
   paper is silent, say it is silent — do not infer.

6. **Winnability figures for the two-deck games the paper covers** — Gargantua,
   Gargantua with redeal, Spider, Forty Thieves, Ultra Klondike 2-deck,
   Klondike two-deck 9. For each: the figure, its confidence interval, and what
   fraction of instances were left unresolved. If two-deck games come out harder
   or partly unresolved, that is exactly what we want to know.

7. **Why are foundation dominances unavailable for two-deck games?** Their
   implementation disables them outright when the two-deck flag is set, with no
   comment. Does the paper explain it, or is it simply unproven there?

8. **Novelty check: has anyone published a winnability figure for Gypsy** (also
   spelled Gipsy)? Check the close relatives and say which are actually distinct
   games rather than assuming — Gargantua, Irmgard, Blockade, Miss Milligan. A
   clean negative is a useful answer.

9. **Has anyone published on Spider solvability specifically?** Spider is
   two-deck and its stock deals one card to every tableau column, which is
   Gypsy's mechanic exactly. If Spider has been solved at scale, those
   techniques are the closest available analogue to what we need.

Decisions already made, so you do not have to guess at them:

- **We will not copy code.** Solvitaire is GPL-2 and this repo is not. We want
  rule statements and proofs, to rewrite ourselves for two decks.
- **Consumer solitaire sites quoting win rates are not sources.** Peer-reviewed
  work, preprints, or solver source only. If a claim exists only on a blog,
  report it as a blog claim.
- **Negative answers and "the paper does not say" are as valuable as positive
  ones.** Do not close a gap with plausible reconstruction; a confident wrong
  threshold here is worse for us than an admitted unknown.
- **If the JAIR version and the arXiv preprint differ** on any of the above, say
  so. The preprint has several versions and we do not know which the solvers
  were written against.

Deliverable: a timestamped markdown report under `docs/reports/`, quoting the
conditions verbatim where they matter, with section or page references we can
cite. Interpretation is not needed — that happens with John.
