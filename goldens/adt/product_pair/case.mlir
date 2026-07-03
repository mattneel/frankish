//
// Product built by snoc chain (D-036), both projections: (30, 12) → 42.
func.func @main() -> i64 attributes {llvm.emit_c_interface} {
  %a = arith.constant 30 : i64
  %b = arith.constant 12 : i64
  %e = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
  %p1 = "frk_adt.product_snoc"(%e, %a) : (!frk_adt.product<[]>, i64) -> !frk_adt.product<[i64]>
  %p = "frk_adt.product_snoc"(%p1, %b) : (!frk_adt.product<[i64]>, i64) -> !frk_adt.product<[i64, i64]>
  %x = "frk_adt.get"(%p) {field = 0 : i64} : (!frk_adt.product<[i64, i64]>) -> i64
  %y = "frk_adt.get"(%p) {field = 1 : i64} : (!frk_adt.product<[i64, i64]>) -> i64
  %sum = arith.addi %x, %y : i64
  return %sum : i64
}
