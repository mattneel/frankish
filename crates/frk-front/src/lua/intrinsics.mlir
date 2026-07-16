// femto_lua intrinsics (M17, D-062; SPEC §6.6) — the Lua protocol
// helpers (D-056.2), authored as kernel IR instead of emitter builder
// code. This file is the SEED MODULE: compilation parses it first and
// the emitter appends the program's functions.
//
// SCOPE (the D-059 sequencing rule): only the CONVENTION-INDEPENDENT
// plain-dyn helpers live here — truthiness, tostring/print, equality,
// coercion, length, pack nil-fill, metatable get/set. The `_v` pack
// wrappers and the iterator protocol are still emitter-built: their
// signatures ride the closure convention that the uniform-signature
// work (D-059's ledgered gap) will rewrite.
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
  func.func @__lua_arg(%arg0: !frk_mem.arr<!frk_dyn.dyn>, %arg1: i64) -> !frk_dyn.dyn {
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
