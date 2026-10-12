// The compile-time evaluation service (D-133, goal §11) through a raw
// artifact: an inner module computing x*x runs under the shared
// interpreter - 6*6 = 36 through every runner (interp builtin and the
// JIT host shim must agree).
func.func private @__calque_eval_i64(!frk_bstr.str, !frk_bstr.str, i64) -> i64

func.func @main() -> i64 attributes {llvm.emit_c_interface} {
  %m = "frk_bstr.lit"() {text = "func.func @inner(%x: i64) -> i64 {\n  %a = arith.muli %x, %x : i64\n  return %a : i64\n}\n"} : () -> !frk_bstr.str
  %e = "frk_bstr.lit"() {text = "inner"} : () -> !frk_bstr.str
  %x = arith.constant 6 : i64
  %r = func.call @__calque_eval_i64(%m, %e, %x) : (!frk_bstr.str, !frk_bstr.str, i64) -> i64
  return %r : i64
}
