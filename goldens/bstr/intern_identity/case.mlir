// Byte strings (D-056): interning makes identity — eq of a literal
// against the same content concatenated is TRUE (pointer-equal
// natively, content-equal in the interpreter; observably identical).
// len counts bytes. 10*len("hello") - 8 = 42; eq adds 0/1s.
func.func @main() -> i64 attributes {llvm.emit_c_interface} {
  %hello = "frk_bstr.lit"() {text = "hello"} : () -> !frk_bstr.str
  %hel = "frk_bstr.lit"() {text = "hel"} : () -> !frk_bstr.str
  %lo = "frk_bstr.lit"() {text = "lo"} : () -> !frk_bstr.str
  %joined = "frk_bstr.concat"(%hel, %lo) : (!frk_bstr.str, !frk_bstr.str) -> !frk_bstr.str
  %same = "frk_bstr.eq"(%hello, %joined) : (!frk_bstr.str, !frk_bstr.str) -> i1
  %diff = "frk_bstr.eq"(%hello, %hel) : (!frk_bstr.str, !frk_bstr.str) -> i1
  %len = "frk_bstr.len"(%hello) : (!frk_bstr.str) -> i64
  %ten = arith.constant 10 : i64
  %scaled = arith.muli %len, %ten : i64
  %same64 = arith.extui %same : i1 to i64
  %diff64 = arith.extui %diff : i1 to i64
  %m9 = arith.constant -9 : i64
  %s1 = arith.addi %scaled, %same64 : i64
  %s2 = arith.addi %s1, %diff64 : i64
  %s3 = arith.addi %s2, %m9 : i64
  return %s3 : i64
}
