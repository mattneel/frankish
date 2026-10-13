// The limited evaluation-service entry (D-151) through a raw artifact:
// the same inner module under an explicit step limit (100 operations,
// ample for 6*6) and under 0 (the default budget) - 36 + 36 = 72
// through every runner (interp builtin and both JIT shims agree).
func.func private @__calque_eval_i64_limited(!frk_bstr.str, !frk_bstr.str, i64, i64) -> i64

func.func @main() -> i64 attributes {llvm.emit_c_interface} {
  %m = "frk_bstr.lit"() {text = "func.func @inner(%x: i64) -> i64 {\n  %a = arith.muli %x, %x : i64\n  return %a : i64\n}\n"} : () -> !frk_bstr.str
  %e = "frk_bstr.lit"() {text = "inner"} : () -> !frk_bstr.str
  %x = arith.constant 6 : i64
  %limit = arith.constant 100 : i64
  %default = arith.constant 0 : i64
  %r1 = func.call @__calque_eval_i64_limited(%m, %e, %x, %limit) : (!frk_bstr.str, !frk_bstr.str, i64, i64) -> i64
  %r2 = func.call @__calque_eval_i64_limited(%m, %e, %x, %default) : (!frk_bstr.str, !frk_bstr.str, i64, i64) -> i64
  %r = arith.addi %r1, %r2 : i64
  return %r : i64
}
