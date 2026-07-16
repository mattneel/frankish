//! Intrinsics modules (M17, D-062; SPEC §6.6) — the frontend kit's
//! authoring surface for language primitives.
//!
//! A language's intrinsics are ordinary kernel IR in a `.mlir` file
//! shipped with its frontend (embedded via `include_str!` — no runtime
//! file paths, L6). Compilation begins by parsing that file as the
//! SEED MODULE; the emitter appends its functions into it. Seeded
//! functions are verified by the same MLIR + frankish semantic
//! verification every module passes — including the D-062 check that
//! every `frk_rt_*` declaration they carry matches the registry — and
//! they run under K2 (interp) and K3 (native) with zero new machinery,
//! because they are just functions.
//!
//! Symbol hygiene: intrinsic symbols are namespaced `__<lang>_…`;
//! runtime declarations an intrinsics file carries are deduplicated
//! against the kernel lowering's own declarer (it skips sym_names the
//! module already has).

use melior::Context;
use melior::ir::Module;

/// Parses `source` (a language's intrinsics module) as the module the
/// emitter will append into. A parse failure names the language: an
/// intrinsics file is compiler-internal, so failing loudly at first
/// use is the correct behavior (there is no user program to blame).
pub fn seed_module<'c>(
    context: &'c Context,
    language: &str,
    source: &str,
) -> Result<Module<'c>, String> {
    Module::parse(context, source)
        .ok_or_else(|| format!("the {language} intrinsics module failed to parse (D-062)"))
}
