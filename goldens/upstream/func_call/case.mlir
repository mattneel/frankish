// Direct call between functions: main squares 7 via a helper. Only the
// entry needs llvm.emit_c_interface; @square is internal.
func.func @square(%x: i64) -> i64 {
  %sq = arith.muli %x, %x : i64
  return %sq : i64
}

func.func @main() -> i64 attributes {llvm.emit_c_interface} {
  %c7 = arith.constant 7 : i64
  %r = func.call @square(%c7) : (i64) -> i64
  return %r : i64
}
