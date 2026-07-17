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

// ---- v1 (M24, D-069): handle/perform — the affine ladder's clause
// classes, verified BEFORE the implementation (L1). ----

const PACKFN: &str = "!frk_closure.fn<[!frk_mem.arr<!frk_dyn.dyn>], [!frk_mem.arr<!frk_dyn.dyn>]>";
const PACK: &str = "!frk_mem.arr<!frk_dyn.dyn>";

/// Shared golden scaffolding: a tail-resume clause that doubles the
/// performed value — reads [v, k] from its pack, applies k([2*v]).
const DOUBLER_CLAUSE: &str = r#"
func.func @clause(%env: !frk_closure.envref, %pack: !frk_mem.arr<!frk_dyn.dyn>) -> !frk_mem.arr<!frk_dyn.dyn> {
    %c0 = arith.constant 0 : i64
    %v = "frk_mem.array_get"(%pack, %c0) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
    %c1 = arith.constant 1 : i64
    %kd = "frk_mem.array_get"(%pack, %c1) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
    %k = "frk_dyn.unwrap"(%kd) {tag = 5 : i64} : (!frk_dyn.dyn) -> !frk_closure.fn<[!frk_mem.arr<!frk_dyn.dyn>], [!frk_mem.arr<!frk_dyn.dyn>]>
    %n = "frk_dyn.unwrap"(%v) {tag = 2 : i64} : (!frk_dyn.dyn) -> f64
    %two = arith.constant 2.0 : f64
    %d = arith.mulf %n, %two : f64
    %dd = "frk_dyn.wrap"(%d) {tag = 2 : i64} : (f64) -> !frk_dyn.dyn
    %argp = "frk_mem.array_new"(%c1) : (i64) -> !frk_mem.arr<!frk_dyn.dyn>
    %z = arith.constant 0 : i64
    "frk_mem.array_set"(%argp, %z, %dd) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
    %pe = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
    %pp = "frk_adt.product_snoc"(%pe, %argp) : (!frk_adt.product<[]>, !frk_mem.arr<!frk_dyn.dyn>) -> !frk_adt.product<[!frk_mem.arr<!frk_dyn.dyn>]>
    %r = "frk_closure.apply"(%k, %pp) : (!frk_closure.fn<[!frk_mem.arr<!frk_dyn.dyn>], [!frk_mem.arr<!frk_dyn.dyn>]>, !frk_adt.product<[!frk_mem.arr<!frk_dyn.dyn>]>) -> !frk_mem.arr<!frk_dyn.dyn>
    return %r : !frk_mem.arr<!frk_dyn.dyn>
}"#;

fn make_handle_main(body_extra: &str) -> String {
    // main: handle "ask" (doubler) around @body; unwraps the dyn result.
    format!(
        r#"func.func @main() -> i64 {{
            %e = "frk_adt.product_new"() : () -> {P_EMPTY}
            %cl = "frk_closure.make"(%e) {{callee = @clause}} : ({P_EMPTY}) -> {PACKFN}
            %bo = "frk_closure.make"(%e) {{callee = @body}} : ({P_EMPTY}) -> {FN_BODY}
            %out = "frk_ctl.handle"(%cl, %bo) {{label = "ask"}} : ({PACKFN}, {FN_BODY}) -> {DYN}
            %f = "frk_dyn.unwrap"(%out) {{tag = {NUM} : i64}} : ({DYN}) -> f64
            %n = arith.fptosi %f : f64 to i64
            return %n : i64
        }}{body_extra}"#
    )
}

#[test]
fn tail_resume_flows_and_body_continues() {
    // perform "ask" 20 → clause doubles → 40; body adds 2 → handle
    // yields 42. Proves: dispatch, κ consumption, the perform
    // evaluating to the clause's return, and the body CONTINUING.
    let source = format!(
        r#"{DOUBLER_CLAUSE}
        func.func @body(%tok: i64) -> {DYN} {{
            %c20 = arith.constant 20.0 : f64
            %v = "frk_dyn.wrap"(%c20) {{tag = {NUM} : i64}} : (f64) -> {DYN}
            %r = "frk_ctl.perform"(%v) {{label = "ask"}} : ({DYN}) -> {DYN}
            %rf = "frk_dyn.unwrap"(%r) {{tag = {NUM} : i64}} : ({DYN}) -> f64
            %c2 = arith.constant 2.0 : f64
            %sum = arith.addf %rf, %c2 : f64
            %out = "frk_dyn.wrap"(%sum) {{tag = {NUM} : i64}} : (f64) -> {DYN}
            return %out : {DYN}
        }}
        {main}"#,
        main = make_handle_main("")
    );
    assert_eq!(interpret_i64(&source).unwrap(), 42);
}

#[test]
fn abortive_clause_discards_body_rest() {
    // The clause returns [99] WITHOUT consuming κ: the handle yields
    // 99; the body's +2 after the perform never runs.
    let source = format!(
        r#"func.func @clause(%env: !frk_closure.envref, %pack: {PACK}) -> {PACK} {{
            %c99 = arith.constant 99.0 : f64
            %d = "frk_dyn.wrap"(%c99) {{tag = {NUM} : i64}} : (f64) -> {DYN}
            %c1 = arith.constant 1 : i64
            %rp = "frk_mem.array_new"(%c1) : (i64) -> {PACK}
            %c0 = arith.constant 0 : i64
            "frk_mem.array_set"(%rp, %c0, %d) : ({PACK}, i64, {DYN}) -> ()
            return %rp : {PACK}
        }}
        func.func @body(%tok: i64) -> {DYN} {{
            %c20 = arith.constant 20.0 : f64
            %v = "frk_dyn.wrap"(%c20) {{tag = {NUM} : i64}} : (f64) -> {DYN}
            %r = "frk_ctl.perform"(%v) {{label = "ask"}} : ({DYN}) -> {DYN}
            %rf = "frk_dyn.unwrap"(%r) {{tag = {NUM} : i64}} : ({DYN}) -> f64
            %c2 = arith.constant 2.0 : f64
            %sum = arith.addf %rf, %c2 : f64
            %out = "frk_dyn.wrap"(%sum) {{tag = {NUM} : i64}} : (f64) -> {DYN}
            return %out : {DYN}
        }}
        {main}"#,
        main = make_handle_main("")
    );
    assert_eq!(interpret_i64(&source).unwrap(), 99);
}

#[test]
fn other_labels_are_transparent() {
    // handle "ask" ( handle "log" ( perform "ask" ) ): the inner
    // handler carries a DIFFERENT label and must be transparent —
    // 20 doubles to 40 through the outer clause, inner body adds 1,
    // outer body adds 2 → 43.
    let source = format!(
        r#"{DOUBLER_CLAUSE}
        func.func @noop_clause(%env: !frk_closure.envref, %pack: {PACK}) -> {PACK} {{
            return %pack : {PACK}
        }}
        func.func @inner_body(%tok: i64) -> {DYN} {{
            %c20 = arith.constant 20.0 : f64
            %v = "frk_dyn.wrap"(%c20) {{tag = {NUM} : i64}} : (f64) -> {DYN}
            %r = "frk_ctl.perform"(%v) {{label = "ask"}} : ({DYN}) -> {DYN}
            %rf = "frk_dyn.unwrap"(%r) {{tag = {NUM} : i64}} : ({DYN}) -> f64
            %c1 = arith.constant 1.0 : f64
            %sum = arith.addf %rf, %c1 : f64
            %out = "frk_dyn.wrap"(%sum) {{tag = {NUM} : i64}} : (f64) -> {DYN}
            return %out : {DYN}
        }}
        func.func @body(%tok: i64) -> {DYN} {{
            %e = "frk_adt.product_new"() : () -> {P_EMPTY}
            %ncl = "frk_closure.make"(%e) {{callee = @noop_clause}} : ({P_EMPTY}) -> {PACKFN}
            %ib = "frk_closure.make"(%e) {{callee = @inner_body}} : ({P_EMPTY}) -> {FN_BODY}
            %r = "frk_ctl.handle"(%ncl, %ib) {{label = "log"}} : ({PACKFN}, {FN_BODY}) -> {DYN}
            %rf = "frk_dyn.unwrap"(%r) {{tag = {NUM} : i64}} : ({DYN}) -> f64
            %c2 = arith.constant 2.0 : f64
            %sum = arith.addf %rf, %c2 : f64
            %out = "frk_dyn.wrap"(%sum) {{tag = {NUM} : i64}} : (f64) -> {DYN}
            return %out : {DYN}
        }}
        {main}"#,
        main = make_handle_main("")
    );
    assert_eq!(interpret_i64(&source).unwrap(), 43);
}

#[test]
fn deep_reentry_finds_the_handler_again() {
    // Two performs in sequence: the mask must lift after the first
    // clause returns (deep reinstall). 5→10, then 10→20, +2 → 22.
    let source = format!(
        r#"{DOUBLER_CLAUSE}
        func.func @body(%tok: i64) -> {DYN} {{
            %c5 = arith.constant 5.0 : f64
            %v0 = "frk_dyn.wrap"(%c5) {{tag = {NUM} : i64}} : (f64) -> {DYN}
            %r0 = "frk_ctl.perform"(%v0) {{label = "ask"}} : ({DYN}) -> {DYN}
            %r1 = "frk_ctl.perform"(%r0) {{label = "ask"}} : ({DYN}) -> {DYN}
            %rf = "frk_dyn.unwrap"(%r1) {{tag = {NUM} : i64}} : ({DYN}) -> f64
            %c2 = arith.constant 2.0 : f64
            %sum = arith.addf %rf, %c2 : f64
            %out = "frk_dyn.wrap"(%sum) {{tag = {NUM} : i64}} : (f64) -> {DYN}
            return %out : {DYN}
        }}
        {main}"#,
        main = make_handle_main("")
    );
    assert_eq!(interpret_i64(&source).unwrap(), 22);
}

#[test]
fn double_resume_is_a_one_shot_violation() {
    // The clause applies κ twice: the second application must trap
    // with the κ_frk wording.
    let source = format!(
        r#"func.func @clause(%env: !frk_closure.envref, %pack: {PACK}) -> {PACK} {{
            %c0 = arith.constant 0 : i64
            %v = "frk_mem.array_get"(%pack, %c0) : ({PACK}, i64) -> {DYN}
            %c1 = arith.constant 1 : i64
            %kd = "frk_mem.array_get"(%pack, %c1) : ({PACK}, i64) -> {DYN}
            %k = "frk_dyn.unwrap"(%kd) {{tag = 5 : i64}} : ({DYN}) -> {PACKFN}
            %argp = "frk_mem.array_new"(%c1) : (i64) -> {PACK}
            "frk_mem.array_set"(%argp, %c0, %v) : ({PACK}, i64, {DYN}) -> ()
            %pe = "frk_adt.product_new"() : () -> {P_EMPTY}
            %pp = "frk_adt.product_snoc"(%pe, %argp) : ({P_EMPTY}, {PACK}) -> !frk_adt.product<[{PACK}]>
            %r1 = "frk_closure.apply"(%k, %pp) : ({PACKFN}, !frk_adt.product<[{PACK}]>) -> {PACK}
            %r2 = "frk_closure.apply"(%k, %pp) : ({PACKFN}, !frk_adt.product<[{PACK}]>) -> {PACK}
            return %r2 : {PACK}
        }}
        func.func @body(%tok: i64) -> {DYN} {{
            %c20 = arith.constant 20.0 : f64
            %v = "frk_dyn.wrap"(%c20) {{tag = {NUM} : i64}} : (f64) -> {DYN}
            %r = "frk_ctl.perform"(%v) {{label = "ask"}} : ({DYN}) -> {DYN}
            return %r : {DYN}
        }}
        {main}"#,
        main = make_handle_main("")
    );
    let error = interpret_i64(&source).unwrap_err();
    assert!(
        matches!(&error, EvalError::Trap(m) if m.contains("one-shot violation")),
        "{error}"
    );
}

#[test]
fn unhandled_perform_traps() {
    let source = format!(
        r#"func.func @main() -> i64 {{
            %c1 = arith.constant 1.0 : f64
            %v = "frk_dyn.wrap"(%c1) {{tag = {NUM} : i64}} : (f64) -> {DYN}
            %r = "frk_ctl.perform"(%v) {{label = "nobody"}} : ({DYN}) -> {DYN}
            %rf = "frk_dyn.unwrap"(%r) {{tag = {NUM} : i64}} : ({DYN}) -> f64
            %n = arith.fptosi %rf : f64 to i64
            return %n : i64
        }}"#
    );
    let error = interpret_i64(&source).unwrap_err();
    assert!(
        matches!(&error, EvalError::Trap(m) if m.contains("unhandled effect")),
        "{error}"
    );
}

// ---- v0.1 (M25, D-070): dynamic-wind, escape-only. ----

#[test]
fn wind_runs_after_on_normal_exit() {
    // before writes 1, thunk yields 42, after writes 2 into the box:
    // result = 42 * 100 + box = 4202.
    let source = format!(
        r#"func.func @before(%env: !frk_closure.envref, %pack: {PACK}) -> {PACK} {{
            %b = "frk_closure.env_load"(%env) {{index = 0 : i64, env = !frk_adt.product<[!frk_mem.box<!frk_dyn.dyn>]>}} : (!frk_closure.envref) -> !frk_mem.box<{DYN}>
            %c1 = arith.constant 1.0 : f64
            %d = "frk_dyn.wrap"(%c1) {{tag = {NUM} : i64}} : (f64) -> {DYN}
            "frk_mem.box_set"(%b, %d) : (!frk_mem.box<{DYN}>, {DYN}) -> ()
            %z = arith.constant 0 : i64
            %rp = "frk_mem.array_new"(%z) : (i64) -> {PACK}
            return %rp : {PACK}
        }}
        func.func @thunk(%env: !frk_closure.envref, %pack: {PACK}) -> {PACK} {{
            %c42 = arith.constant 42.0 : f64
            %d = "frk_dyn.wrap"(%c42) {{tag = {NUM} : i64}} : (f64) -> {DYN}
            %c1 = arith.constant 1 : i64
            %rp = "frk_mem.array_new"(%c1) : (i64) -> {PACK}
            %z = arith.constant 0 : i64
            "frk_mem.array_set"(%rp, %z, %d) : ({PACK}, i64, {DYN}) -> ()
            return %rp : {PACK}
        }}
        func.func @after(%env: !frk_closure.envref, %pack: {PACK}) -> {PACK} {{
            %b = "frk_closure.env_load"(%env) {{index = 0 : i64, env = !frk_adt.product<[!frk_mem.box<!frk_dyn.dyn>]>}} : (!frk_closure.envref) -> !frk_mem.box<{DYN}>
            %c2 = arith.constant 2.0 : f64
            %d = "frk_dyn.wrap"(%c2) {{tag = {NUM} : i64}} : (f64) -> {DYN}
            "frk_mem.box_set"(%b, %d) : (!frk_mem.box<{DYN}>, {DYN}) -> ()
            %z = arith.constant 0 : i64
            %rp = "frk_mem.array_new"(%z) : (i64) -> {PACK}
            return %rp : {PACK}
        }}
        func.func @main() -> i64 {{
            %c0 = arith.constant 0.0 : f64
            %zero = "frk_dyn.wrap"(%c0) {{tag = {NUM} : i64}} : (f64) -> {DYN}
            %box = "frk_mem.box_new"(%zero) : ({DYN}) -> !frk_mem.box<{DYN}>
            %pe = "frk_adt.product_new"() : () -> {P_EMPTY}
            %pb = "frk_adt.product_snoc"(%pe, %box) : ({P_EMPTY}, !frk_mem.box<{DYN}>) -> !frk_adt.product<[!frk_mem.box<{DYN}>]>
            %bf = "frk_closure.make"(%pb) {{callee = @before}} : (!frk_adt.product<[!frk_mem.box<{DYN}>]>) -> {PACKFN}
            %th = "frk_closure.make"(%pe) {{callee = @thunk}} : ({P_EMPTY}) -> {PACKFN}
            %af = "frk_closure.make"(%pb) {{callee = @after}} : (!frk_adt.product<[!frk_mem.box<{DYN}>]>) -> {PACKFN}
            %r = "frk_ctl.wind"(%bf, %th, %af) : ({PACKFN}, {PACKFN}, {PACKFN}) -> {DYN}
            %rf = "frk_dyn.unwrap"(%r) {{tag = {NUM} : i64}} : ({DYN}) -> f64
            %after_d = "frk_mem.box_get"(%box) : (!frk_mem.box<{DYN}>) -> {DYN}
            %av = "frk_dyn.unwrap"(%after_d) {{tag = {NUM} : i64}} : ({DYN}) -> f64
            %c100 = arith.constant 100.0 : f64
            %scaled = arith.mulf %rf, %c100 : f64
            %sum = arith.addf %scaled, %av : f64
            %n = arith.fptosi %sum : f64 to i64
            return %n : i64
        }}"#
    );
    assert_eq!(interpret_i64(&source).unwrap(), 4202);
}

#[test]
fn wind_runs_after_when_an_escape_crosses() {
    // prompt → body(token) → wind whose THUNK aborts to the token.
    // after() must still fire (writes 2 into the box) and the prompt
    // catches 7 → 7 * 100 + 2 = 702. This is the escape-only
    // dynamic-wind contract (D-070) in one number.
    let source = format!(
        r#"func.func @before(%env: !frk_closure.envref, %pack: {PACK}) -> {PACK} {{
            %z = arith.constant 0 : i64
            %rp = "frk_mem.array_new"(%z) : (i64) -> {PACK}
            return %rp : {PACK}
        }}
        func.func @thunk(%env: !frk_closure.envref, %pack: {PACK}) -> {PACK} {{
            %tok = "frk_closure.env_load"(%env) {{index = 0 : i64, env = {P_I64}}} : (!frk_closure.envref) -> i64
            %c7 = arith.constant 7.0 : f64
            %v = "frk_dyn.wrap"(%c7) {{tag = {NUM} : i64}} : (f64) -> {DYN}
            "frk_ctl.abort"(%tok, %v) : (i64, {DYN}) -> ()
            %z = arith.constant 0 : i64
            %rp = "frk_mem.array_new"(%z) : (i64) -> {PACK}
            return %rp : {PACK}
        }}
        func.func @after(%env: !frk_closure.envref, %pack: {PACK}) -> {PACK} {{
            %b = "frk_closure.env_load"(%env) {{index = 0 : i64, env = !frk_adt.product<[!frk_mem.box<!frk_dyn.dyn>]>}} : (!frk_closure.envref) -> !frk_mem.box<{DYN}>
            %c2 = arith.constant 2.0 : f64
            %d = "frk_dyn.wrap"(%c2) {{tag = {NUM} : i64}} : (f64) -> {DYN}
            "frk_mem.box_set"(%b, %d) : (!frk_mem.box<{DYN}>, {DYN}) -> ()
            %z = arith.constant 0 : i64
            %rp = "frk_mem.array_new"(%z) : (i64) -> {PACK}
            return %rp : {PACK}
        }}
        func.func @body(%box: !frk_mem.box<{DYN}>, %tok: i64) -> {DYN} {{
            %pe = "frk_adt.product_new"() : () -> {P_EMPTY}
            %bf = "frk_closure.make"(%pe) {{callee = @before}} : ({P_EMPTY}) -> {PACKFN}
            %pt = "frk_adt.product_snoc"(%pe, %tok) : ({P_EMPTY}, i64) -> {P_I64}
            %th = "frk_closure.make"(%pt) {{callee = @thunk}} : ({P_I64}) -> {PACKFN}
            %pa = "frk_adt.product_snoc"(%pe, %box) : ({P_EMPTY}, !frk_mem.box<{DYN}>) -> !frk_adt.product<[!frk_mem.box<{DYN}>]>
            %af = "frk_closure.make"(%pa) {{callee = @after}} : (!frk_adt.product<[!frk_mem.box<{DYN}>]>) -> {PACKFN}
            %r = "frk_ctl.wind"(%bf, %th, %af) : ({PACKFN}, {PACKFN}, {PACKFN}) -> {DYN}
            return %r : {DYN}
        }}
        func.func @main() -> i64 {{
            %c0 = arith.constant 0.0 : f64
            %zero = "frk_dyn.wrap"(%c0) {{tag = {NUM} : i64}} : (f64) -> {DYN}
            %box = "frk_mem.box_new"(%zero) : ({DYN}) -> !frk_mem.box<{DYN}>
            %pe = "frk_adt.product_new"() : () -> {P_EMPTY}
            %pb = "frk_adt.product_snoc"(%pe, %box) : ({P_EMPTY}, !frk_mem.box<{DYN}>) -> !frk_adt.product<[!frk_mem.box<{DYN}>]>
            %bo = "frk_closure.make"(%pb) {{callee = @body}} : (!frk_adt.product<[!frk_mem.box<{DYN}>]>) -> {FN_BODY}
            %out = "frk_ctl.prompt"(%bo) : ({FN_BODY}) -> {DYN}
            %caught = "frk_dyn.unwrap"(%out) {{tag = {NUM} : i64}} : ({DYN}) -> f64
            %after_d = "frk_mem.box_get"(%box) : (!frk_mem.box<{DYN}>) -> {DYN}
            %av = "frk_dyn.unwrap"(%after_d) {{tag = {NUM} : i64}} : ({DYN}) -> f64
            %c100 = arith.constant 100.0 : f64
            %scaled = arith.mulf %caught, %c100 : f64
            %sum = arith.addf %scaled, %av : f64
            %n = arith.fptosi %sum : f64 to i64
            return %n : i64
        }}"#
    );
    assert_eq!(interpret_i64(&source).unwrap(), 702);
}
