// frk-case: runners=interp
// (flips to all runners when the K3 closure lowering lands, M4)
//
// Church encoding, the M4 exit witness: two = λf.λx. f (f x), built
// from packed closures (D-035/D-036), applied to inc and 40 → 42.
// Exercises closure-capturing-closure and upward escape.
func.func @inc(%n: i64) -> i64 {
  %one = arith.constant 1 : i64
  %r = arith.addi %n, %one : i64
  return %r : i64
}

func.func @two_inner(%f: !frk_closure.fn<[i64], [i64]>, %x: i64) -> i64 {
  %e0 = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
  %a1 = "frk_adt.product_snoc"(%e0, %x) : (!frk_adt.product<[]>, i64) -> !frk_adt.product<[i64]>
  %fx = "frk_closure.apply"(%f, %a1) : (!frk_closure.fn<[i64], [i64]>, !frk_adt.product<[i64]>) -> i64
  %a2 = "frk_adt.product_snoc"(%e0, %fx) : (!frk_adt.product<[]>, i64) -> !frk_adt.product<[i64]>
  %ffx = "frk_closure.apply"(%f, %a2) : (!frk_closure.fn<[i64], [i64]>, !frk_adt.product<[i64]>) -> i64
  return %ffx : i64
}

func.func @two_outer(%f: !frk_closure.fn<[i64], [i64]>) -> !frk_closure.fn<[i64], [i64]> {
  %e = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
  %env = "frk_adt.product_snoc"(%e, %f) : (!frk_adt.product<[]>, !frk_closure.fn<[i64], [i64]>) -> !frk_adt.product<[!frk_closure.fn<[i64], [i64]>]>
  %two = "frk_closure.make"(%env) {callee = @two_inner} : (!frk_adt.product<[!frk_closure.fn<[i64], [i64]>]>) -> !frk_closure.fn<[i64], [i64]>
  return %two : !frk_closure.fn<[i64], [i64]>
}

func.func @main() -> i64 attributes {llvm.emit_c_interface} {
  %c40 = arith.constant 40 : i64
  %e = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
  %inc = "frk_closure.make"(%e) {callee = @inc} : (!frk_adt.product<[]>) -> !frk_closure.fn<[i64], [i64]>
  %two = func.call @two_outer(%inc) : (!frk_closure.fn<[i64], [i64]>) -> !frk_closure.fn<[i64], [i64]>
  %args = "frk_adt.product_snoc"(%e, %c40) : (!frk_adt.product<[]>, i64) -> !frk_adt.product<[i64]>
  %r = "frk_closure.apply"(%two, %args) : (!frk_closure.fn<[i64], [i64]>, !frk_adt.product<[i64]>) -> i64
  return %r : i64
}
