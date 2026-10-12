// The string arm of the compile-time evaluation service (D-136): the
// inner module returns a bstr; the outer compares it against the
// expected literal via the kernel's eq (the bytes are copied into the
// outer world - the inner heap never escapes). Returns 1 on a match.
func.func private @__calque_eval_s(!frk_bstr.str, !frk_bstr.str, i64) -> !frk_bstr.str

func.func @main() -> i64 attributes {llvm.emit_c_interface} {
  %m = "frk_bstr.lit"() {text = "func.func @inner(%x: i64) -> !frk_bstr.str {\n  %s = \"frk_bstr.lit\"() {text = \"comptime-answer\"} : () -> !frk_bstr.str\n  return %s : !frk_bstr.str\n}\n"} : () -> !frk_bstr.str
  %e = "frk_bstr.lit"() {text = "inner"} : () -> !frk_bstr.str
  %x = arith.constant 0 : i64
  %r = func.call @__calque_eval_s(%m, %e, %x) : (!frk_bstr.str, !frk_bstr.str, i64) -> !frk_bstr.str
  %want = "frk_bstr.lit"() {text = "comptime-answer"} : () -> !frk_bstr.str
  %hit = "frk_bstr.eq"(%r, %want) : (!frk_bstr.str, !frk_bstr.str) -> i1
  cf.cond_br %hit, ^ok, ^bad
^ok:
  %one = arith.constant 1 : i64
  return %one : i64
^bad:
  %zero = arith.constant 0 : i64
  return %zero : i64
}
