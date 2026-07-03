// A box stored inside a product (SlotKind::Ptr: one ptrtoint'd slot),
// retrieved and dereferenced. The double use of %b exercises the rc
// retain (shared, not transfer-elided). 41 + 1 = 42.
func.func @main() -> i64 attributes {llvm.emit_c_interface} {
  %x = arith.constant 41 : i64
  %b = "frk_mem.box_new"(%x) : (i64) -> !frk_mem.box<i64>
  %e = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
  %p = "frk_adt.product_snoc"(%e, %b) : (!frk_adt.product<[]>, !frk_mem.box<i64>) -> !frk_adt.product<[!frk_mem.box<i64>]>
  %back = "frk_adt.get"(%p) {field = 0 : i64} : (!frk_adt.product<[!frk_mem.box<i64>]>) -> !frk_mem.box<i64>
  %v = "frk_mem.box_get"(%back) : (!frk_mem.box<i64>) -> i64
  %direct = "frk_mem.box_get"(%b) : (!frk_mem.box<i64>) -> i64
  %sum = arith.addi %v, %direct : i64
  %m40 = arith.constant -40 : i64
  %r = arith.addi %sum, %m40 : i64
  return %r : i64
}
