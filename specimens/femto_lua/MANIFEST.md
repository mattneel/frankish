# specimen: femto_lua — v0.1 (RATIFIED at M10, D-052)

## Identity & pin
Lua 5.1.5 (PUC-Rio C sources = readable spec; official test suite =
corpus source, license-checked before vendoring per specimen law;
LuaJIT = perf yardstick, informational only). Oracle: the lua5.1
binary (5.1.5 pinned in versions.env), LC_ALL=C, through canon.

## Role
Wakes the runtime dragon: frk.dyn (fat values v0, D-051), byte
strings (interned, identity-equal — D-052; NOT frk_str), tables,
metatable dispatch (v0.1: __index only; itab mapping ruled at the
implementation milestone with the table design in hand, D-026),
GC pressure (the M10 gate is DECIDED: rc+cycles, D-053 /
docs/gc-spike.md).

## Scope grammar (v0.1)
nil, boolean, number (f64), string (8-bit-clean bytes, interned,
identity-equal; .. concatenation and # length only); local
declarations; functions + closures (upvalues by reference — frk_mem
boxes); tables with unified array+hash semantics, constructors,
index read/write; if/elseif/else, while, numeric for; metatables:
setmetatable/getmetatable with __index (table and function forms);
print + tostring (oracle protocol); # on strings and tables.

## Fences (v0.1 — law, L5)
Coroutines (arrive with frk.ctl), goto, varargs, multiple return
values except a single call in tail position, load/loadstring,
weak tables, the string library beyond .. and #, string.format,
__newindex and all metamethods beyond __index, the io/os libraries
(print only), integer division/modulo edge exotica until canon
rules them.

## Conformance
Hand corpus per idiom first (the ml_core precedent); the official
5.1 test suite slices in after license verification. lua5.1 is
ground truth through the canon filter; number printing gets its own
canon fence at implementation (Lua spells %.14g — the TS-0 §6
precedent applies).

## Status
v0.3 SHIPPED (M23/D-068): the explist ADJUSTMENT engine — one
mechanism for varargs (`...` pack-native, prologue copy before the
D-067 dispose), mid-explist truncation/final expansion, multi-
expression RHS destructuring, explicit generic-for iterator triples,
and constructor tails — plus __newindex (luaV_settable in
intrinsics.mlir: existing keys raw, absent keys walk the metamethod,
table form re-enters as a tail call). print() went multi-value
(tab-joined) and next() returns one nil at exhaustion — pack LENGTHS
became observable under expansion and the oracle ruled both. Corpus:
18 cases, 100% vs lua5.1, all runners, all five triples. Two
jit-rc-segfault kernel findings fixed en route (created-pack borrow
gate; transfer-requires-owned-producer retain rule). Still fenced
(v0.4+): select(), `...` at top level, string.format, rawset/rawget,
method declarations/colon calls, coroutines, goto, weak tables.

Previously — v0.2 SHIPPED (2026-07-03, M13/D-058): the pack calling convention —
fn<[arr<dyn>], [arr<dyn>]>, one fn type for every function — brings
multiple return values (return explists, destructuring locals and
assignments, tail-position pack forwarding) and DISSOLVES the exact-
arity fence (nil-fill/drop is the callee prologue). Also lifted:
repeat/until, break, generic for with pairs/ipairs/next (iteration
order is implementation-defined — canon: corpus aggregates or uses
ipairs), and string.sub/string.rep as a seeded module. Still fenced
(v0.3+): varargs, mid-explist spreads, explicit iterator triples,
multi-expression RHS. Corpus: 12 cases, 100% vs lua5.1, all runners,
all five triples.

Previously — v0.1 (M11/D-054): the full ratified scope —
locals, functions/closures/upvalues, tables, __index (both forms),
strings, control flow, print/tostring/setmetatable/getmetatable —
compiles through frk_dyn/frk_bstr/frk_mem/frk_closure and runs
byte-identical to lua5.1 across interp, jit×{arena,rc}, and the
five-architecture grid. Corpus: 8 idiom cases, 100% (bar: ≥90%),
including the D-055 %.14g tie case three ways. Fences live: D-052 +
D-054 (exact arity, no coercing arithmetic, in-fence corpus).
