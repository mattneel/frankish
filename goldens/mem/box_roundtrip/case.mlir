// The mem surface's simplest golden: allocate, read back. Identical
// observable behavior under every strategy (D-041).
func.func @main() -> i64 attributes {llvm.emit_c_interface} {
  %x = arith.constant 42 : i64
  %b = "frk_mem.box_new"(%x) : (i64) -> !frk_mem.box<i64>
  %y = "frk_mem.box_get"(%b) : (!frk_mem.box<i64>) -> i64
  return %y : i64
}
