//! frk-core — MLIR context plumbing, source locations, the diagnostics
//! bridge, and the green tree (SPEC §2 L0, §6.2, §6.5).
//!
//! M0 scope: context construction only. Dialects are registered *and
//! loaded* eagerly — melior is alpha and touching an unloaded dialect can
//! segfault (docs/LANDSCAPE.md), so we trade a little startup time for an
//! absent failure mode. Diagnostics currently bridge to stderr; the
//! source-mapped report bridge (SPEC §6.5) arrives with the frontend kit.

use melior::{
    Context,
    dialect::DialectRegistry,
    utility::{register_all_dialects, register_all_llvm_translations},
};

/// Builds an MLIR context with every dialect known to the linked MLIR
/// registered, loaded, and with LLVM translations attached.
pub fn context() -> Context {
    let context = Context::new();

    let registry = DialectRegistry::new();
    register_all_dialects(&registry);
    context.append_dialect_registry(&registry);
    context.load_all_available_dialects();
    register_all_llvm_translations(&context);

    context.attach_diagnostic_handler(|diagnostic| {
        eprintln!("mlir: {diagnostic}");
        true
    });

    context
}

#[cfg(test)]
mod tests {
    #[test]
    fn context_loads_all_dialects_eagerly() {
        let context = super::context();
        // Far more than the builtin dialect; the exact count tracks MLIR.
        assert!(context.loaded_dialect_count() > 10);
    }
}
