// calque-artifact v0
// The deterministic safety trap (D-088.3): the emitter composes the
// message — file:line:col + cause — into the bstr at emission time
// (SPEC §11). Observable contract: a deterministic trap whose message
// contains the composed text.

func.func @main() -> i64 attributes {llvm.emit_c_interface} {
  %s = "frk_bstr.lit"() {text = "demo.calq:3:9: error: CQ0001: integer overflow (i64 add)"} : () -> !frk_bstr.str
  func.call @frk_rt_calque_trap(%s) : (!frk_bstr.str) -> ()
  %zero = arith.constant 0 : i64
  return %zero : i64
}
