// Strings (D-049): literals, concat, eq, len — all at kernel level.
// "hé" (2 units) ++ "😀" (2 units, surrogate pair) → len 4; equality
// is structural; result = len*10 + eq(=1) + ne(=0) = 41 + 1 = 42.
func.func @main() -> i64 attributes {llvm.emit_c_interface} {
  %a = "frk_str.lit"() {text = "hé"} : () -> !frk_str.str
  %b = "frk_str.lit"() {text = "😀"} : () -> !frk_str.str
  %ab = "frk_str.concat"(%a, %b) : (!frk_str.str, !frk_str.str) -> !frk_str.str
  %ab2 = "frk_str.concat"(%a, %b) : (!frk_str.str, !frk_str.str) -> !frk_str.str
  %len = "frk_str.len"(%ab) : (!frk_str.str) -> i64
  %same = "frk_str.eq"(%ab, %ab2) : (!frk_str.str, !frk_str.str) -> i1
  %diff = "frk_str.eq"(%ab, %a) : (!frk_str.str, !frk_str.str) -> i1
  %ten = arith.constant 10 : i64
  %scaled = arith.muli %len, %ten : i64
  %same64 = arith.extui %same : i1 to i64
  %diff64 = arith.extui %diff : i1 to i64
  %s1 = arith.addi %scaled, %same64 : i64
  %s2 = arith.addi %s1, %diff64 : i64
  return %s2 : i64
}
