# frk.dyn — uniform dynamic values (v0: contract only)

Status: K1/K2 shipped at M10 (D-051 — the SPEC §4.5 tagging fork,
ruled); K3+ scheduled with the femto_lua implementation milestone.
Until then dyn goldens ride `runners=interp`.

## The ruling (D-051)

v0 representation is **fat values**: `{tag: i64, payload}` — two
slots, the closure precedent. NaN-boxing and pointer tagging are
representation *optimizations* behind this same surface; the
K-contract makes representation a lowering detail, so the swap is a
later profile knob decided on measurement, not aesthetics.

## Ops

| op | signature | semantics |
|----|-----------|-----------|
| `wrap` | `(T) {tag} -> dyn` | tag + payload |
| `unwrap` | `(dyn) {tag} -> T` | payload; **traps** on tag mismatch, with source location |
| `tag_of` | `(dyn) -> i64` | the tag |

Tag space v0 (closed enum, femto_lua's six): nil=0 bool=1 num=2
str=3 table=4 fun=5. Dispatch (itabs, D-026) is deliberately not in
v0 — metatable dispatch design belongs with the table design.
