// frk-case: runners=interp
// (flips to all runners when the K3 closure lowering lands, M4)
//
// The counter shape available without mutable state: fold a +3 closure
// four times from 30 through scf.for iter_args → 42. The STATEFUL
// counter (a closure over a mutable cell) waits for frk.mem (M7).
func.func @add3(%x: i64) -> i64 {
  %three = arith.constant 3 : i64
  %r = arith.addi %x, %three : i64
  return %r : i64
}

func.func @main() -> i64 attributes {llvm.emit_c_interface} {
  %lb = arith.constant 0 : i64
  %ub = arith.constant 4 : i64
  %step = arith.constant 1 : i64
  %init = arith.constant 30 : i64
  %e = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
  %add3 = "frk_closure.make"(%e) {callee = @add3} : (!frk_adt.product<[]>) -> !frk_closure.fn<[i64], [i64]>
  %sum = scf.for %i = %lb to %ub step %step iter_args(%acc = %init) -> (i64) : i64 {
    %args = "frk_adt.product_snoc"(%e, %acc) : (!frk_adt.product<[]>, i64) -> !frk_adt.product<[i64]>
    %next = "frk_closure.apply"(%add3, %args) : (!frk_closure.fn<[i64], [i64]>, !frk_adt.product<[i64]>) -> i64
    scf.yield %next : i64
  }
  return %sum : i64
}
