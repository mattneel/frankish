// r7rs_core intrinsics (M17, D-062; v0.1 at M25, D-070; SPEC §6.6) —
// the scheme frontend's language primitives, authored as kernel IR
// instead of emitter builder code. This file is the SEED MODULE:
// compilation parses it first and the emitter appends the program's
// functions.
//
// v0.1 (D-070): pairs are wrapped product<[dyn, dyn]> at TAG_PAIR = 6
// (the D-051 widening) — cons/car/cdr ride the EXISTING
// wrap/unwrap/get ops; symbols are tag-3 byte strings (interning
// makes eq? a pointer compare); display grows str, '()-as-"()", and
// pair arms (proper lists spaced, dotted " . ").
//
// Runtime declarations here are checked against the frk-abi registry
// by the frankish semantic verifier (a drifted signature is refused
// at verify time), and the kernel lowering skips re-declaring them.

func.func private @frk_rt_scm_display_num(f64)
func.func private @frk_rt_scm_display_bool(i64)
func.func private @frk_rt_scm_display_str(!frk_bstr.str)
func.func private @frk_rt_scm_newline()

// display's tag dispatch (D-070): numbers via the %.14g twin printer,
// booleans as #t/#f (extended i1 → i64: wasm enforces exact widths,
// D-062), symbols as raw bytes, '() as "()", pairs via the items
// walker. The default arm treats unknown tags as numbers — widening
// the value universe is a manifest amendment.
func.func @__scm_display(%value: !frk_dyn.dyn) {
  %tag = "frk_dyn.tag_of"(%value) : (!frk_dyn.dyn) -> i64
  cf.switch %tag : i64, [
    default: ^num,
    0: ^unit,
    1: ^bool,
    3: ^str,
    6: ^pair
  ]
^num:
  %n = "frk_dyn.unwrap"(%value) {tag = 2 : i64} : (!frk_dyn.dyn) -> f64
  func.call @frk_rt_scm_display_num(%n) : (f64) -> ()
  return
^unit:
  %unit_text = "frk_bstr.lit"() {text = "()"} : () -> !frk_bstr.str
  func.call @frk_rt_scm_display_str(%unit_text) : (!frk_bstr.str) -> ()
  return
^bool:
  %b = "frk_dyn.unwrap"(%value) {tag = 1 : i64} : (!frk_dyn.dyn) -> i1
  %w = arith.extui %b : i1 to i64
  func.call @frk_rt_scm_display_bool(%w) : (i64) -> ()
  return
^str:
  %s = "frk_dyn.unwrap"(%value) {tag = 3 : i64} : (!frk_dyn.dyn) -> !frk_bstr.str
  func.call @frk_rt_scm_display_str(%s) : (!frk_bstr.str) -> ()
  return
^pair:
  %open = "frk_bstr.lit"() {text = "("} : () -> !frk_bstr.str
  func.call @frk_rt_scm_display_str(%open) : (!frk_bstr.str) -> ()
  func.call @__scm_display_items(%value) : (!frk_dyn.dyn) -> ()
  return
}

// Displays the elements of a KNOWN-pair and the closing paren: car,
// then cdr as nil → ")", pair → " " + recurse (a tail call — long
// lists ride the trampoline/musttail machinery), other → " . " +
// display + ")" (dotted).
func.func @__scm_display_items(%p: !frk_dyn.dyn) {
  %cell = "frk_dyn.unwrap"(%p) {tag = 6 : i64} : (!frk_dyn.dyn) -> !frk_adt.product<[!frk_dyn.dyn, !frk_dyn.dyn]>
  %car = "frk_adt.get"(%cell) {field = 0 : i64} : (!frk_adt.product<[!frk_dyn.dyn, !frk_dyn.dyn]>) -> !frk_dyn.dyn
  %cdr = "frk_adt.get"(%cell) {field = 1 : i64} : (!frk_adt.product<[!frk_dyn.dyn, !frk_dyn.dyn]>) -> !frk_dyn.dyn
  func.call @__scm_display(%car) : (!frk_dyn.dyn) -> ()
  %ctag = "frk_dyn.tag_of"(%cdr) : (!frk_dyn.dyn) -> i64
  cf.switch %ctag : i64, [
    default: ^dotted,
    0: ^done,
    6: ^rest
  ]
^done:
  %close = "frk_bstr.lit"() {text = ")"} : () -> !frk_bstr.str
  func.call @frk_rt_scm_display_str(%close) : (!frk_bstr.str) -> ()
  return
^rest:
  %space = "frk_bstr.lit"() {text = " "} : () -> !frk_bstr.str
  func.call @frk_rt_scm_display_str(%space) : (!frk_bstr.str) -> ()
  func.call @__scm_display_items(%cdr) : (!frk_dyn.dyn) -> ()
  return
^dotted:
  %dot = "frk_bstr.lit"() {text = " . "} : () -> !frk_bstr.str
  func.call @frk_rt_scm_display_str(%dot) : (!frk_bstr.str) -> ()
  func.call @__scm_display(%cdr) : (!frk_dyn.dyn) -> ()
  %close2 = "frk_bstr.lit"() {text = ")"} : () -> !frk_bstr.str
  func.call @frk_rt_scm_display_str(%close2) : (!frk_bstr.str) -> ()
  return
}

func.func @__scm_cons(%a: !frk_dyn.dyn, %d: !frk_dyn.dyn) -> !frk_dyn.dyn {
  %e = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
  %p1 = "frk_adt.product_snoc"(%e, %a) : (!frk_adt.product<[]>, !frk_dyn.dyn) -> !frk_adt.product<[!frk_dyn.dyn]>
  %p2 = "frk_adt.product_snoc"(%p1, %d) : (!frk_adt.product<[!frk_dyn.dyn]>, !frk_dyn.dyn) -> !frk_adt.product<[!frk_dyn.dyn, !frk_dyn.dyn]>
  %pair = "frk_dyn.wrap"(%p2) {tag = 6 : i64} : (!frk_adt.product<[!frk_dyn.dyn, !frk_dyn.dyn]>) -> !frk_dyn.dyn
  return %pair : !frk_dyn.dyn
}

func.func @__scm_car(%p: !frk_dyn.dyn) -> !frk_dyn.dyn {
  %cell = "frk_dyn.unwrap"(%p) {tag = 6 : i64} : (!frk_dyn.dyn) -> !frk_adt.product<[!frk_dyn.dyn, !frk_dyn.dyn]>
  %car = "frk_adt.get"(%cell) {field = 0 : i64} : (!frk_adt.product<[!frk_dyn.dyn, !frk_dyn.dyn]>) -> !frk_dyn.dyn
  return %car : !frk_dyn.dyn
}

func.func @__scm_cdr(%p: !frk_dyn.dyn) -> !frk_dyn.dyn {
  %cell = "frk_dyn.unwrap"(%p) {tag = 6 : i64} : (!frk_dyn.dyn) -> !frk_adt.product<[!frk_dyn.dyn, !frk_dyn.dyn]>
  %cdr = "frk_adt.get"(%cell) {field = 1 : i64} : (!frk_adt.product<[!frk_dyn.dyn, !frk_dyn.dyn]>) -> !frk_dyn.dyn
  return %cdr : !frk_dyn.dyn
}

func.func @__scm_nullp(%v: !frk_dyn.dyn) -> !frk_dyn.dyn {
  %tag = "frk_dyn.tag_of"(%v) : (!frk_dyn.dyn) -> i64
  %c0 = arith.constant 0 : i64
  %is = arith.cmpi eq, %tag, %c0 : i64
  %d = "frk_dyn.wrap"(%is) {tag = 1 : i64} : (i1) -> !frk_dyn.dyn
  return %d : !frk_dyn.dyn
}

func.func @__scm_pairp(%v: !frk_dyn.dyn) -> !frk_dyn.dyn {
  %tag = "frk_dyn.tag_of"(%v) : (!frk_dyn.dyn) -> i64
  %c6 = arith.constant 6 : i64
  %is = arith.cmpi eq, %tag, %c6 : i64
  %d = "frk_dyn.wrap"(%is) {tag = 1 : i64} : (i1) -> !frk_dyn.dyn
  return %d : !frk_dyn.dyn
}

// eq? — identity: tags equal, then payloads. Symbols (tag 3) compare
// through frk_bstr.eq — byte equality in the reference interpreter,
// interned-pointer equality natively: CONVERGENT semantics, held
// equal by the differential law (the payload_word shortcut diverged
// interp-side: two literals are distinct interp values). Everything
// else compares payload words (fixnums bit-stable, booleans 0/1,
// '() tag-only, pairs by allocation).
func.func @__scm_eq(%a: !frk_dyn.dyn, %b: !frk_dyn.dyn) -> !frk_dyn.dyn {
  %ta = "frk_dyn.tag_of"(%a) : (!frk_dyn.dyn) -> i64
  %tb = "frk_dyn.tag_of"(%b) : (!frk_dyn.dyn) -> i64
  %teq = arith.cmpi eq, %ta, %tb : i64
  cf.cond_br %teq, ^same, ^differ
^differ:
  %f = arith.constant false
  %fd = "frk_dyn.wrap"(%f) {tag = 1 : i64} : (i1) -> !frk_dyn.dyn
  return %fd : !frk_dyn.dyn
^same:
  %c3 = arith.constant 3 : i64
  %isstr = arith.cmpi eq, %ta, %c3 : i64
  cf.cond_br %isstr, ^strs, ^words
^strs:
  %sa = "frk_dyn.unwrap"(%a) {tag = 3 : i64} : (!frk_dyn.dyn) -> !frk_bstr.str
  %sb = "frk_dyn.unwrap"(%b) {tag = 3 : i64} : (!frk_dyn.dyn) -> !frk_bstr.str
  %seq = "frk_bstr.eq"(%sa, %sb) : (!frk_bstr.str, !frk_bstr.str) -> i1
  %sd = "frk_dyn.wrap"(%seq) {tag = 1 : i64} : (i1) -> !frk_dyn.dyn
  return %sd : !frk_dyn.dyn
^words:
  %pa = "frk_dyn.payload_word"(%a) : (!frk_dyn.dyn) -> i64
  %pb = "frk_dyn.payload_word"(%b) : (!frk_dyn.dyn) -> i64
  %peq = arith.cmpi eq, %pa, %pb : i64
  %pd = "frk_dyn.wrap"(%peq) {tag = 1 : i64} : (i1) -> !frk_dyn.dyn
  return %pd : !frk_dyn.dyn
}
