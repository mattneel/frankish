// Unstructured control flow: cf.cond_br into two blocks converging on a
// block argument. The condition is true, so the merge sees 5.
func.func @main() -> i64 attributes {llvm.emit_c_interface} {
  %five = arith.constant 5 : i64
  %six = arith.constant 6 : i64
  %cond = arith.constant true
  cf.cond_br %cond, ^on_true, ^on_false
^on_true:
  cf.br ^merge(%five : i64)
^on_false:
  cf.br ^merge(%six : i64)
^merge(%v: i64):
  return %v : i64
}
