//! calque — the host side of the Calque workstream (M37, D-087):
//! driver services over shared Frankish machinery. NO Calque frontend
//! logic lives here (goal §4's prohibition): the lexer, parser, type
//! rules, and emitter are Calque source under `tools/calque/`; this
//! crate assembles units, invokes the seed, and composes, parses,
//! verifies, and runs artifacts.
//!
//! Composition contract (BOOTSTRAP §2.1): a runnable module is
//! `intrinsics.mlir` + the artifact text + input injection. v0 of
//! this crate ships the first two pieces and the marker check; input
//! injection and the seed invocation land with the driver slab.

/// The artifact version marker an emitted module MUST carry as its
/// first line (BOOTSTRAP §2). Bumping it is a D-entry that must
/// preserve existing producers.
pub const ARTIFACT_MARKER: &str = "// calque-artifact v0";

/// Calque's kernel-IR intrinsics module — the D-062 authoring-surface
/// pattern, Calque's turn (the fourth frontend after lua/scheme/ts).
/// Bodyless `frk_rt_*` declarations here are checked against the
/// frk-abi registry by the semantic verifier, and the kernel lowering
/// dedups against symbols declared here.
pub fn intrinsics_source() -> &'static str {
    include_str!("intrinsics.mlir")
}

/// Refuses an artifact whose version marker is missing or different
/// (BOOTSTRAP §2: the driver never guesses a version).
pub fn check_artifact_marker(artifact: &str) -> Result<(), String> {
    match artifact.lines().next() {
        Some(line) if line.trim_end() == ARTIFACT_MARKER => Ok(()),
        other => Err(format!(
            "artifact must begin with {ARTIFACT_MARKER:?}, got {:?}",
            other.unwrap_or("<empty artifact>")
        )),
    }
}

/// Composes a runnable module text from an artifact: intrinsics first
/// (the seed-module role), then the artifact's own functions.
pub fn compose_unit(artifact: &str) -> String {
    format!("{}\n{}\n", intrinsics_source(), artifact)
}
