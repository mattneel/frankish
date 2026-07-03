// Recursion through func.call with scf.if arms: fib(10) = 55. Exercises
// the interpreter's call machinery and the JIT's calling convention on
// the same bytes.
func.func @fib(%n: i64) -> i64 {
  %c2 = arith.constant 2 : i64
  %small = arith.cmpi slt, %n, %c2 : i64
  %r = scf.if %small -> (i64) {
    scf.yield %n : i64
  } else {
    %c1 = arith.constant 1 : i64
    %n1 = arith.subi %n, %c1 : i64
    %f1 = func.call @fib(%n1) : (i64) -> i64
    %n2 = arith.subi %n, %c2 : i64
    %f2 = func.call @fib(%n2) : (i64) -> i64
    %s = arith.addi %f1, %f2 : i64
    scf.yield %s : i64
  }
  return %r : i64
}

func.func @main() -> i64 attributes {llvm.emit_c_interface} {
  %c10 = arith.constant 10 : i64
  %r = func.call @fib(%c10) : (i64) -> i64
  return %r : i64
}
