// r7rs_core intrinsics (M17, D-062; SPEC §6.6) — the scheme
// frontend's language primitives, authored as kernel IR instead of
// emitter builder code. This file is the SEED MODULE: compilation
// parses it first and the emitter appends the program's functions.
//
// Runtime declarations here are checked against the frk-abi registry
// by the frankish semantic verifier (a drifted signature is refused
// at verify time), and the kernel lowering skips re-declaring them.

func.func private @frk_rt_scm_display_num(f64)
func.func private @frk_rt_scm_display_bool(i64)
func.func private @frk_rt_scm_newline()

// display's tag dispatch: numbers via the %.14g twin printer, booleans
// as #t/#f (extended i1 → i64: wasm enforces exact widths, D-062).
// The default arm treats unknown tags as numbers — the v0 value
// universe is {num, bool}; widening it is a manifest amendment.
func.func @__scm_display(%value: !frk_dyn.dyn) {
  %tag = "frk_dyn.tag_of"(%value) : (!frk_dyn.dyn) -> i64
  cf.switch %tag : i64, [
    default: ^num,
    2: ^num,
    1: ^bool
  ]
^num:
  %n = "frk_dyn.unwrap"(%value) {tag = 2 : i64} : (!frk_dyn.dyn) -> f64
  func.call @frk_rt_scm_display_num(%n) : (f64) -> ()
  return
^bool:
  %b = "frk_dyn.unwrap"(%value) {tag = 1 : i64} : (!frk_dyn.dyn) -> i1
  %w = arith.extui %b : i1 to i64
  func.call @frk_rt_scm_display_bool(%w) : (i64) -> ()
  return
}
