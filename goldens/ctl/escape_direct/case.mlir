// κ_frk native L1 verifier (D-060): a closure body aborts directly to
// its own prompt — no intervening frames, so NO guards are needed; the
// dummy return after abort IS the native diversion (pending set), and
// the prompt's resolve catches it. interp (real unwind) and jit
// (result-passing) must agree: 42.
func.func @body(%tok: i64) -> !frk_dyn.dyn {
  %c42 = arith.constant 42 : i64
  %v = "frk_dyn.wrap"(%c42) {tag = 2 : i64} : (i64) -> !frk_dyn.dyn
  "frk_ctl.abort"(%tok, %v) : (i64, !frk_dyn.dyn) -> ()
  %c0 = arith.constant 0 : i64
  %dead = "frk_dyn.wrap"(%c0) {tag = 2 : i64} : (i64) -> !frk_dyn.dyn
  return %dead : !frk_dyn.dyn
}
func.func @main() -> i64 attributes {llvm.emit_c_interface} {
  %e = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
  %body = "frk_closure.make"(%e) {callee = @body} : (!frk_adt.product<[]>) -> !frk_closure.fn<[i64], [!frk_dyn.dyn]>
  %out = "frk_ctl.prompt"(%body) : (!frk_closure.fn<[i64], [!frk_dyn.dyn]>) -> !frk_dyn.dyn
  %n = "frk_dyn.unwrap"(%out) {tag = 2 : i64} : (!frk_dyn.dyn) -> i64
  return %n : i64
}
