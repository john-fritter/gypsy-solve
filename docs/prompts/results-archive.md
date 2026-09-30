# Gizmo request: archive the survey results into the repo, 2026-09-28

**Copy every result file behind the published numbers from fritter.lol into
the `gypsy-solve` repository. Do not re-run anything.**

## Why

The project is moving to its writeup. The headline claim is that at least
90.6% of Gypsy deals are winnable, and that worry-back changes no decidable
verdict. It rests on result files that exist only on fritter.lol, on a RAID0
volume with no redundancy. The repository does not have them, so a stranger
cannot replay the claim, and one disk failure would lose it. Several earlier
requests ended with "get it off that box". The files did not reach the repo,
so this time the destination is spelled out.

## The files

Every one of these must be found. They were produced by runs you did between
2026-09-16 and 2026-09-25:

| File | Run |
|---|---|
| `klondike-full-3M-1000deals-restart1.jsonl` | Klondike budget sweep |
| `klondike-full-12M-1000deals-restart1.jsonl` | Klondike 1,000-deal validation |
| `klondike-full-12M-1000deals-restart1-table1024-reference.jsonl` | same, reference table size |
| `klondike-full-12M-1000deals-restart8.jsonl` | same, with restarts |
| `klondike-full-48M-1000deals-restart1.jsonl` | Klondike budget sweep |
| `klondike-full-64M-1000deals-restart1.jsonl` | Klondike 5% gate |
| `gypsy-both-12M-50deals-restart8-twofix.jsonl` | first Gypsy survey, run 1 |
| `gypsy-both-12M-restart8-1000deals.jsonl` | first Gypsy survey |
| `gypsy-both-48M-restart32-1000deals.jsonl` | first Gypsy survey |
| `gypsy-both-12M-1000deals-contiguous.jsonl` | refutation hunt, run 1 (seeds 0–999) |
| `gypsy-both-12M-seeds1000-4999-contiguous.jsonl` | refutation hunt, run 2 |
| `gypsy-both-192M-restart128-48Munknowns.jsonl` | 192M slope pin |
| `gypsy-both-192M-restart32-slice6M-residue93.jsonl` | residue slice test |

**Also include every record from re-solving a refuted deal with a dominance
removed.** Seeds 188, 3796, 3966, 4260 and 4617 were each re-checked this way,
in hunt runs 1 and 2. Keep whatever names those files have now. Include any
other results file from these runs that isn't listed, and say what it is.

If a listed file can't be found, **say so and do not regenerate it.** A re-run
would be a new measurement at a different build. It would not be the file the
published numbers came from.

## Where they go

- Into `docs/results/` in `github.com/john-fritter/gypsy-solve`, on a new branch
  named `gizmo/results-archive`, pushed. **Do not push to `main`**; John merges
  it.
- Plain git, not Git LFS. Records are about 260 bytes, so the whole set should be
  a few MB. **If any single file is over 50 MB, stop and report its size**
  rather than committing it.
- Byte-for-byte as they are on the box. No reformatting, no merging, no
  de-duplication, and no renaming beyond what the table above says.
- If you can't push, say why, and attach the files plus the manifest below to
  your reply as one `.tar.gz`. John will commit them.

## Check each file before committing, and report rather than fix

- A record count per file, and the distinct seeds per ruleset. The expected
  counts: a 1,000-deal Klondike file has 1,000 records; a `gypsy-both` file has
  two per seed (restricted and full), so 2,000 for 1,000 deals, 8,000 for seeds
  1000–4999, 250 for the 125-deal slope pin and 186 for the 93-deal residue.
- Any seed that appears twice in the same ruleset, any line that doesn't parse,
  and a last line cut off by a kill. `--resume` can leave these. **Report them
  and leave the file as it is.** Which record counts is an analysis decision,
  and it will be made from the report.

## The manifest

Commit `docs/results/ARCHIVE-20260928.md` alongside the files, with one row per
file giving:

- file name, bytes, and SHA-256;
- record count, and seeds per ruleset;
- the date of the run, the build commit it ran at, and the full command line,
  including `--budget`, `--restarts`, `--table-mib`, workers and seed range;
- for the dominance-removed re-solves, which dominance was removed and how the
  build was changed.

Where you can't recover a command line or commit exactly, write "not recorded"
rather than reconstructing it. A guess in a provenance record is worse than a
gap.

## What to report

The branch name and commit hash, the manifest, and anything missing,
duplicated or truncated. Nothing else is needed.

## Not part of this

- No new runs of any kind, and no changes to the solver or CLI.
- Don't delete anything from the box. The repository becomes the archive,
  and the box keeps its copies.
- Don't summarise or interpret the results. The analysis is done from the
  files once they're in the repo.
