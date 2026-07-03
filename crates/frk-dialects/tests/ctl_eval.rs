//! K2 verifiers for frk.ctl (law L1; κ_frk, docs/ctl-calculus.md).
//! These are the REFERENCE-SEMANTICS goldens for the five rules of
//! calculus §2: H-return, H-op-drop landing at its prompt, drop
//! passing through an inner prompt to an outer one, and the two traps
//! (escape-past-extent; the value round-trip proves tokens are real
//! first-class values).

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

const DYN: &str = "!frk_dyn.dyn";
const FN_BODY: &str = "!frk_closure.fn<[i64], [!frk_dyn.dyn]>";
const P_EMPTY: &str = "!frk_adt.product<[]>";
const P_I64: &str = "!frk_adt.product<[i64]>";

// num-tagged dyn round-trips an i64 payload (wrap/unwrap are
// payload-agnostic — tag 2 = num, per frk_dyn).
const NUM: i64 = 2;

#[test]
fn normal_return_passes_through() {
    // H-return: body never aborts; the prompt yields its return.
    let result = interpret_i64(&format!(
        r#"func.func @body(%tok: i64) -> {DYN} {{
            %c7 = arith.constant 7 : i64
            %v = "frk_dyn.wrap"(%c7) {{tag = {NUM} : i64}} : (i64) -> {DYN}
            return %v : {DYN}
        }}
        func.func @main() -> i64 {{
            %e = "frk_adt.product_new"() : () -> {P_EMPTY}
            %body = "frk_closure.make"(%e) {{callee = @body}} : ({P_EMPTY}) -> {FN_BODY}
            %out = "frk_ctl.prompt"(%body) : ({FN_BODY}) -> {DYN}
            %n = "frk_dyn.unwrap"(%out) {{tag = {NUM} : i64}} : ({DYN}) -> i64
            return %n : i64
        }}"#
    ))
    .unwrap();
    assert_eq!(result, 7);
}

#[test]
fn abort_escapes_to_its_prompt() {
    // H-op-drop, landed: the body aborts past a would-be return; the
    // prompt yields the aborted value, and the dummy return below the
    // abort is dynamically unreachable.
    let result = interpret_i64(&format!(
        r#"func.func @body(%tok: i64) -> {DYN} {{
            %c42 = arith.constant 42 : i64
            %v = "frk_dyn.wrap"(%c42) {{tag = {NUM} : i64}} : (i64) -> {DYN}
            "frk_ctl.abort"(%tok, %v) : (i64, {DYN}) -> ()
            %c0 = arith.constant 0 : i64
            %dead = "frk_dyn.wrap"(%c0) {{tag = {NUM} : i64}} : (i64) -> {DYN}
            return %dead : {DYN}
        }}
        func.func @main() -> i64 {{
            %e = "frk_adt.product_new"() : () -> {P_EMPTY}
            %body = "frk_closure.make"(%e) {{callee = @body}} : ({P_EMPTY}) -> {FN_BODY}
            %out = "frk_ctl.prompt"(%body) : ({FN_BODY}) -> {DYN}
            %n = "frk_dyn.unwrap"(%out) {{tag = {NUM} : i64}} : ({DYN}) -> i64
            return %n : i64
        }}"#
    ))
    .unwrap();
    assert_eq!(result, 42);
}

#[test]
fn abort_from_deep_tail_chain_escapes() {
    // The abort fires inside a deep TAIL-recursive loop (M14 trampoline
    // territory): the escape must thread up through frame replacement
    // to the prompt, not be swallowed. Loop counts down from 1000 then
    // aborts with 99.
    let result = interpret_i64(&format!(
        r#"func.func @loop(%tok: i64, %n: i64) -> {DYN} {{
            %zero = arith.constant 0 : i64
            %done = arith.cmpi eq, %n, %zero : i64
            cf.cond_br %done, ^abort, ^again
        ^abort:
            %c99 = arith.constant 99 : i64
            %v = "frk_dyn.wrap"(%c99) {{tag = {NUM} : i64}} : (i64) -> {DYN}
            "frk_ctl.abort"(%tok, %v) : (i64, {DYN}) -> ()
            return %v : {DYN}
        ^again:
            %one = arith.constant 1 : i64
            %m = arith.subi %n, %one : i64
            %r = func.call @loop(%tok, %m) : (i64, i64) -> {DYN}
            return %r : {DYN}
        }}
        func.func @body(%tok: i64) -> {DYN} {{
            %n = arith.constant 1000 : i64
            %r = func.call @loop(%tok, %n) : (i64, i64) -> {DYN}
            return %r : {DYN}
        }}
        func.func @main() -> i64 {{
            %e = "frk_adt.product_new"() : () -> {P_EMPTY}
            %body = "frk_closure.make"(%e) {{callee = @body}} : ({P_EMPTY}) -> {FN_BODY}
            %out = "frk_ctl.prompt"(%body) : ({FN_BODY}) -> {DYN}
            %v = "frk_dyn.unwrap"(%out) {{tag = {NUM} : i64}} : ({DYN}) -> i64
            return %v : i64
        }}"#
    ))
    .unwrap();
    assert_eq!(result, 99);
}

#[test]
fn inner_abort_caught_by_inner_prompt() {
    // Two nested prompts; the inner body aborts to ITS OWN token. The
    // inner prompt yields 5; the outer body then adds 100 and returns
    // normally → 105. Proves an inner abort does not disturb the outer.
    let result = interpret_i64(&format!(
        r#"func.func @inner(%itok: i64) -> {DYN} {{
            %c5 = arith.constant 5 : i64
            %v = "frk_dyn.wrap"(%c5) {{tag = {NUM} : i64}} : (i64) -> {DYN}
            "frk_ctl.abort"(%itok, %v) : (i64, {DYN}) -> ()
            return %v : {DYN}
        }}
        func.func @outer(%otok: i64) -> {DYN} {{
            %e = "frk_adt.product_new"() : () -> {P_EMPTY}
            %ib = "frk_closure.make"(%e) {{callee = @inner}} : ({P_EMPTY}) -> {FN_BODY}
            %io = "frk_ctl.prompt"(%ib) : ({FN_BODY}) -> {DYN}
            %in = "frk_dyn.unwrap"(%io) {{tag = {NUM} : i64}} : ({DYN}) -> i64
            %c100 = arith.constant 100 : i64
            %sum = arith.addi %in, %c100 : i64
            %out = "frk_dyn.wrap"(%sum) {{tag = {NUM} : i64}} : (i64) -> {DYN}
            return %out : {DYN}
        }}
        func.func @main() -> i64 {{
            %e = "frk_adt.product_new"() : () -> {P_EMPTY}
            %ob = "frk_closure.make"(%e) {{callee = @outer}} : ({P_EMPTY}) -> {FN_BODY}
            %oo = "frk_ctl.prompt"(%ob) : ({FN_BODY}) -> {DYN}
            %n = "frk_dyn.unwrap"(%oo) {{tag = {NUM} : i64}} : ({DYN}) -> i64
            return %n : i64
        }}"#
    ))
    .unwrap();
    assert_eq!(result, 105);
}

#[test]
fn inner_abort_targets_outer_prompt() {
    // The inner body captures the OUTER token and aborts to it,
    // unwinding THROUGH the inner prompt. The outer prompt yields 8;
    // the outer body's "+100" after the inner prompt is skipped
    // (unwound past) → main returns 8, not 108.
    let result = interpret_i64(&format!(
        r#"func.func @inner(%otok: i64, %itok: i64) -> {DYN} {{
            %c8 = arith.constant 8 : i64
            %v = "frk_dyn.wrap"(%c8) {{tag = {NUM} : i64}} : (i64) -> {DYN}
            "frk_ctl.abort"(%otok, %v) : (i64, {DYN}) -> ()
            return %v : {DYN}
        }}
        func.func @outer(%otok: i64) -> {DYN} {{
            %ce = "frk_adt.product_new"() : () -> {P_EMPTY}
            %cap = "frk_adt.product_snoc"(%ce, %otok) : ({P_EMPTY}, i64) -> {P_I64}
            %ib = "frk_closure.make"(%cap) {{callee = @inner}} : ({P_I64}) -> {FN_BODY}
            %io = "frk_ctl.prompt"(%ib) : ({FN_BODY}) -> {DYN}
            %in = "frk_dyn.unwrap"(%io) {{tag = {NUM} : i64}} : ({DYN}) -> i64
            %c100 = arith.constant 100 : i64
            %sum = arith.addi %in, %c100 : i64
            %out = "frk_dyn.wrap"(%sum) {{tag = {NUM} : i64}} : (i64) -> {DYN}
            return %out : {DYN}
        }}
        func.func @main() -> i64 {{
            %e = "frk_adt.product_new"() : () -> {P_EMPTY}
            %ob = "frk_closure.make"(%e) {{callee = @outer}} : ({P_EMPTY}) -> {FN_BODY}
            %oo = "frk_ctl.prompt"(%ob) : ({FN_BODY}) -> {DYN}
            %n = "frk_dyn.unwrap"(%oo) {{tag = {NUM} : i64}} : ({DYN}) -> i64
            return %n : i64
        }}"#
    ))
    .unwrap();
    assert_eq!(result, 8);
}

#[test]
fn escape_past_extent_traps() {
    // The body returns its token as a first-class value (proving tokens
    // ARE values). main then aborts to that now-dead token: the prompt
    // has been popped, so ctl_prompt_live is false → the κ_frk trap.
    let error = interpret_i64(&format!(
        r#"func.func @body(%tok: i64) -> {DYN} {{
            %v = "frk_dyn.wrap"(%tok) {{tag = {NUM} : i64}} : (i64) -> {DYN}
            return %v : {DYN}
        }}
        func.func @main() -> i64 {{
            %e = "frk_adt.product_new"() : () -> {P_EMPTY}
            %body = "frk_closure.make"(%e) {{callee = @body}} : ({P_EMPTY}) -> {FN_BODY}
            %out = "frk_ctl.prompt"(%body) : ({FN_BODY}) -> {DYN}
            %tok = "frk_dyn.unwrap"(%out) {{tag = {NUM} : i64}} : ({DYN}) -> i64
            %c1 = arith.constant 1 : i64
            %payload = "frk_dyn.wrap"(%c1) {{tag = {NUM} : i64}} : (i64) -> {DYN}
            "frk_ctl.abort"(%tok, %payload) : (i64, {DYN}) -> ()
            %z = arith.constant 0 : i64
            return %z : i64
        }}"#
    ))
    .unwrap_err();
    assert!(
        matches!(&error, EvalError::Trap(m) if m.contains("escape past extent")),
        "{error}"
    );
}
