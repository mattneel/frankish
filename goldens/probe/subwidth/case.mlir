// The sub-width integer probe: i8/i16 arithmetic must AGREE across
// interp/jit/jit-rc/AOT. The contract: addi on a narrow signed type
// is MODULAR (LLVM's semantics), and extsi/extui widen with sign /
// zero respectively. Hand-derived:
//   100 + 55   = 155   -> i8  wraps to -101
//   30000+10000= 40000 -> i16 wraps to -25536
//   200 + 100  = 300   -> i8  wraps to 44 (zero-extended view)
//   -101 + -25536 + 7 + 44 = -25586
func.func @main() -> i64 attributes {llvm.emit_c_interface} {
  %a8 = arith.constant 100 : i8
  %b8 = arith.constant 55 : i8
  %c8 = arith.addi %a8, %b8 : i8
  %e8 = arith.extsi %c8 : i8 to i64
  %a16 = arith.constant 30000 : i16
  %b16 = arith.constant 10000 : i16
  %c16 = arith.addi %a16, %b16 : i16
  %e16 = arith.extsi %c16 : i16 to i64
  %u8a = arith.constant 200 : i8
  %u8b = arith.constant 100 : i8
  %u8c = arith.addi %u8a, %u8b : i8
  %u8e = arith.extui %u8c : i8 to i64
  %s1 = arith.addi %e8, %e16 : i64
  %k = arith.constant 7 : i64
  %s2 = arith.addi %s1, %k : i64
  %s3 = arith.addi %s2, %u8e : i64
  return %s3 : i64
}
