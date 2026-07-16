// κ_frk v1 (D-069): the forced-general-vs-fast-path license gate.
// The interpreter routes this through the general dispatch machinery;
// native uses the evidence stack + a direct uniform apply. Byte-equal
// or the license is void. perform "ask" 20 → clause doubles via κ →
// body adds 2 → 42.
func.func @clause(%env: !frk_closure.envref, %pack: !frk_mem.arr<!frk_dyn.dyn>) -> !frk_mem.arr<!frk_dyn.dyn> {
  %c0 = arith.constant 0 : i64
  %v = "frk_mem.array_get"(%pack, %c0) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
  %c1 = arith.constant 1 : i64
  %kd = "frk_mem.array_get"(%pack, %c1) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
  %k = "frk_dyn.unwrap"(%kd) {tag = 5 : i64} : (!frk_dyn.dyn) -> !frk_closure.fn<[!frk_mem.arr<!frk_dyn.dyn>], [!frk_mem.arr<!frk_dyn.dyn>]>
  %n = "frk_dyn.unwrap"(%v) {tag = 2 : i64} : (!frk_dyn.dyn) -> f64
  %two = arith.constant 2.0 : f64
  %d = arith.mulf %n, %two : f64
  %dd = "frk_dyn.wrap"(%d) {tag = 2 : i64} : (f64) -> !frk_dyn.dyn
  %argp = "frk_mem.array_new"(%c1) : (i64) -> !frk_mem.arr<!frk_dyn.dyn>
  %z = arith.constant 0 : i64
  "frk_mem.array_set"(%argp, %z, %dd) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
  %pe = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
  %pp = "frk_adt.product_snoc"(%pe, %argp) : (!frk_adt.product<[]>, !frk_mem.arr<!frk_dyn.dyn>) -> !frk_adt.product<[!frk_mem.arr<!frk_dyn.dyn>]>
  %r = "frk_closure.apply"(%k, %pp) : (!frk_closure.fn<[!frk_mem.arr<!frk_dyn.dyn>], [!frk_mem.arr<!frk_dyn.dyn>]>, !frk_adt.product<[!frk_mem.arr<!frk_dyn.dyn>]>) -> !frk_mem.arr<!frk_dyn.dyn>
  return %r : !frk_mem.arr<!frk_dyn.dyn>
}
func.func @body(%tok: i64) -> !frk_dyn.dyn {
  %c20 = arith.constant 20.0 : f64
  %v = "frk_dyn.wrap"(%c20) {tag = 2 : i64} : (f64) -> !frk_dyn.dyn
  %r = "frk_ctl.perform"(%v) {label = "ask"} : (!frk_dyn.dyn) -> !frk_dyn.dyn
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
