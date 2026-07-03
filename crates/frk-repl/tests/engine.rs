//! Engine verifiers (L1): the D-043 semantics line by line — decl
//! commit + val printing, expression evaluation with typed rendering,
//! error lines leaving the session unchanged, shadowing, :type.

use frk_repl::Engine;

#[test]
fn expressions_evaluate_and_render_by_type() {
    let mut engine = Engine::new();
    assert_eq!(engine.feed("1 + 41"), "- : int = 42");
    assert_eq!(engine.feed("3 < 4"), "- : bool = true");
    assert_eq!(engine.feed("(1, (true, 2))"), "- : (int * (bool * int)) = (1, (true, 2))");
    assert_eq!(engine.feed("fun x -> x + 1"), "- : (int -> int) = <fun>");
}

#[test]
fn decls_commit_and_accumulate() {
    let mut engine = Engine::new();
    assert_eq!(engine.feed("let x = 40"), "val x : int");
    assert_eq!(engine.feed("let add a b = a + b"), "val add : (int -> (int -> int))");
    assert_eq!(engine.feed("add x 2"), "- : int = 42");
    // Shadowing is ml shadowing.
    assert_eq!(engine.feed("let x = true"), "val x : bool");
    assert_eq!(engine.feed("x"), "- : bool = true");
}

#[test]
fn adts_define_construct_and_render() {
    let mut engine = Engine::new();
    assert_eq!(engine.feed("type opt = None | Some of int"), "type defined");
    assert_eq!(engine.feed("Some 41"), "- : opt = Some 41");
    assert_eq!(engine.feed("None"), "- : opt = None");
    assert_eq!(
        engine.feed("match Some 41 with None -> 0 | Some n -> n + 1"),
        "- : int = 42"
    );
}

#[test]
fn errors_leave_the_session_unchanged() {
    let mut engine = Engine::new();
    assert_eq!(engine.feed("let x = 1"), "val x : int");
    let error = engine.feed("let y = x + true");
    assert!(error.starts_with("error:"), "{error}");
    // y never entered the session; x survived.
    assert!(engine.feed("y").starts_with("error:"));
    assert_eq!(engine.feed("x"), "- : int = 1");
}

#[test]
fn type_command_answers_without_evaluating() {
    let mut engine = Engine::new();
    engine.feed("let rec fact n = if n = 0 then 1 else n * fact (n - 1)");
    assert_eq!(engine.feed(":type fact"), "- : (int -> int)");
    assert_eq!(engine.feed(":type fact 5"), "- : int");
    assert_eq!(engine.feed("fact 5"), "- : int = 120");
}

#[test]
fn profile_switches_and_prompt_tracks() {
    let mut engine = Engine::new();
    assert_eq!(engine.prompt(), "frk[arena]> ");
    assert_eq!(engine.feed(":profile rc"), "profile: Rc");
    assert_eq!(engine.prompt(), "frk[rc]> ");
    assert!(engine.feed(":profile lisp").starts_with("error:"));
}
