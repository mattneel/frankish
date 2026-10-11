// calque intrinsics v0 (M37, D-062 pattern / D-088.6): the primitive
// surface every composed Calque unit may call. Bodyless `frk_rt_*`
// declarations are checked against the frk-abi registry by the
// semantic verifier (the D-062 declaration projection); the kernel
// lowering dedups against symbols already declared here.
//
// v0 carries only the runtime rows (D-088.7 — print is byte-exact
// with NO implicit newline; trap carries an emission-composed
// message). Vector helpers, the i64/bstr/box instantiations, and the
// derived-stdlib surface (str.fromI64) land with the seed's emission
// fixtures.

func.func private @frk_rt_calque_print(!frk_bstr.str)
func.func private @frk_rt_calque_trap(!frk_bstr.str)

// ---------------------------------------------------------------- vectors
//
// Vector(T) instantiations (F34/D-091): a managed box of
// [arr<slot>, len, cap]; growth doubles from an initial capacity of 4
// with a block-arg copy loop; out-of-range get/set TRAP through the
// runtime row. The seed resolves `vector.new/push/get/set/len` to these
// per-slot-kind monomorphic functions at compile time (b0 spells
// types): i64 slots, bstr slots, and erased-reference (box) slots.

func.func @__calque_vector_i64_new() -> !frk_mem.box<!frk_adt.product<[!frk_mem.arr<i64>, i64, i64]>> {
  %cap = arith.constant 4 : i64
  %arr = "frk_mem.array_new"(%cap) : (i64) -> !frk_mem.arr<i64>
  %zero = arith.constant 0 : i64
  %e = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
  %p1 = "frk_adt.product_snoc"(%e, %arr) : (!frk_adt.product<[]>, !frk_mem.arr<i64>) -> !frk_adt.product<[!frk_mem.arr<i64>]>
  %p2 = "frk_adt.product_snoc"(%p1, %zero) : (!frk_adt.product<[!frk_mem.arr<i64>]>, i64) -> !frk_adt.product<[!frk_mem.arr<i64>, i64]>
  %p3 = "frk_adt.product_snoc"(%p2, %cap) : (!frk_adt.product<[!frk_mem.arr<i64>, i64]>, i64) -> !frk_adt.product<[!frk_mem.arr<i64>, i64, i64]>
  %b = "frk_mem.box_new"(%p3) : (!frk_adt.product<[!frk_mem.arr<i64>, i64, i64]>) -> !frk_mem.box<!frk_adt.product<[!frk_mem.arr<i64>, i64, i64]>>
  return %b : !frk_mem.box<!frk_adt.product<[!frk_mem.arr<i64>, i64, i64]>>
}

func.func @__calque_vector_i64_len(%v: !frk_mem.box<!frk_adt.product<[!frk_mem.arr<i64>, i64, i64]>>) -> i64 {
  %n = "frk_mem.field_get"(%v) {field = 1 : i64} : (!frk_mem.box<!frk_adt.product<[!frk_mem.arr<i64>, i64, i64]>>) -> i64
  return %n : i64
}

func.func @__calque_vector_i64_get(%v: !frk_mem.box<!frk_adt.product<[!frk_mem.arr<i64>, i64, i64]>>, %i: i64) -> i64 {
  %n = "frk_mem.field_get"(%v) {field = 1 : i64} : (!frk_mem.box<!frk_adt.product<[!frk_mem.arr<i64>, i64, i64]>>) -> i64
  %ok = arith.cmpi ult, %i, %n : i64
  cf.cond_br %ok, ^hit, ^oob
^oob:
  %msg = "frk_bstr.lit"() {text = "calque: vector.get: index out of bounds"} : () -> !frk_bstr.str
  func.call @frk_rt_calque_trap(%msg) : (!frk_bstr.str) -> ()
  %dummy = arith.constant 0 : i64
  return %dummy : i64
^hit:
  %arr = "frk_mem.field_get"(%v) {field = 0 : i64} : (!frk_mem.box<!frk_adt.product<[!frk_mem.arr<i64>, i64, i64]>>) -> !frk_mem.arr<i64>
  %e = "frk_mem.array_get"(%arr, %i) : (!frk_mem.arr<i64>, i64) -> i64
  return %e : i64
}

func.func @__calque_vector_i64_set(%v: !frk_mem.box<!frk_adt.product<[!frk_mem.arr<i64>, i64, i64]>>, %i: i64, %x: i64) {
  %n = "frk_mem.field_get"(%v) {field = 1 : i64} : (!frk_mem.box<!frk_adt.product<[!frk_mem.arr<i64>, i64, i64]>>) -> i64
  %ok = arith.cmpi ult, %i, %n : i64
  cf.cond_br %ok, ^hit, ^oob
^oob:
  %msg = "frk_bstr.lit"() {text = "calque: vector.set: index out of bounds"} : () -> !frk_bstr.str
  func.call @frk_rt_calque_trap(%msg) : (!frk_bstr.str) -> ()
  return
^hit:
  %arr = "frk_mem.field_get"(%v) {field = 0 : i64} : (!frk_mem.box<!frk_adt.product<[!frk_mem.arr<i64>, i64, i64]>>) -> !frk_mem.arr<i64>
  "frk_mem.array_set"(%arr, %i, %x) : (!frk_mem.arr<i64>, i64, i64) -> ()
  return
}

func.func @__calque_vector_i64_push(%v: !frk_mem.box<!frk_adt.product<[!frk_mem.arr<i64>, i64, i64]>>, %x: i64) {
  %n = "frk_mem.field_get"(%v) {field = 1 : i64} : (!frk_mem.box<!frk_adt.product<[!frk_mem.arr<i64>, i64, i64]>>) -> i64
  %cap = "frk_mem.field_get"(%v) {field = 2 : i64} : (!frk_mem.box<!frk_adt.product<[!frk_mem.arr<i64>, i64, i64]>>) -> i64
  %full = arith.cmpi eq, %n, %cap : i64
  %zero = arith.constant 0 : i64
  %one = arith.constant 1 : i64
  cf.cond_br %full, ^grow, ^store(%n : i64)
^grow:
  %old = "frk_mem.field_get"(%v) {field = 0 : i64} : (!frk_mem.box<!frk_adt.product<[!frk_mem.arr<i64>, i64, i64]>>) -> !frk_mem.arr<i64>
  %two = arith.constant 2 : i64
  %ncap = arith.muli %cap, %two : i64
  %new = "frk_mem.array_new"(%ncap) : (i64) -> !frk_mem.arr<i64>
  cf.br ^copy(%zero : i64)
^copy(%ci: i64):
  %more = arith.cmpi slt, %ci, %n : i64
  cf.cond_br %more, ^copy_body, ^copy_done
^copy_body:
  %e = "frk_mem.array_get"(%old, %ci) : (!frk_mem.arr<i64>, i64) -> i64
  "frk_mem.array_set"(%new, %ci, %e) : (!frk_mem.arr<i64>, i64, i64) -> ()
  %ci1 = arith.addi %ci, %one : i64
  cf.br ^copy(%ci1 : i64)
^copy_done:
  "frk_mem.field_set"(%v, %new) {field = 0 : i64} : (!frk_mem.box<!frk_adt.product<[!frk_mem.arr<i64>, i64, i64]>>, !frk_mem.arr<i64>) -> ()
  "frk_mem.field_set"(%v, %ncap) {field = 2 : i64} : (!frk_mem.box<!frk_adt.product<[!frk_mem.arr<i64>, i64, i64]>>, i64) -> ()
  cf.br ^store(%n : i64)
^store(%cnt: i64):
  %arr2 = "frk_mem.field_get"(%v) {field = 0 : i64} : (!frk_mem.box<!frk_adt.product<[!frk_mem.arr<i64>, i64, i64]>>) -> !frk_mem.arr<i64>
  "frk_mem.array_set"(%arr2, %cnt, %x) : (!frk_mem.arr<i64>, i64, i64) -> ()
  %cnt1 = arith.addi %cnt, %one : i64
  "frk_mem.field_set"(%v, %cnt1) {field = 1 : i64} : (!frk_mem.box<!frk_adt.product<[!frk_mem.arr<i64>, i64, i64]>>, i64) -> ()
  return
}

func.func @__calque_vector_bstr_new() -> !frk_mem.box<!frk_adt.product<[!frk_mem.arr<!frk_bstr.str>, i64, i64]>> {
  %cap = arith.constant 4 : i64
  %arr = "frk_mem.array_new"(%cap) : (i64) -> !frk_mem.arr<!frk_bstr.str>
  %zero = arith.constant 0 : i64
  %e = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
  %p1 = "frk_adt.product_snoc"(%e, %arr) : (!frk_adt.product<[]>, !frk_mem.arr<!frk_bstr.str>) -> !frk_adt.product<[!frk_mem.arr<!frk_bstr.str>]>
  %p2 = "frk_adt.product_snoc"(%p1, %zero) : (!frk_adt.product<[!frk_mem.arr<!frk_bstr.str>]>, i64) -> !frk_adt.product<[!frk_mem.arr<!frk_bstr.str>, i64]>
  %p3 = "frk_adt.product_snoc"(%p2, %cap) : (!frk_adt.product<[!frk_mem.arr<!frk_bstr.str>, i64]>, i64) -> !frk_adt.product<[!frk_mem.arr<!frk_bstr.str>, i64, i64]>
  %b = "frk_mem.box_new"(%p3) : (!frk_adt.product<[!frk_mem.arr<!frk_bstr.str>, i64, i64]>) -> !frk_mem.box<!frk_adt.product<[!frk_mem.arr<!frk_bstr.str>, i64, i64]>>
  return %b : !frk_mem.box<!frk_adt.product<[!frk_mem.arr<!frk_bstr.str>, i64, i64]>>
}

func.func @__calque_vector_bstr_len(%v: !frk_mem.box<!frk_adt.product<[!frk_mem.arr<!frk_bstr.str>, i64, i64]>>) -> i64 {
  %n = "frk_mem.field_get"(%v) {field = 1 : i64} : (!frk_mem.box<!frk_adt.product<[!frk_mem.arr<!frk_bstr.str>, i64, i64]>>) -> i64
  return %n : i64
}

func.func @__calque_vector_bstr_get(%v: !frk_mem.box<!frk_adt.product<[!frk_mem.arr<!frk_bstr.str>, i64, i64]>>, %i: i64) -> !frk_bstr.str {
  %n = "frk_mem.field_get"(%v) {field = 1 : i64} : (!frk_mem.box<!frk_adt.product<[!frk_mem.arr<!frk_bstr.str>, i64, i64]>>) -> i64
  %ok = arith.cmpi ult, %i, %n : i64
  cf.cond_br %ok, ^hit, ^oob
^oob:
  %msg = "frk_bstr.lit"() {text = "calque: vector.get: index out of bounds"} : () -> !frk_bstr.str
  func.call @frk_rt_calque_trap(%msg) : (!frk_bstr.str) -> ()
  %dummy = "frk_bstr.lit"() {text = ""} : () -> !frk_bstr.str
  return %dummy : !frk_bstr.str
^hit:
  %arr = "frk_mem.field_get"(%v) {field = 0 : i64} : (!frk_mem.box<!frk_adt.product<[!frk_mem.arr<!frk_bstr.str>, i64, i64]>>) -> !frk_mem.arr<!frk_bstr.str>
  %e = "frk_mem.array_get"(%arr, %i) : (!frk_mem.arr<!frk_bstr.str>, i64) -> !frk_bstr.str
  return %e : !frk_bstr.str
}

func.func @__calque_vector_bstr_set(%v: !frk_mem.box<!frk_adt.product<[!frk_mem.arr<!frk_bstr.str>, i64, i64]>>, %i: i64, %x: !frk_bstr.str) {
  %n = "frk_mem.field_get"(%v) {field = 1 : i64} : (!frk_mem.box<!frk_adt.product<[!frk_mem.arr<!frk_bstr.str>, i64, i64]>>) -> i64
  %ok = arith.cmpi ult, %i, %n : i64
  cf.cond_br %ok, ^hit, ^oob
^oob:
  %msg = "frk_bstr.lit"() {text = "calque: vector.set: index out of bounds"} : () -> !frk_bstr.str
  func.call @frk_rt_calque_trap(%msg) : (!frk_bstr.str) -> ()
  return
^hit:
  %arr = "frk_mem.field_get"(%v) {field = 0 : i64} : (!frk_mem.box<!frk_adt.product<[!frk_mem.arr<!frk_bstr.str>, i64, i64]>>) -> !frk_mem.arr<!frk_bstr.str>
  "frk_mem.array_set"(%arr, %i, %x) : (!frk_mem.arr<!frk_bstr.str>, i64, !frk_bstr.str) -> ()
  return
}

func.func @__calque_vector_bstr_push(%v: !frk_mem.box<!frk_adt.product<[!frk_mem.arr<!frk_bstr.str>, i64, i64]>>, %x: !frk_bstr.str) {
  %n = "frk_mem.field_get"(%v) {field = 1 : i64} : (!frk_mem.box<!frk_adt.product<[!frk_mem.arr<!frk_bstr.str>, i64, i64]>>) -> i64
  %cap = "frk_mem.field_get"(%v) {field = 2 : i64} : (!frk_mem.box<!frk_adt.product<[!frk_mem.arr<!frk_bstr.str>, i64, i64]>>) -> i64
  %full = arith.cmpi eq, %n, %cap : i64
  %zero = arith.constant 0 : i64
  %one = arith.constant 1 : i64
  cf.cond_br %full, ^grow, ^store(%n : i64)
^grow:
  %old = "frk_mem.field_get"(%v) {field = 0 : i64} : (!frk_mem.box<!frk_adt.product<[!frk_mem.arr<!frk_bstr.str>, i64, i64]>>) -> !frk_mem.arr<!frk_bstr.str>
  %two = arith.constant 2 : i64
  %ncap = arith.muli %cap, %two : i64
  %new = "frk_mem.array_new"(%ncap) : (i64) -> !frk_mem.arr<!frk_bstr.str>
  cf.br ^copy(%zero : i64)
^copy(%ci: i64):
  %more = arith.cmpi slt, %ci, %n : i64
  cf.cond_br %more, ^copy_body, ^copy_done
^copy_body:
  %e = "frk_mem.array_get"(%old, %ci) : (!frk_mem.arr<!frk_bstr.str>, i64) -> !frk_bstr.str
  "frk_mem.array_set"(%new, %ci, %e) : (!frk_mem.arr<!frk_bstr.str>, i64, !frk_bstr.str) -> ()
  %ci1 = arith.addi %ci, %one : i64
  cf.br ^copy(%ci1 : i64)
^copy_done:
  "frk_mem.field_set"(%v, %new) {field = 0 : i64} : (!frk_mem.box<!frk_adt.product<[!frk_mem.arr<!frk_bstr.str>, i64, i64]>>, !frk_mem.arr<!frk_bstr.str>) -> ()
  "frk_mem.field_set"(%v, %ncap) {field = 2 : i64} : (!frk_mem.box<!frk_adt.product<[!frk_mem.arr<!frk_bstr.str>, i64, i64]>>, i64) -> ()
  cf.br ^store(%n : i64)
^store(%cnt: i64):
  %arr2 = "frk_mem.field_get"(%v) {field = 0 : i64} : (!frk_mem.box<!frk_adt.product<[!frk_mem.arr<!frk_bstr.str>, i64, i64]>>) -> !frk_mem.arr<!frk_bstr.str>
  "frk_mem.array_set"(%arr2, %cnt, %x) : (!frk_mem.arr<!frk_bstr.str>, i64, !frk_bstr.str) -> ()
  %cnt1 = arith.addi %cnt, %one : i64
  "frk_mem.field_set"(%v, %cnt1) {field = 1 : i64} : (!frk_mem.box<!frk_adt.product<[!frk_mem.arr<!frk_bstr.str>, i64, i64]>>, i64) -> ()
  return
}

func.func @__calque_vector_box_new() -> !frk_mem.box<!frk_adt.product<[!frk_mem.arr<!frk_mem.recref>, i64, i64]>> {
  %cap = arith.constant 4 : i64
  %arr = "frk_mem.array_new"(%cap) : (i64) -> !frk_mem.arr<!frk_mem.recref>
  %zero = arith.constant 0 : i64
  %e = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
  %p1 = "frk_adt.product_snoc"(%e, %arr) : (!frk_adt.product<[]>, !frk_mem.arr<!frk_mem.recref>) -> !frk_adt.product<[!frk_mem.arr<!frk_mem.recref>]>
  %p2 = "frk_adt.product_snoc"(%p1, %zero) : (!frk_adt.product<[!frk_mem.arr<!frk_mem.recref>]>, i64) -> !frk_adt.product<[!frk_mem.arr<!frk_mem.recref>, i64]>
  %p3 = "frk_adt.product_snoc"(%p2, %cap) : (!frk_adt.product<[!frk_mem.arr<!frk_mem.recref>, i64]>, i64) -> !frk_adt.product<[!frk_mem.arr<!frk_mem.recref>, i64, i64]>
  %b = "frk_mem.box_new"(%p3) : (!frk_adt.product<[!frk_mem.arr<!frk_mem.recref>, i64, i64]>) -> !frk_mem.box<!frk_adt.product<[!frk_mem.arr<!frk_mem.recref>, i64, i64]>>
  return %b : !frk_mem.box<!frk_adt.product<[!frk_mem.arr<!frk_mem.recref>, i64, i64]>>
}

func.func @__calque_vector_box_len(%v: !frk_mem.box<!frk_adt.product<[!frk_mem.arr<!frk_mem.recref>, i64, i64]>>) -> i64 {
  %n = "frk_mem.field_get"(%v) {field = 1 : i64} : (!frk_mem.box<!frk_adt.product<[!frk_mem.arr<!frk_mem.recref>, i64, i64]>>) -> i64
  return %n : i64
}

func.func @__calque_vector_box_get(%v: !frk_mem.box<!frk_adt.product<[!frk_mem.arr<!frk_mem.recref>, i64, i64]>>, %i: i64) -> !frk_mem.recref {
  %n = "frk_mem.field_get"(%v) {field = 1 : i64} : (!frk_mem.box<!frk_adt.product<[!frk_mem.arr<!frk_mem.recref>, i64, i64]>>) -> i64
  %ok = arith.cmpi ult, %i, %n : i64
  cf.cond_br %ok, ^hit, ^oob
^oob:
  %msg = "frk_bstr.lit"() {text = "calque: vector.get: index out of bounds"} : () -> !frk_bstr.str
  func.call @frk_rt_calque_trap(%msg) : (!frk_bstr.str) -> ()
  %dummy = "frk_mem.recref_null"() : () -> !frk_mem.recref
  return %dummy : !frk_mem.recref
^hit:
  %arr = "frk_mem.field_get"(%v) {field = 0 : i64} : (!frk_mem.box<!frk_adt.product<[!frk_mem.arr<!frk_mem.recref>, i64, i64]>>) -> !frk_mem.arr<!frk_mem.recref>
  %e = "frk_mem.array_get"(%arr, %i) : (!frk_mem.arr<!frk_mem.recref>, i64) -> !frk_mem.recref
  return %e : !frk_mem.recref
}

func.func @__calque_vector_box_set(%v: !frk_mem.box<!frk_adt.product<[!frk_mem.arr<!frk_mem.recref>, i64, i64]>>, %i: i64, %x: !frk_mem.recref) {
  %n = "frk_mem.field_get"(%v) {field = 1 : i64} : (!frk_mem.box<!frk_adt.product<[!frk_mem.arr<!frk_mem.recref>, i64, i64]>>) -> i64
  %ok = arith.cmpi ult, %i, %n : i64
  cf.cond_br %ok, ^hit, ^oob
^oob:
  %msg = "frk_bstr.lit"() {text = "calque: vector.set: index out of bounds"} : () -> !frk_bstr.str
  func.call @frk_rt_calque_trap(%msg) : (!frk_bstr.str) -> ()
  return
^hit:
  %arr = "frk_mem.field_get"(%v) {field = 0 : i64} : (!frk_mem.box<!frk_adt.product<[!frk_mem.arr<!frk_mem.recref>, i64, i64]>>) -> !frk_mem.arr<!frk_mem.recref>
  "frk_mem.array_set"(%arr, %i, %x) : (!frk_mem.arr<!frk_mem.recref>, i64, !frk_mem.recref) -> ()
  return
}

func.func @__calque_vector_box_push(%v: !frk_mem.box<!frk_adt.product<[!frk_mem.arr<!frk_mem.recref>, i64, i64]>>, %x: !frk_mem.recref) {
  %n = "frk_mem.field_get"(%v) {field = 1 : i64} : (!frk_mem.box<!frk_adt.product<[!frk_mem.arr<!frk_mem.recref>, i64, i64]>>) -> i64
  %cap = "frk_mem.field_get"(%v) {field = 2 : i64} : (!frk_mem.box<!frk_adt.product<[!frk_mem.arr<!frk_mem.recref>, i64, i64]>>) -> i64
  %full = arith.cmpi eq, %n, %cap : i64
  %zero = arith.constant 0 : i64
  %one = arith.constant 1 : i64
  cf.cond_br %full, ^grow, ^store(%n : i64)
^grow:
  %old = "frk_mem.field_get"(%v) {field = 0 : i64} : (!frk_mem.box<!frk_adt.product<[!frk_mem.arr<!frk_mem.recref>, i64, i64]>>) -> !frk_mem.arr<!frk_mem.recref>
  %two = arith.constant 2 : i64
  %ncap = arith.muli %cap, %two : i64
  %new = "frk_mem.array_new"(%ncap) : (i64) -> !frk_mem.arr<!frk_mem.recref>
  cf.br ^copy(%zero : i64)
^copy(%ci: i64):
  %more = arith.cmpi slt, %ci, %n : i64
  cf.cond_br %more, ^copy_body, ^copy_done
^copy_body:
  %e = "frk_mem.array_get"(%old, %ci) : (!frk_mem.arr<!frk_mem.recref>, i64) -> !frk_mem.recref
  "frk_mem.array_set"(%new, %ci, %e) : (!frk_mem.arr<!frk_mem.recref>, i64, !frk_mem.recref) -> ()
  %ci1 = arith.addi %ci, %one : i64
  cf.br ^copy(%ci1 : i64)
^copy_done:
  "frk_mem.field_set"(%v, %new) {field = 0 : i64} : (!frk_mem.box<!frk_adt.product<[!frk_mem.arr<!frk_mem.recref>, i64, i64]>>, !frk_mem.arr<!frk_mem.recref>) -> ()
  "frk_mem.field_set"(%v, %ncap) {field = 2 : i64} : (!frk_mem.box<!frk_adt.product<[!frk_mem.arr<!frk_mem.recref>, i64, i64]>>, i64) -> ()
  cf.br ^store(%n : i64)
^store(%cnt: i64):
  %arr2 = "frk_mem.field_get"(%v) {field = 0 : i64} : (!frk_mem.box<!frk_adt.product<[!frk_mem.arr<!frk_mem.recref>, i64, i64]>>) -> !frk_mem.arr<!frk_mem.recref>
  "frk_mem.array_set"(%arr2, %cnt, %x) : (!frk_mem.arr<!frk_mem.recref>, i64, !frk_mem.recref) -> ()
  %cnt1 = arith.addi %cnt, %one : i64
  "frk_mem.field_set"(%v, %cnt1) {field = 1 : i64} : (!frk_mem.box<!frk_adt.product<[!frk_mem.arr<!frk_mem.recref>, i64, i64]>>, i64) -> ()
  return
}
