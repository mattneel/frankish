// κ_frk v1 (D-069): the abortive clause — returns [99] WITHOUT
// consuming κ; the handle yields 99 and the body's +2 never runs.
// Natively the abort rides the pending cell, so the body carries the
// D-061 guard EXPLICITLY after its perform (hand-written frontends'
// discipline, visible here as IR).
func.func @clause(%env: !frk_closure.envref, %pack: !frk_mem.arr<!frk_dyn.dyn>) -> !frk_mem.arr<!frk_dyn.dyn> {
  %c99 = arith.constant 99.0 : f64
  %d = "frk_dyn.wrap"(%c99) {tag = 2 : i64} : (f64) -> !frk_dyn.dyn
  %c1 = arith.constant 1 : i64
  %rp = "frk_mem.array_new"(%c1) : (i64) -> !frk_mem.arr<!frk_dyn.dyn>
  %c0 = arith.constant 0 : i64
  "frk_mem.array_set"(%rp, %c0, %d) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
  return %rp : !frk_mem.arr<!frk_dyn.dyn>
}
func.func @body(%tok: i64) -> !frk_dyn.dyn {
  %c20 = arith.constant 20.0 : f64
  %v = "frk_dyn.wrap"(%c20) {tag = 2 : i64} : (f64) -> !frk_dyn.dyn
  %r = "frk_ctl.perform"(%v) {label = "ask"} : (!frk_dyn.dyn) -> !frk_dyn.dyn
  %p = "frk_ctl.pending"() : () -> i64
  %z = arith.constant 0 : i64
  %pend = arith.cmpi ne, %p, %z : i64
  cf.cond_br %pend, ^divert, ^continue
^divert:
  %nilw = arith.constant 0 : i64
  %nil = "frk_dyn.wrap"(%nilw) {tag = 0 : i64} : (i64) -> !frk_dyn.dyn
  return %nil : !frk_dyn.dyn
^continue:
  %rf = "frk_dyn.unwrap"(%r) {tag = 2 : i64} : (!frk_dyn.dyn) -> f64
  %c2 = arith.constant 2.0 : f64
  %sum = arith.addf %rf, %c2 : f64
  %out = "frk_dyn.wrap"(%sum) {tag = 2 : i64} : (f64) -> !frk_dyn.dyn
  return %out : !frk_dyn.dyn
}
func.func @main() -> i64 attributes {llvm.emit_c_interface} {
  %e = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
  %cl = "frk_closure.make"(%e) {callee = @clause} : (!frk_adt.product<[]>) -> !frk_closure.fn<[!frk_mem.arr<!frk_dyn.dyn>], [!frk_mem.arr<!frk_dyn.dyn>]>
  %bo = "frk_closure.make"(%e) {callee = @body} : (!frk_adt.product<[]>) -> !frk_closure.fn<[i64], [!frk_dyn.dyn]>
  %out = "frk_ctl.handle"(%cl, %bo) {label = "ask"} : (!frk_closure.fn<[!frk_mem.arr<!frk_dyn.dyn>], [!frk_mem.arr<!frk_dyn.dyn>]>, !frk_closure.fn<[i64], [!frk_dyn.dyn]>) -> !frk_dyn.dyn
  %f = "frk_dyn.unwrap"(%out) {tag = 2 : i64} : (!frk_dyn.dyn) -> f64
  %n = arith.fptosi %f : f64 to i64
  return %n : i64
}
