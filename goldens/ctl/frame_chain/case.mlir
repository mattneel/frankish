// M35 Q2 SPIKE (panel scratch — NOT a shipping golden): frame records
// for re-entrant one-shot continuations as ORDINARY managed values.
//
// Proves, on the reference interpreter AND the jit/aot runners:
//   1. A frame record {fn identity, resume-state id, env boxes} is an
//      ordinary closure over an ordinary product — two styles:
//        @inner : env = [i64 state, box acc]      (raw state field;
//                 functional refresh on capture)
//        @outer : env = [box state, box chain, box partial]
//                 (mutable-box state; in-place update)
//   2. The frame (wrapped TAG_FUN) is stored in an ORDINARY box<dyn>
//      (main's %kbox — the "stored continuation" in a plain cell),
//      retrieved later, and re-entered INDIRECTLY through the one
//      uniform lua convention fn<[arr<dyn>],[arr<dyn>]> — natively
//      (ptr,ptr)->ptr.
//   3. The chain composes innermost-out at capture (inner's frame
//      travels in outer's frame's chain box) and is walked
//      OUTERMOST-IN at resume (outer re-applies inner), with pending
//      work in the middle frame running AFTER the resumed callee
//      returns — the clause shape today's tail-resume cannot express.
//   4. One-shot consumption: outer nils its chain box after the walk.
//
// Numbers: inner captures acc=10 at suspend; resume value 32 →
// inner returns 42; outer's pending partial=7 → main sees 49.

// ---- inner: raw-i64-state frame, functionally refreshed ----
func.func @inner(%env: !frk_closure.envref, %pack: !frk_mem.arr<!frk_dyn.dyn>) -> !frk_mem.arr<!frk_dyn.dyn> {
  %state = "frk_closure.env_load"(%env) {index = 0 : i64, env = !frk_adt.product<[i64, !frk_mem.box<!frk_dyn.dyn>]>} : (!frk_closure.envref) -> i64
  %accb = "frk_closure.env_load"(%env) {index = 1 : i64, env = !frk_adt.product<[i64, !frk_mem.box<!frk_dyn.dyn>]>} : (!frk_closure.envref) -> !frk_mem.box<!frk_dyn.dyn>
  %c0 = arith.constant 0 : i64
  %is0 = arith.cmpi eq, %state, %c0 : i64
  cf.cond_br %is0, ^start, ^resume
^start:  // state 0: capture pack[0] into acc, refresh frame at state 1
  %i0 = arith.constant 0 : i64
  %a0 = "frk_mem.array_get"(%pack, %i0) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
  "frk_mem.box_set"(%accb, %a0) : (!frk_mem.box<!frk_dyn.dyn>, !frk_dyn.dyn) -> ()
  %one = arith.constant 1 : i64
  %fp0 = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
  %fp1 = "frk_adt.product_snoc"(%fp0, %one) : (!frk_adt.product<[]>, i64) -> !frk_adt.product<[i64]>
  %fp2 = "frk_adt.product_snoc"(%fp1, %accb) : (!frk_adt.product<[i64]>, !frk_mem.box<!frk_dyn.dyn>) -> !frk_adt.product<[i64, !frk_mem.box<!frk_dyn.dyn>]>
  %k = "frk_closure.make"(%fp2) {callee = @inner} : (!frk_adt.product<[i64, !frk_mem.box<!frk_dyn.dyn>]>) -> !frk_closure.fn<[!frk_mem.arr<!frk_dyn.dyn>], [!frk_mem.arr<!frk_dyn.dyn>]>
  %kd = "frk_dyn.wrap"(%k) {tag = 5 : i64} : (!frk_closure.fn<[!frk_mem.arr<!frk_dyn.dyn>], [!frk_mem.arr<!frk_dyn.dyn>]>) -> !frk_dyn.dyn
  %c1 = arith.constant 1 : i64
  %sp = "frk_mem.array_new"(%c1) : (i64) -> !frk_mem.arr<!frk_dyn.dyn>
  %z = arith.constant 0 : i64
  "frk_mem.array_set"(%sp, %z, %kd) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
  return %sp : !frk_mem.arr<!frk_dyn.dyn>
^resume:  // state 1: acc + resume-value
  %j0 = arith.constant 0 : i64
  %rv = "frk_mem.array_get"(%pack, %j0) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
  %rn = "frk_dyn.unwrap"(%rv) {tag = 2 : i64} : (!frk_dyn.dyn) -> i64
  %ad = "frk_mem.box_get"(%accb) : (!frk_mem.box<!frk_dyn.dyn>) -> !frk_dyn.dyn
  %an = "frk_dyn.unwrap"(%ad) {tag = 2 : i64} : (!frk_dyn.dyn) -> i64
  %sum = arith.addi %an, %rn : i64
  %sd = "frk_dyn.wrap"(%sum) {tag = 2 : i64} : (i64) -> !frk_dyn.dyn
  %c1b = arith.constant 1 : i64
  %rp = "frk_mem.array_new"(%c1b) : (i64) -> !frk_mem.arr<!frk_dyn.dyn>
  %zb = arith.constant 0 : i64
  "frk_mem.array_set"(%rp, %zb, %sd) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
  return %rp : !frk_mem.arr<!frk_dyn.dyn>
}

// ---- outer: box-state frame, in-place update; chain holder ----
func.func @outer(%env: !frk_closure.envref, %pack: !frk_mem.arr<!frk_dyn.dyn>) -> !frk_mem.arr<!frk_dyn.dyn> {
  %sb = "frk_closure.env_load"(%env) {index = 0 : i64, env = !frk_adt.product<[!frk_mem.box<!frk_dyn.dyn>, !frk_mem.box<!frk_dyn.dyn>, !frk_mem.box<!frk_dyn.dyn>]>} : (!frk_closure.envref) -> !frk_mem.box<!frk_dyn.dyn>
  %cb = "frk_closure.env_load"(%env) {index = 1 : i64, env = !frk_adt.product<[!frk_mem.box<!frk_dyn.dyn>, !frk_mem.box<!frk_dyn.dyn>, !frk_mem.box<!frk_dyn.dyn>]>} : (!frk_closure.envref) -> !frk_mem.box<!frk_dyn.dyn>
  %pb = "frk_closure.env_load"(%env) {index = 2 : i64, env = !frk_adt.product<[!frk_mem.box<!frk_dyn.dyn>, !frk_mem.box<!frk_dyn.dyn>, !frk_mem.box<!frk_dyn.dyn>]>} : (!frk_closure.envref) -> !frk_mem.box<!frk_dyn.dyn>
  %sd = "frk_mem.box_get"(%sb) : (!frk_mem.box<!frk_dyn.dyn>) -> !frk_dyn.dyn
  %s = "frk_dyn.unwrap"(%sd) {tag = 2 : i64} : (!frk_dyn.dyn) -> i64
  %c0 = arith.constant 0 : i64
  %is0 = arith.cmpi eq, %s, %c0 : i64
  cf.cond_br %is0, ^start, ^resume
^start:
  // pending work this frame owes after the resume: partial = 7
  %seven = arith.constant 7 : i64
  %pd = "frk_dyn.wrap"(%seven) {tag = 2 : i64} : (i64) -> !frk_dyn.dyn
  "frk_mem.box_set"(%pb, %pd) : (!frk_mem.box<!frk_dyn.dyn>, !frk_dyn.dyn) -> ()
  // build inner's initial frame {state=0, acc box} and CALL it (non-tail)
  %izero = arith.constant 0 : i64
  %nil0 = arith.constant 0 : i64
  %nild = "frk_dyn.wrap"(%nil0) {tag = 0 : i64} : (i64) -> !frk_dyn.dyn
  %ab = "frk_mem.box_new"(%nild) : (!frk_dyn.dyn) -> !frk_mem.box<!frk_dyn.dyn>
  %ip0 = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
  %ip1 = "frk_adt.product_snoc"(%ip0, %izero) : (!frk_adt.product<[]>, i64) -> !frk_adt.product<[i64]>
  %ip2 = "frk_adt.product_snoc"(%ip1, %ab) : (!frk_adt.product<[i64]>, !frk_mem.box<!frk_dyn.dyn>) -> !frk_adt.product<[i64, !frk_mem.box<!frk_dyn.dyn>]>
  %ik = "frk_closure.make"(%ip2) {callee = @inner} : (!frk_adt.product<[i64, !frk_mem.box<!frk_dyn.dyn>]>) -> !frk_closure.fn<[!frk_mem.arr<!frk_dyn.dyn>], [!frk_mem.arr<!frk_dyn.dyn>]>
  %ten = arith.constant 10 : i64
  %tend = "frk_dyn.wrap"(%ten) {tag = 2 : i64} : (i64) -> !frk_dyn.dyn
  %c1 = arith.constant 1 : i64
  %ap = "frk_mem.array_new"(%c1) : (i64) -> !frk_mem.arr<!frk_dyn.dyn>
  %z0 = arith.constant 0 : i64
  "frk_mem.array_set"(%ap, %z0, %tend) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
  %aw0 = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
  %aw1 = "frk_adt.product_snoc"(%aw0, %ap) : (!frk_adt.product<[]>, !frk_mem.arr<!frk_dyn.dyn>) -> !frk_adt.product<[!frk_mem.arr<!frk_dyn.dyn>]>
  %rp = "frk_closure.apply"(%ik, %aw1) : (!frk_closure.fn<[!frk_mem.arr<!frk_dyn.dyn>], [!frk_mem.arr<!frk_dyn.dyn>]>, !frk_adt.product<[!frk_mem.arr<!frk_dyn.dyn>]>) -> !frk_mem.arr<!frk_dyn.dyn>
  // GUARD COLD PATH (suspended outcome, simulated deterministically):
  // link the callee's frame into MY chain box, flip MY state, return
  // MY frame — the chain builds innermost-out through returns.
  %g0 = arith.constant 0 : i64
  %kfd = "frk_mem.array_get"(%rp, %g0) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
  "frk_mem.box_set"(%cb, %kfd) : (!frk_mem.box<!frk_dyn.dyn>, !frk_dyn.dyn) -> ()
  %one = arith.constant 1 : i64
  %oned = "frk_dyn.wrap"(%one) {tag = 2 : i64} : (i64) -> !frk_dyn.dyn
  "frk_mem.box_set"(%sb, %oned) : (!frk_mem.box<!frk_dyn.dyn>, !frk_dyn.dyn) -> ()
  %mp0 = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
  %mp1 = "frk_adt.product_snoc"(%mp0, %sb) : (!frk_adt.product<[]>, !frk_mem.box<!frk_dyn.dyn>) -> !frk_adt.product<[!frk_mem.box<!frk_dyn.dyn>]>
  %mp2 = "frk_adt.product_snoc"(%mp1, %cb) : (!frk_adt.product<[!frk_mem.box<!frk_dyn.dyn>]>, !frk_mem.box<!frk_dyn.dyn>) -> !frk_adt.product<[!frk_mem.box<!frk_dyn.dyn>, !frk_mem.box<!frk_dyn.dyn>]>
  %mp3 = "frk_adt.product_snoc"(%mp2, %pb) : (!frk_adt.product<[!frk_mem.box<!frk_dyn.dyn>, !frk_mem.box<!frk_dyn.dyn>]>, !frk_mem.box<!frk_dyn.dyn>) -> !frk_adt.product<[!frk_mem.box<!frk_dyn.dyn>, !frk_mem.box<!frk_dyn.dyn>, !frk_mem.box<!frk_dyn.dyn>]>
  %mk = "frk_closure.make"(%mp3) {callee = @outer} : (!frk_adt.product<[!frk_mem.box<!frk_dyn.dyn>, !frk_mem.box<!frk_dyn.dyn>, !frk_mem.box<!frk_dyn.dyn>]>) -> !frk_closure.fn<[!frk_mem.arr<!frk_dyn.dyn>], [!frk_mem.arr<!frk_dyn.dyn>]>
  %mkd = "frk_dyn.wrap"(%mk) {tag = 5 : i64} : (!frk_closure.fn<[!frk_mem.arr<!frk_dyn.dyn>], [!frk_mem.arr<!frk_dyn.dyn>]>) -> !frk_dyn.dyn
  %c1s = arith.constant 1 : i64
  %sp = "frk_mem.array_new"(%c1s) : (i64) -> !frk_mem.arr<!frk_dyn.dyn>
  %zs = arith.constant 0 : i64
  "frk_mem.array_set"(%sp, %zs, %mkd) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
  return %sp : !frk_mem.arr<!frk_dyn.dyn>
^resume:
  // outermost-in walk: retrieve the stored inner frame, CONSUME it
  // (one-shot: nil the chain box), re-enter it with the resume pack.
  %kd = "frk_mem.box_get"(%cb) : (!frk_mem.box<!frk_dyn.dyn>) -> !frk_dyn.dyn
  %niln = arith.constant 0 : i64
  %nild2 = "frk_dyn.wrap"(%niln) {tag = 0 : i64} : (i64) -> !frk_dyn.dyn
  "frk_mem.box_set"(%cb, %nild2) : (!frk_mem.box<!frk_dyn.dyn>, !frk_dyn.dyn) -> ()
  %k = "frk_dyn.unwrap"(%kd) {tag = 5 : i64} : (!frk_dyn.dyn) -> !frk_closure.fn<[!frk_mem.arr<!frk_dyn.dyn>], [!frk_mem.arr<!frk_dyn.dyn>]>
  %rw0 = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
  %rw1 = "frk_adt.product_snoc"(%rw0, %pack) : (!frk_adt.product<[]>, !frk_mem.arr<!frk_dyn.dyn>) -> !frk_adt.product<[!frk_mem.arr<!frk_dyn.dyn>]>
  %irp = "frk_closure.apply"(%k, %rw1) : (!frk_closure.fn<[!frk_mem.arr<!frk_dyn.dyn>], [!frk_mem.arr<!frk_dyn.dyn>]>, !frk_adt.product<[!frk_mem.arr<!frk_dyn.dyn>]>) -> !frk_mem.arr<!frk_dyn.dyn>
  // PENDING WORK AFTER THE RESUMED CALLEE RETURNS (the re-entrant
  // clause shape): result + partial.
  %h0 = arith.constant 0 : i64
  %ivd = "frk_mem.array_get"(%irp, %h0) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
  %iv = "frk_dyn.unwrap"(%ivd) {tag = 2 : i64} : (!frk_dyn.dyn) -> i64
  %pd2 = "frk_mem.box_get"(%pb) : (!frk_mem.box<!frk_dyn.dyn>) -> !frk_dyn.dyn
  %pn = "frk_dyn.unwrap"(%pd2) {tag = 2 : i64} : (!frk_dyn.dyn) -> i64
  %tot = arith.addi %iv, %pn : i64
  %td = "frk_dyn.wrap"(%tot) {tag = 2 : i64} : (i64) -> !frk_dyn.dyn
  %c1r = arith.constant 1 : i64
  %fp = "frk_mem.array_new"(%c1r) : (i64) -> !frk_mem.arr<!frk_dyn.dyn>
  %zr = arith.constant 0 : i64
  "frk_mem.array_set"(%fp, %zr, %td) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
  return %fp : !frk_mem.arr<!frk_dyn.dyn>
}

func.func @main() -> i64 attributes {llvm.emit_c_interface} {
  // outer's env boxes
  %z = arith.constant 0 : i64
  %zd = "frk_dyn.wrap"(%z) {tag = 2 : i64} : (i64) -> !frk_dyn.dyn
  %sb = "frk_mem.box_new"(%zd) : (!frk_dyn.dyn) -> !frk_mem.box<!frk_dyn.dyn>
  %n0 = arith.constant 0 : i64
  %nd = "frk_dyn.wrap"(%n0) {tag = 0 : i64} : (i64) -> !frk_dyn.dyn
  %cb = "frk_mem.box_new"(%nd) : (!frk_dyn.dyn) -> !frk_mem.box<!frk_dyn.dyn>
  %n1 = arith.constant 0 : i64
  %nd2 = "frk_dyn.wrap"(%n1) {tag = 0 : i64} : (i64) -> !frk_dyn.dyn
  %pb = "frk_mem.box_new"(%nd2) : (!frk_dyn.dyn) -> !frk_mem.box<!frk_dyn.dyn>
  %e0 = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
  %e1 = "frk_adt.product_snoc"(%e0, %sb) : (!frk_adt.product<[]>, !frk_mem.box<!frk_dyn.dyn>) -> !frk_adt.product<[!frk_mem.box<!frk_dyn.dyn>]>
  %e2 = "frk_adt.product_snoc"(%e1, %cb) : (!frk_adt.product<[!frk_mem.box<!frk_dyn.dyn>]>, !frk_mem.box<!frk_dyn.dyn>) -> !frk_adt.product<[!frk_mem.box<!frk_dyn.dyn>, !frk_mem.box<!frk_dyn.dyn>]>
  %e3 = "frk_adt.product_snoc"(%e2, %pb) : (!frk_adt.product<[!frk_mem.box<!frk_dyn.dyn>, !frk_mem.box<!frk_dyn.dyn>]>, !frk_mem.box<!frk_dyn.dyn>) -> !frk_adt.product<[!frk_mem.box<!frk_dyn.dyn>, !frk_mem.box<!frk_dyn.dyn>, !frk_mem.box<!frk_dyn.dyn>]>
  %ok = "frk_closure.make"(%e3) {callee = @outer} : (!frk_adt.product<[!frk_mem.box<!frk_dyn.dyn>, !frk_mem.box<!frk_dyn.dyn>, !frk_mem.box<!frk_dyn.dyn>]>) -> !frk_closure.fn<[!frk_mem.arr<!frk_dyn.dyn>], [!frk_mem.arr<!frk_dyn.dyn>]>
  // first entry: outer(pack) → suspended, returns the chain head
  %c1 = arith.constant 1 : i64
  %ap = "frk_mem.array_new"(%c1) : (i64) -> !frk_mem.arr<!frk_dyn.dyn>
  %z1 = arith.constant 0 : i64
  %dummy = arith.constant 0 : i64
  %dd = "frk_dyn.wrap"(%dummy) {tag = 0 : i64} : (i64) -> !frk_dyn.dyn
  "frk_mem.array_set"(%ap, %z1, %dd) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
  %w0 = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
  %w1 = "frk_adt.product_snoc"(%w0, %ap) : (!frk_adt.product<[]>, !frk_mem.arr<!frk_dyn.dyn>) -> !frk_adt.product<[!frk_mem.arr<!frk_dyn.dyn>]>
  %sp = "frk_closure.apply"(%ok, %w1) : (!frk_closure.fn<[!frk_mem.arr<!frk_dyn.dyn>], [!frk_mem.arr<!frk_dyn.dyn>]>, !frk_adt.product<[!frk_mem.arr<!frk_dyn.dyn>]>) -> !frk_mem.arr<!frk_dyn.dyn>
  %g0 = arith.constant 0 : i64
  %kfd = "frk_mem.array_get"(%sp, %g0) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
  // THE STORED CONTINUATION: an ordinary box<dyn> holds the frame.
  %kbox = "frk_mem.box_new"(%kfd) : (!frk_dyn.dyn) -> !frk_mem.box<!frk_dyn.dyn>
  // ... arbitrary time passes ... retrieve and resume with 32.
  %kd = "frk_mem.box_get"(%kbox) : (!frk_mem.box<!frk_dyn.dyn>) -> !frk_dyn.dyn
  %k = "frk_dyn.unwrap"(%kd) {tag = 5 : i64} : (!frk_dyn.dyn) -> !frk_closure.fn<[!frk_mem.arr<!frk_dyn.dyn>], [!frk_mem.arr<!frk_dyn.dyn>]>
  %c1b = arith.constant 1 : i64
  %rp = "frk_mem.array_new"(%c1b) : (i64) -> !frk_mem.arr<!frk_dyn.dyn>
  %z2 = arith.constant 0 : i64
  %tt = arith.constant 32 : i64
  %ttd = "frk_dyn.wrap"(%tt) {tag = 2 : i64} : (i64) -> !frk_dyn.dyn
  "frk_mem.array_set"(%rp, %z2, %ttd) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
  %v0 = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
  %v1 = "frk_adt.product_snoc"(%v0, %rp) : (!frk_adt.product<[]>, !frk_mem.arr<!frk_dyn.dyn>) -> !frk_adt.product<[!frk_mem.arr<!frk_dyn.dyn>]>
  %fin = "frk_closure.apply"(%k, %v1) : (!frk_closure.fn<[!frk_mem.arr<!frk_dyn.dyn>], [!frk_mem.arr<!frk_dyn.dyn>]>, !frk_adt.product<[!frk_mem.arr<!frk_dyn.dyn>]>) -> !frk_mem.arr<!frk_dyn.dyn>
  %f0 = arith.constant 0 : i64
  %fvd = "frk_mem.array_get"(%fin, %f0) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
  %fv = "frk_dyn.unwrap"(%fvd) {tag = 2 : i64} : (!frk_dyn.dyn) -> i64
  return %fv : i64
}
