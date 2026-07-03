//! End-to-end verifiers for the ml_core frontend (law L1): source →
//! parse → infer → emit → frk verify → reference interpreter. The JIT
//! side of the same programs is covered corpus-wide by the harness
//! (.ml golden cases under the differential law).

use frk_interp::Interp;
use melior::ir::operation::OperationLike;

fn run_ml(source: &str) -> Result<i64, String> {
    let context = frk_core::context();
    frk_dialects::register(&context).expect("registration");
    let module = frk_front::compile_ml(&context, source).map_err(|e| e.to_string())?;
    assert!(
        module.as_operation().verify(),
        "compiled module must pass MLIR verification"
    );
    frk_dialects::verify(&context, &module)
        .map_err(|e| format!("frk verify: {e}"))?;

    let mut interp = Interp::new(&module).map_err(|e| e.to_string())?;
    frk_dialects::register_eval(&mut interp);
    let values = interp
        .eval_function("main", &[])
        .map_err(|e| e.to_string())?;
    values[0].as_signed().map_err(|e| e.to_string())
}

fn expect(source: &str, value: i64) {
    match run_ml(source) {
        Ok(result) => assert_eq!(result, value, "program:\n{source}"),
        Err(error) => panic!("program failed: {error}\n{source}"),
    }
}

fn expect_compile_error(source: &str, needle: &str) {
    let context = frk_core::context();
    frk_dialects::register(&context).expect("registration");
    match frk_front::compile_ml(&context, source) {
        Ok(_) => panic!("expected a compile error mentioning {needle:?}"),
        Err(error) => {
            let message = error.to_string();
            assert!(
                message.contains(needle),
                "error should mention {needle:?}, got: {message}"
            );
        }
    }
}

#[test]
fn arithmetic_and_if() {
    expect("let main () = if 1 + 2 * 3 = 7 then 40 + 2 else 0", 42);
    expect("let main () = -5 + 47", 42);
    expect("let main () = (100 - 16) / 2", 42);
}

#[test]
fn lets_shadowing_and_tuples() {
    expect(
        "let main () = let x = 40 in let x = x + 1 in x + 1",
        42,
    );
    expect(
        "let main () = let p = (40, 2) in let (a, b) = p in a + b",
        42,
    );
    expect(
        "let main () = let (a, (b, c)) = (2, (10, 30)) in a + b + c",
        42,
    );
}

#[test]
fn booleans_short_circuit_shapes() {
    expect(
        "let main () = if true && false || true then 42 else 0",
        42,
    );
    expect("let main () = if 3 < 4 && 4 <= 4 then 42 else 0", 42);
    expect("let main () = if 3 <> 4 then 42 else 0", 42);
}

#[test]
fn currying_and_higher_order() {
    expect(
        "let add x y = x + y\nlet main () = add 40 2",
        42,
    );
    expect(
        "let add x y = x + y\nlet main () = let add40 = add 40 in add40 2",
        42,
    );
    expect(
        "let twice f x = f (f x)\nlet inc n = n + 1\nlet main () = twice inc 40",
        42,
    );
}

#[test]
fn closures_capture_by_value() {
    expect(
        "let main () = let n = 40 in let addn = fun x -> x + n in addn 2",
        42,
    );
    // Capture snapshots: rebinding n later must not change the closure.
    expect(
        "let main () = let n = 40 in let addn = fun x -> x + n in \
         let n = 0 in addn 2 + n",
        42,
    );
}

#[test]
fn let_rec_and_mutual_recursion() {
    expect(
        "let rec fact n = if n = 0 then 1 else n * fact (n - 1)\n\
         let main () = fact 5 - 78",
        42,
    );
    expect(
        "let rec even n = if n = 0 then true else odd (n - 1)\n\
         and odd n = if n = 0 then false else even (n - 1)\n\
         let main () = if even 10 && odd 7 then 42 else 0",
        42,
    );
    // Local let rec: the closure re-makes itself (D-035 spin pattern).
    expect(
        "let main () = let rec sum n = if n = 0 then 0 else n + sum (n - 1) in sum 6 + 21",
        42,
    );
}

#[test]
fn adts_and_matches() {
    expect(
        "type opt = None | Some of int\n\
         let get_or d o = match o with None -> d | Some x -> x\n\
         let main () = get_or 0 (Some 40) + get_or 2 None",
        42,
    );
    expect(
        "type shape = Circle of int | Rect of int * int | Point\n\
         let area s = match s with\n\
           | Circle r -> 3 * r * r\n\
           | Rect (w, h) -> w * h\n\
           | Point -> 0\n\
         let main () = area (Rect (6, 7)) + area Point",
        42,
    );
    // Nested patterns drive nested switches.
    expect(
        "type opt = None | Some of int\n\
         type box = Box of opt\n\
         let peek b = match b with\n\
           | Box (Some x) -> x\n\
           | Box None -> 0\n\
         let main () = peek (Box (Some 40)) + peek (Box None) + 2",
        42,
    );
    // Bool scrutinees dispatch as two-variant sums.
    expect(
        "let flip b = match b with true -> false | false -> true\n\
         let main () = if flip false then 42 else 0",
        42,
    );
    // Int literals + default arm.
    expect(
        "let classify n = match n with 0 -> 10 | 1 -> 20 | _ -> 30\n\
         let main () = classify 0 + classify 1 + classify 5 - 18",
        42,
    );
}

#[test]
fn adt_values_cross_closure_boundaries() {
    // Tuples and sums as params/captures — the Words slot path.
    expect(
        "type opt = None | Some of int\n\
         let unwrap o = match o with None -> 0 | Some x -> x\n\
         let main () = let o = Some 40 in let f = fun d -> unwrap o + d in f 2",
        42,
    );
    expect(
        "let swap p = let (a, b) = p in (b, a)\n\
         let main () = let (x, y) = swap (2, 40) in x - y + 4",
        42,
    );
}

#[test]
fn single_instantiation_polymorphism_emits() {
    expect(
        "let id x = x\nlet main () = id 42",
        42,
    );
    expect(
        "let fst p = let (a, _) = p in a\n\
         let main () = fst (42, 7)",
        42,
    );
}

#[test]
fn frontend_rejections() {
    expect_compile_error("let main () = nope + 1", "unbound variable nope");
    expect_compile_error("let main () = 1 + true", "cannot unify");
    expect_compile_error(
        "type opt = None | Some of int\n\
         let main () = match Some 1 with Some x -> x",
        "non-exhaustive",
    );
    expect_compile_error(
        "let main () = match 1 with 0 -> 1 | 0 -> 2 | _ -> 3",
        "redundant",
    );
    expect_compile_error(
        "type list = Nil | Cons of int * list\nlet main () = 0",
        "recursive ADT",
    );
    expect_compile_error(
        "let id x = x\nlet main () = id 1 + (if id true then 1 else 0)",
        "distinct types",
    );
    expect_compile_error("let f x = x", "no `main`");
}
