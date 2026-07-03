//
// The None arm of the same match shape: no payload, dispatch lands on
// the tag-0 successor. None → 0.
func.func @main() -> i64 attributes {llvm.emit_c_interface} {
  %s = "frk_adt.make_sum"() {variant = 0 : i64} : () -> !frk_adt.sum<[[], [i64]]>
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
  cf.br ^exit(%v : i64)
^unreachable:
  %m1 = arith.constant -1 : i64
  cf.br ^exit(%m1 : i64)
^exit(%r: i64):
  return %r : i64
}
