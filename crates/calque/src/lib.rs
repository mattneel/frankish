//! calque — the Calque workstream's host services, re-exported from
//! `frk_harness::calque` (M37, D-087): unit assembly, the seed
//! invocation, artifact composition, and the run path. NO Calque
//! frontend logic lives in Rust (goal §4): the lexer, parser, and
//! emitter are Calque source under `tools/calque/`.
//!
//! The implementation lives in frk-harness because the golden/grid
//! machinery must reach it and the harness cannot depend on this crate
//! (this crate depends on the harness).

pub use frk_harness::calque::{
    ARTIFACT_MARKER, RunOutcome, UnitFile, check_artifact_marker, compiler_sources,
    compose_unit, compose_unit_with_input, intrinsics_source, input_injection, run_artifact,
    compile_source, opt_source, res_source, run_test_blocks, seq_source,
    run_artifact_with_input, seed_translate, seed_translate_jit, stdlib_source,
};
