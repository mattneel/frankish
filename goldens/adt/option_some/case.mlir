//
// The de-regioned match (D-031) over Option<i64>, Some arm: tag dispatch
// via cf.switch, guarded extract, +1. Packed construction per D-036.
// Some(41) → 42.
func.func @main() -> i64 attributes {llvm.emit_c_interface} {
  %x = arith.constant 41 : i64
  %e = "frk_adt.product_new"() : () -> !frk_adt.product<[]>
  %p = "frk_adt.product_snoc"(%e, %x) : (!frk_adt.product<[]>, i64) -> !frk_adt.product<[i64]>
  %s = "frk_adt.make_sum"(%p) {variant = 1 : i64} : (!frk_adt.product<[i64]>) -> !frk_adt.sum<[[], [i64]]>
  %tag = "frk_adt.tag_of"(%s) : (!frk_adt.sum<[[], [i64]]>) -> i64
  cf.switch %tag : i64, [
    default: ^unreachable,
    0: ^none,
    1: ^some
  ]
^none:
  %zero = arith.constant 0 : i64
  cf.br ^exit(%zero : i64)
^some:
  %v = "frk_adt.extract"(%s) {variant = 1 : i64, field = 0 : i64} : (!frk_adt.sum<[[], [i64]]>) -> i64
  %one = arith.constant 1 : i64
  %v1 = arith.addi %v, %one : i64
  cf.br ^exit(%v1 : i64)
^unreachable:
  %m1 = arith.constant -1 : i64
  cf.br ^exit(%m1 : i64)
^exit(%r: i64):
  return %r : i64
}
