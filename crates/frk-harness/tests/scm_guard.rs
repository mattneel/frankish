//! Guard fence witnesses (D-081.5) — the chibi-poison shapes that
//! must NEVER enter the corpus, pinned as unit tests: chibi computes
//! a value for the continuable re-raise (needs re-entrant kappa —
//! Tier-2) and exits 0 on handler-returned-under-guard (secondary
//! catchable exception); we trap deterministically on both.

use frk_harness::runner::{InterpRunner, Runner};

fn run_scheme(name: &str, source: &str) -> Result<String, String> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dir = root.join(format!("target/{name}-fixture"));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("case.scm"), source).unwrap();
    std::fs::write(dir.join("expected.out"), "unreachable\n").unwrap();
    let cases = frk_harness::case::discover(&dir).unwrap();
    InterpRunner.run(&cases[0]).map_err(|e| e.to_string())
}

#[test]
fn continuable_reraise_through_else_less_guard_hits_the_tier2_fence_trap() {
    // P13: chibi resumes the original raise site with the outer
    // handler's return value (11). We cannot before Tier-2 — and a
    // silent wrong value would be an L3 lie, so the fence is LOUD.
    let error = run_scheme(
        "scm-guard-p13",
        "(with-exception-handler (lambda (e) 10)\n  (lambda () (guard (e ((eq? e 'x) 1)) (+ 1 (raise-continuable 'y)))))\n",
    )
    .expect_err("continuable re-raise through an else-less guard must trap");
    assert!(
        error.contains("guard re-raise of a continuable condition is fenced"),
        "{error}"
    );
    assert!(error.contains("Tier-2"), "{error}");
}

#[test]
fn handler_returning_under_an_enclosing_guard_still_traps() {
    // r_hret_in_guard: chibi converts handler-returned into a
    // SECONDARY CATCHABLE exception the guard catches (exit 0!) —
    // "catch everything" is not enough to keep this shape green on
    // both sides, hence the sharper manifest law.
    let error = run_scheme(
        "scm-guard-hret",
        "(display (guard (e (#t 'caught))\n  (with-exception-handler (lambda (x) 'returned)\n    (lambda () (raise 'boom)))))\n",
    )
    .expect_err("a handler returning from a plain raise traps even under a guard");
    assert!(error.contains("exception handler returned (raise)"), "{error}");
}

#[test]
fn arrow_and_bare_test_clauses_are_parse_rejections() {
    let arrow = run_scheme(
        "scm-guard-arrow",
        "(display (guard (e ((eq? e 'a) => car)) (raise 'a)))\n",
    )
    .expect_err("(test => proc) must be refused at parse");
    assert!(arrow.contains("=>"), "{arrow}");
    assert!(arrow.contains("D-081"), "{arrow}");

    let bare = run_scheme(
        "scm-guard-bare",
        "(display (guard (e ((+ e 1))) (raise 41)))\n",
    )
    .expect_err("(test) with no expressions must be refused at parse");
    assert!(bare.contains("without expressions"), "{bare}");
    assert!(bare.contains("D-081"), "{bare}");
}
