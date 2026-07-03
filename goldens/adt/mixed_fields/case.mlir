//
// Mixed-type fields (i64 + i1) — the very case that exposed IRDL's
// variadic-unification ceiling and forced the packed surface (D-036).
// Under the old variadic ops this program was inexpressible.
// 41 + (true ? 1 : 0) = 42.
func.func @main() -> i64 attributes {llvm.emit_c_interface} {
  %x = arith.constant 41 : i64
  %b = arith.constant true
  %e = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
  %p1 = "frk_adt.product_snoc"(%e, %x) : (!frk_adt.product<[]>, i64) -> !frk_adt.product<[i64]>
  %p = "frk_adt.product_snoc"(%p1, %b) : (!frk_adt.product<[i64]>, i1) -> !frk_adt.product<[i64, i1]>
  %s = "frk_adt.make_sum"(%p) {variant = 0 : i64} : (!frk_adt.product<[i64, i1]>) -> !frk_adt.sum<[[i64, i1]]>
  %v = "frk_adt.extract"(%s) {variant = 0 : i64, field = 0 : i64} : (!frk_adt.sum<[[i64, i1]]>) -> i64
  %flag = "frk_adt.extract"(%s) {variant = 0 : i64, field = 1 : i64} : (!frk_adt.sum<[[i64, i1]]>) -> i1
  %one = arith.constant 1 : i64
  %zero = arith.constant 0 : i64
  %bonus = arith.select %flag, %one, %zero : i64
  %r = arith.addi %v, %bonus : i64
  return %r : i64
}
