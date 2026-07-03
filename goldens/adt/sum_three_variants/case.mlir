//
// Three variants, the chosen one carrying two fields. Dispatch picks
// variant 2, extracts (40, 2): 40 + 2 = 42, plus tag*100 = 242.
func.func @main() -> i64 attributes {llvm.emit_c_interface} {
  %a = arith.constant 40 : i64
  %b = arith.constant 2 : i64
  %s = "frk_adt.make_sum"(%a, %b) {variant = 2 : i64} : (i64, i64) -> !frk_adt.sum<[[], [i64], [i64, i64]]>
  %tag = "frk_adt.tag_of"(%s) : (!frk_adt.sum<[[], [i64], [i64, i64]]>) -> i64
  cf.switch %tag : i64, [
    default: ^other,
    2: ^pair
  ]
^pair:
  %x = "frk_adt.extract"(%s) {variant = 2 : i64, field = 0 : i64} : (!frk_adt.sum<[[], [i64], [i64, i64]]>) -> i64
  %y = "frk_adt.extract"(%s) {variant = 2 : i64, field = 1 : i64} : (!frk_adt.sum<[[], [i64], [i64, i64]]>) -> i64
  %xy = arith.addi %x, %y : i64
  %c100 = arith.constant 100 : i64
  %scaled = arith.muli %tag, %c100 : i64
  %r = arith.addi %xy, %scaled : i64
  cf.br ^exit(%r : i64)
^other:
  %m1 = arith.constant -1 : i64
  cf.br ^exit(%m1 : i64)
^exit(%out: i64):
  return %out : i64
}
