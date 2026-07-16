//! Interpreter verifiers (law L1: landed with the interpreter itself).
//! Every registered op family is exercised; MLIR-level UB must trap
//! deterministically (D-029); coverage boundaries must fail loudly.

use frk_interp::{EvalError, Value, interpret_entry};
use melior::ir::Module;
use melior::ir::operation::OperationLike;

fn interpret(source: &str, entry: &str) -> Result<Vec<Value>, EvalError> {
    let context = frk_core::context();
    let module = Module::parse(&context, source).expect("test source must parse");
    assert!(
        module.as_operation().verify(),
        "test source must pass the MLIR verifier"
    );
    interpret_entry(&module, entry, &[])
}

fn interpret_i64(source: &str) -> Result<i64, EvalError> {
    let values = interpret(source, "main")?;
    assert_eq!(values.len(), 1, "entry returned {values:?}");
    assert_eq!(values[0].width().unwrap(), 64, "entry returned {values:?}");
    values[0].as_signed()
}

#[test]
fn addi_wraps_modulo_two_to_the_n() {
    let result = interpret_i64(
        r#"func.func @main() -> i64 {
            %max = arith.constant 9223372036854775807 : i64
            %one = arith.constant 1 : i64
            %sum = arith.addi %max, %one : i64
            return %sum : i64
        }"#,
    )
    .unwrap();
    assert_eq!(result, i64::MIN);
}

#[test]
fn mixed_arithmetic_matches_hand_math() {
    // (7*6 - 12) / 2 = 15; 15 > 10 → select 100.
    let result = interpret_i64(
        r#"func.func @main() -> i64 {
            %c7 = arith.constant 7 : i64
            %c6 = arith.constant 6 : i64
            %c12 = arith.constant 12 : i64
            %c2 = arith.constant 2 : i64
            %c10 = arith.constant 10 : i64
            %c100 = arith.constant 100 : i64
            %c200 = arith.constant 200 : i64
            %prod = arith.muli %c7, %c6 : i64
            %diff = arith.subi %prod, %c12 : i64
            %quot = arith.divsi %diff, %c2 : i64
            %gt = arith.cmpi sgt, %quot, %c10 : i64
            %sel = arith.select %gt, %c100, %c200 : i64
            return %sel : i64
        }"#,
    )
    .unwrap();
    assert_eq!(result, 100);
}

#[test]
fn divsi_truncates_toward_zero() {
    let result = interpret_i64(
        r#"func.func @main() -> i64 {
            %a = arith.constant 7 : i64
            %b = arith.constant -2 : i64
            %q = arith.divsi %a, %b : i64
            return %q : i64
        }"#,
    )
    .unwrap();
    assert_eq!(result, -3);
}

#[test]
fn divsi_by_zero_traps() {
    let error = interpret_i64(
        r#"func.func @main() -> i64 {
            %a = arith.constant 1 : i64
            %z = arith.constant 0 : i64
            %q = arith.divsi %a, %z : i64
            return %q : i64
        }"#,
    )
    .unwrap_err();
    assert!(matches!(error, EvalError::Trap(_)), "{error}");
}

#[test]
fn divsi_min_by_minus_one_traps() {
    let error = interpret_i64(
        r#"func.func @main() -> i64 {
            %min = arith.constant -9223372036854775808 : i64
            %m1 = arith.constant -1 : i64
            %q = arith.divsi %min, %m1 : i64
            return %q : i64
        }"#,
    )
    .unwrap_err();
    assert!(matches!(error, EvalError::Trap(_)), "{error}");
}

#[test]
fn cmpi_distinguishes_signed_from_unsigned() {
    // -1 slt 1 is true; -1 ult 1 is false (as unsigned, -1 is max).
    let source = |predicate: &str| {
        format!(
            r#"func.func @main() -> i64 {{
                %m1 = arith.constant -1 : i64
                %p1 = arith.constant 1 : i64
                %yes = arith.constant 1 : i64
                %no = arith.constant 0 : i64
                %cmp = arith.cmpi {predicate}, %m1, %p1 : i64
                %sel = arith.select %cmp, %yes, %no : i64
                return %sel : i64
            }}"#
        )
    };
    assert_eq!(interpret_i64(&source("slt")).unwrap(), 1);
    assert_eq!(interpret_i64(&source("ult")).unwrap(), 0);
    assert_eq!(interpret_i64(&source("sge")).unwrap(), 0);
    assert_eq!(interpret_i64(&source("uge")).unwrap(), 1);
    assert_eq!(interpret_i64(&source("eq")).unwrap(), 0);
    assert_eq!(interpret_i64(&source("ne")).unwrap(), 1);
}

#[test]
fn scf_for_accumulates_iter_args() {
    let result = interpret_i64(
        r#"func.func @main() -> i64 {
            %lb = arith.constant 0 : i64
            %ub = arith.constant 10 : i64
            %step = arith.constant 1 : i64
            %zero = arith.constant 0 : i64
            %sum = scf.for %i = %lb to %ub step %step iter_args(%acc = %zero) -> (i64) : i64 {
                %next = arith.addi %acc, %i : i64
                scf.yield %next : i64
            }
            return %sum : i64
        }"#,
    )
    .unwrap();
    assert_eq!(result, 45);
}

#[test]
fn scf_for_with_empty_range_returns_init() {
    let result = interpret_i64(
        r#"func.func @main() -> i64 {
            %lb = arith.constant 5 : i64
            %step = arith.constant 1 : i64
            %init = arith.constant 77 : i64
            %sum = scf.for %i = %lb to %lb step %step iter_args(%acc = %init) -> (i64) : i64 {
                %bogus = arith.constant 0 : i64
                scf.yield %bogus : i64
            }
            return %sum : i64
        }"#,
    )
    .unwrap();
    assert_eq!(result, 77);
}

#[test]
fn scf_if_takes_both_arms() {
    let source = |flag: &str| {
        format!(
            r#"func.func @main() -> i64 {{
                %cond = arith.constant {flag}
                %r = scf.if %cond -> (i64) {{
                    %a = arith.constant 111 : i64
                    scf.yield %a : i64
                }} else {{
                    %b = arith.constant 222 : i64
                    scf.yield %b : i64
                }}
                return %r : i64
            }}"#
        )
    };
    assert_eq!(interpret_i64(&source("true")).unwrap(), 111);
    assert_eq!(interpret_i64(&source("false")).unwrap(), 222);
}

#[test]
fn cf_backedge_loop_rebinds_block_arguments() {
    // Unstructured countdown: 5 → 0 through a loop-carried block arg.
    let result = interpret_i64(
        r#"func.func @main() -> i64 {
            %n = arith.constant 5 : i64
            cf.br ^loop(%n : i64)
        ^loop(%v: i64):
            %zero = arith.constant 0 : i64
            %done = arith.cmpi sle, %v, %zero : i64
            cf.cond_br %done, ^exit(%v : i64), ^body(%v : i64)
        ^body(%w: i64):
            %one = arith.constant 1 : i64
            %next = arith.subi %w, %one : i64
            cf.br ^loop(%next : i64)
        ^exit(%out: i64):
            return %out : i64
        }"#,
    )
    .unwrap();
    assert_eq!(result, 0);
}

#[test]
fn func_call_recursion_computes_fib() {
    let result = interpret_i64(
        r#"func.func @fib(%n: i64) -> i64 {
            %c2 = arith.constant 2 : i64
            %small = arith.cmpi slt, %n, %c2 : i64
            %r = scf.if %small -> (i64) {
                scf.yield %n : i64
            } else {
                %c1 = arith.constant 1 : i64
                %n1 = arith.subi %n, %c1 : i64
                %f1 = func.call @fib(%n1) : (i64) -> i64
                %n2 = arith.subi %n, %c2 : i64
                %f2 = func.call @fib(%n2) : (i64) -> i64
                %s = arith.addi %f1, %f2 : i64
                scf.yield %s : i64
            }
            return %r : i64
        }
        func.func @main() -> i64 {
            %c10 = arith.constant 10 : i64
            %r = func.call @fib(%c10) : (i64) -> i64
            return %r : i64
        }"#,
    )
    .unwrap();
    assert_eq!(result, 55);
}

#[test]
fn runaway_recursion_traps_at_the_depth_ceiling() {
    // MAX_CALL_DEPTH frames need more host stack than the 2 MiB libtest
    // default — exactly the situation frk_interp::STACK_SIZE exists for.
    let error = std::thread::Builder::new()
        .stack_size(frk_interp::STACK_SIZE)
        .spawn(|| {
            // NON-tail runaway: the result is USED after the call, so
            // the D-059 trampoline does not apply and the D-029 depth
            // cap must trap. (A TAIL-shaped `return f()` is now a
            // legitimate infinite loop, like `while true` — the law
            // this test guarded moved to the tailcall goldens.)
            interpret_i64(
                r#"func.func @forever() -> i64 {
                    %r = func.call @forever() : () -> i64
                    %one = arith.constant 1 : i64
                    %x = arith.addi %r, %one : i64
                    return %x : i64
                }
                func.func @main() -> i64 {
                    %r = func.call @forever() : () -> i64
                    return %r : i64
                }"#,
            )
            .unwrap_err()
        })
        .expect("spawning the deep-stack thread")
        .join()
        .expect("deep-stack thread panicked");
    assert!(matches!(error, EvalError::Trap(_)), "{error}");
}

#[test]
fn multiple_return_values_come_back_in_order() {
    let values = interpret(
        r#"func.func @main() -> (i64, i64) {
            %a = arith.constant 1 : i64
            %b = arith.constant 2 : i64
            return %a, %b : i64, i64
        }"#,
        "main",
    )
    .unwrap();
    assert_eq!(values.len(), 2);
    assert_eq!(values[0].as_signed().unwrap(), 1);
    assert_eq!(values[1].as_signed().unwrap(), 2);
}

#[test]
fn cf_switch_hits_cases_and_default() {
    // i32 flag; the default edge carries a block argument.
    let source = |flag: i64| {
        format!(
            r#"func.func @main() -> i64 {{
                %flag = arith.constant {flag} : i32
                %d = arith.constant 10 : i64
                cf.switch %flag : i32, [
                    default: ^exit(%d : i64),
                    0: ^zero,
                    1: ^one
                ]
            ^zero:
                %z = arith.constant 100 : i64
                cf.br ^exit(%z : i64)
            ^one:
                %o = arith.constant 200 : i64
                cf.br ^exit(%o : i64)
            ^exit(%r: i64):
                return %r : i64
            }}"#
        )
    };
    assert_eq!(interpret_i64(&source(0)).unwrap(), 100);
    assert_eq!(interpret_i64(&source(1)).unwrap(), 200);
    assert_eq!(interpret_i64(&source(7)).unwrap(), 10);
}

#[test]
fn cf_switch_works_on_i64_flags() {
    let result = interpret_i64(
        r#"func.func @main() -> i64 {
            %flag = arith.constant 43 : i64
            %d = arith.constant -1 : i64
            cf.switch %flag : i64, [
                default: ^exit(%d : i64),
                42: ^a,
                43: ^b
            ]
        ^a:
            %x = arith.constant 1 : i64
            cf.br ^exit(%x : i64)
        ^b:
            %y = arith.constant 2 : i64
            cf.br ^exit(%y : i64)
        ^exit(%r: i64):
            return %r : i64
        }"#,
    )
    .unwrap();
    assert_eq!(result, 2);
}

#[test]
fn unregistered_ops_fail_loudly() {
    let error = interpret_i64(
        r#"func.func @main() -> i64 {
            %a = arith.constant 6 : i64
            %b = arith.constant 3 : i64
            %r = arith.andi %a, %b : i64
            return %r : i64
        }"#,
    )
    .unwrap_err();
    assert_eq!(error, EvalError::UnknownOp("arith.andi".into()));
}

#[test]
fn missing_entry_is_callee_not_found() {
    let error = interpret(
        r#"func.func @main() -> i64 {
            %a = arith.constant 0 : i64
            return %a : i64
        }"#,
        "nonexistent",
    )
    .unwrap_err();
    assert_eq!(error, EvalError::CalleeNotFound("nonexistent".into()));
}

#[test]
fn maxsi_picks_the_signed_maximum() {
    // M23: arith.maxsi joins the upstream registry (the vararg
    // tail-length clamp); signedness matters — -1 < 0.
    let picked = interpret_i64(
        r#"func.func @main() -> i64 {
            %a = arith.constant -1 : i64
            %b = arith.constant 0 : i64
            %m = arith.maxsi %a, %b : i64
            return %m : i64
        }"#,
    )
    .unwrap();
    assert_eq!(picked, 0);
}
