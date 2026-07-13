# The Constitution

frankish is developed under a written constitution — `AGENTS.md` at the
repository root (symlinked as `CLAUDE.md`; the two must never diverge). It
binds any agent, human or machine, working in the tree. The laws are short
because they are enforced mechanically; this chapter states each one and
what actually enforces it.

## L1 — Verifier first

> No implementation lands without its verifier landing in the same commit
> or an earlier one.

"Verifier" is defined concretely: a golden test, a conformance entry, a
differential check, or a property test. The consequence is a strict
epistemic ordering — **the verifier is the spec; the implementation is
fungible**. When the `frk_ctl` dialect landed at M15, its six interpreter
verifiers (covering every reduction rule of the calculus, both traps
included) landed in the same commit. When tail calls became law at M14,
the commit carried two 500,000-frame fixed-stack goldens that *fail*
without each half of the implementation.

L1 changes what a bug means. A defect that slips through is never just
fixed; the first question is *which verifier was missing*, and that
verifier lands with the fix.

## L2 — Golden discipline

> Goldens are byte-exact after canonicalization. Blessing new goldens
> requires a commit message line explaining why the output changed.

Golden outputs are compared as bytes, after the canonicalization rules in
`docs/canon.md` (locale pinned to `LC_ALL=C`, number formatting specified
to the digit). `make bless` rewrites expectations from the reference
runner — and the law forbids blessing a diff you don't understand. Every
blessed change is explained in the commit that carries it.

## L3 — The differential law

> The derived interpreter is the reference semantics. The JIT/AOT path
> must agree with it byte-exactly on every golden. Specimen frontends add
> the upstream implementation as a third oracle. A disagreement is a
> first-rank finding.

This is the load-bearing law; it gets [its own chapter](differential.md).
The operative word is *halt*: a divergence between runners stops the
feature that exposed it. Nothing may be built on top of a disagreement.

## L4 — The decision protocol

> Before making a design choice, check the ledger. If ruled: follow it.
> If unruled and blocking: make the call, log a D-entry, proceed.

The ledger is `docs/DECISIONS.md` — append-only, 61 entries at the time
of writing. L4 has two teeth: *never relitigate silently* (a settled
ruling is followed or superseded in writing, not eroded), and *never
stall* (an unadjudicated fork is not an excuse to stop; make the call,
record it with rationale and a revisit condition, keep moving). The
ledger gets [its own chapter](ledger.md) too.

## L5 — Subset fences are law

> Specimen scope is defined solely by its MANIFEST. No feature enters a
> specimen unless it carries an idiom the kernel dialect library lacks.

This is the admission rule that keeps specimens honest. femto_lua does
not grow coroutines because coroutines would be *nice*; a feature enters
when the kernel needs the idiom it forces, and not before. Fence lists in
manifests are boundaries, not backlogs.

## L6 — Agent portability

> Every workflow must run via `make` targets using POSIX-portable
> scripts. No vendor-specific agent features may be load-bearing for
> build, test, or CI. Any agent (or human) must be able to take over from
> `make test` + `STATE.md` alone.

The whole project builds and verifies with `make test`, `make diff`,
`make grid` — plain cargo and shell underneath, toolchain pins in
`versions.env`, no absolute user paths, no machine-specific
configuration. This book itself obeys L6: `make book` builds it.

## L7 — Handoff hygiene

> Update `STATE.md` before ending any session. Never leave the tree red
> and undocumented.

`STATE.md` is the live handoff file: current phase, in-flight work, next
action, open questions for the human, a milestone log, and a session log
that records *landmines* — the non-obvious hazards a future session must
not rediscover the hard way. If a session degrades mid-task, the law is
to drive to a green point (or stash), write the entry, and push.

## L8 — Cadence

> Commit at every green step; push at minimum every three green steps and
> at every milestone exit.

Commit subjects follow `[Mn] area: summary`; bodies carry the context the
diff can't. The observable effect is a history that reads as a narrative
of green states — `git log --oneline` on this repository is a usable
project chronology.

## L9 — Scope guard

> Consult the non-goals before adding any capability.

The spec maintains an explicit non-goals list (SPEC §14). Multi-shot
`call/cc`, for instance, is fenced there — and when the control-effects
calculus arrived, it *kept* that fence as a design axiom (one-shot
continuations are κ_frk's keystone) rather than quietly outgrowing it.

## Why laws instead of judgment

Every law above replaces a recurring judgment call with a mechanical
check, which is what lets the project absorb contributors — human or
agent — without eroding. The pattern to notice: none of the laws is
aspirational. Each one names its enforcement (a harness, a file, a make
target, a commit convention), and the enforcement is what makes the next
chapter possible: a matrix of eight execution paths that has stayed at
zero divergence across fifteen milestones.
