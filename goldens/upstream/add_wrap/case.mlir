// Two's-complement wrap-around: i64::MAX + 1. MLIR arith.addi without
// overflow flags is arithmetic modulo 2^64 — defined behavior, and a
// canary for any runner that misrepresents integer storage.
func.func @main() -> i64 attributes {llvm.emit_c_interface} {
  %max = arith.constant 9223372036854775807 : i64
  %one = arith.constant 1 : i64
  %sum = arith.addi %max, %one : i64
  return %sum : i64
}
