// The uniform-signature convention's law golden (D-063): 100k
// closure-apply tail iterations at FIXED stack. Fails without the
// apply trampoline (interp depth cap 1024) and without INDIRECT
// musttail (the callsite prototype equals the caller's type only
// under the uniform convention). rc runs UNFENCED since D-064: the tail-aware
// release scheduler relocates paired frame releases to before the
// call, so the tail shape survives the rc discipline.
func.func @spin(%env: !frk_closure.envref, %n: i64) -> i64 {
  %zero = arith.constant 0 : i64
  %done = arith.cmpi eq, %n, %zero : i64
  cf.cond_br %done, ^exit, ^next
^exit:
  %c42 = arith.constant 42 : i64
  return %c42 : i64
^next:
  %e = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
  %self = "frk_closure.make"(%e) {callee = @spin} : (!frk_adt.product<[]>) -> !frk_closure.fn<[i64], [i64]>
  %one = arith.constant 1 : i64
  %m = arith.subi %n, %one : i64
  %ea = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
  %args = "frk_adt.product_snoc"(%ea, %m) : (!frk_adt.product<[]>, i64) -> !frk_adt.product<[i64]>
  %r = "frk_closure.apply"(%self, %args) : (!frk_closure.fn<[i64], [i64]>, !frk_adt.product<[i64]>) -> i64
  return %r : i64
}
func.func @main() -> i64 attributes {llvm.emit_c_interface} {
  %e = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
  %spin = "frk_closure.make"(%e) {callee = @spin} : (!frk_adt.product<[]>) -> !frk_closure.fn<[i64], [i64]>
  %n = arith.constant 100000 : i64
  %ea = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
  %args = "frk_adt.product_snoc"(%ea, %n) : (!frk_adt.product<[]>, i64) -> !frk_adt.product<[i64]>
  %r = "frk_closure.apply"(%spin, %args) : (!frk_closure.fn<[i64], [i64]>, !frk_adt.product<[i64]>) -> i64
  return %r : i64
}
