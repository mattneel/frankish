// calque-artifact v0
// The byte-exact print row (D-088.7): io.print writes EXACTLY the
// bytes given — no implicit newline (SPEC §6.6). Observable contract:
// output is exactly `abc`, main returns 0.

func.func @main() -> i64 attributes {llvm.emit_c_interface} {
  %s = "frk_bstr.lit"() {text = "abc"} : () -> !frk_bstr.str
  func.call @frk_rt_calque_print(%s) : (!frk_bstr.str) -> ()
  %zero = arith.constant 0 : i64
  return %zero : i64
}
