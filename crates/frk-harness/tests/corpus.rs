//! The repository golden corpus, run for real: every case under /goldens
//! through the JIT runner in Check mode. This test is what makes
//! `make test` the L2 gate — and it encodes the M1 exit criterion
//! (≥5 goldens over upstream-dialect programs) so the suite regresses if
//! the corpus ever shrinks below it.

use std::path::PathBuf;

use frk_harness::golden::{Mode, run_goldens};
use frk_harness::runner::JitRunner;

fn corpus_root() -> PathBuf {
    // crates/frk-harness → repo root → goldens/
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("goldens")
}

#[test]
fn repository_goldens_are_green_under_jit() {
    let report = run_goldens(&corpus_root(), &JitRunner, Mode::Check)
        .expect("corpus discovery failed");
    assert!(
        report.outcomes.len() >= 5,
        "M1 exit criterion: ≥5 goldens (found {})",
        report.outcomes.len()
    );
    assert!(report.is_green(), "\n{report}\n");
}

/// The standing L3 gate: every registered runner must agree byte-exactly
/// on every golden. Trivial with jit alone; the moment M2 appends the
/// interpreter to default_runners(), this test becomes the two-way diff
/// required by that milestone's exit criterion — no new wiring needed.
#[test]
fn repository_goldens_agree_across_default_runners() {
    let runners = frk_harness::runner::default_runners();
    let refs: Vec<&dyn frk_harness::runner::Runner> =
        runners.iter().map(|boxed| boxed.as_ref()).collect();
    let report = frk_harness::diff::diff_corpus(&corpus_root(), &refs)
        .expect("corpus discovery failed");
    assert!(report.is_green(), "\n{report}\n");
}
