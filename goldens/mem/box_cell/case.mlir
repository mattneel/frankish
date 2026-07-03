// The mutable cell: set overwrites, get observes. 40 → 42.
func.func @main() -> i64 attributes {llvm.emit_c_interface} {
  %a = arith.constant 40 : i64
  %b = "frk_mem.box_new"(%a) : (i64) -> !frk_mem.box<i64>
  %v = "frk_mem.box_get"(%b) : (!frk_mem.box<i64>) -> i64
  %two = arith.constant 2 : i64
  %v2 = arith.addi %v, %two : i64
  "frk_mem.box_set"(%b, %v2) : (!frk_mem.box<i64>, i64) -> ()
  %r = "frk_mem.box_get"(%b) : (!frk_mem.box<i64>) -> i64
  return %r : i64
}
