// A closure THROUGH dyn (D-051 tag 5): two-word payloads heap-box via
// the strategy allocator on wrap and reload on unwrap — the fat-value
// model's boxed arm, exercised end to end.
func.func @add_one(%x: i64) -> i64 {
  %one = arith.constant 1 : i64
  %r = arith.addi %x, %one : i64
  return %r : i64
}
func.func @main() -> i64 attributes {llvm.emit_c_interface} {
  %env = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
  %f = "frk_closure.make"(%env) {callee = @add_one} : (!frk_adt.product<[]>) -> !frk_closure.fn<[i64], [i64]>
  %d = "frk_dyn.wrap"(%f) {tag = 5 : i64} : (!frk_closure.fn<[i64], [i64]>) -> !frk_dyn.dyn
  %t = "frk_dyn.tag_of"(%d) : (!frk_dyn.dyn) -> i64
  %g = "frk_dyn.unwrap"(%d) {tag = 5 : i64} : (!frk_dyn.dyn) -> !frk_closure.fn<[i64], [i64]>
  %e2 = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
  %forty = arith.constant 36 : i64
  %pack = "frk_adt.product_snoc"(%e2, %forty) : (!frk_adt.product<[]>, i64) -> !frk_adt.product<[i64]>
  %r = "frk_closure.apply"(%g, %pack) : (!frk_closure.fn<[i64], [i64]>, !frk_adt.product<[i64]>) -> i64
  %sum = arith.addi %r, %t : i64
  return %sum : i64
}
