//! frk-interp — per-op Eval interface and the derived interpreter
//! (SPEC §7.1). The interpreter is the reference semantics: JIT/AOT must
//! byte-match it on every golden (law L3, D-008).
//!
//! Built at M2, over upstream dialects first.
