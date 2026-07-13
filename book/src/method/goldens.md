# Golden Discipline and Canon

Two laws of the constitution govern everything in this chapter. **L1 —
verifier first**: no implementation lands without its verifier landing in
the same commit or an earlier one, and "the verifier is the spec; the
implementation is fungible." **L2 — golden discipline**: goldens are
byte-exact after canonicalization, and blessing new goldens requires a
commit-message line explaining *why* the output changed. The golden engine
(`crates/frk-harness/src/golden.rs`, `case.rs`) and the canonicalization
contract (`docs/canon.md`) are the machinery that makes those laws
mechanical rather than aspirational.

## Anatomy of a golden case

A golden case is a directory (format ruled in D-027, documented in
`goldens/README.md`):

```
goldens/<suite>/<case>/
  case.mlir       the program (or case.ml / case.ts / case.lua /
                  case.scm / transcript.in — the file present decides
                  the source kind)
  expected.out    blessed canonical output (committed)
  output.actual   written on mismatch for diffing (gitignored)
```

Suites are just directories: `upstream/` holds MLIR programs over
func/arith/scf/cf; `adt/`, `closure/`, `mem/`, `bstr/`, `dyn/`, `str/`,
`ctl/`, `tailcall/` exercise the kernel dialects; `ml_core/`, `ts0/`,
`lua/`, `scheme/` hold specimen programs in their own languages; `repl/`
holds scripted shell transcripts. One directory, one program, one blessed
byte sequence — and every applicable runner must reproduce those bytes.

Cases carry optional directives as comment lines — `// frk-case: key=value`
in `.mlir`, `(* frk-case: ... *)` in `.ml` (which must stay valid OCaml,
because the same file runs verbatim under the `ocaml` oracle):

```
// frk-case: entry=main      entry function symbol   (default: main)
// frk-case: result=i64      return rendering        (default: i64; v0's only type)
// frk-case: runners=a,b     applicable runners      (default: all — SPEC §7.2)
```

The directive parser is strict by design: an unknown key or an unsupported
value is a `CaseError::Directive`, never a silent default. `runners=`
exists for op sets that are ahead of some execution path (D-033 — e.g.
`frk_adt` goldens before the K3 lowering existed); its guard rails are law:
skips print per case, and D-033 adds a rot check — grep the corpus for
`runners=` at every milestone exit, because a skip that never flips back is
a smell.

## Refusing vacuous green

The discovery and comparison code treats "accidentally tested nothing" as
red, not green:

- A corpus with **zero cases** is `CaseError::EmptyCorpus` — "a corpus with
  zero cases is a wrong path, not a green suite."
- A corpus where **every case skips** the current runner is
  `CaseError::NothingApplies` — "almost certainly a typo'd runner name,
  never a green suite."
- A case with **no `expected.out`** is `Status::MissingExpected` and red:
  an unblessed case is a missing verifier, and L1 says the verifier comes
  first.
- Discovery sorts cases by name so reports and diffs are deterministic
  (the spirit of canon §3 applied to the harness itself).

On mismatch, the engine writes the canonical actual bytes to
`output.actual` next to `expected.out` — the failure artifact is a file you
can `diff`, not a log line you have to reconstruct.

## The canonicalization contract

All cross-runner and cross-oracle comparison happens over **canonical
bytes**. `frk_harness::canon::canonicalize` is the single implementation of
the contract in `docs/canon.md`, "and no diff is judged outside it" — the
M1 handoff note makes the corollary explicit: never add a second
normalization anywhere. Amending the contract requires a DECISIONS.md
entry, and any golden whose bytes change under the amendment is re-blessed
with a justification line in the commit (L2).

The rules, in full:

- **Byte discipline (§1).** Encoding is UTF-8; runners must not emit
  locale-dependent text. CRLF and lone CR normalize to LF. Non-empty output
  ends with exactly one trailing LF, added if missing — but *extra*
  trailing LFs are preserved: the filter hides line-ending flavor, not
  runner bugs. No other whitespace transformation; interior bytes are
  untouched.
- **Scalar rendering (§2).** v0's only entry result type is `i64`: decimal
  digits, `-` for negatives, one LF. The float policy was pinned at M1,
  years of grief ahead of its first use: shortest round-trip decimal
  exactly as produced by Rust's `{}` Display; `NaN`, `inf`, `-inf`;
  negative zero renders `-0`.
- **Ordering (§3).** Output derived from unordered collections must be
  explicitly sorted by the producer before printing. "Hash-order output is
  a bug even on the day it happens to match."
- **Error text (§4).** Failure *classification* will be comparable; failure
  *prose* is not. v0 goldens must succeed on every applicable runner —
  there are no expected-failure goldens yet.
- **Oracles (§5).** Oracle processes run under `LC_ALL=C`; their stdout
  goes through §1 unchanged. An oracle that cannot be made byte-stable gets
  a per-oracle normalizer documented in its specimen MANIFEST — never an
  exemption from comparison.

Two specimen-driven fences extend the contract with numbered sections:

- **§6 (TS-0, M9, D-047).** `console.log` output compares across *four*
  printers — the interpreter builtin and the JIT capture (both Rust
  `Display`), the C runtime's round-trip-precision search on every AOT
  triple, and V8 itself. They agree byte-exactly inside the fence: printed
  values are 0 or |v| ∈ [1e-4, 1e15), finite. Outside it, JS switches to
  exponent spellings the frankish printers do not reproduce; widening the
  fence is a canon change and takes a D-entry.
- **§7 (Lua, M11, D-052/D-055).** Lua prints via `%.14g`, which **rounds**
  — 14 significant digits, half-to-even — so parity across the Rust twin's
  emulation, the C twin's native `printf("%.14g")`, and the lua5.1 oracle
  is a *rounding* contract, not just a digits contract. The cross-twin rig
  (`crates/frk-harness/tests/lua_print_parity.rs`) proves it on deliberate
  tie values whose 15th significant digit is exactly 5, binary-exact. The
  fence upper bound is 1e14 — one decade tighter than §6's, because `%g`
  switches to exponent form at exponent ≥ precision. The corpus tie case
  ships in `goldens/lua/hello`:

```lua
print(1/3)
print(12345678901234.5)
```

```
0.33333333333333
12345678901234
```

Three independent printers produce those exact bytes, including the
half-even round at the 14th digit.

## Blessing

`make bless` re-runs the corpus and overwrites every `expected.out` with
current canonical output. The Makefile carries the law inline:

> Rewrite golden expectations from the reference runner. Law L2: the
> commit blessing new bytes must say WHY the output changed; never bless
> a diff you don't understand.

The engine keeps you honest mechanically: blessing reports
`Blessed { changed }` per case, and a bless that changed nothing prints
`(unchanged)` — the test suite calls this "the L2 smell test for pointless
blesses." Blessing also deletes any stale `output.actual`.

Since M2, blessing writes the **reference interpreter's** bytes (D-008:
`reference_runner()` is `interp`). The M2 handoff note draws the
consequence: if the JIT disagrees with a freshly blessed golden, that is an
L3 first-rank finding, not a blessing mistake — the discipline separates
"the semantics changed, justify it" from "two implementations of the same
semantics diverged, halt."

## Corpus admissibility

Two standing rules bound what may be a golden at all:

- **UB is inadmissible.** Cases must be free of MLIR-level undefined
  behavior — division by zero, signed-division overflow, non-positive
  `scf.for` steps. The reference interpreter traps deterministically on
  these (D-029); native paths do whatever LLVM does; nothing comparable
  comes out. Wrap-around integer arithmetic is *defined* (modulo 2^n) and
  fair game — `goldens/upstream` carries an `add_wrap` canary for exactly
  that.
- **Entry protocol.** The v0 entry takes no arguments, returns one `i64`
  rendered per canon §2, and carries `llvm.emit_c_interface` (the JIT
  invokes through the generated `_mlir_ciface_` wrapper). The AOT runners
  rename the entry to `frk_entry` before lowering (D-042 — the C shim owns
  `main`), so entry functions must be externally-invoked-only.

## The golden is the spec

A single case directory is simultaneously the verifier and the
specification, which is L1's whole point. Take
`goldens/ml_core/adt_option`:

```ocaml
type opt = None | Some of int
let get_or d o = match o with None -> d | Some x -> x
let main () = get_or 0 (Some 40) + get_or 2 None
```

with `expected.out` = `42`. That one file is executed by the reference
interpreter (kernel-dialect semantics), by two JIT configurations under
both memory strategies (lowering correctness), by the AOT grid on five
architectures (ABI and portability), and — verbatim, with
`print_int (main ())` appended — by `ocaml` 4.14.1 (fidelity to the real
language). Any implementation that reproduces the blessed bytes on all of
those paths is admissible; any that does not is wrong, with the diff in
`output.actual`. The prose spec can mislead; the golden cannot.
