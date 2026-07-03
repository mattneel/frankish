# canon.md — Canonicalization Contract v0 (SPEC §7.4)

Status: v0, written at M1. Amending this contract requires a DECISIONS.md
entry, and any golden whose bytes change under the amendment is re-blessed
with a justification line in the commit message (law L2).

All cross-runner and cross-oracle comparison in frankish happens over
**canonical bytes**: `frk_harness::canon::canonicalize` is the single
implementation of this contract, and no diff is judged outside it
(SPEC §7.4). Oracle outputs (specimen upstreams) pass through the same
filter as our own runners.

## 1. Byte discipline

- Encoding is UTF-8. Runners must not emit locale-dependent text; the
  harness compares bytes after the transforms below and nothing else.
- Line endings: CRLF and lone CR normalize to LF.
- Non-empty output ends with exactly one trailing LF, added if missing.
  Extra trailing LFs are *preserved* — the filter hides line-ending
  flavor, not runner bugs; goldens catch those.
- No other whitespace transformation. Interior bytes are untouched.

## 2. Scalar rendering (the v0 "output" definition)

Until a runtime print surface exists (frk-rt, M7+), a golden's output is
the rendering of its entry function's return value:

- `i64` — the only v0 result type: decimal digits, `-` for negatives, no
  digit separators, one LF. Rendered by `frk_harness::canon::render_i64`.
- Floats (policy pinned now, exercised when floats first print): shortest
  round-trip decimal exactly as produced by Rust's `{}` Display for
  f64/f32; `NaN`, `inf`, `-inf`; negative zero renders `-0`.

## 3. Ordering

Output derived from unordered collections (hash maps, symbol tables,
diagnostic sets) must be explicitly sorted by the producer before
printing — bytewise lexicographic unless a specific doc rules otherwise.
Hash-order output is a bug even on the day it happens to match.

## 4. Error text

Failure *classification* will be comparable; failure *prose* is not. v0
goldens must succeed on every applicable runner — there are no
expected-failure goldens yet. When diagnostics goldens arrive with the
frontend kit (SPEC §6.5), this contract gains a scrub filter (absolute
paths, addresses, pass names) before any error text is compared.

## 5. Oracles

Oracle processes run under `LC_ALL=C`. Their stdout goes through §1
unchanged. An oracle that cannot be made byte-stable under this contract
gets a per-oracle normalizer documented in its specimen MANIFEST — never
an exemption from comparison.

## §6 TS-0 number printing (M9; D-047)

console.log output compares across four printers: the interpreter
builtin and the JIT capture (both Rust `Display`), the C runtime's
round-trip-precision search (AOT, all triples), and V8 itself. They
agree byte-exactly within the fence: printed values are 0 or
|v| ∈ [1e-4, 1e15), finite. Corpus law: stay inside it. Outside it JS
switches to exponent spellings ("1e+21", "1e-7", "Infinity") that the
frankish printers do not reproduce yet — widening the fence is a
canon change and takes a D-entry (TS-1 candidate).
