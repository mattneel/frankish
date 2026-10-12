// The interp's `index` support (D-132): an scf.for over index-typed
// bounds with an iter_arg accumulator. Hand-derived: 0+1+2+3+4 = 10.
func.func @main() -> i64 attributes {llvm.emit_c_interface} {
  %c0 = arith.constant 0 : index
  %c5 = arith.constant 5 : index
  %c1 = arith.constant 1 : index
  %sum = arith.constant 0 : i64
  %r = scf.for %i = %c0 to %c5 step %c1 iter_args(%acc = %sum) -> (i64) {
    %ii = arith.index_cast %i : index to i64
    %acc2 = arith.addi %acc, %ii : i64
    scf.yield %acc2 : i64
  }
  return %r : i64
}
