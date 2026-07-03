// frk-case: entry=main
// frk-case: result=i64
// Structured branch: scf.if yielding a value from each region.
// 7 > 3, so the then-region's 111 wins.
func.func @main() -> i64 attributes {llvm.emit_c_interface} {
  %c7 = arith.constant 7 : i64
  %c3 = arith.constant 3 : i64
  %c111 = arith.constant 111 : i64
  %c222 = arith.constant 222 : i64
  %cond = arith.cmpi sgt, %c7, %c3 : i64
  %r = scf.if %cond -> (i64) {
    scf.yield %c111 : i64
  } else {
    scf.yield %c222 : i64
  }
  return %r : i64
}
