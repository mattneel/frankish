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
func.func private @frk_rt_scm_trap(i64)

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
  %cellb = "frk_dyn.unwrap"(%p) {tag = 6 : i64} : (!frk_dyn.dyn) -> !frk_mem.box<!frk_adt.product<[!frk_dyn.dyn, !frk_dyn.dyn]>>
  %cell = "frk_mem.box_get"(%cellb) : (!frk_mem.box<!frk_adt.product<[!frk_dyn.dyn, !frk_dyn.dyn]>>) -> !frk_adt.product<[!frk_dyn.dyn, !frk_dyn.dyn]>
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

// pack[i] with nil-fill (M26, D-071): the scheme twin of __lua_arg —
// pack-fn parameters and first-class application results read through
// it. Borrows its pack.
func.func @__scm_arg(%pack: !frk_mem.arr<!frk_dyn.dyn>, %i: i64) -> !frk_dyn.dyn attributes {frk.borrows} {
  %len = "frk_mem.array_len"(%pack) : (!frk_mem.arr<!frk_dyn.dyn>) -> i64
  %in = arith.cmpi slt, %i, %len : i64
  cf.cond_br %in, ^read, ^nil
^read:
  %v = "frk_mem.array_get"(%pack, %i) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
  return %v : !frk_dyn.dyn
^nil:
  %z = arith.constant 0 : i64
  %n = "frk_dyn.wrap"(%z) {tag = 0 : i64} : (i64) -> !frk_dyn.dyn
  return %n : !frk_dyn.dyn
}

// A pair is a BOXED product (D-077): mutation needs the shared cell —
// interp aliases share the Rc, native aliases share the heap cell.
func.func @__scm_cons(%a: !frk_dyn.dyn, %d: !frk_dyn.dyn) -> !frk_dyn.dyn {
  %e = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
  %p1 = "frk_adt.product_snoc"(%e, %a) : (!frk_adt.product<[]>, !frk_dyn.dyn) -> !frk_adt.product<[!frk_dyn.dyn]>
  %p2 = "frk_adt.product_snoc"(%p1, %d) : (!frk_adt.product<[!frk_dyn.dyn]>, !frk_dyn.dyn) -> !frk_adt.product<[!frk_dyn.dyn, !frk_dyn.dyn]>
  %cell = "frk_mem.box_new"(%p2) : (!frk_adt.product<[!frk_dyn.dyn, !frk_dyn.dyn]>) -> !frk_mem.box<!frk_adt.product<[!frk_dyn.dyn, !frk_dyn.dyn]>>
  %pair = "frk_dyn.wrap"(%cell) {tag = 6 : i64} : (!frk_mem.box<!frk_adt.product<[!frk_dyn.dyn, !frk_dyn.dyn]>>) -> !frk_dyn.dyn
  return %pair : !frk_dyn.dyn
}

func.func @__scm_car(%p: !frk_dyn.dyn) -> !frk_dyn.dyn {
  %cell = "frk_dyn.unwrap"(%p) {tag = 6 : i64} : (!frk_dyn.dyn) -> !frk_mem.box<!frk_adt.product<[!frk_dyn.dyn, !frk_dyn.dyn]>>
  %car = "frk_mem.field_get"(%cell) {field = 0 : i64} : (!frk_mem.box<!frk_adt.product<[!frk_dyn.dyn, !frk_dyn.dyn]>>) -> !frk_dyn.dyn
  return %car : !frk_dyn.dyn
}

func.func @__scm_cdr(%p: !frk_dyn.dyn) -> !frk_dyn.dyn {
  %cell = "frk_dyn.unwrap"(%p) {tag = 6 : i64} : (!frk_dyn.dyn) -> !frk_mem.box<!frk_adt.product<[!frk_dyn.dyn, !frk_dyn.dyn]>>
  %cdr = "frk_mem.field_get"(%cell) {field = 1 : i64} : (!frk_mem.box<!frk_adt.product<[!frk_dyn.dyn, !frk_dyn.dyn]>>) -> !frk_dyn.dyn
  return %cdr : !frk_dyn.dyn
}

// Pair mutation (M31, D-077): the M28 record rung, consumed by scheme.
func.func @__scm_setcar(%p: !frk_dyn.dyn, %v: !frk_dyn.dyn) -> !frk_dyn.dyn {
  %cell = "frk_dyn.unwrap"(%p) {tag = 6 : i64} : (!frk_dyn.dyn) -> !frk_mem.box<!frk_adt.product<[!frk_dyn.dyn, !frk_dyn.dyn]>>
  "frk_mem.field_set"(%cell, %v) {field = 0 : i64} : (!frk_mem.box<!frk_adt.product<[!frk_dyn.dyn, !frk_dyn.dyn]>>, !frk_dyn.dyn) -> ()
  %z = arith.constant 0 : i64
  %nil = "frk_dyn.wrap"(%z) {tag = 0 : i64} : (i64) -> !frk_dyn.dyn
  return %nil : !frk_dyn.dyn
}

func.func @__scm_setcdr(%p: !frk_dyn.dyn, %v: !frk_dyn.dyn) -> !frk_dyn.dyn {
  %cell = "frk_dyn.unwrap"(%p) {tag = 6 : i64} : (!frk_dyn.dyn) -> !frk_mem.box<!frk_adt.product<[!frk_dyn.dyn, !frk_dyn.dyn]>>
  "frk_mem.field_set"(%cell, %v) {field = 1 : i64} : (!frk_mem.box<!frk_adt.product<[!frk_dyn.dyn, !frk_dyn.dyn]>>, !frk_dyn.dyn) -> ()
  %z = arith.constant 0 : i64
  %nil = "frk_dyn.wrap"(%z) {tag = 0 : i64} : (i64) -> !frk_dyn.dyn
  return %nil : !frk_dyn.dyn
}

// Strings (M31, D-077): tag-3 bstrs like symbols; interning makes
// string=? a pointer compare even for dynamic strings.
func.func @__scm_strapp(%a: !frk_dyn.dyn, %b: !frk_dyn.dyn) -> !frk_dyn.dyn {
  %sa = "frk_dyn.unwrap"(%a) {tag = 3 : i64} : (!frk_dyn.dyn) -> !frk_bstr.str
  %sb = "frk_dyn.unwrap"(%b) {tag = 3 : i64} : (!frk_dyn.dyn) -> !frk_bstr.str
  %r = "frk_bstr.concat"(%sa, %sb) : (!frk_bstr.str, !frk_bstr.str) -> !frk_bstr.str
  %d = "frk_dyn.wrap"(%r) {tag = 3 : i64} : (!frk_bstr.str) -> !frk_dyn.dyn
  return %d : !frk_dyn.dyn
}

func.func @__scm_strlen(%s: !frk_dyn.dyn) -> !frk_dyn.dyn {
  %b = "frk_dyn.unwrap"(%s) {tag = 3 : i64} : (!frk_dyn.dyn) -> !frk_bstr.str
  %n = "frk_bstr.len"(%b) : (!frk_bstr.str) -> i64
  %f = arith.sitofp %n : i64 to f64
  %d = "frk_dyn.wrap"(%f) {tag = 2 : i64} : (f64) -> !frk_dyn.dyn
  return %d : !frk_dyn.dyn
}

func.func @__scm_streq(%a: !frk_dyn.dyn, %b: !frk_dyn.dyn) -> !frk_dyn.dyn {
  %sa = "frk_dyn.unwrap"(%a) {tag = 3 : i64} : (!frk_dyn.dyn) -> !frk_bstr.str
  %sb = "frk_dyn.unwrap"(%b) {tag = 3 : i64} : (!frk_dyn.dyn) -> !frk_bstr.str
  %e = "frk_bstr.eq"(%sa, %sb) : (!frk_bstr.str, !frk_bstr.str) -> i1
  %d = "frk_dyn.wrap"(%e) {tag = 1 : i64} : (i1) -> !frk_dyn.dyn
  return %d : !frk_dyn.dyn
}

// R7RS (substring s start end): 0-based, end-exclusive — adapted to
// bstr.sub's Lua convention (1-based, inclusive): sub(start+1, end).
func.func @__scm_substr(%s: !frk_dyn.dyn, %from: !frk_dyn.dyn, %to: !frk_dyn.dyn) -> !frk_dyn.dyn {
  %b = "frk_dyn.unwrap"(%s) {tag = 3 : i64} : (!frk_dyn.dyn) -> !frk_bstr.str
  %ff = "frk_dyn.unwrap"(%from) {tag = 2 : i64} : (!frk_dyn.dyn) -> f64
  %tf = "frk_dyn.unwrap"(%to) {tag = 2 : i64} : (!frk_dyn.dyn) -> f64
  %fi = arith.fptosi %ff : f64 to i64
  %ti = arith.fptosi %tf : f64 to i64
  %one = arith.constant 1 : i64
  %fi1 = arith.addi %fi, %one : i64
  %r = "frk_bstr.sub"(%b, %fi1, %ti) : (!frk_bstr.str, i64, i64) -> !frk_bstr.str
  %d = "frk_dyn.wrap"(%r) {tag = 3 : i64} : (!frk_bstr.str) -> !frk_dyn.dyn
  return %d : !frk_dyn.dyn
}

// Vectors (M31, D-077): TAG_VECTOR = 7 over !frk_mem.arr<!frk_dyn.dyn>.
func.func @__scm_vec_ref(%v: !frk_dyn.dyn, %i: !frk_dyn.dyn) -> !frk_dyn.dyn {
  %a = "frk_dyn.unwrap"(%v) {tag = 7 : i64} : (!frk_dyn.dyn) -> !frk_mem.arr<!frk_dyn.dyn>
  %f = "frk_dyn.unwrap"(%i) {tag = 2 : i64} : (!frk_dyn.dyn) -> f64
  %n = arith.fptosi %f : f64 to i64
  %e = "frk_mem.array_get"(%a, %n) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
  return %e : !frk_dyn.dyn
}

func.func @__scm_vec_set(%v: !frk_dyn.dyn, %i: !frk_dyn.dyn, %x: !frk_dyn.dyn) -> !frk_dyn.dyn {
  %a = "frk_dyn.unwrap"(%v) {tag = 7 : i64} : (!frk_dyn.dyn) -> !frk_mem.arr<!frk_dyn.dyn>
  %f = "frk_dyn.unwrap"(%i) {tag = 2 : i64} : (!frk_dyn.dyn) -> f64
  %n = arith.fptosi %f : f64 to i64
  "frk_mem.array_set"(%a, %n, %x) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
  %z = arith.constant 0 : i64
  %nil = "frk_dyn.wrap"(%z) {tag = 0 : i64} : (i64) -> !frk_dyn.dyn
  return %nil : !frk_dyn.dyn
}

func.func @__scm_vec_len(%v: !frk_dyn.dyn) -> !frk_dyn.dyn {
  %a = "frk_dyn.unwrap"(%v) {tag = 7 : i64} : (!frk_dyn.dyn) -> !frk_mem.arr<!frk_dyn.dyn>
  %n = "frk_mem.array_len"(%a) : (!frk_mem.arr<!frk_dyn.dyn>) -> i64
  %f = arith.sitofp %n : i64 to f64
  %d = "frk_dyn.wrap"(%f) {tag = 2 : i64} : (f64) -> !frk_dyn.dyn
  return %d : !frk_dyn.dyn
}

// make-vector with a REQUIRED fill (R7RS's unspecified default is
// refused, D-077): tail-recursive fill.
func.func @__scm_vec_fill(%a: !frk_mem.arr<!frk_dyn.dyn>, %i: i64, %n: i64, %fill: !frk_dyn.dyn) -> !frk_dyn.dyn {
  %done = arith.cmpi sge, %i, %n : i64
  cf.cond_br %done, ^ret, ^body
^body:
  "frk_mem.array_set"(%a, %i, %fill) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
  %one = arith.constant 1 : i64
  %next = arith.addi %i, %one : i64
  %r = func.call @__scm_vec_fill(%a, %next, %n, %fill) : (!frk_mem.arr<!frk_dyn.dyn>, i64, i64, !frk_dyn.dyn) -> !frk_dyn.dyn
  return %r : !frk_dyn.dyn
^ret:
  %z = arith.constant 0 : i64
  %nil = "frk_dyn.wrap"(%z) {tag = 0 : i64} : (i64) -> !frk_dyn.dyn
  return %nil : !frk_dyn.dyn
}

func.func @__scm_make_vector(%k: !frk_dyn.dyn, %fill: !frk_dyn.dyn) -> !frk_dyn.dyn {
  %f = "frk_dyn.unwrap"(%k) {tag = 2 : i64} : (!frk_dyn.dyn) -> f64
  %n = arith.fptosi %f : f64 to i64
  %a = "frk_mem.array_new"(%n) : (i64) -> !frk_mem.arr<!frk_dyn.dyn>
  %z = arith.constant 0 : i64
  %ignored = func.call @__scm_vec_fill(%a, %z, %n, %fill) : (!frk_mem.arr<!frk_dyn.dyn>, i64, i64, !frk_dyn.dyn) -> !frk_dyn.dyn
  %d = "frk_dyn.wrap"(%a) {tag = 7 : i64} : (!frk_mem.arr<!frk_dyn.dyn>) -> !frk_dyn.dyn
  return %d : !frk_dyn.dyn
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

// ---- M26 (D-071) / M33 (D-081.4): the R7RS exception wrappers.
// STATIC functions — per-site closures differ only in their env (the
// captured user handler / thunk), so one pair serves every
// with-exception-handler.

// The CLAUSE: (h, pack[(flag . e), κ]) — every perform{"exn"} carries
// a FLAGGED PAIR (D-081.4: #t = raise-continuable, #f = plain raise;
// the flag travels WITH the value because after-thunks run between
// perform and handle and would clobber any cell). Apply h to [e];
// if h ESCAPED (pending set) return early — the in-flight abort wins
// (perform_end preserves it). Then the flag decides: #t → apply κ to
// [r], the clause's return IS the resume value (D-069); #f → the
// handler RETURNED from a non-continuable raise — the deterministic
// D-081 trap (chibi raises a secondary exception; we refuse loud).
func.func @__scm_exn_clause(%h: !frk_dyn.dyn, %pack: !frk_mem.arr<!frk_dyn.dyn>) -> !frk_mem.arr<!frk_dyn.dyn> {
  %c0 = arith.constant 0 : i64
  %we = func.call @__scm_arg(%pack, %c0) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
  %c1 = arith.constant 1 : i64
  %kd = func.call @__scm_arg(%pack, %c1) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
  %wcell = "frk_dyn.unwrap"(%we) {tag = 6 : i64} : (!frk_dyn.dyn) -> !frk_mem.box<!frk_adt.product<[!frk_dyn.dyn, !frk_dyn.dyn]>>
  %flagd = "frk_mem.field_get"(%wcell) {field = 0 : i64} : (!frk_mem.box<!frk_adt.product<[!frk_dyn.dyn, !frk_dyn.dyn]>>) -> !frk_dyn.dyn
  %e = "frk_mem.field_get"(%wcell) {field = 1 : i64} : (!frk_mem.box<!frk_adt.product<[!frk_dyn.dyn, !frk_dyn.dyn]>>) -> !frk_dyn.dyn
  %hf = "frk_dyn.unwrap"(%h) {tag = 5 : i64} : (!frk_dyn.dyn) -> !frk_closure.fn<[!frk_mem.arr<!frk_dyn.dyn>], [!frk_mem.arr<!frk_dyn.dyn>]>
  %hp = "frk_mem.array_new"(%c1) : (i64) -> !frk_mem.arr<!frk_dyn.dyn>
  "frk_mem.array_set"(%hp, %c0, %e) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
  %pe = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
  %pp = "frk_adt.product_snoc"(%pe, %hp) : (!frk_adt.product<[]>, !frk_mem.arr<!frk_dyn.dyn>) -> !frk_adt.product<[!frk_mem.arr<!frk_dyn.dyn>]>
  %hr = "frk_closure.apply"(%hf, %pp) : (!frk_closure.fn<[!frk_mem.arr<!frk_dyn.dyn>], [!frk_mem.arr<!frk_dyn.dyn>]>, !frk_adt.product<[!frk_mem.arr<!frk_dyn.dyn>]>) -> !frk_mem.arr<!frk_dyn.dyn>
  %pend = "frk_ctl.pending"() : () -> i64
  %z = arith.constant 0 : i64
  %escaped = arith.cmpi ne, %pend, %z : i64
  cf.cond_br %escaped, ^divert, ^flagcheck
^divert:
  %ep = "frk_mem.array_new"(%z) : (i64) -> !frk_mem.arr<!frk_dyn.dyn>
  return %ep : !frk_mem.arr<!frk_dyn.dyn>
^flagcheck:
  %flag = "frk_dyn.unwrap"(%flagd) {tag = 1 : i64} : (!frk_dyn.dyn) -> i1
  cf.cond_br %flag, ^resume, ^returned
^returned:
  %code = arith.constant 1 : i64
  func.call @frk_rt_scm_trap(%code) : (i64) -> ()
  %dead = "frk_mem.array_new"(%z) : (i64) -> !frk_mem.arr<!frk_dyn.dyn>
  return %dead : !frk_mem.arr<!frk_dyn.dyn>
^resume:
  %r = func.call @__scm_arg(%hr, %c0) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
  %kf = "frk_dyn.unwrap"(%kd) {tag = 5 : i64} : (!frk_dyn.dyn) -> !frk_closure.fn<[!frk_mem.arr<!frk_dyn.dyn>], [!frk_mem.arr<!frk_dyn.dyn>]>
  %kp = "frk_mem.array_new"(%c1) : (i64) -> !frk_mem.arr<!frk_dyn.dyn>
  "frk_mem.array_set"(%kp, %c0, %r) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
  %pe2 = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
  %pp2 = "frk_adt.product_snoc"(%pe2, %kp) : (!frk_adt.product<[]>, !frk_mem.arr<!frk_dyn.dyn>) -> !frk_adt.product<[!frk_mem.arr<!frk_dyn.dyn>]>
  %kr = "frk_closure.apply"(%kf, %pp2) : (!frk_closure.fn<[!frk_mem.arr<!frk_dyn.dyn>], [!frk_mem.arr<!frk_dyn.dyn>]>, !frk_adt.product<[!frk_mem.arr<!frk_dyn.dyn>]>) -> !frk_mem.arr<!frk_dyn.dyn>
  return %kr : !frk_mem.arr<!frk_dyn.dyn>
}

// ---- M33 (D-081.2): parameters. A parameter object is a uniform
// pack-fn closure over a mutable state PAIR — (cons value '()), the
// cdr reserved for the fenced converter. Protocol BY PACK LENGTH,
// exactly two live arms (no dead IR): len 0 → get (car); len 2 →
// RAW set (the parameterize desugar's bind/restore spelling — never
// converts, so restores can't double-convert when the converter
// lands); anything else → the D-081 arity trap ((p v) user setter
// spellings are outside the surface). INVARIANT (D-081.5): the state
// pair's dyn is wrapped ONCE by __scm_cons — no path here re-wraps a
// tag-6 payload (interp eq?-identity lives in the wrapper Rc).

func.func @__scm_param_fn(%cell: !frk_dyn.dyn, %pack: !frk_mem.arr<!frk_dyn.dyn>) -> !frk_mem.arr<!frk_dyn.dyn> {
  %len = "frk_mem.array_len"(%pack) : (!frk_mem.arr<!frk_dyn.dyn>) -> i64
  %z = arith.constant 0 : i64
  %one = arith.constant 1 : i64
  %isget = arith.cmpi eq, %len, %z : i64
  cf.cond_br %isget, ^get, ^checkset
^get:
  %v = func.call @__scm_car(%cell) : (!frk_dyn.dyn) -> !frk_dyn.dyn
  %gp = "frk_mem.array_new"(%one) : (i64) -> !frk_mem.arr<!frk_dyn.dyn>
  "frk_mem.array_set"(%gp, %z, %v) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
  return %gp : !frk_mem.arr<!frk_dyn.dyn>
^checkset:
  %two = arith.constant 2 : i64
  %isset = arith.cmpi eq, %len, %two : i64
  cf.cond_br %isset, ^set, ^trap
^set:
  %nv = "frk_mem.array_get"(%pack, %z) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
  %ignored = func.call @__scm_setcar(%cell, %nv) : (!frk_dyn.dyn, !frk_dyn.dyn) -> !frk_dyn.dyn
  %sp = "frk_mem.array_new"(%one) : (i64) -> !frk_mem.arr<!frk_dyn.dyn>
  %nil = "frk_dyn.wrap"(%z) {tag = 0 : i64} : (i64) -> !frk_dyn.dyn
  "frk_mem.array_set"(%sp, %z, %nil) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
  return %sp : !frk_mem.arr<!frk_dyn.dyn>
^trap:
  %code = arith.constant 3 : i64
  func.call @frk_rt_scm_trap(%code) : (i64) -> ()
  %dead = "frk_mem.array_new"(%z) : (i64) -> !frk_mem.arr<!frk_dyn.dyn>
  return %dead : !frk_mem.arr<!frk_dyn.dyn>
}

func.func @__scm_param_make(%init: !frk_dyn.dyn) -> !frk_dyn.dyn {
  %z = arith.constant 0 : i64
  %nil = "frk_dyn.wrap"(%z) {tag = 0 : i64} : (i64) -> !frk_dyn.dyn
  %cell = func.call @__scm_cons(%init, %nil) : (!frk_dyn.dyn, !frk_dyn.dyn) -> !frk_dyn.dyn
  %pe = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
  %env = "frk_adt.product_snoc"(%pe, %cell) : (!frk_adt.product<[]>, !frk_dyn.dyn) -> !frk_adt.product<[!frk_dyn.dyn]>
  %cl = "frk_closure.make"(%env) {callee = @__scm_param_fn} : (!frk_adt.product<[!frk_dyn.dyn]>) -> !frk_closure.fn<[!frk_mem.arr<!frk_dyn.dyn>], [!frk_mem.arr<!frk_dyn.dyn>]>
  %d = "frk_dyn.wrap"(%cl) {tag = 5 : i64} : (!frk_closure.fn<[!frk_mem.arr<!frk_dyn.dyn>], [!frk_mem.arr<!frk_dyn.dyn>]>) -> !frk_dyn.dyn
  return %d : !frk_dyn.dyn
}

// The prompt-shaped BODY: (t, token) → apply the captured thunk with
// an empty pack; guard (a crossing escape propagates); head.
func.func @__scm_exn_body(%t: !frk_dyn.dyn, %token: i64) -> !frk_dyn.dyn {
  %tf = "frk_dyn.unwrap"(%t) {tag = 5 : i64} : (!frk_dyn.dyn) -> !frk_closure.fn<[!frk_mem.arr<!frk_dyn.dyn>], [!frk_mem.arr<!frk_dyn.dyn>]>
  %c0 = arith.constant 0 : i64
  %ep = "frk_mem.array_new"(%c0) : (i64) -> !frk_mem.arr<!frk_dyn.dyn>
  %pe = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
  %pp = "frk_adt.product_snoc"(%pe, %ep) : (!frk_adt.product<[]>, !frk_mem.arr<!frk_dyn.dyn>) -> !frk_adt.product<[!frk_mem.arr<!frk_dyn.dyn>]>
  %rp = "frk_closure.apply"(%tf, %pp) : (!frk_closure.fn<[!frk_mem.arr<!frk_dyn.dyn>], [!frk_mem.arr<!frk_dyn.dyn>]>, !frk_adt.product<[!frk_mem.arr<!frk_dyn.dyn>]>) -> !frk_mem.arr<!frk_dyn.dyn>
  %pend = "frk_ctl.pending"() : () -> i64
  %escaped = arith.cmpi ne, %pend, %c0 : i64
  cf.cond_br %escaped, ^divert, ^done
^divert:
  %nil = "frk_dyn.wrap"(%c0) {tag = 0 : i64} : (i64) -> !frk_dyn.dyn
  return %nil : !frk_dyn.dyn
^done:
  %r = func.call @__scm_arg(%rp, %c0) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
  return %r : !frk_dyn.dyn
}
