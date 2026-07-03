// Simplest possible execution golden: two constants and an add.
func.func @main() -> i64 attributes {llvm.emit_c_interface} {
  %a = arith.constant 40 : i64
  %b = arith.constant 2 : i64
  %sum = arith.addi %a, %b : i64
  return %sum : i64
}
