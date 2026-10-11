// calque-artifact v0
// The M37 reference emission for specimens/calque/examples/fib.calq
// (D-092), hand-written and run end to end by tests/reference.rs.
// Shapes pinned here:
//   - every Calque function is envref-first uniform (`@fib`);
//   - direct calls thread the caller's envref;
//   - the entry `main` is convention-exempt: a nullary wrapper that
//     makes a captureless closure over the entry BODY (whose symbol
//     takes the reserved __calque_ prefix) and APPLIES it;
//   - the entry carries llvm.emit_c_interface (entry protocol v0).
// Arithmetic is unchecked in this reference (fib(10) cannot
// overflow); the checked sequence (D-088.3) is pinned with the
// seed's emission fixtures.
// Observable contract: main returns 55, no output.

func.func @fib(%env: !frk_closure.envref, %n: i64) -> i64 {
  %zero = arith.constant 0 : i64
  %two = arith.constant 2 : i64
  %one = arith.constant 1 : i64
  %lt = arith.cmpi slt, %n, %two : i64
  cf.cond_br %lt, ^base, ^rec
^base:
  return %n : i64
^rec:
  %n1 = arith.subi %n, %one : i64
  %a = func.call @fib(%env, %n1) : (!frk_closure.envref, i64) -> i64
  %n2 = arith.subi %n, %two : i64
  %b = func.call @fib(%env, %n2) : (!frk_closure.envref, i64) -> i64
  %sum = arith.addi %a, %b : i64
  return %sum : i64
}

func.func @__calque_main(%env: !frk_closure.envref) -> i64 {
  %n = arith.constant 10 : i64
  %r = func.call @fib(%env, %n) : (!frk_closure.envref, i64) -> i64
  return %r : i64
}

func.func @main() -> i64 attributes {llvm.emit_c_interface} {
  %env = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
  %cl = "frk_closure.make"(%env) {callee = @__calque_main} : (!frk_adt.product<[]>) -> !frk_closure.fn<[], [i64]>
  %args = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
  %r = "frk_closure.apply"(%cl, %args) : (!frk_closure.fn<[], [i64]>, !frk_adt.product<[]>) -> i64
  return %r : i64
}
