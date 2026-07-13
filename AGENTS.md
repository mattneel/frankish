# frankish — Agent Constitution

frankish is a language-construction workbench built on MLIR. Its product is a
curated library of **kernel dialects** — the PL-idiom middle layer (ADTs,
closures, memory strategies, control effects, dynamic dispatch, contracts,
staging) that sits between language frontends and MLIR's upstream compute
dialects — plus a frontend kit, a verification harness, and a driver
(`frnksh`). Real languages are implemented as **specimens** to force the
dialects into existence, then re-based onto what they forced.

This file is law for any coding agent working in this repository, regardless
of vendor. `CLAUDE.md` is a symlink to this file; other agents read it as
`AGENTS.md`. The two must never diverge.

## Session start protocol (mandatory, in order)

1. Read this file completely.
2. Read `STATE.md` — current phase, in-flight work, next action.
3. Read `docs/DECISIONS.md` — the veto ledger. Rulings there are settled law.
4. Read the sections of `docs/SPEC.md` relevant to the current milestone
   (SPEC §0 maps milestones to sections).
5. Read the `specimens/*/MANIFEST.md` for any specimen you will touch.
6. Run `git log --oneline -15` and `make test` (if M0 is complete) before
   writing anything.

## The Laws

**L1 — Verifier first.** No implementation lands without its verifier landing
in the same commit or an earlier one. "Verifier" means: golden test, conformance
entry, differential check, or property test. The verifier is the spec; the
implementation is fungible.

**L2 — Golden discipline.** Goldens are byte-exact after canonicalization
(`docs/SPEC.md` §7.4). Blessing new goldens (`make bless`) requires a commit
message line explaining *why* the output changed. Never bless to silence a
diff you don't understand.

**L3 — Differential law.** The derived interpreter is the reference semantics.
The JIT/AOT path must agree with it byte-exactly on every golden. Specimen
frontends add the upstream implementation as a third oracle. A disagreement is
a first-rank finding: halt the feature, file it in STATE.md, fix or fence
before proceeding.

**L4 — Decision protocol.** Before making a design choice, check
`docs/DECISIONS.md`. If ruled: follow it; do not relitigate silently. If
unruled and blocking: make the call, log a new D-entry (one line, rationale,
revisit condition), and proceed. Never stall on an unadjudicated fork; never
bury a call in code without a ledger entry.

**L5 — Subset fences are law.** Specimen scope is defined solely by its
MANIFEST. No feature enters a specimen unless it carries an idiom the kernel
dialect library lacks (the admission rule). Fence lists are not TODO lists.

**L6 — Agent portability.** Every workflow must run via `make` targets using
POSIX-portable scripts — no vendor-specific agent features (skills, plugins,
MCP calls) may be load-bearing for build, test, or CI. No absolute user paths,
no machine-specific config, no secrets in the repo. Any agent (or human) must
be able to take over from `make test` + `STATE.md` alone.

**L7 — Handoff hygiene.** Update `STATE.md` before ending any session, using
its embedded template. If context, tokens, or budget degrade mid-task: drive
to a green point (or stash), write the STATE entry, push. Never leave the
tree red and undocumented — the next agent may not be you.

**L8 — Cadence.** Commit at every green step; push at minimum every three
green steps and at every milestone exit. Commit subject format:
`[Mn] area: summary` (e.g. `[M3] adt: decision-tree lowering for nested tags`).
Bodies carry context the diff can't.

**L9 — Scope guard.** Consult SPEC §14 (non-goals) before adding any
capability. If a task seems to require a non-goal, stop and log the conflict
in STATE.md instead of building it.

## Milestone loop

Work proceeds by the milestones in `docs/SPEC.md` §13. For each milestone:
plan against the exit criteria, implement under L1–L3, run the full suite,
write the milestone note in STATE.md (what shipped, what was learned, what
cheats exist awaiting promotion), tag `mN-done`, push. Do not start Mn+1 with
Mn red.

## Repository map

    AGENTS.md            this file (law)          CLAUDE.md -> AGENTS.md
    STATE.md             live handoff state
    README.md            public face
    docs/SPEC.md         full design + milestones
    docs/DECISIONS.md    veto ledger (append-only)
    docs/LANDSCAPE.md    verified prior art, pinned facts, watch items
    specimens/           per-specimen MANIFESTs (frozen subsets, oracles)
    crates/              Rust workspace (created in M0; layout in SPEC §12)
    tools/loanword-ts/   TypeScript frontend package (created in M9)
    book/                the mdbook (deep docs; `make book`; CI deploys
                         to GitHub Pages via .github/workflows/book.yml)

## Escalation

Questions only the human can answer (taste, scope expansion, striking a
ledger entry) go in STATE.md under "For the human", and work continues on
whatever is not blocked by them. Batch questions at milestone boundaries;
do not ping mid-flight for anything L4 lets you rule yourself.
