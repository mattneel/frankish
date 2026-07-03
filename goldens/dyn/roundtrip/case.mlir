// frk.dyn (D-051): wrap/tag_of/unwrap through the fat-value model.
// The M10 interp fence LIFTED at M11 (D-054 bar 1): this golden now
// rides every runner and the grid.
func.func @main() -> i64 attributes {llvm.emit_c_interface} {
  %x = arith.constant 40.0 : f64
  %d = "frk_dyn.wrap"(%x) {tag = 2 : i64} : (f64) -> !frk_dyn.dyn
  %tag = "frk_dyn.tag_of"(%d) : (!frk_dyn.dyn) -> i64
  %y = "frk_dyn.unwrap"(%d) {tag = 2 : i64} : (!frk_dyn.dyn) -> f64
  %yi = arith.fptosi %y : f64 to i64
  %sum = arith.addi %yi, %tag : i64
  return %sum : i64
}
