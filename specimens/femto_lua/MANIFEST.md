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
v0.1 SHIPPED (2026-07-03, M11/D-054): the full ratified scope —
locals, functions/closures/upvalues, tables, __index (both forms),
strings, control flow, print/tostring/setmetatable/getmetatable —
compiles through frk_dyn/frk_bstr/frk_mem/frk_closure and runs
byte-identical to lua5.1 across interp, jit×{arena,rc}, and the
five-architecture grid. Corpus: 8 idiom cases, 100% (bar: ≥90%),
including the D-055 %.14g tie case three ways. Fences live: D-052 +
D-054 (exact arity, no coercing arithmetic, in-fence corpus).
