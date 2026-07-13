# The Decision Ledger

`docs/DECISIONS.md` opens with its own rules:

> Append-only. Format: `D-NNN [scope] ruling — rationale. Revisit:
> condition.` Agents: consult before designing (law L4); append with
> rationale when ruling an unadjudicated blocking fork; never relitigate
> an entry silently. Humans: strike by appending a superseding entry,
> never by editing history.

As of `m15-done` the ledger holds D-001 through D-061. Entries D-001..D-026
were ratified in the founding design conversation (2026-07-02); everything
after was appended in flight, one entry per fork, as the forks arrived.

## The protocol

Law L4 is a two-branch instruction with two prohibitions:

- Before making a design choice, check the ledger. **If ruled: follow it**
  — do not relitigate silently.
- **If unruled and blocking: make the call**, log a new D-entry (one line,
  rationale, revisit condition), and proceed.
- Never stall on an unadjudicated fork; never bury a call in code without
  a ledger entry.

The escape valve is deliberately narrow: only questions the human alone can
answer (taste, scope expansion, striking an entry) go to the "For the
human" queue in `STATE.md`, and work continues on whatever those questions
do not block. Entries marked ⚑ are calls made *for* the human under this
veto-ledger pattern — decided, executed, and flagged as most deserving of
review, rather than parked awaiting permission.

## Anatomy

Every entry carries a scope tag (`[mem]`, `[harness]`, `[m15/ctl]`…), the
ruling, its rationale, and a **revisit condition** — the named future
evidence that would reopen the question. Revisit conditions are not
decoration; they fire. D-048 records the D-039 green-tree trigger firing at
M9 and resolving to "not adopted," with the evidence (no reprinter exists
or is scheduled; artifacts are self-contained). D-040 records D-009's
scheduled retrospective confirming the specimen order. D-044.3 records a
revisit clause being *retired*: D-005's "if melior gaps dominate two
milestones" clause was closed by evidence, "ratified with prejudice."

Supersession works the same way in the other direction. When the human
struck D-030 (the two-tier dialect-registration ruling), the strike was
itself an appended entry — D-031 opens "**Supersedes D-030 (struck by the
human, 2026-07-02)**" — and the session log fixes the discipline: D-030
stays in the ledger struck-but-visible; never edit it; the strike lives in
D-031's first line. History is auditable because it is append-only.

## Reading the range

Eight entries, chosen to show the kinds of decision the ledger carries.

**D-041 — a strategy ruling made ahead of code (⚑).** The frk.mem surface:
`!frk_mem.box<T>` with `box_new/box_get/box_set`, and *strategy as a
lowering parameter, never IR* — the kernel lowering takes
`Strategy ∈ {Arena, Rc}`. The entry designs one surface to retire four
previously ledgered debts at once, then flags its sharpest sub-call
separately: rc v0 inserts retains (with SSA ownership-transfer elision) but
**no releases** — "v0 rc therefore proves the strategy plumbing end to end
… but collects nothing" — and ends with an explicit strike offer: "Strike
this clause if liveness-based releases should gate M7 instead." The human
ratified it at D-044.1 ("releases without a liveness pass are either wrong
or theater") with a rider: `frk_rt_alloc_count()` lands in both runtime
twins now, so the future release pass has a measurable target.

**D-031 — a hard pivot recorded honestly.** The human struck D-030's C++
shim tier; D-031 rules IRDL-only registration and makes the *design* bend
instead of the build: no kernel op may require traits, frk.adt drops its
region-based `match` op, dispatch rides upstream `cf.switch`, and surface
`match` compiles through the Maranget decision-tree pass. Deep invariants
move into a verification pass — "K1's 'verifier enforcing invariants'
hosted in a pass, not in C++." The revisit condition demands evidence, not
appetite: "only if a future dialect design demonstrably suffers from
de-regioning (bring the suffering as evidence)."

**D-036 — a ceiling discovered mid-milestone, hardened into law.** M4
found that LLVM-22 IRDL constraint variables bind once per op instance, so
heterogeneous variadics are inexpressible — and the entry files the
uncomfortable half of the discovery as a first-rank finding: mixed-type
`make_sum` *never* worked; "the corpus passed only because it was uniformly
typed." The response is structural: no variadic operand/result groups in
kernel dialects, ever; explicit packing (`product_new`/`product_snoc`)
instead, "sound by construction" because every IRDL variable sits in one
position.

**D-044 — a human-review ratification.** The first adjudication of the ⚑
queue, dispositions recorded in one entry: D-041 ratified with the counter
rider; D-038 ratified all three flags (and the femto-second consequence
executed — the manifest's own scope line amended by the entry); D-005
ratified with prejudice; and the M8 exit bar *amended* — shell errors must
echo the offending source line, because "a REPL whose trap messages point
at nothing ships §6.5's bug-by-law." The entry closes by declaring itself
"a record of rulings, not a fork" — even meta-decisions get entries.

**D-050 — a review crossing an implementation in flight.** The second
review recommended deferring the UTF-16 string ruling until `.length` made
representation observable — but the shipped slice already included
`.length` and the surrogate-counting golden. Clause 4 records the crossing
instead of unwinding it: "The ruling stands as what the trigger would have
produced … Recorded rather than unwound — ripping out .length to restore
deferability would be theater." The same entry turns a review note into
standing law: when the oracle offers a flag that eliminates a divergence
class (`noImplicitReturns`), set the flag — the checker-as-oracle
corollary.

**D-058 — a milestone contract.** femto_lua v0.2 is scoped *in the
ledger*: the pack calling convention (`fn<[arr<dyn>], [arr<dyn>]>`), the
consequences it buys (the exact-arity fence dissolves; multiple returns are
the pack itself), what stays fenced (varargs, mid-explist spreads), a canon
rule (pairs iteration order is implementation-defined — corpus prints only
order-independent aggregates), and the exit bars, including
regression-first: "the eight v0.1 cases stay green under the new convention
(regression is the first bar)." The milestone note later confirms the
entry's prediction — the convention change was frontend-only, exactly as
D-058 said.

**D-060 — the delegation ruling.** SPEC §4.4 anchored the control-effects
design to the human's Rocq handler calculus, and M14 escalated for it. The
human's answer — "Why do I need to provide the calculus? You can do it just
fine." — is recorded verbatim, and the entry converts it into artifacts:
`docs/ctl-calculus.md` (κ_frk) is promoted from **atli**, the human's own
graded handler calculus, whose Rocq development is the mechanization κ_frk
leans on. The entry lists what κ_frk takes (effect rows, the drop/resume
clause taxonomy, deep one-shot continuations as keystone, both traps) and
what it leaves in atli, with revisit conditions. It also discharges a gate:
the r7rs_core stub's "do not ratify before the ctl effects design lands" is
satisfied, and the manifest is ratified in the same commit.

**D-061 — a design-panel reconciliation.** The frk.ctl native lowering was
put to a 3-designer-plus-judge panel; the entry records what the judge
chose, what all parties agreed on (the ctl ops lower inside
`lower-frk-kernel`; the branchless resolve over a 2-word alloca out-slot),
and the **one deliberate divergence** from the judge's base: guards are
emitted by the frontend, not by a post-LLVM block-splitting pass — with the
argument spelled out (melior block-splitting was the panel's unanimous top
risk; the judge's verifier-first objection "does not bind here" because the
hand-written ctl goldens are interp-only and native verification is the
scheme differential). Heeded panel catches are listed and fenced. Even the
disagreement with the judge is auditable.

## What the ledger buys

The ledger is the project's memory of *why*. Nothing in the tree may embody
a design fork the ledger does not name; nothing in the ledger may be
overturned except in writing, by a numbered successor. The practical
effects observed over sixteen milestones: agents do not re-argue settled
questions (D-031's trait-free rule shaped every dialect after M3 without
being re-derived once); reviews attach to specific numbered claims rather
than to vibes; and when reality contradicts a ruling, the contradiction
becomes a first-rank finding with a place to live instead of a silent
patch. The format's terseness is load-bearing — one entry, one fork, one
revisit condition — because a ledger you cannot read in a sitting is a
ledger nobody consults before designing, and consultation is the law's
first verb.
