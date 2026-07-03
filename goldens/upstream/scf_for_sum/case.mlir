// Structured loop: sum of 0..9 via scf.for with an i64 induction variable
// and an iter_args accumulator. 0+1+...+9 = 45.
func.func @main() -> i64 attributes {llvm.emit_c_interface} {
  %lb = arith.constant 0 : i64
  %ub = arith.constant 10 : i64
  %step = arith.constant 1 : i64
  %zero = arith.constant 0 : i64
  %sum = scf.for %i = %lb to %ub step %step iter_args(%acc = %zero) -> (i64) : i64 {
    %next = arith.addi %acc, %i : i64
    scf.yield %next : i64
  }
  return %sum : i64
}
