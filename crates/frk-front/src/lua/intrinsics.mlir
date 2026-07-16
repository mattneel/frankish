// femto_lua intrinsics (M17, D-062; SPEC §6.6) — the Lua protocol
// helpers (D-056.2), authored as kernel IR instead of emitter builder
// code. This file is the SEED MODULE: compilation parses it first and
// the emitter appends the program's functions.
//
// SCOPE (completed at M20, D-065): the ENTIRE lua protocol library —
// the plain-dyn helpers (truthiness, tostring/print, equality,
// coercion, length, pack nil-fill, metatable get/set), the `_v` pack
// wrappers and iterator protocol (signature-stable since D-063's
// uniform convention: (envref, pack) -> pack), the string module
// wrappers, and the metatable index helper. The emitter builds NO
// helper IR — it seeds this module and appends the program.
//
// Runtime declarations here are checked against the frk-abi registry
// by the frankish semantic verifier; the kernel lowering skips
// re-declaring them.

  func.func private @frk_rt_bstr_from_num(f64) -> !frk_bstr.str
  func.func private @frk_rt_print_lua_str(!frk_bstr.str)
  func.func private @frk_rt_lua_error(i64)
  func.func @__lua_truthy(%arg0: !frk_dyn.dyn) -> i1 {
    %0 = "frk_dyn.tag_of"(%arg0) : (!frk_dyn.dyn) -> i64
    cf.switch %0 : i64, [
      default: ^bb2,
      0: ^bb1,
      1: ^bb3
    ]
  ^bb1:  // pred: ^bb0
    %false = arith.constant false
    return %false : i1
  ^bb2:  // pred: ^bb0
    %true = arith.constant true
    return %true : i1
  ^bb3:  // pred: ^bb0
    %1 = "frk_dyn.unwrap"(%arg0) {tag = 1 : i64} : (!frk_dyn.dyn) -> i1
    return %1 : i1
  }
  func.func @__lua_tostring(%arg0: !frk_dyn.dyn) -> !frk_dyn.dyn {
    %0 = "frk_dyn.tag_of"(%arg0) : (!frk_dyn.dyn) -> i64
    cf.switch %0 : i64, [
      default: ^bb5,
      0: ^bb1,
      1: ^bb2,
      2: ^bb3,
      3: ^bb4
    ]
  ^bb1:  // pred: ^bb0
    %1 = "frk_bstr.lit"() {text = "nil"} : () -> !frk_bstr.str
    %2 = "frk_dyn.wrap"(%1) {tag = 3 : i64} : (!frk_bstr.str) -> !frk_dyn.dyn
    return %2 : !frk_dyn.dyn
  ^bb2:  // pred: ^bb0
    %3 = "frk_dyn.unwrap"(%arg0) {tag = 1 : i64} : (!frk_dyn.dyn) -> i1
    cf.cond_br %3, ^bb6, ^bb7
  ^bb3:  // pred: ^bb0
    %4 = "frk_dyn.unwrap"(%arg0) {tag = 2 : i64} : (!frk_dyn.dyn) -> f64
    %5 = call @frk_rt_bstr_from_num(%4) : (f64) -> !frk_bstr.str
    %6 = "frk_dyn.wrap"(%5) {tag = 3 : i64} : (!frk_bstr.str) -> !frk_dyn.dyn
    return %6 : !frk_dyn.dyn
  ^bb4:  // pred: ^bb0
    return %arg0 : !frk_dyn.dyn
  ^bb5:  // pred: ^bb0
    %c1_i64 = arith.constant 1 : i64
    call @frk_rt_lua_error(%c1_i64) : (i64) -> ()
    %c0_i64 = arith.constant 0 : i64
    %7 = "frk_dyn.wrap"(%c0_i64) {tag = 0 : i64} : (i64) -> !frk_dyn.dyn
    return %7 : !frk_dyn.dyn
  ^bb6:  // pred: ^bb2
    %8 = "frk_bstr.lit"() {text = "true"} : () -> !frk_bstr.str
    %9 = "frk_dyn.wrap"(%8) {tag = 3 : i64} : (!frk_bstr.str) -> !frk_dyn.dyn
    return %9 : !frk_dyn.dyn
  ^bb7:  // pred: ^bb2
    %10 = "frk_bstr.lit"() {text = "false"} : () -> !frk_bstr.str
    %11 = "frk_dyn.wrap"(%10) {tag = 3 : i64} : (!frk_bstr.str) -> !frk_dyn.dyn
    return %11 : !frk_dyn.dyn
  }
  func.func @__lua_print(%arg0: !frk_dyn.dyn) -> !frk_dyn.dyn {
    %0 = call @__lua_tostring(%arg0) : (!frk_dyn.dyn) -> !frk_dyn.dyn
    %1 = "frk_dyn.unwrap"(%0) {tag = 3 : i64} : (!frk_dyn.dyn) -> !frk_bstr.str
    call @frk_rt_print_lua_str(%1) : (!frk_bstr.str) -> ()
    %c0_i64 = arith.constant 0 : i64
    %2 = "frk_dyn.wrap"(%c0_i64) {tag = 0 : i64} : (i64) -> !frk_dyn.dyn
    return %2 : !frk_dyn.dyn
  }
  func.func @__lua_eq(%arg0: !frk_dyn.dyn, %arg1: !frk_dyn.dyn) -> i1 {
    %0 = "frk_dyn.tag_of"(%arg0) : (!frk_dyn.dyn) -> i64
    %1 = "frk_dyn.tag_of"(%arg1) : (!frk_dyn.dyn) -> i64
    %2 = arith.cmpi eq, %0, %1 : i64
    cf.cond_br %2, ^bb1, ^bb2
  ^bb1:  // pred: ^bb0
    cf.switch %0 : i64, [
      default: ^bb7,
      0: ^bb3,
      1: ^bb4,
      2: ^bb5,
      3: ^bb6
    ]
  ^bb2:  // pred: ^bb0
    %false = arith.constant false
    return %false : i1
  ^bb3:  // pred: ^bb1
    %true = arith.constant true
    return %true : i1
  ^bb4:  // pred: ^bb1
    %3 = "frk_dyn.unwrap"(%arg0) {tag = 1 : i64} : (!frk_dyn.dyn) -> i1
    %4 = "frk_dyn.unwrap"(%arg1) {tag = 1 : i64} : (!frk_dyn.dyn) -> i1
    %5 = arith.cmpi eq, %3, %4 : i1
    return %5 : i1
  ^bb5:  // pred: ^bb1
    %6 = "frk_dyn.unwrap"(%arg0) {tag = 2 : i64} : (!frk_dyn.dyn) -> f64
    %7 = "frk_dyn.unwrap"(%arg1) {tag = 2 : i64} : (!frk_dyn.dyn) -> f64
    %8 = arith.cmpf oeq, %6, %7 : f64
    return %8 : i1
  ^bb6:  // pred: ^bb1
    %9 = "frk_dyn.unwrap"(%arg0) {tag = 3 : i64} : (!frk_dyn.dyn) -> !frk_bstr.str
    %10 = "frk_dyn.unwrap"(%arg1) {tag = 3 : i64} : (!frk_dyn.dyn) -> !frk_bstr.str
    %11 = "frk_bstr.eq"(%9, %10) : (!frk_bstr.str, !frk_bstr.str) -> i1
    return %11 : i1
  ^bb7:  // pred: ^bb1
    %12 = "frk_dyn.payload_word"(%arg0) : (!frk_dyn.dyn) -> i64
    %13 = "frk_dyn.payload_word"(%arg1) : (!frk_dyn.dyn) -> i64
    %14 = arith.cmpi eq, %12, %13 : i64
    return %14 : i1
  }
  func.func @__lua_costr(%arg0: !frk_dyn.dyn) -> !frk_bstr.str {
    %0 = "frk_dyn.tag_of"(%arg0) : (!frk_dyn.dyn) -> i64
    cf.switch %0 : i64, [
      default: ^bb3,
      3: ^bb1,
      2: ^bb2
    ]
  ^bb1:  // pred: ^bb0
    %1 = "frk_dyn.unwrap"(%arg0) {tag = 3 : i64} : (!frk_dyn.dyn) -> !frk_bstr.str
    return %1 : !frk_bstr.str
  ^bb2:  // pred: ^bb0
    %2 = "frk_dyn.unwrap"(%arg0) {tag = 2 : i64} : (!frk_dyn.dyn) -> f64
    %3 = call @frk_rt_bstr_from_num(%2) : (f64) -> !frk_bstr.str
    return %3 : !frk_bstr.str
  ^bb3:  // pred: ^bb0
    %c2_i64 = arith.constant 2 : i64
    call @frk_rt_lua_error(%c2_i64) : (i64) -> ()
    %4 = "frk_bstr.lit"() {text = ""} : () -> !frk_bstr.str
    return %4 : !frk_bstr.str
  }
  func.func @__lua_len(%arg0: !frk_dyn.dyn) -> !frk_dyn.dyn {
    %0 = "frk_dyn.tag_of"(%arg0) : (!frk_dyn.dyn) -> i64
    cf.switch %0 : i64, [
      default: ^bb3,
      3: ^bb1,
      4: ^bb2
    ]
  ^bb1:  // pred: ^bb0
    %1 = "frk_dyn.unwrap"(%arg0) {tag = 3 : i64} : (!frk_dyn.dyn) -> !frk_bstr.str
    %2 = "frk_bstr.len"(%1) : (!frk_bstr.str) -> i64
    %3 = arith.sitofp %2 : i64 to f64
    %4 = "frk_dyn.wrap"(%3) {tag = 2 : i64} : (f64) -> !frk_dyn.dyn
    return %4 : !frk_dyn.dyn
  ^bb2:  // pred: ^bb0
    %5 = "frk_dyn.table_len"(%arg0) : (!frk_dyn.dyn) -> i64
    %6 = arith.sitofp %5 : i64 to f64
    %7 = "frk_dyn.wrap"(%6) {tag = 2 : i64} : (f64) -> !frk_dyn.dyn
    return %7 : !frk_dyn.dyn
  ^bb3:  // pred: ^bb0
    %c3_i64 = arith.constant 3 : i64
    call @frk_rt_lua_error(%c3_i64) : (i64) -> ()
    %c0_i64 = arith.constant 0 : i64
    %8 = "frk_dyn.wrap"(%c0_i64) {tag = 0 : i64} : (i64) -> !frk_dyn.dyn
    return %8 : !frk_dyn.dyn
  }
  func.func @__lua_arg(%arg0: !frk_mem.arr<!frk_dyn.dyn>, %arg1: i64) -> !frk_dyn.dyn attributes {frk.borrows} {
    %0 = "frk_mem.array_len"(%arg0) : (!frk_mem.arr<!frk_dyn.dyn>) -> i64
    %1 = arith.cmpi slt, %arg1, %0 : i64
    cf.cond_br %1, ^bb1, ^bb2
  ^bb1:  // pred: ^bb0
    %2 = "frk_mem.array_get"(%arg0, %arg1) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
    return %2 : !frk_dyn.dyn
  ^bb2:  // pred: ^bb0
    %c0_i64 = arith.constant 0 : i64
    %3 = "frk_dyn.wrap"(%c0_i64) {tag = 0 : i64} : (i64) -> !frk_dyn.dyn
    return %3 : !frk_dyn.dyn
  }
  func.func @__lua_setmetatable(%arg0: !frk_dyn.dyn, %arg1: !frk_dyn.dyn) -> !frk_dyn.dyn {
    "frk_dyn.set_meta"(%arg0, %arg1) : (!frk_dyn.dyn, !frk_dyn.dyn) -> ()
    return %arg0 : !frk_dyn.dyn
  }
  func.func @__lua_getmetatable(%arg0: !frk_dyn.dyn) -> !frk_dyn.dyn {
    %0 = "frk_dyn.get_meta"(%arg0) : (!frk_dyn.dyn) -> !frk_dyn.dyn
    return %0 : !frk_dyn.dyn
  }

// ---- the pack-convention wrappers and iterator protocol (D-058,
// uniform since D-063; migrated from emitter builder code at M20) ----

  func.func @__lua_print_v(%arg0: !frk_closure.envref, %arg1: !frk_mem.arr<!frk_dyn.dyn>) -> !frk_mem.arr<!frk_dyn.dyn> {
    %c0_i64 = arith.constant 0 : i64
    %0 = call @__lua_arg(%arg1, %c0_i64) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
    %1 = call @__lua_print(%0) : (!frk_dyn.dyn) -> !frk_dyn.dyn
    %c1_i64 = arith.constant 1 : i64
    %2 = "frk_mem.array_new"(%c1_i64) : (i64) -> !frk_mem.arr<!frk_dyn.dyn>
    %c0_i64_0 = arith.constant 0 : i64
    "frk_mem.array_set"(%2, %c0_i64_0, %1) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
    "frk_mem.dispose"(%arg1) : (!frk_mem.arr<!frk_dyn.dyn>) -> ()
    return %2 : !frk_mem.arr<!frk_dyn.dyn>
  }
  func.func @__lua_tostring_v(%arg0: !frk_closure.envref, %arg1: !frk_mem.arr<!frk_dyn.dyn>) -> !frk_mem.arr<!frk_dyn.dyn> {
    %c0_i64 = arith.constant 0 : i64
    %0 = call @__lua_arg(%arg1, %c0_i64) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
    %1 = call @__lua_tostring(%0) : (!frk_dyn.dyn) -> !frk_dyn.dyn
    %c1_i64 = arith.constant 1 : i64
    %2 = "frk_mem.array_new"(%c1_i64) : (i64) -> !frk_mem.arr<!frk_dyn.dyn>
    %c0_i64_0 = arith.constant 0 : i64
    "frk_mem.array_set"(%2, %c0_i64_0, %1) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
    "frk_mem.dispose"(%arg1) : (!frk_mem.arr<!frk_dyn.dyn>) -> ()
    return %2 : !frk_mem.arr<!frk_dyn.dyn>
  }
  func.func @__lua_setmetatable_v(%arg0: !frk_closure.envref, %arg1: !frk_mem.arr<!frk_dyn.dyn>) -> !frk_mem.arr<!frk_dyn.dyn> {
    %c0_i64 = arith.constant 0 : i64
    %0 = call @__lua_arg(%arg1, %c0_i64) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
    %c1_i64 = arith.constant 1 : i64
    %1 = call @__lua_arg(%arg1, %c1_i64) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
    %2 = call @__lua_setmetatable(%0, %1) : (!frk_dyn.dyn, !frk_dyn.dyn) -> !frk_dyn.dyn
    %c1_i64_0 = arith.constant 1 : i64
    %3 = "frk_mem.array_new"(%c1_i64_0) : (i64) -> !frk_mem.arr<!frk_dyn.dyn>
    %c0_i64_1 = arith.constant 0 : i64
    "frk_mem.array_set"(%3, %c0_i64_1, %2) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
    "frk_mem.dispose"(%arg1) : (!frk_mem.arr<!frk_dyn.dyn>) -> ()
    return %3 : !frk_mem.arr<!frk_dyn.dyn>
  }
  func.func @__lua_getmetatable_v(%arg0: !frk_closure.envref, %arg1: !frk_mem.arr<!frk_dyn.dyn>) -> !frk_mem.arr<!frk_dyn.dyn> {
    %c0_i64 = arith.constant 0 : i64
    %0 = call @__lua_arg(%arg1, %c0_i64) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
    %1 = call @__lua_getmetatable(%0) : (!frk_dyn.dyn) -> !frk_dyn.dyn
    %c1_i64 = arith.constant 1 : i64
    %2 = "frk_mem.array_new"(%c1_i64) : (i64) -> !frk_mem.arr<!frk_dyn.dyn>
    %c0_i64_0 = arith.constant 0 : i64
    "frk_mem.array_set"(%2, %c0_i64_0, %1) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
    "frk_mem.dispose"(%arg1) : (!frk_mem.arr<!frk_dyn.dyn>) -> ()
    return %2 : !frk_mem.arr<!frk_dyn.dyn>
  }
  func.func @__lua_next_v(%arg0: !frk_closure.envref, %arg1: !frk_mem.arr<!frk_dyn.dyn>) -> !frk_mem.arr<!frk_dyn.dyn> {
    %c0_i64 = arith.constant 0 : i64
    %0 = call @__lua_arg(%arg1, %c0_i64) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
    %c1_i64 = arith.constant 1 : i64
    %1 = call @__lua_arg(%arg1, %c1_i64) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
    %2 = "frk_dyn.tag_of"(%0) : (!frk_dyn.dyn) -> i64
    %c4_i64 = arith.constant 4 : i64
    %3 = arith.cmpi eq, %2, %c4_i64 : i64
    cf.cond_br %3, ^bb1, ^bb2
  ^bb1:  // pred: ^bb0
    %4:2 = "frk_dyn.table_next"(%0, %1) : (!frk_dyn.dyn, !frk_dyn.dyn) -> (!frk_dyn.dyn, !frk_dyn.dyn)
    %c2_i64 = arith.constant 2 : i64
    %5 = "frk_mem.array_new"(%c2_i64) : (i64) -> !frk_mem.arr<!frk_dyn.dyn>
    %c0_i64_0 = arith.constant 0 : i64
    "frk_mem.array_set"(%5, %c0_i64_0, %4#0) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
    %c1_i64_1 = arith.constant 1 : i64
    "frk_mem.array_set"(%5, %c1_i64_1, %4#1) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
    "frk_mem.dispose"(%arg1) : (!frk_mem.arr<!frk_dyn.dyn>) -> ()
    return %5 : !frk_mem.arr<!frk_dyn.dyn>
  ^bb2:  // pred: ^bb0
    %c5_i64 = arith.constant 5 : i64
    call @frk_rt_lua_error(%c5_i64) : (i64) -> ()
    %c0_i64_2 = arith.constant 0 : i64
    %6 = "frk_dyn.wrap"(%c0_i64_2) {tag = 0 : i64} : (i64) -> !frk_dyn.dyn
    %c1_i64_3 = arith.constant 1 : i64
    %7 = "frk_mem.array_new"(%c1_i64_3) : (i64) -> !frk_mem.arr<!frk_dyn.dyn>
    %c0_i64_4 = arith.constant 0 : i64
    "frk_mem.array_set"(%7, %c0_i64_4, %6) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
    "frk_mem.dispose"(%arg1) : (!frk_mem.arr<!frk_dyn.dyn>) -> ()
    return %7 : !frk_mem.arr<!frk_dyn.dyn>
  }
  func.func @__lua_pairs_v(%arg0: !frk_closure.envref, %arg1: !frk_mem.arr<!frk_dyn.dyn>) -> !frk_mem.arr<!frk_dyn.dyn> {
    %c0_i64 = arith.constant 0 : i64
    %0 = call @__lua_arg(%arg1, %c0_i64) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
    %1 = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
    %2 = "frk_closure.make"(%1) {callee = @__lua_next_v} : (!frk_adt.product<[]>) -> !frk_closure.fn<[!frk_mem.arr<!frk_dyn.dyn>], [!frk_mem.arr<!frk_dyn.dyn>]>
    %3 = "frk_dyn.wrap"(%2) {tag = 5 : i64} : (!frk_closure.fn<[!frk_mem.arr<!frk_dyn.dyn>], [!frk_mem.arr<!frk_dyn.dyn>]>) -> !frk_dyn.dyn
    %c0_i64_0 = arith.constant 0 : i64
    %4 = "frk_dyn.wrap"(%c0_i64_0) {tag = 0 : i64} : (i64) -> !frk_dyn.dyn
    %c3_i64 = arith.constant 3 : i64
    %5 = "frk_mem.array_new"(%c3_i64) : (i64) -> !frk_mem.arr<!frk_dyn.dyn>
    %c0_i64_1 = arith.constant 0 : i64
    "frk_mem.array_set"(%5, %c0_i64_1, %3) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
    %c1_i64 = arith.constant 1 : i64
    "frk_mem.array_set"(%5, %c1_i64, %0) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
    %c2_i64 = arith.constant 2 : i64
    "frk_mem.array_set"(%5, %c2_i64, %4) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
    "frk_mem.dispose"(%arg1) : (!frk_mem.arr<!frk_dyn.dyn>) -> ()
    return %5 : !frk_mem.arr<!frk_dyn.dyn>
  }
  func.func @__lua_ipairs_iter_v(%arg0: !frk_closure.envref, %arg1: !frk_mem.arr<!frk_dyn.dyn>) -> !frk_mem.arr<!frk_dyn.dyn> {
    %c0_i64 = arith.constant 0 : i64
    %0 = call @__lua_arg(%arg1, %c0_i64) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
    %c1_i64 = arith.constant 1 : i64
    %1 = call @__lua_arg(%arg1, %c1_i64) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
    %2 = "frk_dyn.unwrap"(%1) {tag = 2 : i64} : (!frk_dyn.dyn) -> f64
    %cst = arith.constant 1.000000e+00 : f64
    %3 = arith.addf %2, %cst : f64
    %4 = "frk_dyn.wrap"(%3) {tag = 2 : i64} : (f64) -> !frk_dyn.dyn
    %5 = "frk_dyn.raw_get"(%0, %4) : (!frk_dyn.dyn, !frk_dyn.dyn) -> !frk_dyn.dyn
    %6 = "frk_dyn.tag_of"(%5) : (!frk_dyn.dyn) -> i64
    %c0_i64_0 = arith.constant 0 : i64
    %7 = arith.cmpi eq, %6, %c0_i64_0 : i64
    cf.cond_br %7, ^bb1, ^bb2
  ^bb1:  // pred: ^bb0
    %c0_i64_1 = arith.constant 0 : i64
    %8 = "frk_dyn.wrap"(%c0_i64_1) {tag = 0 : i64} : (i64) -> !frk_dyn.dyn
    %c1_i64_2 = arith.constant 1 : i64
    %9 = "frk_mem.array_new"(%c1_i64_2) : (i64) -> !frk_mem.arr<!frk_dyn.dyn>
    %c0_i64_3 = arith.constant 0 : i64
    "frk_mem.array_set"(%9, %c0_i64_3, %8) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
    "frk_mem.dispose"(%arg1) : (!frk_mem.arr<!frk_dyn.dyn>) -> ()
    return %9 : !frk_mem.arr<!frk_dyn.dyn>
  ^bb2:  // pred: ^bb0
    %c2_i64 = arith.constant 2 : i64
    %10 = "frk_mem.array_new"(%c2_i64) : (i64) -> !frk_mem.arr<!frk_dyn.dyn>
    %c0_i64_4 = arith.constant 0 : i64
    "frk_mem.array_set"(%10, %c0_i64_4, %4) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
    %c1_i64_5 = arith.constant 1 : i64
    "frk_mem.array_set"(%10, %c1_i64_5, %5) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
    "frk_mem.dispose"(%arg1) : (!frk_mem.arr<!frk_dyn.dyn>) -> ()
    return %10 : !frk_mem.arr<!frk_dyn.dyn>
  }
  func.func @__lua_ipairs_v(%arg0: !frk_closure.envref, %arg1: !frk_mem.arr<!frk_dyn.dyn>) -> !frk_mem.arr<!frk_dyn.dyn> {
    %c0_i64 = arith.constant 0 : i64
    %0 = call @__lua_arg(%arg1, %c0_i64) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
    %1 = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
    %2 = "frk_closure.make"(%1) {callee = @__lua_ipairs_iter_v} : (!frk_adt.product<[]>) -> !frk_closure.fn<[!frk_mem.arr<!frk_dyn.dyn>], [!frk_mem.arr<!frk_dyn.dyn>]>
    %3 = "frk_dyn.wrap"(%2) {tag = 5 : i64} : (!frk_closure.fn<[!frk_mem.arr<!frk_dyn.dyn>], [!frk_mem.arr<!frk_dyn.dyn>]>) -> !frk_dyn.dyn
    %cst = arith.constant 0.000000e+00 : f64
    %4 = "frk_dyn.wrap"(%cst) {tag = 2 : i64} : (f64) -> !frk_dyn.dyn
    %c3_i64 = arith.constant 3 : i64
    %5 = "frk_mem.array_new"(%c3_i64) : (i64) -> !frk_mem.arr<!frk_dyn.dyn>
    %c0_i64_0 = arith.constant 0 : i64
    "frk_mem.array_set"(%5, %c0_i64_0, %3) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
    %c1_i64 = arith.constant 1 : i64
    "frk_mem.array_set"(%5, %c1_i64, %0) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
    %c2_i64 = arith.constant 2 : i64
    "frk_mem.array_set"(%5, %c2_i64, %4) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
    "frk_mem.dispose"(%arg1) : (!frk_mem.arr<!frk_dyn.dyn>) -> ()
    return %5 : !frk_mem.arr<!frk_dyn.dyn>
  }
  func.func @__lua_string_sub_v(%arg0: !frk_closure.envref, %arg1: !frk_mem.arr<!frk_dyn.dyn>) -> !frk_mem.arr<!frk_dyn.dyn> {
    %c0_i64 = arith.constant 0 : i64
    %0 = call @__lua_arg(%arg1, %c0_i64) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
    %1 = "frk_dyn.unwrap"(%0) {tag = 3 : i64} : (!frk_dyn.dyn) -> !frk_bstr.str
    %c1_i64 = arith.constant 1 : i64
    %2 = call @__lua_arg(%arg1, %c1_i64) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
    %3 = "frk_dyn.unwrap"(%2) {tag = 2 : i64} : (!frk_dyn.dyn) -> f64
    %4 = arith.fptosi %3 : f64 to i64
    %c2_i64 = arith.constant 2 : i64
    %5 = call @__lua_arg(%arg1, %c2_i64) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
    %6 = "frk_dyn.tag_of"(%5) : (!frk_dyn.dyn) -> i64
    %c0_i64_0 = arith.constant 0 : i64
    %7 = arith.cmpi eq, %6, %c0_i64_0 : i64
    cf.cond_br %7, ^bb1, ^bb2
  ^bb1:  // pred: ^bb0
    %c-1_i64 = arith.constant -1 : i64
    cf.br ^bb3(%c-1_i64 : i64)
  ^bb2:  // pred: ^bb0
    %8 = "frk_dyn.unwrap"(%5) {tag = 2 : i64} : (!frk_dyn.dyn) -> f64
    %9 = arith.fptosi %8 : f64 to i64
    cf.br ^bb3(%9 : i64)
  ^bb3(%10: i64):  // 2 preds: ^bb1, ^bb2
    %11 = "frk_bstr.sub"(%1, %4, %10) : (!frk_bstr.str, i64, i64) -> !frk_bstr.str
    %12 = "frk_dyn.wrap"(%11) {tag = 3 : i64} : (!frk_bstr.str) -> !frk_dyn.dyn
    %c1_i64_1 = arith.constant 1 : i64
    %13 = "frk_mem.array_new"(%c1_i64_1) : (i64) -> !frk_mem.arr<!frk_dyn.dyn>
    %c0_i64_2 = arith.constant 0 : i64
    "frk_mem.array_set"(%13, %c0_i64_2, %12) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
    "frk_mem.dispose"(%arg1) : (!frk_mem.arr<!frk_dyn.dyn>) -> ()
    return %13 : !frk_mem.arr<!frk_dyn.dyn>
  }
  func.func @__lua_string_rep_v(%arg0: !frk_closure.envref, %arg1: !frk_mem.arr<!frk_dyn.dyn>) -> !frk_mem.arr<!frk_dyn.dyn> {
    %c0_i64 = arith.constant 0 : i64
    %0 = call @__lua_arg(%arg1, %c0_i64) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
    %1 = "frk_dyn.unwrap"(%0) {tag = 3 : i64} : (!frk_dyn.dyn) -> !frk_bstr.str
    %c1_i64 = arith.constant 1 : i64
    %2 = call @__lua_arg(%arg1, %c1_i64) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
    %3 = "frk_dyn.unwrap"(%2) {tag = 2 : i64} : (!frk_dyn.dyn) -> f64
    %4 = arith.fptosi %3 : f64 to i64
    %5 = "frk_bstr.rep"(%1, %4) : (!frk_bstr.str, i64) -> !frk_bstr.str
    %6 = "frk_dyn.wrap"(%5) {tag = 3 : i64} : (!frk_bstr.str) -> !frk_dyn.dyn
    %c1_i64_0 = arith.constant 1 : i64
    %7 = "frk_mem.array_new"(%c1_i64_0) : (i64) -> !frk_mem.arr<!frk_dyn.dyn>
    %c0_i64_1 = arith.constant 0 : i64
    "frk_mem.array_set"(%7, %c0_i64_1, %6) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
    "frk_mem.dispose"(%arg1) : (!frk_mem.arr<!frk_dyn.dyn>) -> ()
    return %7 : !frk_mem.arr<!frk_dyn.dyn>
  }
  func.func @__lua_index(%arg0: !frk_dyn.dyn, %arg1: !frk_dyn.dyn) -> !frk_dyn.dyn {
    %0 = "frk_dyn.tag_of"(%arg0) : (!frk_dyn.dyn) -> i64
    %c4_i64 = arith.constant 4 : i64
    %1 = arith.cmpi eq, %0, %c4_i64 : i64
    cf.cond_br %1, ^bb1, ^bb2
  ^bb1:  // pred: ^bb0
    %2 = "frk_dyn.raw_get"(%arg0, %arg1) : (!frk_dyn.dyn, !frk_dyn.dyn) -> !frk_dyn.dyn
    %3 = "frk_dyn.tag_of"(%2) : (!frk_dyn.dyn) -> i64
    %c0_i64 = arith.constant 0 : i64
    %4 = arith.cmpi eq, %3, %c0_i64 : i64
    cf.cond_br %4, ^bb3, ^bb4
  ^bb2:  // pred: ^bb0
    %c5_i64 = arith.constant 5 : i64
    call @frk_rt_lua_error(%c5_i64) : (i64) -> ()
    %c0_i64_0 = arith.constant 0 : i64
    %5 = "frk_dyn.wrap"(%c0_i64_0) {tag = 0 : i64} : (i64) -> !frk_dyn.dyn
    return %5 : !frk_dyn.dyn
  ^bb3:  // pred: ^bb1
    %6 = "frk_dyn.get_meta"(%arg0) : (!frk_dyn.dyn) -> !frk_dyn.dyn
    %7 = "frk_dyn.tag_of"(%6) : (!frk_dyn.dyn) -> i64
    %c0_i64_1 = arith.constant 0 : i64
    %8 = arith.cmpi eq, %7, %c0_i64_1 : i64
    cf.cond_br %8, ^bb5, ^bb6
  ^bb4:  // pred: ^bb1
    return %2 : !frk_dyn.dyn
  ^bb5:  // pred: ^bb3
    return %2 : !frk_dyn.dyn
  ^bb6:  // pred: ^bb3
    %9 = "frk_bstr.lit"() {text = "__index"} : () -> !frk_bstr.str
    %10 = "frk_dyn.wrap"(%9) {tag = 3 : i64} : (!frk_bstr.str) -> !frk_dyn.dyn
    %11 = "frk_dyn.raw_get"(%6, %10) : (!frk_dyn.dyn, !frk_dyn.dyn) -> !frk_dyn.dyn
    %12 = "frk_dyn.tag_of"(%11) : (!frk_dyn.dyn) -> i64
    cf.switch %12 : i64, [
      default: ^bb10,
      0: ^bb7,
      4: ^bb8,
      5: ^bb9
    ]
  ^bb7:  // pred: ^bb6
    %c0_i64_2 = arith.constant 0 : i64
    %13 = "frk_dyn.wrap"(%c0_i64_2) {tag = 0 : i64} : (i64) -> !frk_dyn.dyn
    return %13 : !frk_dyn.dyn
  ^bb8:  // pred: ^bb6
    %14 = call @__lua_index(%11, %arg1) : (!frk_dyn.dyn, !frk_dyn.dyn) -> !frk_dyn.dyn
    return %14 : !frk_dyn.dyn
  ^bb9:  // pred: ^bb6
    %15 = "frk_dyn.unwrap"(%11) {tag = 5 : i64} : (!frk_dyn.dyn) -> !frk_closure.fn<[!frk_mem.arr<!frk_dyn.dyn>], [!frk_mem.arr<!frk_dyn.dyn>]>
    %c2_i64 = arith.constant 2 : i64
    %16 = "frk_mem.array_new"(%c2_i64) : (i64) -> !frk_mem.arr<!frk_dyn.dyn>
    %c0_i64_3 = arith.constant 0 : i64
    "frk_mem.array_set"(%16, %c0_i64_3, %arg0) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
    %c1_i64 = arith.constant 1 : i64
    "frk_mem.array_set"(%16, %c1_i64, %arg1) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
    %17 = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
    %18 = "frk_adt.product_snoc"(%17, %16) : (!frk_adt.product<[]>, !frk_mem.arr<!frk_dyn.dyn>) -> !frk_adt.product<[!frk_mem.arr<!frk_dyn.dyn>]>
    %19 = "frk_closure.apply"(%15, %18) : (!frk_closure.fn<[!frk_mem.arr<!frk_dyn.dyn>], [!frk_mem.arr<!frk_dyn.dyn>]>, !frk_adt.product<[!frk_mem.arr<!frk_dyn.dyn>]>) -> !frk_mem.arr<!frk_dyn.dyn>
    %c0_i64_4 = arith.constant 0 : i64
    %20 = call @__lua_arg(%19, %c0_i64_4) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
    return %20 : !frk_dyn.dyn
  ^bb10:  // pred: ^bb6
    %c5_i64_5 = arith.constant 5 : i64
    call @frk_rt_lua_error(%c5_i64_5) : (i64) -> ()
    %c0_i64_6 = arith.constant 0 : i64
    %21 = "frk_dyn.wrap"(%c0_i64_6) {tag = 0 : i64} : (i64) -> !frk_dyn.dyn
    return %21 : !frk_dyn.dyn
  }
