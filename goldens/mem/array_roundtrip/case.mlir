// Arrays (D-049): new + set + get + len; aliasing through a second
// SSA use observes writes (reference semantics).
func.func @main() -> i64 attributes {llvm.emit_c_interface} {
  %n = arith.constant 3 : i64
  %a = "frk_mem.array_new"(%n) : (i64) -> !frk_mem.arr<i64>
  %i0 = arith.constant 0 : i64
  %i1 = arith.constant 1 : i64
  %i2 = arith.constant 2 : i64
  %v10 = arith.constant 10 : i64
  %v12 = arith.constant 12 : i64
  %v20 = arith.constant 20 : i64
  "frk_mem.array_set"(%a, %i0, %v10) : (!frk_mem.arr<i64>, i64, i64) -> ()
  "frk_mem.array_set"(%a, %i1, %v12) : (!frk_mem.arr<i64>, i64, i64) -> ()
  "frk_mem.array_set"(%a, %i2, %v20) : (!frk_mem.arr<i64>, i64, i64) -> ()
  %x = "frk_mem.array_get"(%a, %i0) : (!frk_mem.arr<i64>, i64) -> i64
  %y = "frk_mem.array_get"(%a, %i1) : (!frk_mem.arr<i64>, i64) -> i64
  %z = "frk_mem.array_get"(%a, %i2) : (!frk_mem.arr<i64>, i64) -> i64
  %len = "frk_mem.array_len"(%a) : (!frk_mem.arr<i64>) -> i64
  %s1 = arith.addi %x, %y : i64
  %s2 = arith.addi %s1, %z : i64
  %s3 = arith.subi %s2, %len : i64
  return %s3 : i64
}
