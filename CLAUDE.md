# CLAUDE.md

Working context for `github.com/john-fritter/gypsy-solve`.

Design and rationale live in `DESIGN.md`; the running record of decisions lives
in `DECISIONS.md`. This file is stable context — architectural decisions, hard
constraints, and how work gets divided. It is not a status log. Do not record
current implementation state here.

## What this is

A solver for two-deck Gypsy solitaire, plus batch analysis of how often the game
is winnable and how much of that winnability depends on worrying back (returning
cards from foundation to tableau).

Deliverable order: a defensible number with a confidence interval, a writeup,
then an interactive page. The playable front end comes last.

## Hard constraints

- **One implementation of the rules.** The engine is the single source of truth.
  Do not reimplement rules in JS for the web front end — compile the core to
  WASM. Divergence here silently corrupts published results.
- **Seeded deals everywhere.** Every deal is reproducible from a seed. Every
  published claim must be replayable by a stranger.
- **Three-valued solver results**: solvable / proven unsolvable / unknown. Never
  collapse `unknown` into `unsolvable`. Budget-exhausted is not a proof.
- **Never report a bounded search as exact.** A capped worry-back search yields a
  lower bound and must be labeled as one.
- **Prove dominance rules before using them.** Worry-back invalidates the
  standard safe-autoplay rule. The Solvitaire authors found this same bug in
  their own solver and in a published Klondike solver. Assume any inherited
  dominance is wrong until it's shown correct for this ruleset.
- **Validate against Klondike (~81.9%) before trusting any Gypsy number.**
- The exact ruleset is in `DESIGN.md` and is the permissive variant (any
  alternating-color sequence moves as a unit). It is not classic Gypsy. Do not
  "correct" it toward the textbook rules.

## Agents and division of labor

Three parties work on this repo. Keep the lanes clean.

**John** — designs, decides, reviews. Works primarily from mobile during work
hours, so prefer output that reads well on a phone: short, dense, no padding.

**Claude Code (you)** — repo changes and implementation. Engine, solver,
analysis code, front end. You read the codebase and infer mechanics; you are not
given pseudocode and should not ask for it.

**Gizmo** — a Hermes agent running on the `fritter.lol` server, reached over
Slack. Gizmo owns everything that happens on the box: deployment, long-running
batch execution, DB queries, log wrangling, and reporting results back as
timestamped markdown files.

### Why Gizmo matters here

Batch solving is the compute-heavy core of this project — thousands of deals,
embarrassingly parallel, hours to days of wall clock, far more memory than a
laptop. That work runs on fritter.lol under Gizmo, not locally. Design the CLI
with that in mind: resumable batch runs, per-deal output written incrementally,
structured results Gizmo can summarize without re-running anything.

### How to work with Gizmo

- Treat Gizmo as a capable junior developer and sysadmin who already knows the
  system. Do not write step-by-step shell commands.
- Prompts to Gizmo carry **high-level intent and non-obvious decisions only**.
  No pseudocode, no hand-holding.
- State every design choice explicitly. Do not leave an undisclosed assumption
  for Gizmo to guess at.
- Gizmo returns findings as markdown. Analysis and interpretation happen with
  John, not inside Gizmo.
- Gizmo needs repo access on the server (account permissions, checkout, toolchain
  install). That's setup work, not a per-task concern.

## Working style

- **One PR per improvement.** Each change independently shippable and testable.
  Bundled changes have caused real problems before.
- **No drive-by refactors.** Scope exactly to the task. Unsolicited cleanup is
  noise and risk.
- **Resist speculative architecture.** Complexity must be justified in concrete
  terms — a measured slowdown, a real correctness need — before it goes in.
- **Act on evidence, not on a theory the evidence contradicts.** If a benchmark
  says the clever optimization is slower, it's slower.
- **Extreme concision.** No redundancy, no restating the same point three ways,
  no re-requesting data already provided. Trust the reader's competence.
- Correctness beats speed until Klondike validation passes. After that, speed
  is the whole game.

## Documentation

Three files, three jobs:

- `DESIGN.md` — the plan and its rationale. A starting point, not carved in
  stone. Correct it when reality contradicts it.
- `DECISIONS.md` — append-only log of decisions: what was chosen, why, what was
  rejected, and whether it is firm or provisional. **Every PR that makes a
  non-obvious choice appends an entry.** A decision explained only in chat is a
  decision that has been lost.
- `CLAUDE.md` — this file. Stable context only.

Rationale is part of the deliverable, not overhead. The writeup needs it, and
anyone reading a dominance rule later needs to know what was already considered
and rejected. Do not edit old entries to look right in hindsight — change the
status and add a new entry explaining the reversal.

## Layout

```
core/      rules, state, move generation, seeded deals
solver/    game trait, search, transposition table, dominances
klondike/  Klondike, for validating the solver. Not part of the deliverable
cli/       batch runner, single-deal solve, replay dump
analysis/  stats, plots, writeup source
web/       WASM bindings + front end (last)
```

`klondike/` is separate from `core/` on purpose: `core/` is the one
implementation of the Gypsy rules and is what compiles to WASM, and a game
nobody plays on the site does not belong in that payload. The solver is generic
over a `Game` trait so that the search validated against Klondike is literally
the search that publishes the Gypsy numbers.

## Deployment

The site is **solve.fritter.lol**, served statically from the fritter.lol box
alongside the other subdomains. Static assets only: the WASM bundle plus
precomputed results. No backend, no API, no server-side solving. Gizmo owns the
deploy.

## Debug visualizer

Build early, before the solver is trusted. Not a game — a way to render a state
and step through a move list. Terminal or static HTML dump both fine. Watching
the search make obviously stupid moves is the primary tool for finding a bad
dominance rule.

## Language

**Rust** for engine, solver, and CLI. **Python** for analysis and plots. Decided
— do not relitigate without a concrete reason.

Rust because the same compiled core has to serve both the batch runner and the
browser (WASM is a first-class target), because memory safety here protects the
published result rather than just the process, and because this workload — flat
arrays of copyable values, bit manipulation, one large fixed-size table — has
almost none of what makes Rust painful. Use `rayon` for the parallel batch and
`criterion` for benchmarks.

The Rust/Python boundary is results files on disk. Do not call Python from Rust
or vice versa.
