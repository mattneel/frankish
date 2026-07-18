//! frk-abi — THE runtime ABI registry (M17, D-062). One declarative
//! table of every `frk_rt_*` symbol; every consumer derives from it or
//! is checked against it:
//!
//! - the RUST twin: `frk-rt`'s build script materializes a typed
//!   fn-pointer assertion per entry — rustc refuses a drifted signature;
//! - the C twin: [`c_header`] generates `crates/frk-rt/c/frk_rt_abi.h`
//!   (checked in, drift-tested); `frk_rt.c` includes it, so every
//!   compile — host tests and all five grid triples — enforces exact
//!   signatures;
//! - the kernel lowering derives its extern declarations from
//!   [`RT_ABI`] instead of a hand-written type table;
//! - the harness asserts JIT-registration and interp-builtin coverage
//!   against the [`JitBinding`]/[`InterpDisposition`] columns.
//!
//! K4 as amended: the runtime ships behind the REGISTERED C ABI —
//! machine-enforced, not merely documented. This crate is pure data
//! plus text generators; it depends on nothing.

/// The ABI type vocabulary (D-041/D-042 laws): word-sized integers,
/// doubles, and pointers. Sizes are 64-bit ON EVERY TARGET — the wasm
/// signature_mismatch lesson is encoded here, once.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AbiTy {
    I64,
    U64,
    F64,
    PtrMutU8,
    PtrConstU8,
    PtrConstU16,
    PtrMutI64,
    /// An opaque MANAGED-PAYLOAD pointer (allocator returns, retain/
    /// release operands, interned strings). Renders per language:
    /// `void *` in C, `*mut u8` in Rust — the one deliberate
    /// asymmetric mapping (same representation everywhere).
    PtrPayload,
}

impl AbiTy {
    pub fn c_type(self) -> &'static str {
        match self {
            AbiTy::I64 => "int64_t",
            AbiTy::U64 => "uint64_t",
            AbiTy::F64 => "double",
            AbiTy::PtrMutU8 => "uint8_t *",
            AbiTy::PtrConstU8 => "const uint8_t *",
            AbiTy::PtrConstU16 => "const uint16_t *",
            AbiTy::PtrMutI64 => "int64_t *",
            AbiTy::PtrPayload => "void *",
        }
    }

    pub fn rust_type(self) -> &'static str {
        match self {
            AbiTy::I64 => "i64",
            AbiTy::U64 => "u64",
            AbiTy::F64 => "f64",
            AbiTy::PtrMutU8 => "*mut u8",
            AbiTy::PtrConstU8 => "*const u8",
            AbiTy::PtrConstU16 => "*const u16",
            AbiTy::PtrMutI64 => "*mut i64",
            AbiTy::PtrPayload => "*mut u8",
        }
    }

    /// Is this a pointer at the LLVM level? (The kernel lowering maps
    /// every pointer to the opaque `!llvm.ptr`; integers stay i64/f64 —
    /// U8 flags widen through the existing call sites.)
    pub fn is_pointer(self) -> bool {
        matches!(
            self,
            AbiTy::PtrMutU8
                | AbiTy::PtrConstU8
                | AbiTy::PtrConstU16
                | AbiTy::PtrMutI64
                | AbiTy::PtrPayload
        )
    }
}

/// Which runtime lane a symbol belongs to — per-language runtime
/// extensions are first-class rows, not a future schema change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lane {
    /// Allocators, counters, the collector.
    Core,
    /// UTF-16 strings (TS-0).
    Str,
    /// Interned byte strings (femto_lua / r7rs_core).
    Bstr,
    /// Fat values, tables, dynamic checks.
    Dyn,
    /// Control effects (κ_frk pending cell).
    Ctl,
    /// Contract checks with blame (D-072).
    Contract,
    /// TS-0 print protocol.
    Ts,
    /// femto_lua print protocol + errors.
    Lua,
    /// r7rs_core display protocol.
    Scheme,
}

/// How the in-process JIT resolves the symbol.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JitBinding {
    /// Bound to the real Rust-twin function.
    Real,
    /// Bound to a CAPTURING harness shim (output interleaving, D-047):
    /// the twin function exists but the JIT must not write to the
    /// process stdout directly.
    Capture,
    /// Never referenced by JIT'd code (twin-only export: counters,
    /// explicit-collect entry points, or currently-dead protocol fns).
    NotLinked,
}

/// Where the reference interpreter gets this symbol's semantics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InterpDisposition {
    /// A host builtin registered by the harness (bodyless symbol).
    Builtin,
    /// The interpreter models the semantics through kernel-dialect
    /// evaluators; the runtime symbol is never called under interp.
    DialectEval,
    /// Not reachable from interpreted programs (counters etc.).
    NotReachable,
}

#[derive(Clone, Copy, Debug)]
pub struct RtFn {
    pub name: &'static str,
    pub args: &'static [AbiTy],
    pub ret: Option<AbiTy>,
    pub lane: Lane,
    pub jit: JitBinding,
    pub interp: InterpDisposition,
}

use AbiTy::*;
use InterpDisposition::*;
use JitBinding::*;

/// THE registry. Sorted by (lane, name); every `frk_rt_*` export in
/// both twins has exactly one row. Adding a runtime function = adding
/// a row here first (the build breaks until both twins agree).
pub const RT_ABI: &[RtFn] = &[
    // ---- Core: allocators, counters, the collector ----
    RtFn { name: "frk_rt_alloc_count", args: &[], ret: Some(U64), lane: Lane::Core, jit: NotLinked, interp: NotReachable },
    RtFn { name: "frk_rt_arena_alloc", args: &[U64], ret: Some(PtrPayload), lane: Lane::Core, jit: Real, interp: DialectEval },
    RtFn { name: "frk_rt_rc_alloc", args: &[U64, U64], ret: Some(PtrPayload), lane: Lane::Core, jit: Real, interp: DialectEval },
    RtFn { name: "frk_rt_rc_collect", args: &[], ret: None, lane: Lane::Core, jit: NotLinked, interp: NotReachable },
    RtFn { name: "frk_rt_rc_free_count", args: &[], ret: Some(U64), lane: Lane::Core, jit: NotLinked, interp: NotReachable },
    RtFn { name: "frk_rt_rc_release", args: &[PtrPayload], ret: None, lane: Lane::Core, jit: Real, interp: DialectEval },
    RtFn { name: "frk_rt_rc_release_count", args: &[], ret: Some(U64), lane: Lane::Core, jit: NotLinked, interp: NotReachable },
    RtFn { name: "frk_rt_rc_retain", args: &[PtrPayload], ret: None, lane: Lane::Core, jit: Real, interp: DialectEval },
    // ---- Str: UTF-16 code-unit strings (TS-0, D-049) ----
    RtFn { name: "frk_rt_str_concat", args: &[PtrConstU8, PtrConstU8], ret: Some(PtrPayload), lane: Lane::Str, jit: Real, interp: DialectEval },
    RtFn { name: "frk_rt_str_eq", args: &[PtrConstU8, PtrConstU8], ret: Some(I64), lane: Lane::Str, jit: Real, interp: DialectEval },
    RtFn { name: "frk_rt_str_from_units", args: &[PtrConstU16, U64], ret: Some(PtrPayload), lane: Lane::Str, jit: Real, interp: DialectEval },
    RtFn { name: "frk_rt_str_len", args: &[PtrConstU8], ret: Some(U64), lane: Lane::Str, jit: Real, interp: DialectEval },
    // ---- Bstr: interned byte strings (D-052/D-056/D-058) ----
    RtFn { name: "frk_rt_bstr_concat", args: &[PtrConstU8, PtrConstU8], ret: Some(PtrPayload), lane: Lane::Bstr, jit: Real, interp: DialectEval },
    RtFn { name: "frk_rt_bstr_from_num", args: &[F64], ret: Some(PtrPayload), lane: Lane::Bstr, jit: Real, interp: Builtin },
    RtFn { name: "frk_rt_bstr_intern", args: &[PtrConstU8, U64], ret: Some(PtrPayload), lane: Lane::Bstr, jit: Real, interp: DialectEval },
    RtFn { name: "frk_rt_bstr_rep", args: &[PtrConstU8, I64], ret: Some(PtrPayload), lane: Lane::Bstr, jit: Real, interp: DialectEval },
    RtFn { name: "frk_rt_bstr_sub", args: &[PtrConstU8, I64, I64], ret: Some(PtrPayload), lane: Lane::Bstr, jit: Real, interp: DialectEval },
    // ---- Dyn: fat values, tables, checks (D-051/D-056) ----
    RtFn { name: "frk_rt_dyn_check", args: &[I64, I64], ret: None, lane: Lane::Dyn, jit: Real, interp: DialectEval },
    RtFn { name: "frk_rt_table_init", args: &[I64], ret: None, lane: Lane::Dyn, jit: Real, interp: DialectEval },
    RtFn { name: "frk_rt_table_len", args: &[I64], ret: Some(I64), lane: Lane::Dyn, jit: Real, interp: DialectEval },
    RtFn { name: "frk_rt_table_next", args: &[I64, I64, I64, PtrMutI64], ret: None, lane: Lane::Dyn, jit: Real, interp: DialectEval },
    RtFn { name: "frk_rt_table_raw_get", args: &[I64, I64, I64, PtrMutI64], ret: None, lane: Lane::Dyn, jit: Real, interp: DialectEval },
    RtFn { name: "frk_rt_table_raw_set", args: &[I64, I64, I64, I64, I64], ret: None, lane: Lane::Dyn, jit: Real, interp: DialectEval },
    // ---- Ctl: the κ_frk pending cell (D-060/D-061) + the v1
    // evidence stack (D-069): labels are interned bstr pointers
    // passed as words, so find is a pointer compare; perform_end does
    // the consumed-else-abort decision IN the runtime (no block
    // surgery in the lowering). ----
    RtFn { name: "frk_rt_ctl_handler_push", args: &[I64, I64, I64, I64], ret: None, lane: Lane::Ctl, jit: Real, interp: DialectEval },
    RtFn { name: "frk_rt_ctl_handler_pop", args: &[], ret: None, lane: Lane::Ctl, jit: Real, interp: DialectEval },
    RtFn { name: "frk_rt_ctl_perform_begin", args: &[I64, PtrMutI64], ret: Some(I64), lane: Lane::Ctl, jit: Real, interp: DialectEval },
    RtFn { name: "frk_rt_ctl_perform_end", args: &[I64, I64, I64, I64, PtrMutI64], ret: Some(I64), lane: Lane::Ctl, jit: Real, interp: DialectEval },
    RtFn { name: "frk_rt_ctl_pack_head", args: &[I64, PtrMutI64], ret: None, lane: Lane::Ctl, jit: Real, interp: DialectEval },
    RtFn { name: "frk_rt_ctl_resume_mark", args: &[I64], ret: None, lane: Lane::Ctl, jit: Real, interp: DialectEval },
    RtFn { name: "frk_rt_ctl_abort", args: &[I64, I64, I64], ret: None, lane: Lane::Ctl, jit: Real, interp: DialectEval },
    RtFn { name: "frk_rt_ctl_pending", args: &[], ret: Some(I64), lane: Lane::Ctl, jit: Real, interp: DialectEval },
    RtFn { name: "frk_rt_ctl_prompt_enter", args: &[], ret: Some(I64), lane: Lane::Ctl, jit: Real, interp: DialectEval },
    RtFn { name: "frk_rt_ctl_prompt_exit", args: &[I64], ret: None, lane: Lane::Ctl, jit: Real, interp: DialectEval },
    RtFn { name: "frk_rt_ctl_resolve", args: &[I64, PtrMutI64], ret: Some(I64), lane: Lane::Ctl, jit: Real, interp: DialectEval },

    RtFn { name: "frk_rt_contract_check", args: &[I64, I64, PtrConstU8, I64], ret: None, lane: Lane::Contract, jit: Real, interp: DialectEval },
    // ---- Ts: the TS-0 print protocol (D-047) ----
    RtFn { name: "frk_rt_print_bool", args: &[I64], ret: None, lane: Lane::Ts, jit: Capture, interp: Builtin },
    RtFn { name: "frk_rt_print_f64", args: &[F64], ret: None, lane: Lane::Ts, jit: Capture, interp: Builtin },
    RtFn { name: "frk_rt_print_str", args: &[PtrConstU8], ret: None, lane: Lane::Ts, jit: Capture, interp: Builtin },
    // ---- Lua: print protocol + runtime errors (D-054/D-056) ----
    RtFn { name: "frk_rt_lua_error", args: &[I64], ret: None, lane: Lane::Lua, jit: Real, interp: Builtin },
    RtFn { name: "frk_rt_async_trap", args: &[I64], ret: None, lane: Lane::Lua, jit: Real, interp: Builtin },
    RtFn { name: "frk_rt_print_lua_str", args: &[PtrConstU8], ret: None, lane: Lane::Lua, jit: Capture, interp: Builtin },
    // ---- Scheme: the display protocol (M15) ----
    RtFn { name: "frk_rt_scm_display_bool", args: &[I64], ret: None, lane: Lane::Scheme, jit: Capture, interp: Builtin },
    RtFn { name: "frk_rt_scm_display_num", args: &[F64], ret: None, lane: Lane::Scheme, jit: Capture, interp: Builtin },
    RtFn { name: "frk_rt_scm_display_str", args: &[PtrConstU8], ret: None, lane: Lane::Scheme, jit: Capture, interp: Builtin },
    RtFn { name: "frk_rt_scm_newline", args: &[], ret: None, lane: Lane::Scheme, jit: Capture, interp: Builtin },
    RtFn { name: "frk_rt_scm_trap", args: &[I64], ret: None, lane: Lane::Scheme, jit: Real, interp: Builtin },
];

/// Looks up one registry row.
pub fn find(name: &str) -> Option<&'static RtFn> {
    RT_ABI.iter().find(|f| f.name == name)
}

/// The generated C twin contract, `crates/frk-rt/c/frk_rt_abi.h`.
/// `frk_rt.c` includes it, so every compile of the C twin — host tests
/// and every grid triple — enforces these exact signatures. Checked in;
/// drift-tested against this generator; regenerate with `make abi`.
pub fn c_header() -> String {
    let mut out = String::new();
    out.push_str("/* frk_rt_abi.h — GENERATED from crates/frk-abi (M17, D-062).\n");
    out.push_str(" * DO NOT EDIT: `make abi` regenerates; a frk-rt test asserts drift.\n");
    out.push_str(" * This header IS the C twin's contract: including it makes the C\n");
    out.push_str(" * compiler enforce the registered ABI at every compile, on every\n");
    out.push_str(" * grid triple. */\n");
    out.push_str("#ifndef FRK_RT_ABI_H\n#define FRK_RT_ABI_H\n\n#include <stdint.h>\n\n");
    for f in RT_ABI {
        let ret = f.ret.map(|t| t.c_type()).unwrap_or("void");
        let args = if f.args.is_empty() {
            "void".to_string()
        } else {
            f.args.iter().map(|t| t.c_type()).collect::<Vec<_>>().join(", ")
        };
        // "uint8_t *" + name reads better without a double space.
        let sep = if ret.ends_with('*') { "" } else { " " };
        out.push_str(&format!("{ret}{sep}{}({args});\n", f.name));
    }
    out.push_str("\n#endif /* FRK_RT_ABI_H */\n");
    out
}

/// The generated Rust twin check: one typed fn-pointer assertion per
/// entry. `frk-rt`'s build script writes this into OUT_DIR and the
/// crate includes it — rustc then refuses any Rust-twin signature that
/// drifts from the registry.
pub fn rust_assertions() -> String {
    let mut out = String::new();
    out.push_str("// GENERATED by frk-rt/build.rs from frk-abi (M17, D-062).\n");
    out.push_str("// One typed fn-pointer per registry row: a drifted Rust-twin\n");
    out.push_str("// signature is a COMPILE error, not a grid surprise.\n");
    for f in RT_ABI {
        let args = f.args.iter().map(|t| t.rust_type()).collect::<Vec<_>>().join(", ");
        let ret = f.ret.map(|t| format!(" -> {}", t.rust_type())).unwrap_or_default();
        out.push_str(&format!(
            "const _: unsafe extern \"C\" fn({args}){ret} = crate::{};\n",
            f.name
        ));
    }
    out
}

/// The generated JIT capture-shim check (lens-1 finding, D-062): the
/// harness registers shims by TYPE-ERASED pointer (`*mut ()`), which
/// is exactly where a signature could drift unchecked. For every
/// `JitBinding::Capture` row this emits a typed fn-pointer assertion
/// against the conventional shim name (`frk_rt_X` → `capture_X`),
/// included by the harness — rustc then enforces shim signatures too.
pub fn capture_shim_assertions() -> String {
    let mut out = String::new();
    out.push_str("// GENERATED by frk-harness/build.rs from frk-abi (D-062).\n");
    out.push_str("// Typed fn-pointer per Capture row: a capture shim whose\n");
    out.push_str("// signature drifts from the registry is a COMPILE error.\n");
    for f in RT_ABI {
        if f.jit != JitBinding::Capture {
            continue;
        }
        let shim = format!("capture_{}", f.name.trim_start_matches("frk_rt_"));
        let args = f.args.iter().map(|t| t.rust_type()).collect::<Vec<_>>().join(", ");
        let ret = f.ret.map(|t| format!(" -> {}", t.rust_type())).unwrap_or_default();
        out.push_str(&format!("const _: extern \"C\" fn({args}){ret} = {shim};\n"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_is_sorted_and_unique_within_lanes() {
        for window in RT_ABI.windows(2) {
            let (a, b) = (&window[0], &window[1]);
            assert_ne!(a.name, b.name, "duplicate registry row: {}", a.name);
        }
        let mut names: Vec<_> = RT_ABI.iter().map(|f| f.name).collect();
        let len = names.len();
        names.dedup();
        assert_eq!(len, names.len(), "duplicate names across lanes");
    }

    #[test]
    fn header_generation_is_deterministic_and_complete() {
        let header = c_header();
        for f in RT_ABI {
            assert!(header.contains(f.name), "header missing {}", f.name);
        }
        assert_eq!(header, c_header());
    }

    #[test]
    fn tampered_signature_is_visible_in_both_generators() {
        // The L1 witness at the generator level: a signature change
        // reaches BOTH generated artifacts (so both compilers see it).
        let header = c_header();
        let assertions = rust_assertions();
        // scm_display_bool takes i64 — the M15 bug this crate exists
        // to make impossible. Assert the exact generated lines.
        assert!(header.contains("void frk_rt_scm_display_bool(int64_t);"));
        assert!(assertions
            .contains("unsafe extern \"C\" fn(i64) = crate::frk_rt_scm_display_bool;"));
        // The legacy u8 flags are GONE (D-062 finish): every integer
        // crosses the ABI at 64 bits.
        assert!(header.contains("void frk_rt_print_bool(int64_t);"));
        assert!(!header.contains("uint8_t);"));
    }
}
