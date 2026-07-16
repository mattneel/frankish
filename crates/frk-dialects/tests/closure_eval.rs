//! K2 verifiers for frk.closure (law L1). The composition mirrors the
//! harness runner; the star witness is church encoding end to end —
//! closures capturing closures, escaping upward, applied to 40 → 42.

use frk_interp::{EvalError, Interp};
use melior::ir::Module;
use melior::ir::operation::OperationLike;

fn interpret_i64(source: &str) -> Result<i64, EvalError> {
    let context = frk_core::context();
    frk_dialects::register(&context).expect("registration");
    let module = Module::parse(&context, source).expect("test source must parse");
    assert!(module.as_operation().verify(), "must pass MLIR verification");
    frk_dialects::verify(&context, &module).expect("must pass frk semantic verification");

    let mut interp = Interp::new(&module)?;
    frk_dialects::register_eval(&mut interp);
    let values = interp.eval_function("main", &[])?;
    assert_eq!(values.len(), 1, "entry returned {values:?}");
    values[0].as_signed()
}

const FN_I64: &str = "!frk_closure.fn<[i64], [i64]>";
const P_EMPTY: &str = "!frk_adt.product<[]>";
const P_I64: &str = "!frk_adt.product<[i64]>";
const P_FN: &str = "!frk_adt.product<[!frk_closure.fn<[i64], [i64]>]>";

#[test]
fn captureless_closure_applies() {
    let result = interpret_i64(&format!(
        r#"func.func @inc(%n: i64) -> i64 {{
            %one = arith.constant 1 : i64
            %r = arith.addi %n, %one : i64
            return %r : i64
        }}
        func.func @main() -> i64 {{
            %c41 = arith.constant 41 : i64
            %e = "frk_adt.product_new"() : () -> {P_EMPTY}
            %inc = "frk_closure.make"(%e) {{callee = @inc}} : ({P_EMPTY}) -> {FN_I64}
            %args = "frk_adt.product_snoc"(%e, %c41) : ({P_EMPTY}, i64) -> {P_I64}
            %r = "frk_closure.apply"(%inc, %args) : ({FN_I64}, {P_I64}) -> i64
            return %r : i64
        }}"#
    ))
    .unwrap();
    assert_eq!(result, 42);
}

#[test]
fn captures_snapshot_by_value() {
    // adder(n) captures n; apply(adder(40), 2) = 42.
    let result = interpret_i64(&format!(
        r#"func.func @add_captured(%n: i64, %x: i64) -> i64 {{
            %r = arith.addi %n, %x : i64
            return %r : i64
        }}
        func.func @main() -> i64 {{
            %c40 = arith.constant 40 : i64
            %c2 = arith.constant 2 : i64
            %e = "frk_adt.product_new"() : () -> {P_EMPTY}
            %env = "frk_adt.product_snoc"(%e, %c40) : ({P_EMPTY}, i64) -> {P_I64}
            %adder = "frk_closure.make"(%env) {{callee = @add_captured}} : ({P_I64}) -> {FN_I64}
            %args = "frk_adt.product_snoc"(%e, %c2) : ({P_EMPTY}, i64) -> {P_I64}
            %r = "frk_closure.apply"(%adder, %args) : ({FN_I64}, {P_I64}) -> i64
            return %r : i64
        }}"#
    ))
    .unwrap();
    assert_eq!(result, 42);
}

/// Church encoding end to end: two = λf.λx. f (f x), applied to inc and
/// 40. Exercises closure-capturing-closure, upward escape across a
/// function return, and application through captured values.
#[test]
fn church_two_applied_to_inc_yields_42() {
    let result = interpret_i64(&format!(
        r#"func.func @inc(%n: i64) -> i64 {{
            %one = arith.constant 1 : i64
            %r = arith.addi %n, %one : i64
            return %r : i64
        }}
        func.func @two_inner(%f: {FN_I64}, %x: i64) -> i64 {{
            %e0 = "frk_adt.product_new"() : () -> {P_EMPTY}
            %a1 = "frk_adt.product_snoc"(%e0, %x) : ({P_EMPTY}, i64) -> {P_I64}
            %fx = "frk_closure.apply"(%f, %a1) : ({FN_I64}, {P_I64}) -> i64
            %a2 = "frk_adt.product_snoc"(%e0, %fx) : ({P_EMPTY}, i64) -> {P_I64}
            %ffx = "frk_closure.apply"(%f, %a2) : ({FN_I64}, {P_I64}) -> i64
            return %ffx : i64
        }}
        func.func @two_outer(%f: {FN_I64}) -> {FN_I64} {{
            %e = "frk_adt.product_new"() : () -> {P_EMPTY}
            %env = "frk_adt.product_snoc"(%e, %f) : ({P_EMPTY}, {FN_I64}) -> {P_FN}
            %two = "frk_closure.make"(%env) {{callee = @two_inner}} : ({P_FN}) -> {FN_I64}
            return %two : {FN_I64}
        }}
        func.func @main() -> i64 {{
            %c40 = arith.constant 40 : i64
            %e = "frk_adt.product_new"() : () -> {P_EMPTY}
            %inc = "frk_closure.make"(%e) {{callee = @inc}} : ({P_EMPTY}) -> {FN_I64}
            %two = func.call @two_outer(%inc) : ({FN_I64}) -> {FN_I64}
            %args = "frk_adt.product_snoc"(%e, %c40) : ({P_EMPTY}, i64) -> {P_I64}
            %r = "frk_closure.apply"(%two, %args) : ({FN_I64}, {P_I64}) -> i64
            return %r : i64
        }}"#
    ))
    .unwrap();
    assert_eq!(result, 42);
}

/// The counter shape available without mutable state (that one waits
/// for frk.mem): fold a +3 closure four times from 30 → 42 through
/// scf.for iter_args.
#[test]
fn counter_fold_applies_a_closure_in_a_loop() {
    let result = interpret_i64(&format!(
        r#"func.func @add3(%x: i64) -> i64 {{
            %three = arith.constant 3 : i64
            %r = arith.addi %x, %three : i64
            return %r : i64
        }}
        func.func @main() -> i64 {{
            %lb = arith.constant 0 : i64
            %ub = arith.constant 4 : i64
            %step = arith.constant 1 : i64
            %init = arith.constant 30 : i64
            %e = "frk_adt.product_new"() : () -> {P_EMPTY}
            %add3 = "frk_closure.make"(%e) {{callee = @add3}} : ({P_EMPTY}) -> {FN_I64}
            %sum = scf.for %i = %lb to %ub step %step iter_args(%acc = %init) -> (i64) : i64 {{
                %args = "frk_adt.product_snoc"(%e, %acc) : ({P_EMPTY}, i64) -> {P_I64}
                %next = "frk_closure.apply"(%add3, %args) : ({FN_I64}, {P_I64}) -> i64
                scf.yield %next : i64
            }}
            return %sum : i64
        }}"#
    ))
    .unwrap();
    assert_eq!(result, 42);
}

#[test]
fn runaway_closure_recursion_still_traps_at_the_depth_ceiling() {
    // apply re-enters eval_function, so the D-029 guard must fire.
    // By-value capture cannot tie a self-referential knot (that's the
    // D-035 point) — but a function can re-make a captureless closure of
    // itself every level, which recurses forever all the same.
    // NON-TAIL by construction (the result is consumed): a TAIL-shaped
    // runaway apply is a legitimate infinite loop under D-063's
    // trampoline — the M14 lesson, replayed for closures — so the
    // depth cap only governs the non-tail world.
    let error = std::thread::Builder::new()
        .stack_size(frk_interp::STACK_SIZE)
        .spawn(|| {
            interpret_i64(&format!(
                r#"func.func @spin(%x: i64) -> i64 {{
                    %e = "frk_adt.product_new"() : () -> {P_EMPTY}
                    %self = "frk_closure.make"(%e) {{callee = @spin}} : ({P_EMPTY}) -> {FN_I64}
                    %args = "frk_adt.product_snoc"(%e, %x) : ({P_EMPTY}, i64) -> {P_I64}
                    %r = "frk_closure.apply"(%self, %args) : ({FN_I64}, {P_I64}) -> i64
                    %one = arith.constant 1 : i64
                    %used = arith.addi %r, %one : i64
                    return %used : i64
                }}
                func.func @main() -> i64 {{
                    %x = arith.constant 1 : i64
                    %r = func.call @spin(%x) : (i64) -> i64
                    return %r : i64
                }}"#
            ))
            .unwrap_err()
        })
        .expect("spawning the deep-stack thread")
        .join()
        .expect("deep-stack thread panicked");
    assert!(matches!(error, EvalError::Trap(_)), "{error}");
}
