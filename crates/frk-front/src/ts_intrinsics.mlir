// TS-3b async intrinsics (D-078/D-079) — the seed-module surface's
// fourth frontend. Pure kernel IR: promises are records, the microtask
// queue is a global-cells RING BUFFER of (continuation, value) slot
// pairs. The ring bounds PENDING tasks (256), not lifetime tasks — and
// genuine overflow of either the queue or a promise's subscriber list
// aborts DETERMINISTICALLY via frk_rt_async_trap on both twins (never a
// write past the array, which the native array_set would not catch).
//
// Promise record: box<product<[state f64 (0 pending / 1 resolved),
// value dyn, cbs arr<dyn> (16 subscriber slots), cbcount f64]>>.
// Continuations are pack closures fn<[arr<dyn>],[arr<dyn>]> wrapped as
// tag-5 dyns; the drain applies each with a one-element pack [value].

func.func private @frk_rt_async_trap(i64)

"frk_mem.global_decl"() {sym = "ts_qinit", cell = f64} : () -> ()
"frk_mem.global_decl"() {sym = "ts_qhead", cell = f64} : () -> ()
"frk_mem.global_decl"() {sym = "ts_qtail", cell = f64} : () -> ()
"frk_mem.global_decl"() {sym = "ts_queue", cell = !frk_mem.arr<!frk_dyn.dyn>} : () -> ()

func.func @__ts_qensure() {
  %flag = "frk_mem.global_get"() {sym = "ts_qinit"} : () -> !frk_mem.box<f64>
  %f = "frk_mem.box_get"(%flag) : (!frk_mem.box<f64>) -> f64
  %zero = arith.constant 0.0 : f64
  %uninit = arith.cmpf oeq, %f, %zero : f64
  cf.cond_br %uninit, ^init, ^done
^init:
  %cap = arith.constant 512 : i64
  %storage = "frk_mem.array_new"(%cap) : (i64) -> !frk_mem.arr<!frk_dyn.dyn>
  %cell = "frk_mem.global_get"() {sym = "ts_queue"} : () -> !frk_mem.box<!frk_mem.arr<!frk_dyn.dyn>>
  "frk_mem.box_set"(%cell, %storage) : (!frk_mem.box<!frk_mem.arr<!frk_dyn.dyn>>, !frk_mem.arr<!frk_dyn.dyn>) -> ()
  %one = arith.constant 1.0 : f64
  "frk_mem.box_set"(%flag, %one) : (!frk_mem.box<f64>, f64) -> ()
  cf.br ^done
^done:
  return
}

func.func @__ts_queue_push(%cont: !frk_dyn.dyn, %val: !frk_dyn.dyn) {
  func.call @__ts_qensure() : () -> ()
  %hc = "frk_mem.global_get"() {sym = "ts_qhead"} : () -> !frk_mem.box<f64>
  %tc = "frk_mem.global_get"() {sym = "ts_qtail"} : () -> !frk_mem.box<f64>
  %hf = "frk_mem.box_get"(%hc) : (!frk_mem.box<f64>) -> f64
  %tf = "frk_mem.box_get"(%tc) : (!frk_mem.box<f64>) -> f64
  // Pending = tail - head. Overflow only if 256 tasks queued at once.
  %pending = arith.subf %tf, %hf : f64
  %capf = arith.constant 256.0 : f64
  %over = arith.cmpf oge, %pending, %capf : f64
  cf.cond_br %over, ^trap, ^ok
^trap:
  %kq = arith.constant 1 : i64
  func.call @frk_rt_async_trap(%kq) : (i64) -> ()
  return
^ok:
  %qc = "frk_mem.global_get"() {sym = "ts_queue"} : () -> !frk_mem.box<!frk_mem.arr<!frk_dyn.dyn>>
  %q = "frk_mem.box_get"(%qc) : (!frk_mem.box<!frk_mem.arr<!frk_dyn.dyn>>) -> !frk_mem.arr<!frk_dyn.dyn>
  %t = arith.fptosi %tf : f64 to i64
  %capi = arith.constant 256 : i64
  %ring = arith.remsi %t, %capi : i64
  %two = arith.constant 2 : i64
  %base = arith.muli %ring, %two : i64
  "frk_mem.array_set"(%q, %base, %cont) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
  %one = arith.constant 1 : i64
  %vslot = arith.addi %base, %one : i64
  "frk_mem.array_set"(%q, %vslot, %val) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
  %onef = arith.constant 1.0 : f64
  %tn = arith.addf %tf, %onef : f64
  "frk_mem.box_set"(%tc, %tn) : (!frk_mem.box<f64>, f64) -> ()
  return
}

func.func @__ts_promise_new() -> !frk_mem.box<!frk_adt.product<[f64, !frk_dyn.dyn, !frk_mem.arr<!frk_dyn.dyn>, f64]>> {
  %zero = arith.constant 0.0 : f64
  %zi = arith.constant 0 : i64
  %nil = "frk_dyn.wrap"(%zi) {tag = 0 : i64} : (i64) -> !frk_dyn.dyn
  %cap = arith.constant 16 : i64
  %cbs = "frk_mem.array_new"(%cap) : (i64) -> !frk_mem.arr<!frk_dyn.dyn>
  %e = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
  %p1 = "frk_adt.product_snoc"(%e, %zero) : (!frk_adt.product<[]>, f64) -> !frk_adt.product<[f64]>
  %p2 = "frk_adt.product_snoc"(%p1, %nil) : (!frk_adt.product<[f64]>, !frk_dyn.dyn) -> !frk_adt.product<[f64, !frk_dyn.dyn]>
  %p3 = "frk_adt.product_snoc"(%p2, %cbs) : (!frk_adt.product<[f64, !frk_dyn.dyn]>, !frk_mem.arr<!frk_dyn.dyn>) -> !frk_adt.product<[f64, !frk_dyn.dyn, !frk_mem.arr<!frk_dyn.dyn>]>
  %p4 = "frk_adt.product_snoc"(%p3, %zero) : (!frk_adt.product<[f64, !frk_dyn.dyn, !frk_mem.arr<!frk_dyn.dyn>]>, f64) -> !frk_adt.product<[f64, !frk_dyn.dyn, !frk_mem.arr<!frk_dyn.dyn>, f64]>
  %p = "frk_mem.box_new"(%p4) : (!frk_adt.product<[f64, !frk_dyn.dyn, !frk_mem.arr<!frk_dyn.dyn>, f64]>) -> !frk_mem.box<!frk_adt.product<[f64, !frk_dyn.dyn, !frk_mem.arr<!frk_dyn.dyn>, f64]>>
  return %p : !frk_mem.box<!frk_adt.product<[f64, !frk_dyn.dyn, !frk_mem.arr<!frk_dyn.dyn>, f64]>>
}

// await of a plain value (D-079 rule 2): one tick.
func.func @__ts_await_value(%v: !frk_dyn.dyn, %cont: !frk_dyn.dyn) {
  func.call @__ts_queue_push(%cont, %v) : (!frk_dyn.dyn, !frk_dyn.dyn) -> ()
  return
}

// await of a promise (rule 3): resolved → queue now (one tick);
// pending → subscribe (queued at resolution, subscription order).
func.func @__ts_await_promise(%p: !frk_mem.box<!frk_adt.product<[f64, !frk_dyn.dyn, !frk_mem.arr<!frk_dyn.dyn>, f64]>>, %cont: !frk_dyn.dyn) {
  %state = "frk_mem.field_get"(%p) {field = 0 : i64} : (!frk_mem.box<!frk_adt.product<[f64, !frk_dyn.dyn, !frk_mem.arr<!frk_dyn.dyn>, f64]>>) -> f64
  %zero = arith.constant 0.0 : f64
  %pending = arith.cmpf oeq, %state, %zero : f64
  cf.cond_br %pending, ^subscribe, ^ready
^subscribe:
  %cbs = "frk_mem.field_get"(%p) {field = 2 : i64} : (!frk_mem.box<!frk_adt.product<[f64, !frk_dyn.dyn, !frk_mem.arr<!frk_dyn.dyn>, f64]>>) -> !frk_mem.arr<!frk_dyn.dyn>
  %nf = "frk_mem.field_get"(%p) {field = 3 : i64} : (!frk_mem.box<!frk_adt.product<[f64, !frk_dyn.dyn, !frk_mem.arr<!frk_dyn.dyn>, f64]>>) -> f64
  %cbcap = arith.constant 16.0 : f64
  %full = arith.cmpf oge, %nf, %cbcap : f64
  cf.cond_br %full, ^cbtrap, ^cbok
^cbtrap:
  %ks = arith.constant 2 : i64
  func.call @frk_rt_async_trap(%ks) : (i64) -> ()
  return
^cbok:
  %n = arith.fptosi %nf : f64 to i64
  "frk_mem.array_set"(%cbs, %n, %cont) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
  %onef = arith.constant 1.0 : f64
  %nn = arith.addf %nf, %onef : f64
  "frk_mem.field_set"(%p, %nn) {field = 3 : i64} : (!frk_mem.box<!frk_adt.product<[f64, !frk_dyn.dyn, !frk_mem.arr<!frk_dyn.dyn>, f64]>>, f64) -> ()
  return
^ready:
  %v = "frk_mem.field_get"(%p) {field = 1 : i64} : (!frk_mem.box<!frk_adt.product<[f64, !frk_dyn.dyn, !frk_mem.arr<!frk_dyn.dyn>, f64]>>) -> !frk_dyn.dyn
  func.call @__ts_queue_push(%cont, %v) : (!frk_dyn.dyn, !frk_dyn.dyn) -> ()
  return
}

func.func @__ts_notify(%cbs: !frk_mem.arr<!frk_dyn.dyn>, %i: i64, %n: i64, %v: !frk_dyn.dyn) {
  %done = arith.cmpi sge, %i, %n : i64
  cf.cond_br %done, ^ret, ^body
^body:
  %cont = "frk_mem.array_get"(%cbs, %i) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
  func.call @__ts_queue_push(%cont, %v) : (!frk_dyn.dyn, !frk_dyn.dyn) -> ()
  %one = arith.constant 1 : i64
  %next = arith.addi %i, %one : i64
  func.call @__ts_notify(%cbs, %next, %n, %v) : (!frk_mem.arr<!frk_dyn.dyn>, i64, i64, !frk_dyn.dyn) -> ()
  return
^ret:
  return
}

// Resolution (rule 4): mark resolved, queue ALL subscribers FIFO —
// zero extra ticks (the panel-certified FulfillPromise collapse).
func.func @__ts_resolve(%p: !frk_mem.box<!frk_adt.product<[f64, !frk_dyn.dyn, !frk_mem.arr<!frk_dyn.dyn>, f64]>>, %v: !frk_dyn.dyn) {
  %one = arith.constant 1.0 : f64
  "frk_mem.field_set"(%p, %one) {field = 0 : i64} : (!frk_mem.box<!frk_adt.product<[f64, !frk_dyn.dyn, !frk_mem.arr<!frk_dyn.dyn>, f64]>>, f64) -> ()
  "frk_mem.field_set"(%p, %v) {field = 1 : i64} : (!frk_mem.box<!frk_adt.product<[f64, !frk_dyn.dyn, !frk_mem.arr<!frk_dyn.dyn>, f64]>>, !frk_dyn.dyn) -> ()
  %cbs = "frk_mem.field_get"(%p) {field = 2 : i64} : (!frk_mem.box<!frk_adt.product<[f64, !frk_dyn.dyn, !frk_mem.arr<!frk_dyn.dyn>, f64]>>) -> !frk_mem.arr<!frk_dyn.dyn>
  %nf = "frk_mem.field_get"(%p) {field = 3 : i64} : (!frk_mem.box<!frk_adt.product<[f64, !frk_dyn.dyn, !frk_mem.arr<!frk_dyn.dyn>, f64]>>) -> f64
  %n = arith.fptosi %nf : f64 to i64
  %zi = arith.constant 0 : i64
  func.call @__ts_notify(%cbs, %zi, %n, %v) : (!frk_mem.arr<!frk_dyn.dyn>, i64, i64, !frk_dyn.dyn) -> ()
  return
}

// The drain (rule 5): pop-head-and-apply until empty. An explicit CFG
// LOOP (not self-recursion) — constant stack regardless of how many
// microtasks run, so a long-lived async program cannot exhaust the
// native stack (found by the M32 review: the in-process JIT runs on a
// small thread stack).
func.func @__ts_drain() {
  func.call @__ts_qensure() : () -> ()
  cf.br ^loop
^loop:
  %hc = "frk_mem.global_get"() {sym = "ts_qhead"} : () -> !frk_mem.box<f64>
  %tc = "frk_mem.global_get"() {sym = "ts_qtail"} : () -> !frk_mem.box<f64>
  %hf = "frk_mem.box_get"(%hc) : (!frk_mem.box<f64>) -> f64
  %tf = "frk_mem.box_get"(%tc) : (!frk_mem.box<f64>) -> f64
  %empty = arith.cmpf oge, %hf, %tf : f64
  cf.cond_br %empty, ^ret, ^pop
^pop:
  %onef = arith.constant 1.0 : f64
  %hn = arith.addf %hf, %onef : f64
  "frk_mem.box_set"(%hc, %hn) : (!frk_mem.box<f64>, f64) -> ()
  %qc = "frk_mem.global_get"() {sym = "ts_queue"} : () -> !frk_mem.box<!frk_mem.arr<!frk_dyn.dyn>>
  %q = "frk_mem.box_get"(%qc) : (!frk_mem.box<!frk_mem.arr<!frk_dyn.dyn>>) -> !frk_mem.arr<!frk_dyn.dyn>
  %h = arith.fptosi %hf : f64 to i64
  %capi = arith.constant 256 : i64
  %ring = arith.remsi %h, %capi : i64
  %two = arith.constant 2 : i64
  %base = arith.muli %ring, %two : i64
  %cont = "frk_mem.array_get"(%q, %base) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
  %onei = arith.constant 1 : i64
  %vslot = arith.addi %base, %onei : i64
  %val = "frk_mem.array_get"(%q, %vslot) : (!frk_mem.arr<!frk_dyn.dyn>, i64) -> !frk_dyn.dyn
  %fn = "frk_dyn.unwrap"(%cont) {tag = 5 : i64} : (!frk_dyn.dyn) -> !frk_closure.fn<[!frk_mem.arr<!frk_dyn.dyn>], [!frk_mem.arr<!frk_dyn.dyn>]>
  %pk = "frk_mem.array_new"(%onei) : (i64) -> !frk_mem.arr<!frk_dyn.dyn>
  %zi = arith.constant 0 : i64
  "frk_mem.array_set"(%pk, %zi, %val) : (!frk_mem.arr<!frk_dyn.dyn>, i64, !frk_dyn.dyn) -> ()
  %pe = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
  %pp = "frk_adt.product_snoc"(%pe, %pk) : (!frk_adt.product<[]>, !frk_mem.arr<!frk_dyn.dyn>) -> !frk_adt.product<[!frk_mem.arr<!frk_dyn.dyn>]>
  %r = "frk_closure.apply"(%fn, %pp) : (!frk_closure.fn<[!frk_mem.arr<!frk_dyn.dyn>], [!frk_mem.arr<!frk_dyn.dyn>]>, !frk_adt.product<[!frk_mem.arr<!frk_dyn.dyn>]>) -> !frk_mem.arr<!frk_dyn.dyn>
  cf.br ^loop
^ret:
  return
}
