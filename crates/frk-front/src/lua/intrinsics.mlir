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
  func.func private @frk_rt_coro_trap(i64)
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

  // print(...) — MULTI-VALUE since v0.3 (D-068): every argument
  // tostring'd, tab-joined, one trailing newline (lua5.1 semantics).
  func.func @__lua_print_v(%arg0: !frk_closure.envref, %arg1: !frk_mem.arr<!frk_dyn.dyn>) -> !frk_mem.arr<!frk_dyn.dyn> {
    %n = "frk_mem.array_len"(%arg1) : (!frk_mem.arr<!frk_dyn.dyn>) -> i64
    %pv_c0 = arith.constant 0 : i64
    %pv_c1 = arith.constant 1 : i64
    %pv_empty = arith.cmpi eq, %n, %pv_c0 : i64
    cf.cond_br %pv_empty, ^pv_blank, ^pv_first
  ^pv_blank:
    %pv_nothing = "frk_bstr.lit"() {text = ""} : () -> !frk_bstr.str
    call @frk_rt_print_lua_str(%pv_nothing) : (!frk_bstr.str) -> ()
    cf.br ^pv_done
  ^pv_first:
    %pv_v0 = "frk_mem.array_get"(%arg1, %pv_c0) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
    %pv_d0 = call @__lua_tostring(%pv_v0) : (!frk_dyn.dyn) -> !frk_dyn.dyn
    %pv_s0 = "frk_dyn.unwrap"(%pv_d0) {tag = 3 : i64} : (!frk_dyn.dyn) -> !frk_bstr.str
    cf.br ^pv_head(%pv_c1, %pv_s0 : i64, !frk_bstr.str)
  ^pv_head(%pv_i: i64, %pv_acc: !frk_bstr.str):
    %pv_more = arith.cmpi slt, %pv_i, %n : i64
    cf.cond_br %pv_more, ^pv_body(%pv_i, %pv_acc : i64, !frk_bstr.str), ^pv_flush(%pv_acc : !frk_bstr.str)
  ^pv_body(%pv_j: i64, %pv_a: !frk_bstr.str):
    %pv_tab = "frk_bstr.lit"() {text = "\09"} : () -> !frk_bstr.str
    %pv_a1 = "frk_bstr.concat"(%pv_a, %pv_tab) : (!frk_bstr.str, !frk_bstr.str) -> !frk_bstr.str
    %pv_vj = "frk_mem.array_get"(%arg1, %pv_j) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
    %pv_dj = call @__lua_tostring(%pv_vj) : (!frk_dyn.dyn) -> !frk_dyn.dyn
    %pv_sj = "frk_dyn.unwrap"(%pv_dj) {tag = 3 : i64} : (!frk_dyn.dyn) -> !frk_bstr.str
    %pv_a2 = "frk_bstr.concat"(%pv_a1, %pv_sj) : (!frk_bstr.str, !frk_bstr.str) -> !frk_bstr.str
    %pv_j1 = arith.addi %pv_j, %pv_c1 : i64
    cf.br ^pv_head(%pv_j1, %pv_a2 : i64, !frk_bstr.str)
  ^pv_flush(%pv_line: !frk_bstr.str):
    call @frk_rt_print_lua_str(%pv_line) : (!frk_bstr.str) -> ()
    cf.br ^pv_done
  ^pv_done:
    %pv_ret = "frk_mem.array_new"(%pv_c1) : (i64) -> !frk_mem.arr<!frk_dyn.dyn>
    %pv_nil_w = arith.constant 0 : i64
    %pv_nil = "frk_dyn.wrap"(%pv_nil_w) {tag = 0 : i64} : (i64) -> !frk_dyn.dyn
    "frk_mem.array_set"(%pv_ret, %pv_c0, %pv_nil) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
    "frk_mem.dispose"(%arg1) : (!frk_mem.arr<!frk_dyn.dyn>) -> ()
    return %pv_ret : !frk_mem.arr<!frk_dyn.dyn>
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
    // Exhaustion returns ONE nil, not (nil, nil) — the pack length is
    // observable under D-068's explist expansion (lua5.1 semantics).
    %nv_ktag = "frk_dyn.tag_of"(%4#0) : (!frk_dyn.dyn) -> i64
    %nv_c0 = arith.constant 0 : i64
    %nv_done = arith.cmpi eq, %nv_ktag, %nv_c0 : i64
    cf.cond_br %nv_done, ^nv_end, ^nv_pair
  ^nv_end:
    %c1_i64_e = arith.constant 1 : i64
    %nv_one = "frk_mem.array_new"(%c1_i64_e) : (i64) -> !frk_mem.arr<!frk_dyn.dyn>
    %nv_c0b = arith.constant 0 : i64
    "frk_mem.array_set"(%nv_one, %nv_c0b, %4#0) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
    "frk_mem.dispose"(%arg1) : (!frk_mem.arr<!frk_dyn.dyn>) -> ()
    return %nv_one : !frk_mem.arr<!frk_dyn.dyn>
  ^nv_pair:
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

  // ---- v0.3 (D-068): varargs plumbing + the settable protocol ----

  // pack[start..] as a FRESH arr (the vararg prologue copy — before
  // the D-067 dispose). Borrows its source; owns its result.
  func.func @__lua_pack_tail(%src: !frk_mem.arr<!frk_dyn.dyn>, %start: i64) -> !frk_mem.arr<!frk_dyn.dyn> attributes {frk.borrows} {
    %pt_len = "frk_mem.array_len"(%src) : (!frk_mem.arr<!frk_dyn.dyn>) -> i64
    %pt_diff = arith.subi %pt_len, %start : i64
    %pt_c0 = arith.constant 0 : i64
    %pt_c1 = arith.constant 1 : i64
    %pt_n = arith.maxsi %pt_diff, %pt_c0 : i64
    %pt_dst = "frk_mem.array_new"(%pt_n) : (i64) -> !frk_mem.arr<!frk_dyn.dyn>
    cf.br ^pt_head(%pt_c0 : i64)
  ^pt_head(%pt_i: i64):
    %pt_more = arith.cmpi slt, %pt_i, %pt_n : i64
    cf.cond_br %pt_more, ^pt_body(%pt_i : i64), ^pt_exit
  ^pt_body(%pt_j: i64):
    %pt_k = arith.addi %pt_j, %start : i64
    %pt_v = "frk_mem.array_get"(%src, %pt_k) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
    "frk_mem.array_set"(%pt_dst, %pt_j, %pt_v) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
    %pt_j1 = arith.addi %pt_j, %pt_c1 : i64
    cf.br ^pt_head(%pt_j1 : i64)
  ^pt_exit:
    return %pt_dst : !frk_mem.arr<!frk_dyn.dyn>
  }

  // Copies every element of src into dst starting at dst[at] (the
  // explist-engine tail append). The rc retain discipline is the
  // kernel lowering's array_set rule — nothing hand-written here.
  func.func @__lua_pack_copy_into(%dst: !frk_mem.arr<!frk_dyn.dyn>, %at: i64, %src: !frk_mem.arr<!frk_dyn.dyn>) attributes {frk.borrows} {
    %pc_len = "frk_mem.array_len"(%src) : (!frk_mem.arr<!frk_dyn.dyn>) -> i64
    %pc_c0 = arith.constant 0 : i64
    %pc_c1 = arith.constant 1 : i64
    cf.br ^pc_head(%pc_c0 : i64)
  ^pc_head(%pc_i: i64):
    %pc_more = arith.cmpi slt, %pc_i, %pc_len : i64
    cf.cond_br %pc_more, ^pc_body(%pc_i : i64), ^pc_exit
  ^pc_body(%pc_j: i64):
    %pc_v = "frk_mem.array_get"(%src, %pc_j) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
    %pc_k = arith.addi %at, %pc_j : i64
    "frk_mem.array_set"(%dst, %pc_k, %pc_v) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
    %pc_j1 = arith.addi %pc_j, %pc_c1 : i64
    cf.br ^pc_head(%pc_j1 : i64)
  ^pc_exit:
    return
  }

  // Table-constructor tail expansion (D-068): appends src[j] at
  // number keys first+j — `{ a, b, f(...) }` array semantics.
  func.func @__lua_ctor_append(%t: !frk_dyn.dyn, %first: f64, %src: !frk_mem.arr<!frk_dyn.dyn>) attributes {frk.borrows} {
    %ca_len = "frk_mem.array_len"(%src) : (!frk_mem.arr<!frk_dyn.dyn>) -> i64
    %ca_c0 = arith.constant 0 : i64
    %ca_c1 = arith.constant 1 : i64
    cf.br ^ca_head(%ca_c0 : i64)
  ^ca_head(%ca_i: i64):
    %ca_more = arith.cmpi slt, %ca_i, %ca_len : i64
    cf.cond_br %ca_more, ^ca_body(%ca_i : i64), ^ca_exit
  ^ca_body(%ca_j: i64):
    %ca_v = "frk_mem.array_get"(%src, %ca_j) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
    %ca_jf = arith.sitofp %ca_j : i64 to f64
    %ca_kf = arith.addf %first, %ca_jf : f64
    %ca_key = "frk_dyn.wrap"(%ca_kf) {tag = 2 : i64} : (f64) -> !frk_dyn.dyn
    "frk_dyn.raw_set"(%t, %ca_key, %ca_v) : (!frk_dyn.dyn, !frk_dyn.dyn, !frk_dyn.dyn) -> ()
    %ca_j1 = arith.addi %ca_j, %ca_c1 : i64
    cf.br ^ca_head(%ca_j1 : i64)
  ^ca_exit:
    return
  }

  // luaV_settable (D-068): an EXISTING key raw-assigns without
  // consulting metamethods; an absent key walks __newindex — nil
  // handler raw-assigns, table handler RE-ENTERS settable on the
  // target (a tail call: metatable chains ride the trampoline and
  // musttail like __lua_index's), function handler is called (t,k,v)
  // through the uniform convention.
  func.func @__lua_setindex(%t: !frk_dyn.dyn, %k: !frk_dyn.dyn, %v: !frk_dyn.dyn) {
    %si_tag = "frk_dyn.tag_of"(%t) : (!frk_dyn.dyn) -> i64
    %si_c4 = arith.constant 4 : i64
    %si_is_table = arith.cmpi eq, %si_tag, %si_c4 : i64
    cf.cond_br %si_is_table, ^si_check, ^si_err
  ^si_err:
    %si_code = arith.constant 5 : i64
    call @frk_rt_lua_error(%si_code) : (i64) -> ()
    return
  ^si_check:
    %si_existing = "frk_dyn.raw_get"(%t, %k) : (!frk_dyn.dyn, !frk_dyn.dyn) -> !frk_dyn.dyn
    %si_etag = "frk_dyn.tag_of"(%si_existing) : (!frk_dyn.dyn) -> i64
    %si_c0 = arith.constant 0 : i64
    %si_absent = arith.cmpi eq, %si_etag, %si_c0 : i64
    cf.cond_br %si_absent, ^si_meta, ^si_raw
  ^si_raw:
    "frk_dyn.raw_set"(%t, %k, %v) : (!frk_dyn.dyn, !frk_dyn.dyn, !frk_dyn.dyn) -> ()
    return
  ^si_meta:
    %si_m = "frk_dyn.get_meta"(%t) : (!frk_dyn.dyn) -> !frk_dyn.dyn
    %si_mtag = "frk_dyn.tag_of"(%si_m) : (!frk_dyn.dyn) -> i64
    %si_no_meta = arith.cmpi eq, %si_mtag, %si_c0 : i64
    cf.cond_br %si_no_meta, ^si_raw, ^si_handler
  ^si_handler:
    %si_key_lit = "frk_bstr.lit"() {text = "__newindex"} : () -> !frk_bstr.str
    %si_key = "frk_dyn.wrap"(%si_key_lit) {tag = 3 : i64} : (!frk_bstr.str) -> !frk_dyn.dyn
    %si_h = "frk_dyn.raw_get"(%si_m, %si_key) : (!frk_dyn.dyn, !frk_dyn.dyn) -> !frk_dyn.dyn
    %si_htag = "frk_dyn.tag_of"(%si_h) : (!frk_dyn.dyn) -> i64
    cf.switch %si_htag : i64, [
      default: ^si_err,
      0: ^si_raw,
      4: ^si_redirect,
      5: ^si_call
    ]
  ^si_redirect:
    call @__lua_setindex(%si_h, %k, %v) : (!frk_dyn.dyn, !frk_dyn.dyn, !frk_dyn.dyn) -> ()
    return
  ^si_call:
    %si_f = "frk_dyn.unwrap"(%si_h) {tag = 5 : i64} : (!frk_dyn.dyn) -> !frk_closure.fn<[!frk_mem.arr<!frk_dyn.dyn>], [!frk_mem.arr<!frk_dyn.dyn>]>
    %si_c3 = arith.constant 3 : i64
    %si_pack = "frk_mem.array_new"(%si_c3) : (i64) -> !frk_mem.arr<!frk_dyn.dyn>
    "frk_mem.array_set"(%si_pack, %si_c0, %t) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
    %si_c1 = arith.constant 1 : i64
    "frk_mem.array_set"(%si_pack, %si_c1, %k) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
    %si_c2 = arith.constant 2 : i64
    "frk_mem.array_set"(%si_pack, %si_c2, %v) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
    %si_p0 = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
    %si_p1 = "frk_adt.product_snoc"(%si_p0, %si_pack) : (!frk_adt.product<[]>, !frk_mem.arr<!frk_dyn.dyn>) -> !frk_adt.product<[!frk_mem.arr<!frk_dyn.dyn>]>
    %si_r = "frk_closure.apply"(%si_f, %si_p1) : (!frk_closure.fn<[!frk_mem.arr<!frk_dyn.dyn>], [!frk_mem.arr<!frk_dyn.dyn>]>, !frk_adt.product<[!frk_mem.arr<!frk_dyn.dyn>]>) -> !frk_mem.arr<!frk_dyn.dyn>
    return
  }

  // ---- M35 (D-084): coroutines — the resumable-frame pattern's
  // substrate. The suspend channel is PARALLEL to the ctl pending
  // cell (D-084.2): lua_susp (f64 flag) + lua_coro (arr<dyn>:
  // [0] chain head frame (fun dyn or nil), [1] parked yield pack
  // (tag-7 wrapped), [2] current thread (tag-8 or nil)). The chain
  // IS the env linkage: each frame closure's env carries its own
  // chain-next; the cell holds only the outermost frame. A thread
  // record (TAG_THREAD = 8) = arr<dyn> [status num (0 suspended /
  // 1 running / 2 normal / 3 dead), started bool, chain dyn,
  // resumer dyn, body fun, stash nil].
  "frk_mem.global_decl"() {sym = "lua_susp", cell = f64} : () -> ()
  "frk_mem.global_decl"() {sym = "lua_coro", cell = !frk_mem.arr<!frk_dyn.dyn>} : () -> ()

  func.func @__lua_coro_init() {
    %ci_n = arith.constant 3 : i64
    %ci_arr = "frk_mem.array_new"(%ci_n) : (i64) -> !frk_mem.arr<!frk_dyn.dyn>
    %ci_z = arith.constant 0 : i64
    %ci_nil = "frk_dyn.wrap"(%ci_z) {tag = 0 : i64} : (i64) -> !frk_dyn.dyn
    %ci_i0 = arith.constant 0 : i64
    "frk_mem.array_set"(%ci_arr, %ci_i0, %ci_nil) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
    %ci_i1 = arith.constant 1 : i64
    "frk_mem.array_set"(%ci_arr, %ci_i1, %ci_nil) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
    %ci_i2 = arith.constant 2 : i64
    "frk_mem.array_set"(%ci_arr, %ci_i2, %ci_nil) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
    %ci_cell = "frk_mem.global_get"() {sym = "lua_coro"} : () -> !frk_mem.box<!frk_mem.arr<!frk_dyn.dyn>>
    "frk_mem.box_set"(%ci_cell, %ci_arr) : (!frk_mem.box<!frk_mem.arr<!frk_dyn.dyn>>, !frk_mem.arr<!frk_dyn.dyn>) -> ()
    %ci_fc = "frk_mem.global_get"() {sym = "lua_susp"} : () -> !frk_mem.box<f64>
    %ci_zero = arith.constant 0.000000e+00 : f64
    "frk_mem.box_set"(%ci_fc, %ci_zero) : (!frk_mem.box<f64>, f64) -> ()
    return
  }

  // The coro cells, re-read (borrows nothing; a plain accessor).
  func.func @__lua_coro_cells() -> !frk_mem.arr<!frk_dyn.dyn> {
    %cc_cell = "frk_mem.global_get"() {sym = "lua_coro"} : () -> !frk_mem.box<!frk_mem.arr<!frk_dyn.dyn>>
    %cc_arr = "frk_mem.box_get"(%cc_cell) : (!frk_mem.box<!frk_mem.arr<!frk_dyn.dyn>>) -> !frk_mem.arr<!frk_dyn.dyn>
    return %cc_arr : !frk_mem.arr<!frk_dyn.dyn>
  }

  // coroutine.create(f) -> thread.
  func.func @__lua_coro_create_v(%cr_env: !frk_closure.envref, %cr_pack: !frk_mem.arr<!frk_dyn.dyn>) -> !frk_mem.arr<!frk_dyn.dyn> {
    %cr_c0 = arith.constant 0 : i64
    %cr_body = call @__lua_arg(%cr_pack, %cr_c0) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
    %cr_n = arith.constant 6 : i64
    %cr_rec = "frk_mem.array_new"(%cr_n) : (i64) -> !frk_mem.arr<!frk_dyn.dyn>
    %cr_zf = arith.constant 0.000000e+00 : f64
    %cr_susp = "frk_dyn.wrap"(%cr_zf) {tag = 2 : i64} : (f64) -> !frk_dyn.dyn
    "frk_mem.array_set"(%cr_rec, %cr_c0, %cr_susp) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
    %cr_c1 = arith.constant 1 : i64
    %cr_false = arith.constant false
    %cr_started = "frk_dyn.wrap"(%cr_false) {tag = 1 : i64} : (i1) -> !frk_dyn.dyn
    "frk_mem.array_set"(%cr_rec, %cr_c1, %cr_started) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
    %cr_zi = arith.constant 0 : i64
    %cr_nil = "frk_dyn.wrap"(%cr_zi) {tag = 0 : i64} : (i64) -> !frk_dyn.dyn
    %cr_c2 = arith.constant 2 : i64
    "frk_mem.array_set"(%cr_rec, %cr_c2, %cr_nil) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
    %cr_c3 = arith.constant 3 : i64
    "frk_mem.array_set"(%cr_rec, %cr_c3, %cr_nil) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
    %cr_c4 = arith.constant 4 : i64
    "frk_mem.array_set"(%cr_rec, %cr_c4, %cr_body) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
    %cr_c5 = arith.constant 5 : i64
    "frk_mem.array_set"(%cr_rec, %cr_c5, %cr_nil) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
    %cr_th = "frk_dyn.wrap"(%cr_rec) {tag = 8 : i64} : (!frk_mem.arr<!frk_dyn.dyn>) -> !frk_dyn.dyn
    "frk_mem.dispose"(%cr_pack) : (!frk_mem.arr<!frk_dyn.dyn>) -> ()
    %cr_one = arith.constant 1 : i64
    %cr_out = "frk_mem.array_new"(%cr_one) : (i64) -> !frk_mem.arr<!frk_dyn.dyn>
    "frk_mem.array_set"(%cr_out, %cr_c0, %cr_th) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
    return %cr_out : !frk_mem.arr<!frk_dyn.dyn>
  }

  // coroutine.status(co) -> "suspended"|"running"|"normal"|"dead".
  func.func @__lua_coro_status_v(%st_env: !frk_closure.envref, %st_pack: !frk_mem.arr<!frk_dyn.dyn>) -> !frk_mem.arr<!frk_dyn.dyn> {
    %st_c0 = arith.constant 0 : i64
    %st_th = call @__lua_arg(%st_pack, %st_c0) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
    %st_rec = "frk_dyn.unwrap"(%st_th) {tag = 8 : i64} : (!frk_dyn.dyn) -> !frk_mem.arr<!frk_dyn.dyn>
    %st_sd = "frk_mem.array_get"(%st_rec, %st_c0) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
    %st_sf = "frk_dyn.unwrap"(%st_sd) {tag = 2 : i64} : (!frk_dyn.dyn) -> f64
    %st_si = arith.fptosi %st_sf : f64 to i64
    cf.switch %st_si : i64, [
      default: ^st_dead,
      0: ^st_susp,
      1: ^st_run,
      2: ^st_norm
    ]
  ^st_susp:
    %st_s0 = "frk_bstr.lit"() {text = "suspended"} : () -> !frk_bstr.str
    cf.br ^st_done(%st_s0 : !frk_bstr.str)
  ^st_run:
    %st_s1 = "frk_bstr.lit"() {text = "running"} : () -> !frk_bstr.str
    cf.br ^st_done(%st_s1 : !frk_bstr.str)
  ^st_norm:
    %st_s2 = "frk_bstr.lit"() {text = "normal"} : () -> !frk_bstr.str
    cf.br ^st_done(%st_s2 : !frk_bstr.str)
  ^st_dead:
    %st_s3 = "frk_bstr.lit"() {text = "dead"} : () -> !frk_bstr.str
    cf.br ^st_done(%st_s3 : !frk_bstr.str)
  ^st_done(%st_s: !frk_bstr.str):
    %st_d = "frk_dyn.wrap"(%st_s) {tag = 3 : i64} : (!frk_bstr.str) -> !frk_dyn.dyn
    "frk_mem.dispose"(%st_pack) : (!frk_mem.arr<!frk_dyn.dyn>) -> ()
    %st_one = arith.constant 1 : i64
    %st_out = "frk_mem.array_new"(%st_one) : (i64) -> !frk_mem.arr<!frk_dyn.dyn>
    "frk_mem.array_set"(%st_out, %st_c0, %st_d) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
    return %st_out : !frk_mem.arr<!frk_dyn.dyn>
  }

  // coroutine.yield(...) — park the pack, raise the suspend flag,
  // return the suspended dummy (an empty pack). The GUARDS in the
  // transformed callers do the frame capture as this bubbles out.
  // Yield with no current thread = the main-chunk trap (D-084.5).
  func.func @__lua_coro_yield_v(%yl_env: !frk_closure.envref, %yl_pack: !frk_mem.arr<!frk_dyn.dyn>) -> !frk_mem.arr<!frk_dyn.dyn> {
    %yl_cells = call @__lua_coro_cells() : () -> !frk_mem.arr<!frk_dyn.dyn>
    %yl_c2 = arith.constant 2 : i64
    %yl_cur = "frk_mem.array_get"(%yl_cells, %yl_c2) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
    %yl_tag = "frk_dyn.tag_of"(%yl_cur) : (!frk_dyn.dyn) -> i64
    %yl_c8 = arith.constant 8 : i64
    %yl_in = arith.cmpi eq, %yl_tag, %yl_c8 : i64
    cf.cond_br %yl_in, ^yl_ok, ^yl_main
  ^yl_main:
    %yl_code = arith.constant 3 : i64
    call @frk_rt_coro_trap(%yl_code) : (i64) -> ()
    %yl_z = arith.constant 0 : i64
    %yl_dead = "frk_mem.array_new"(%yl_z) : (i64) -> !frk_mem.arr<!frk_dyn.dyn>
    return %yl_dead : !frk_mem.arr<!frk_dyn.dyn>
  ^yl_ok:
    %yl_wrapped = "frk_dyn.wrap"(%yl_pack) {tag = 7 : i64} : (!frk_mem.arr<!frk_dyn.dyn>) -> !frk_dyn.dyn
    %yl_c1 = arith.constant 1 : i64
    "frk_mem.array_set"(%yl_cells, %yl_c1, %yl_wrapped) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
    %yl_fc = "frk_mem.global_get"() {sym = "lua_susp"} : () -> !frk_mem.box<f64>
    %yl_one = arith.constant 1.000000e+00 : f64
    "frk_mem.box_set"(%yl_fc, %yl_one) : (!frk_mem.box<f64>, f64) -> ()
    %yl_z2 = arith.constant 0 : i64
    %yl_empty = "frk_mem.array_new"(%yl_z2) : (i64) -> !frk_mem.arr<!frk_dyn.dyn>
    return %yl_empty : !frk_mem.arr<!frk_dyn.dyn>
  }

  // The chain walker (D-084.3): nil chain-next => deliver the resume
  // pack (both the innermost delivery and the tail-yield empty
  // chain, one rule); else re-enter the frame closure with it.
  func.func @__lua_coro_walk(%wk_next: !frk_dyn.dyn, %wk_pack: !frk_mem.arr<!frk_dyn.dyn>) -> !frk_mem.arr<!frk_dyn.dyn> {
    %wk_tag = "frk_dyn.tag_of"(%wk_next) : (!frk_dyn.dyn) -> i64
    %wk_c5 = arith.constant 5 : i64
    %wk_is = arith.cmpi eq, %wk_tag, %wk_c5 : i64
    cf.cond_br %wk_is, ^wk_go, ^wk_deliver
  ^wk_deliver:
    return %wk_pack : !frk_mem.arr<!frk_dyn.dyn>
  ^wk_go:
    %wk_f = "frk_dyn.unwrap"(%wk_next) {tag = 5 : i64} : (!frk_dyn.dyn) -> !frk_closure.fn<[!frk_mem.arr<!frk_dyn.dyn>], [!frk_mem.arr<!frk_dyn.dyn>]>
    %wk_p0 = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
    %wk_p1 = "frk_adt.product_snoc"(%wk_p0, %wk_pack) : (!frk_adt.product<[]>, !frk_mem.arr<!frk_dyn.dyn>) -> !frk_adt.product<[!frk_mem.arr<!frk_dyn.dyn>]>
    %wk_r = "frk_closure.apply"(%wk_f, %wk_p1) : (!frk_closure.fn<[!frk_mem.arr<!frk_dyn.dyn>], [!frk_mem.arr<!frk_dyn.dyn>]>, !frk_adt.product<[!frk_mem.arr<!frk_dyn.dyn>]>) -> !frk_mem.arr<!frk_dyn.dyn>
    return %wk_r : !frk_mem.arr<!frk_dyn.dyn>
  }

  // [true|false, ...src] — the resume result shape.
  func.func @__lua_coro_prepend(%pp_ok: i1, %pp_src: !frk_mem.arr<!frk_dyn.dyn>) -> !frk_mem.arr<!frk_dyn.dyn> attributes {frk.borrows} {
    %pp_len = "frk_mem.array_len"(%pp_src) : (!frk_mem.arr<!frk_dyn.dyn>) -> i64
    %pp_one = arith.constant 1 : i64
    %pp_total = arith.addi %pp_len, %pp_one : i64
    %pp_out = "frk_mem.array_new"(%pp_total) : (i64) -> !frk_mem.arr<!frk_dyn.dyn>
    %pp_okd = "frk_dyn.wrap"(%pp_ok) {tag = 1 : i64} : (i1) -> !frk_dyn.dyn
    %pp_c0 = arith.constant 0 : i64
    "frk_mem.array_set"(%pp_out, %pp_c0, %pp_okd) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
    call @__lua_pack_copy_into(%pp_out, %pp_one, %pp_src) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_mem.arr<!frk_dyn.dyn>) -> ()
    return %pp_out : !frk_mem.arr<!frk_dyn.dyn>
  }

  // [false, "message"] — the tuple guards.
  func.func @__lua_coro_no(%no_msg: !frk_bstr.str) -> !frk_mem.arr<!frk_dyn.dyn> {
    %no_two = arith.constant 2 : i64
    %no_out = "frk_mem.array_new"(%no_two) : (i64) -> !frk_mem.arr<!frk_dyn.dyn>
    %no_false = arith.constant false
    %no_fd = "frk_dyn.wrap"(%no_false) {tag = 1 : i64} : (i1) -> !frk_dyn.dyn
    %no_c0 = arith.constant 0 : i64
    "frk_mem.array_set"(%no_out, %no_c0, %no_fd) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
    %no_md = "frk_dyn.wrap"(%no_msg) {tag = 3 : i64} : (!frk_bstr.str) -> !frk_dyn.dyn
    %no_c1 = arith.constant 1 : i64
    "frk_mem.array_set"(%no_out, %no_c1, %no_md) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
    return %no_out : !frk_mem.arr<!frk_dyn.dyn>
  }

  // resume's engine: status guards (tuples), status flips (BEFORE
  // the walk — D-084.3's law), the resumer link, the walk, and the
  // suspend-vs-return decision on the way out.
  func.func @__lua_coro_resume_core(%rc_th: !frk_dyn.dyn, %rc_args: !frk_mem.arr<!frk_dyn.dyn>) -> !frk_mem.arr<!frk_dyn.dyn> {
    %rc_rec = "frk_dyn.unwrap"(%rc_th) {tag = 8 : i64} : (!frk_dyn.dyn) -> !frk_mem.arr<!frk_dyn.dyn>
    %rc_c0 = arith.constant 0 : i64
    %rc_sd = "frk_mem.array_get"(%rc_rec, %rc_c0) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
    %rc_sf = "frk_dyn.unwrap"(%rc_sd) {tag = 2 : i64} : (!frk_dyn.dyn) -> f64
    %rc_si = arith.fptosi %rc_sf : f64 to i64
    cf.switch %rc_si : i64, [
      default: ^rc_dead,
      0: ^rc_go,
      1: ^rc_running,
      2: ^rc_normal
    ]
  ^rc_dead:
    %rc_m3 = "frk_bstr.lit"() {text = "cannot resume dead coroutine"} : () -> !frk_bstr.str
    %rc_r3 = call @__lua_coro_no(%rc_m3) : (!frk_bstr.str) -> !frk_mem.arr<!frk_dyn.dyn>
    return %rc_r3 : !frk_mem.arr<!frk_dyn.dyn>
  ^rc_running:
    %rc_m1 = "frk_bstr.lit"() {text = "cannot resume running coroutine"} : () -> !frk_bstr.str
    %rc_r1 = call @__lua_coro_no(%rc_m1) : (!frk_bstr.str) -> !frk_mem.arr<!frk_dyn.dyn>
    return %rc_r1 : !frk_mem.arr<!frk_dyn.dyn>
  ^rc_normal:
    %rc_m2 = "frk_bstr.lit"() {text = "cannot resume normal coroutine"} : () -> !frk_bstr.str
    %rc_r2 = call @__lua_coro_no(%rc_m2) : (!frk_bstr.str) -> !frk_mem.arr<!frk_dyn.dyn>
    return %rc_r2 : !frk_mem.arr<!frk_dyn.dyn>
  ^rc_go:
    // status := running BEFORE anything else (D-084.3 law).
    %rc_onef = arith.constant 1.000000e+00 : f64
    %rc_rund = "frk_dyn.wrap"(%rc_onef) {tag = 2 : i64} : (f64) -> !frk_dyn.dyn
    "frk_mem.array_set"(%rc_rec, %rc_c0, %rc_rund) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
    // resumer link + current-thread swap; the previous current (if a
    // thread) turns 'normal'.
    %rc_cells = call @__lua_coro_cells() : () -> !frk_mem.arr<!frk_dyn.dyn>
    %rc_c2 = arith.constant 2 : i64
    %rc_prev = "frk_mem.array_get"(%rc_cells, %rc_c2) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
    %rc_c3 = arith.constant 3 : i64
    "frk_mem.array_set"(%rc_rec, %rc_c3, %rc_prev) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
    "frk_mem.array_set"(%rc_cells, %rc_c2, %rc_th) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
    %rc_ptag = "frk_dyn.tag_of"(%rc_prev) : (!frk_dyn.dyn) -> i64
    %rc_c8 = arith.constant 8 : i64
    %rc_pth = arith.cmpi eq, %rc_ptag, %rc_c8 : i64
    cf.cond_br %rc_pth, ^rc_mark_prev, ^rc_dispatch
  ^rc_mark_prev:
    %rc_prec = "frk_dyn.unwrap"(%rc_prev) {tag = 8 : i64} : (!frk_dyn.dyn) -> !frk_mem.arr<!frk_dyn.dyn>
    %rc_twof = arith.constant 2.000000e+00 : f64
    %rc_normd = "frk_dyn.wrap"(%rc_twof) {tag = 2 : i64} : (f64) -> !frk_dyn.dyn
    "frk_mem.array_set"(%rc_prec, %rc_c0, %rc_normd) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
    cf.br ^rc_dispatch
  ^rc_dispatch:
    // started? first resume calls the body; later resumes walk the
    // stored chain (nil chain = the tail-yield case: deliver).
    %rc_c1 = arith.constant 1 : i64
    %rc_std = "frk_mem.array_get"(%rc_rec, %rc_c1) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
    %rc_stb = "frk_dyn.unwrap"(%rc_std) {tag = 1 : i64} : (!frk_dyn.dyn) -> i1
    cf.cond_br %rc_stb, ^rc_walk, ^rc_first
  ^rc_first:
    %rc_true = arith.constant true
    %rc_trued = "frk_dyn.wrap"(%rc_true) {tag = 1 : i64} : (i1) -> !frk_dyn.dyn
    "frk_mem.array_set"(%rc_rec, %rc_c1, %rc_trued) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
    %rc_c4 = arith.constant 4 : i64
    %rc_body = "frk_mem.array_get"(%rc_rec, %rc_c4) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
    %rc_out1 = call @__lua_coro_walk(%rc_body, %rc_args) : (!frk_dyn.dyn, !frk_mem.arr<!frk_dyn.dyn>) -> !frk_mem.arr<!frk_dyn.dyn>
    cf.br ^rc_after(%rc_out1 : !frk_mem.arr<!frk_dyn.dyn>)
  ^rc_walk:
    %rc_ch = "frk_mem.array_get"(%rc_rec, %rc_c2) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
    %rc_zi = arith.constant 0 : i64
    %rc_nild = "frk_dyn.wrap"(%rc_zi) {tag = 0 : i64} : (i64) -> !frk_dyn.dyn
    "frk_mem.array_set"(%rc_rec, %rc_c2, %rc_nild) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
    %rc_out2 = call @__lua_coro_walk(%rc_ch, %rc_args) : (!frk_dyn.dyn, !frk_mem.arr<!frk_dyn.dyn>) -> !frk_mem.arr<!frk_dyn.dyn>
    cf.br ^rc_after(%rc_out2 : !frk_mem.arr<!frk_dyn.dyn>)
  ^rc_after(%rc_res: !frk_mem.arr<!frk_dyn.dyn>):
    // restore the current-thread swap; the resumer runs again.
    %rc_cells2 = call @__lua_coro_cells() : () -> !frk_mem.arr<!frk_dyn.dyn>
    %rc_prev2 = "frk_mem.array_get"(%rc_rec, %rc_c3) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
    "frk_mem.array_set"(%rc_cells2, %rc_c2, %rc_prev2) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
    %rc_p2tag = "frk_dyn.tag_of"(%rc_prev2) : (!frk_dyn.dyn) -> i64
    %rc_p2th = arith.cmpi eq, %rc_p2tag, %rc_c8 : i64
    cf.cond_br %rc_p2th, ^rc_unmark_prev, ^rc_decide
  ^rc_unmark_prev:
    %rc_p2rec = "frk_dyn.unwrap"(%rc_prev2) {tag = 8 : i64} : (!frk_dyn.dyn) -> !frk_mem.arr<!frk_dyn.dyn>
    %rc_onef2 = arith.constant 1.000000e+00 : f64
    %rc_rund2 = "frk_dyn.wrap"(%rc_onef2) {tag = 2 : i64} : (f64) -> !frk_dyn.dyn
    "frk_mem.array_set"(%rc_p2rec, %rc_c0, %rc_rund2) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
    cf.br ^rc_decide
  ^rc_decide:
    // suspended again, or returned?
    %rc_fc = "frk_mem.global_get"() {sym = "lua_susp"} : () -> !frk_mem.box<f64>
    %rc_fv = "frk_mem.box_get"(%rc_fc) : (!frk_mem.box<f64>) -> f64
    %rc_zf = arith.constant 0.000000e+00 : f64
    %rc_is = arith.cmpf one, %rc_fv, %rc_zf : f64
    cf.cond_br %rc_is, ^rc_suspended, ^rc_returned
  ^rc_suspended:
    "frk_mem.box_set"(%rc_fc, %rc_zf) : (!frk_mem.box<f64>, f64) -> ()
    %rc_cells3 = call @__lua_coro_cells() : () -> !frk_mem.arr<!frk_dyn.dyn>
    %rc_zi0 = arith.constant 0 : i64
    %rc_chain = "frk_mem.array_get"(%rc_cells3, %rc_zi0) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
    %rc_zi2 = arith.constant 0 : i64
    %rc_nild2 = "frk_dyn.wrap"(%rc_zi2) {tag = 0 : i64} : (i64) -> !frk_dyn.dyn
    "frk_mem.array_set"(%rc_cells3, %rc_zi0, %rc_nild2) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
    "frk_mem.array_set"(%rc_rec, %rc_c2, %rc_chain) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
    %rc_zf0 = arith.constant 0.000000e+00 : f64
    %rc_suspd = "frk_dyn.wrap"(%rc_zf0) {tag = 2 : i64} : (f64) -> !frk_dyn.dyn
    "frk_mem.array_set"(%rc_rec, %rc_c0, %rc_suspd) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
    %rc_ci1 = arith.constant 1 : i64
    %rc_parkd = "frk_mem.array_get"(%rc_cells3, %rc_ci1) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
    %rc_park = "frk_dyn.unwrap"(%rc_parkd) {tag = 7 : i64} : (!frk_dyn.dyn) -> !frk_mem.arr<!frk_dyn.dyn>
    %rc_true2 = arith.constant true
    %rc_ok1 = call @__lua_coro_prepend(%rc_true2, %rc_park) : (i1, !frk_mem.arr<!frk_dyn.dyn>) -> !frk_mem.arr<!frk_dyn.dyn>
    return %rc_ok1 : !frk_mem.arr<!frk_dyn.dyn>
  ^rc_returned:
    %rc_threef = arith.constant 3.000000e+00 : f64
    %rc_deadd = "frk_dyn.wrap"(%rc_threef) {tag = 2 : i64} : (f64) -> !frk_dyn.dyn
    "frk_mem.array_set"(%rc_rec, %rc_c0, %rc_deadd) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
    %rc_true3 = arith.constant true
    %rc_ok2 = call @__lua_coro_prepend(%rc_true3, %rc_res) : (i1, !frk_mem.arr<!frk_dyn.dyn>) -> !frk_mem.arr<!frk_dyn.dyn>
    return %rc_ok2 : !frk_mem.arr<!frk_dyn.dyn>
  }

  func.func @__lua_coro_resume_v(%rv_env: !frk_closure.envref, %rv_pack: !frk_mem.arr<!frk_dyn.dyn>) -> !frk_mem.arr<!frk_dyn.dyn> {
    %rv_c0 = arith.constant 0 : i64
    %rv_th = call @__lua_arg(%rv_pack, %rv_c0) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
    %rv_c1 = arith.constant 1 : i64
    %rv_args = call @__lua_pack_tail(%rv_pack, %rv_c1) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_mem.arr<!frk_dyn.dyn>
    "frk_mem.dispose"(%rv_pack) : (!frk_mem.arr<!frk_dyn.dyn>) -> ()
    %rv_out = call @__lua_coro_resume_core(%rv_th, %rv_args) : (!frk_dyn.dyn, !frk_mem.arr<!frk_dyn.dyn>) -> !frk_mem.arr<!frk_dyn.dyn>
    return %rv_out : !frk_mem.arr<!frk_dyn.dyn>
  }

  // coroutine.wrap(f): a closure over the thread; strips the leading
  // true; a false head is the abort path (D-084.5 fence).
  func.func @__lua_coro_wrapped(%wr_th: !frk_dyn.dyn, %wr_pack: !frk_mem.arr<!frk_dyn.dyn>) -> !frk_mem.arr<!frk_dyn.dyn> {
    %wr_out = call @__lua_coro_resume_core(%wr_th, %wr_pack) : (!frk_dyn.dyn, !frk_mem.arr<!frk_dyn.dyn>) -> !frk_mem.arr<!frk_dyn.dyn>
    %wr_c0 = arith.constant 0 : i64
    %wr_head = call @__lua_arg(%wr_out, %wr_c0) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
    %wr_ok = "frk_dyn.unwrap"(%wr_head) {tag = 1 : i64} : (!frk_dyn.dyn) -> i1
    cf.cond_br %wr_ok, ^wr_strip, ^wr_die
  ^wr_die:
    %wr_code = arith.constant 4 : i64
    call @frk_rt_coro_trap(%wr_code) : (i64) -> ()
    %wr_z = arith.constant 0 : i64
    %wr_dead = "frk_mem.array_new"(%wr_z) : (i64) -> !frk_mem.arr<!frk_dyn.dyn>
    return %wr_dead : !frk_mem.arr<!frk_dyn.dyn>
  ^wr_strip:
    %wr_c1 = arith.constant 1 : i64
    %wr_tail = call @__lua_pack_tail(%wr_out, %wr_c1) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_mem.arr<!frk_dyn.dyn>
    return %wr_tail : !frk_mem.arr<!frk_dyn.dyn>
  }

  func.func @__lua_coro_wrap_v(%wp_env: !frk_closure.envref, %wp_pack: !frk_mem.arr<!frk_dyn.dyn>) -> !frk_mem.arr<!frk_dyn.dyn> {
    %wp_c0 = arith.constant 0 : i64
    %wp_body = call @__lua_arg(%wp_pack, %wp_c0) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
    "frk_mem.dispose"(%wp_pack) : (!frk_mem.arr<!frk_dyn.dyn>) -> ()
    // create, then close over the thread.
    %wp_one = arith.constant 1 : i64
    %wp_cargs = "frk_mem.array_new"(%wp_one) : (i64) -> !frk_mem.arr<!frk_dyn.dyn>
    "frk_mem.array_set"(%wp_cargs, %wp_c0, %wp_body) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
    %wp_e0 = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
    %wp_cf = "frk_closure.make"(%wp_e0) {callee = @__lua_coro_create_v} : (!frk_adt.product<[]>) -> !frk_closure.fn<[!frk_mem.arr<!frk_dyn.dyn>],[!frk_mem.arr<!frk_dyn.dyn>]>
    %wp_a0 = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
    %wp_a1 = "frk_adt.product_snoc"(%wp_a0, %wp_cargs) : (!frk_adt.product<[]>, !frk_mem.arr<!frk_dyn.dyn>) -> !frk_adt.product<[!frk_mem.arr<!frk_dyn.dyn>]>
    %wp_thp = "frk_closure.apply"(%wp_cf, %wp_a1) : (!frk_closure.fn<[!frk_mem.arr<!frk_dyn.dyn>],[!frk_mem.arr<!frk_dyn.dyn>]>, !frk_adt.product<[!frk_mem.arr<!frk_dyn.dyn>]>) -> !frk_mem.arr<!frk_dyn.dyn>
    %wp_th = call @__lua_arg(%wp_thp, %wp_c0) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
    %wp_p0 = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
    %wp_p1 = "frk_adt.product_snoc"(%wp_p0, %wp_th) : (!frk_adt.product<[]>, !frk_dyn.dyn) -> !frk_adt.product<[!frk_dyn.dyn]>
    %wp_cl = "frk_closure.make"(%wp_p1) {callee = @__lua_coro_wrapped} : (!frk_adt.product<[!frk_dyn.dyn]>) -> !frk_closure.fn<[!frk_mem.arr<!frk_dyn.dyn>],[!frk_mem.arr<!frk_dyn.dyn>]>
    %wp_fd = "frk_dyn.wrap"(%wp_cl) {tag = 5 : i64} : (!frk_closure.fn<[!frk_mem.arr<!frk_dyn.dyn>],[!frk_mem.arr<!frk_dyn.dyn>]>) -> !frk_dyn.dyn
    %wp_out = "frk_mem.array_new"(%wp_one) : (i64) -> !frk_mem.arr<!frk_dyn.dyn>
    "frk_mem.array_set"(%wp_out, %wp_c0, %wp_fd) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
    return %wp_out : !frk_mem.arr<!frk_dyn.dyn>
  }

  // type(v) — newly seeded at v0.4 (D-084.5).
  func.func @__lua_type_v(%tp_env: !frk_closure.envref, %tp_pack: !frk_mem.arr<!frk_dyn.dyn>) -> !frk_mem.arr<!frk_dyn.dyn> {
    %tp_c0 = arith.constant 0 : i64
    %tp_v = call @__lua_arg(%tp_pack, %tp_c0) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
    %tp_tag = "frk_dyn.tag_of"(%tp_v) : (!frk_dyn.dyn) -> i64
    cf.switch %tp_tag : i64, [
      default: ^tp_num,
      0: ^tp_nil,
      1: ^tp_bool,
      3: ^tp_str,
      4: ^tp_table,
      5: ^tp_fun,
      8: ^tp_thread
    ]
  ^tp_nil:
    %tp_s0 = "frk_bstr.lit"() {text = "nil"} : () -> !frk_bstr.str
    cf.br ^tp_done(%tp_s0 : !frk_bstr.str)
  ^tp_bool:
    %tp_s1 = "frk_bstr.lit"() {text = "boolean"} : () -> !frk_bstr.str
    cf.br ^tp_done(%tp_s1 : !frk_bstr.str)
  ^tp_num:
    %tp_s2 = "frk_bstr.lit"() {text = "number"} : () -> !frk_bstr.str
    cf.br ^tp_done(%tp_s2 : !frk_bstr.str)
  ^tp_str:
    %tp_s3 = "frk_bstr.lit"() {text = "string"} : () -> !frk_bstr.str
    cf.br ^tp_done(%tp_s3 : !frk_bstr.str)
  ^tp_table:
    %tp_s4 = "frk_bstr.lit"() {text = "table"} : () -> !frk_bstr.str
    cf.br ^tp_done(%tp_s4 : !frk_bstr.str)
  ^tp_fun:
    %tp_s5 = "frk_bstr.lit"() {text = "function"} : () -> !frk_bstr.str
    cf.br ^tp_done(%tp_s5 : !frk_bstr.str)
  ^tp_thread:
    %tp_s6 = "frk_bstr.lit"() {text = "thread"} : () -> !frk_bstr.str
    cf.br ^tp_done(%tp_s6 : !frk_bstr.str)
  ^tp_done(%tp_s: !frk_bstr.str):
    %tp_d = "frk_dyn.wrap"(%tp_s) {tag = 3 : i64} : (!frk_bstr.str) -> !frk_dyn.dyn
    "frk_mem.dispose"(%tp_pack) : (!frk_mem.arr<!frk_dyn.dyn>) -> ()
    %tp_one = arith.constant 1 : i64
    %tp_out = "frk_mem.array_new"(%tp_one) : (i64) -> !frk_mem.arr<!frk_dyn.dyn>
    "frk_mem.array_set"(%tp_out, %tp_c0, %tp_d) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
    return %tp_out : !frk_mem.arr<!frk_dyn.dyn>
  }

