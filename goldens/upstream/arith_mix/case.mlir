// Mixed arith: mul, sub, signed div, compare, select.
// (7*6 - 12) / 2 = 15; 15 > 10, so select yields 100.
func.func @main() -> i64 attributes {llvm.emit_c_interface} {
  %c7 = arith.constant 7 : i64
  %c6 = arith.constant 6 : i64
  %c12 = arith.constant 12 : i64
  %c2 = arith.constant 2 : i64
  %c10 = arith.constant 10 : i64
  %c100 = arith.constant 100 : i64
  %c200 = arith.constant 200 : i64
  %prod = arith.muli %c7, %c6 : i64
  %diff = arith.subi %prod, %c12 : i64
  %quot = arith.divsi %diff, %c2 : i64
  %gt = arith.cmpi sgt, %quot, %c10 : i64
  %sel = arith.select %gt, %c100, %c200 : i64
  return %sel : i64
}
