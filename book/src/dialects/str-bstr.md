# frk_str and frk_bstr — Two Kinds of Strings

frankish carries two string dialects, and the duplication is a ruling, not
an accident. UTF-16 code-unit semantics is TypeScript's law; 8-bit-clean
interned byte semantics is Lua's; one dialect faking both would divert both
oracles (D-052, D-056). When the semantics differ at the observable surface
— `.length` counts code units, `==` costs a pointer compare — sharing an op
set means lying to at least one upstream implementation, and the
differential law (L3) exists precisely to make that lie fail loudly.

`frk_bstr` is deliberately a sibling of `frk_str`, not an overload or a
unit-width parameter. Same op names where the semantics coincide, different
dialect, different representation contract.

## frk_str — UTF-16 values (M9, D-049)

Immutable strings with JS semantics: `.length` counts code units, so
surrogate pairs count 2. The interpreter stores `Vec<u16>`, not a Rust
`String` — the representation was decided on the evidence D-050 asked for:
`"😀".length === 2`, diffed against V8 across every runner.

| Op | Operands | Results | Semantics |
|---|---|---|---|
| `lit` {text} | — | `!frk_str.str` | Literal; the IR attribute is UTF-8, the lowering re-encodes UTF-16. |
| `concat` | lhs, rhs | `!frk_str.str` | Concatenation. |
| `eq` | lhs, rhs | i1 | Structural equality over code units. |
| `len` | value | i64 | Code-unit count (kernel-level i64; TS emission converts `sitofp` — JS lengths are numbers). |

Everything lowers to runtime calls. A literal becomes a module-level
`llvm.mlir.global` of `!llvm.array<N x i16>` holding the encoded units
(symbol `__frk_str_N`), and the op becomes
`frk_rt_str_from_units(units_ptr, len)`. `concat`, `eq`, `len` become
`frk_rt_str_concat` / `_eq` / `_len`; `eq` returns i64 from the rt and
truncates to i1 in IR.

The runtime layout is `{len: u64, units: u16 × len}`, one allocation:

```rust
fn str_alloc(len_units: u64) -> *mut u8 {
    let bytes = 8u64.saturating_add(len_units.saturating_mul(2));
    let base = raw_alloc(bytes);
    if !base.is_null() {
        unsafe { (base as *mut u64).write(len_units) };
    }
    base
}
```

Strings are rt-owned values allocated with plain malloc inside the runtime,
uniform across strategies — deliberately outside the strategy axis until the
tracer wants them (revisit named at the M10 GC gate). The correctness
corollary reached further than strings: it forced the managed/unmanaged
split in the lowering's slot model. A string pointer carries **no** rc
header, so a retain on it would corrupt the word at ptr-8; `SlotKind::Ptr`
therefore records `managed: false` for `!frk_str.str` and `!frk_bstr.str`,
and the rc lowering retains only managed pointers (D-049).

`frk_rt_print_str` converts UTF-16 back out to UTF-8 for stdout — the
console.log path.

## frk_bstr — interned byte strings (M11, D-052/D-056)

Lua 5.1 strings are 8-bit-clean byte strings, interned at creation, with
equality = pointer identity after intern — the PUC-Rio model, observable
through table keys and the cost of `==` (D-052 fixed the semantics; D-056
executed the representation).

| Op | Operands | Results | Semantics |
|---|---|---|---|
| `lit` {text} | — | `!frk_bstr.str` | Literal. v0.1 fence: printable ASCII + standard escapes only (the verifier rejects bytes ≥ 0x80; Lua *values* are 8-bit clean via concat/from_num regardless). |
| `concat` | lhs, rhs | `!frk_bstr.str` | Concatenation; result is interned. |
| `eq` | lhs, rhs | i1 | Equality — lowers to an inline pointer comparison. |
| `len` | value | i64 | Byte count — lowers to an inline header load. |
| `sub` | value, from : i64, to : i64 | `!frk_bstr.str` | Lua `string.sub` (D-058). |
| `rep` | value, count : i64 | `!frk_bstr.str` | Lua `string.rep`; count clamps at 0. |

**Interning IS the representation.** The runtime owns a global intern table;
`lit` and `concat` (and `sub`, `rep`, `from_num`) all return canonical
pointers. The payoff is that the two hottest ops leave the runtime entirely:
`eq` is `ptrtoint` ×2 + `arith.cmpi eq` — identity ≡ content by
construction — and `len` is a single `llvm.load` of the `{u64 len, bytes}`
header. Only intern, concat, sub, rep, and from_num are rt calls. String
table keys hash by canonical pointer.

Both runtime twins carry the table. The Rust twin:

```rust
fn bstr_intern_bytes(bytes: &[u8]) -> *mut u8 {
    let mut table = intern_table().lock().expect("intern table");
    if let Some(&canonical) = table.get(bytes) {
        return canonical as *mut u8;
    }
    let base = raw_alloc(8 + bytes.len() as u64);
    ...
    table.insert(bytes.to_vec(), base as usize);
    base
}
```

The C twin (`crates/frk-rt/c/frk_rt.c`, the file the AOT grid compiles per
triple) mirrors it with its own `bstr_intern_bytes`; the grid holds the two
behaviorally equal — AOT must byte-match JIT on every golden (D-042, L3).
The twin test asserts the property directly: interning `"hello"` twice
yields the same pointer, and `concat("hel","lo")` interns to that same
canonical pointer.

The interpreter, deliberately, has **no intern table**. Reference semantics
uses `Value::Bytes` with content equality — observably identical to interned
identity, because intern makes pointer equality if and only if content
equality. Noted in D-056 as a deliberate asymmetry: the reference semantics
stays simple, and the equivalence is exactly the kind of claim the
differential runners exist to keep honest.

## sub and rep — Lua's indexing rules

`string.sub` is 1-based, negative indices count from the end, both ends
clamp, and an inverted range is empty (D-058). The rule is implemented three
times — interp, Rust rt, C rt — and pinned by the same corpus:

```rust
/// Lua string.sub semantics (D-058): 1-based, negative counts from
/// the end, clamped; empty when the range inverts.
pub(crate) fn sub_range(len: usize, from: i64, to: i64) -> (usize, usize) {
    let len = len as i64;
    let mut i = if from < 0 { len + from + 1 } else { from };
    let mut j = if to < 0 { len + to + 1 } else { to };
    if i < 1 { i = 1; }
    if j > len { j = len; }
    if i > j { (0, 0) } else { ((i - 1) as usize, j as usize) }
}
```

Natively both return interned results (`frk_rt_bstr_sub` /
`frk_rt_bstr_rep`), so a substring is `==`-comparable by pointer like any
other string.

## Numbers becoming strings

`frk_rt_bstr_from_num` formats through the `%.14g` emulation — the same
correctly-rounding, half-to-even formatter the Lua print path uses, proven
byte-equal against the C twin's native `%.14g` on deliberate tie values
(D-055.2, D-056). `tostring` and `..`-coercion ride one formatter; there is
no second rounding rule to drift.

## Rulings

| Entry | Ruling |
|---|---|
| D-049 | `frk_str`: UTF-16 code-unit semantics; rt-owned malloc-domain values; unmanaged-pointer corollary. |
| D-050 | UTF-16 ruling recorded on the `.length` evidence; the trigger fired at implementation time. |
| D-052 | Lua strings are interned 8-bit byte strings, identity-equal; not `frk_str` values. |
| D-056 | `frk_bstr` as a sibling micro-dialect; interning as representation; inline eq/len; ASCII literal fence. |
| D-058 | `sub`/`rep` join the surface with Lua's negative-tolerant indexing. |

The `%.14g` contract and the print-canon fences that make three-way string
output comparable at all are covered in
[Golden Discipline and Canon](../method/goldens.md).
