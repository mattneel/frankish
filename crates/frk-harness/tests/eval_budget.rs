//! D-150: the compile-time evaluation service's resource contract,
//! tested at the service boundary (no Calque involved): a
//! non-terminating module fails deterministically with the step-budget
//! diagnostic, and a terminating one is unaffected.

use frk_harness::runner::{eval_module_i64, COMPTIME_STEP_BUDGET};

const SPIN: &str = r#"
func.func @spin(%arg: i64) -> i64 {
  cf.br ^loop
^loop:
  cf.br ^loop
}
"#;

const SIX_BY_SIX: &str = r#"
func.func @square(%arg: i64) -> i64 {
  %c = arith.constant 6 : i64
  %r = arith.muli %c, %c : i64
  return %r : i64
}
"#;

fn on_big_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(frk_interp::STACK_SIZE)
        .spawn(f)
        .expect("spawn")
        .join()
        .expect("join")
}

#[test]
fn a_non_terminating_request_exhausts_the_step_budget() {
    let err = on_big_stack(|| eval_module_i64(SPIN, "spin", 0)).expect_err("the loop is cut off");
    assert!(
        err.contains(&format!("step budget exhausted after {COMPTIME_STEP_BUDGET} operations")),
        "{err}"
    );
    // Deterministic: the same request fails with the same text.
    let again = on_big_stack(|| eval_module_i64(SPIN, "spin", 0)).expect_err("again");
    assert_eq!(err, again);
}

#[test]
fn a_terminating_request_is_unaffected() {
    assert_eq!(on_big_stack(|| eval_module_i64(SIX_BY_SIX, "square", 0)), Ok(36));
}

// D-151: an explicit per-request limit replaces the default; 0 selects
// the default; out-of-range limits are refused before evaluation.
#[test]
fn an_explicit_limit_bounds_the_request() {
    use frk_harness::runner::eval_module_i64_limited;
    let err = on_big_stack(|| eval_module_i64_limited(SPIN, "spin", 0, 1000)).expect_err("cut off");
    assert!(err.contains("step budget exhausted after 1000 operations"), "{err}");
    assert_eq!(on_big_stack(|| eval_module_i64_limited(SIX_BY_SIX, "square", 0, 3)), Ok(36));
    let tight = on_big_stack(|| eval_module_i64_limited(SIX_BY_SIX, "square", 0, 2)).expect_err("3 ops needed");
    assert!(tight.contains("step budget exhausted after 2 operations"), "{tight}");
    let neg = on_big_stack(|| eval_module_i64_limited(SIX_BY_SIX, "square", 0, -5)).expect_err("refused");
    assert!(neg.contains("a step limit must be between 1 and"), "{neg}");
}
