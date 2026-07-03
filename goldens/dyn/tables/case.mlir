// Tables (D-056): new/raw_set/raw_get with num and str keys, nil
// deletion, the border probe, and the inline meta slot round trip.
// Sum: get(k1)=10 + get("x")=30 + len-after-delete(1) + meta-tag(4)
// - nil-tag-of-deleted(0) = 10+30+1+4-3 = 42.
func.func @main() -> i64 attributes {llvm.emit_c_interface} {
  %t = "frk_dyn.table_new"() : () -> !frk_dyn.dyn
  %mt = "frk_dyn.table_new"() : () -> !frk_dyn.dyn

  %one_f = arith.constant 1.0 : f64
  %two_f = arith.constant 2.0 : f64
  %k1 = "frk_dyn.wrap"(%one_f) {tag = 2 : i64} : (f64) -> !frk_dyn.dyn
  %k2 = "frk_dyn.wrap"(%two_f) {tag = 2 : i64} : (f64) -> !frk_dyn.dyn
  %xs = "frk_bstr.lit"() {text = "x"} : () -> !frk_bstr.str
  %kx = "frk_dyn.wrap"(%xs) {tag = 3 : i64} : (!frk_bstr.str) -> !frk_dyn.dyn

  %ten_f = arith.constant 10.0 : f64
  %twenty_f = arith.constant 20.0 : f64
  %thirty_f = arith.constant 30.0 : f64
  %v10 = "frk_dyn.wrap"(%ten_f) {tag = 2 : i64} : (f64) -> !frk_dyn.dyn
  %v20 = "frk_dyn.wrap"(%twenty_f) {tag = 2 : i64} : (f64) -> !frk_dyn.dyn
  %v30 = "frk_dyn.wrap"(%thirty_f) {tag = 2 : i64} : (f64) -> !frk_dyn.dyn

  "frk_dyn.raw_set"(%t, %k1, %v10) : (!frk_dyn.dyn, !frk_dyn.dyn, !frk_dyn.dyn) -> ()
  "frk_dyn.raw_set"(%t, %k2, %v20) : (!frk_dyn.dyn, !frk_dyn.dyn, !frk_dyn.dyn) -> ()
  "frk_dyn.raw_set"(%t, %kx, %v30) : (!frk_dyn.dyn, !frk_dyn.dyn, !frk_dyn.dyn) -> ()

  // Interned identity: a SECOND "x" literal is the same key.
  %xs2 = "frk_bstr.lit"() {text = "x"} : () -> !frk_bstr.str
  %kx2 = "frk_dyn.wrap"(%xs2) {tag = 3 : i64} : (!frk_bstr.str) -> !frk_dyn.dyn
  %gx = "frk_dyn.raw_get"(%t, %kx2) : (!frk_dyn.dyn, !frk_dyn.dyn) -> !frk_dyn.dyn
  %g1 = "frk_dyn.raw_get"(%t, %k1) : (!frk_dyn.dyn, !frk_dyn.dyn) -> !frk_dyn.dyn

  // Delete t[2] via nil; border falls to 1.
  %zero_i = arith.constant 0 : i64
  %nil = "frk_dyn.wrap"(%zero_i) {tag = 0 : i64} : (i64) -> !frk_dyn.dyn
  "frk_dyn.raw_set"(%t, %k2, %nil) : (!frk_dyn.dyn, !frk_dyn.dyn, !frk_dyn.dyn) -> ()
  %len = "frk_dyn.table_len"(%t) : (!frk_dyn.dyn) -> i64
  %gone = "frk_dyn.raw_get"(%t, %k2) : (!frk_dyn.dyn, !frk_dyn.dyn) -> !frk_dyn.dyn
  %gone_tag = "frk_dyn.tag_of"(%gone) : (!frk_dyn.dyn) -> i64

  // Meta round trip.
  "frk_dyn.set_meta"(%t, %mt) : (!frk_dyn.dyn, !frk_dyn.dyn) -> ()
  %back = "frk_dyn.get_meta"(%t) : (!frk_dyn.dyn) -> !frk_dyn.dyn
  %meta_tag = "frk_dyn.tag_of"(%back) : (!frk_dyn.dyn) -> i64

  %x_num = "frk_dyn.unwrap"(%gx) {tag = 2 : i64} : (!frk_dyn.dyn) -> f64
  %one_num = "frk_dyn.unwrap"(%g1) {tag = 2 : i64} : (!frk_dyn.dyn) -> f64
  %sum_f = arith.addf %x_num, %one_num : f64
  %sum_i = arith.fptosi %sum_f : f64 to i64
  %s1 = arith.addi %sum_i, %len : i64
  %s2 = arith.addi %s1, %meta_tag : i64
  %m3 = arith.constant -3 : i64
  %s3 = arith.addi %s2, %m3 : i64
  %s4 = arith.addi %s3, %gone_tag : i64
  return %s4 : i64
}
