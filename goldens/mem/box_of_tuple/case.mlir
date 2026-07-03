// A box holding a struct payload (product<[i64, i64]>): typed struct
// store/load through the box pointer. 30 + 12 = 42.
func.func @main() -> i64 attributes {llvm.emit_c_interface} {
  %a = arith.constant 30 : i64
  %c = arith.constant 12 : i64
  %e = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
  %p1 = "frk_adt.product_snoc"(%e, %a) : (!frk_adt.product<[]>, i64) -> !frk_adt.product<[i64]>
  %p = "frk_adt.product_snoc"(%p1, %c) : (!frk_adt.product<[i64]>, i64) -> !frk_adt.product<[i64, i64]>
  %b = "frk_mem.box_new"(%p) : (!frk_adt.product<[i64, i64]>) -> !frk_mem.box<!frk_adt.product<[i64, i64]>>
  %q = "frk_mem.box_get"(%b) : (!frk_mem.box<!frk_adt.product<[i64, i64]>>) -> !frk_adt.product<[i64, i64]>
  %x = "frk_adt.get"(%q) {field = 0 : i64} : (!frk_adt.product<[i64, i64]>) -> i64
  %y = "frk_adt.get"(%q) {field = 1 : i64} : (!frk_adt.product<[i64, i64]>) -> i64
  %r = arith.addi %x, %y : i64
  return %r : i64
}
