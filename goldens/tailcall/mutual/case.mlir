// Mutual tail recursion at IDENTICAL signatures (the D-059 native
// frontier includes this case): even/odd ping-pong, 500k deep.
func.func @even(%n: i64) -> i64 {
  %zero = arith.constant 0 : i64
  %done = arith.cmpi eq, %n, %zero : i64
  cf.cond_br %done, ^yes, ^recurse
^yes:
  %one = arith.constant 1 : i64
  return %one : i64
^recurse:
  %one2 = arith.constant 1 : i64
  %m = arith.subi %n, %one2 : i64
  %r = func.call @odd(%m) : (i64) -> i64
  return %r : i64
}
func.func @odd(%n: i64) -> i64 {
  %zero = arith.constant 0 : i64
  %done = arith.cmpi eq, %n, %zero : i64
  cf.cond_br %done, ^no, ^recurse
^no:
  return %zero : i64
^recurse:
  %one = arith.constant 1 : i64
  %m = arith.subi %n, %one : i64
  %r = func.call @even(%m) : (i64) -> i64
  return %r : i64
}
func.func @main() -> i64 attributes {llvm.emit_c_interface} {
  %n = arith.constant 500000 : i64
  %e = func.call @even(%n) : (i64) -> i64
  %fortytwo = arith.constant 42 : i64
  %r = arith.muli %e, %fortytwo : i64
  return %r : i64
}
