// The tail-call law (D-059): 500k self-tail iterations at FIXED
// stack. Fails without the interpreter trampoline (depth cap 1024)
// and without native musttail (~24MB of frames into an 8MB stack).
// Sum 1..500000 = 125000250000.
func.func @spin(%n: i64, %acc: i64) -> i64 {
  %zero = arith.constant 0 : i64
  %done = arith.cmpi eq, %n, %zero : i64
  cf.cond_br %done, ^exit, ^next
^exit:
  return %acc : i64
^next:
  %one = arith.constant 1 : i64
  %n2 = arith.subi %n, %one : i64
  %acc2 = arith.addi %acc, %n : i64
  %r = func.call @spin(%n2, %acc2) : (i64, i64) -> i64
  return %r : i64
}
func.func @main() -> i64 attributes {llvm.emit_c_interface} {
  %n = arith.constant 500000 : i64
  %zero = arith.constant 0 : i64
  %r = func.call @spin(%n, %zero) : (i64, i64) -> i64
  return %r : i64
}
