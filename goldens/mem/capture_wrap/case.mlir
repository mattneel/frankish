// M35 (D-084.4): the frk.capture discipline — a wrap-transferred
// vector whose wrap result is ALSO parked by a guard cold path.
// The capture store carries {frk.capture}: planner-invisible (the
// hot store keeps its sole-use TRANSFER; wrap_transferred keeps the
// allocation's die_at off) and ALWAYS-retained (the frame owns a
// real reference). Both readbacks must see the live payload — under
// jit-rc a miscount is a UAF/abort, which is the drill.
func.func @main() -> i64 attributes {llvm.emit_c_interface} {
  %c1 = arith.constant 1 : i64
  %c0 = arith.constant 0 : i64
  %v = arith.constant 4.200000e+01 : f64
  %vd = "frk_dyn.wrap"(%v) {tag = 2 : i64} : (f64) -> !frk_dyn.dyn

  // The "vector": arr<dyn> holding 42.0, wrapped tag 7.
  %arr = "frk_mem.array_new"(%c1) : (i64) -> !frk_mem.arr<!frk_dyn.dyn>
  "frk_mem.array_set"(%arr, %c0, %vd) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
  %d = "frk_dyn.wrap"(%arr) {tag = 7 : i64} : (!frk_mem.arr<!frk_dyn.dyn>) -> !frk_dyn.dyn

  // The guard cold path parks the SAME dyn into a frame record slot.
  %frame = "frk_mem.array_new"(%c1) : (i64) -> !frk_mem.arr<!frk_dyn.dyn>
  "frk_mem.array_set"(%frame, %c0, %d) {frk.capture} : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()

  // The hot path: sole visible use — ownership TRANSFERS here.
  %globals = "frk_mem.array_new"(%c1) : (i64) -> !frk_mem.arr<!frk_dyn.dyn>
  "frk_mem.array_set"(%globals, %c0, %d) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()

  // Read back through BOTH owners; each must see the live vector.
  %g = "frk_mem.array_get"(%globals, %c0) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
  %ga = "frk_dyn.unwrap"(%g) {tag = 7 : i64} : (!frk_dyn.dyn) -> !frk_mem.arr<!frk_dyn.dyn>
  %ge = "frk_mem.array_get"(%ga, %c0) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
  %gf = "frk_dyn.unwrap"(%ge) {tag = 2 : i64} : (!frk_dyn.dyn) -> f64
  %gi = arith.fptosi %gf : f64 to i64

  %f = "frk_mem.array_get"(%frame, %c0) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
  %fa = "frk_dyn.unwrap"(%f) {tag = 7 : i64} : (!frk_dyn.dyn) -> !frk_mem.arr<!frk_dyn.dyn>
  %fl = "frk_mem.array_len"(%fa) : (!frk_mem.arr<!frk_dyn.dyn>) -> i64
  %c3 = arith.constant 3 : i64
  %fx = arith.muli %fl, %c3 : i64
  %sum = arith.addi %gi, %fx : i64
  return %sum : i64
}
