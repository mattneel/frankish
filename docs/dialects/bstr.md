# frk.bstr — interned byte strings

Lua's string law (D-052/D-056): 8-bit values, interned at creation,
identity-equal. A sibling of frk_str, never an overload — UTF-16
code-unit semantics is TS's, byte semantics is Lua's.

| op | signature | lowering |
|----|-----------|----------|
| `lit {text}` | `() -> str` | bytes global + `frk_rt_bstr_intern` |
| `concat` | `(str, str) -> str` | `frk_rt_bstr_concat` (re-interns) |
| `eq` | `(str, str) -> i1` | **inline pointer compare** |
| `len` | `(str) -> i64` | **inline header load** |

Interning is the representation: eq and len never touch the runtime.
The interpreter uses content equality (Value::Bytes) — observably
identical to interned identity. `frk_rt_bstr_from_num` formats %.14g
into an interned string (canon §7's formatter). v0.1 literals are
ASCII (verifier-enforced); values are 8-bit clean regardless.
