//
// Product construction and both projections: (30, 12) → 30 + 12 = 42.
func.func @main() -> i64 attributes {llvm.emit_c_interface} {
  %a = arith.constant 30 : i64
  %b = arith.constant 12 : i64
  %p = "frk_adt.make_product"(%a, %b) : (i64, i64) -> !frk_adt.product<[i64, i64]>
  %x = "frk_adt.get"(%p) {field = 0 : i64} : (!frk_adt.product<[i64, i64]>) -> i64
  %y = "frk_adt.get"(%p) {field = 1 : i64} : (!frk_adt.product<[i64, i64]>) -> i64
  %sum = arith.addi %x, %y : i64
  return %sum : i64
}
