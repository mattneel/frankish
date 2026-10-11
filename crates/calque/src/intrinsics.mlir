// calque intrinsics v0 (M37, D-062 pattern / D-088.6): the primitive
// surface every composed Calque unit may call. Bodyless `frk_rt_*`
// declarations are checked against the frk-abi registry by the
// semantic verifier (the D-062 declaration projection); the kernel
// lowering dedups against symbols already declared here.
//
// v0 carries only the runtime rows (D-088.7 — print is byte-exact
// with NO implicit newline; trap carries an emission-composed
// message). Vector helpers, the i64/bstr/box instantiations, and the
// derived-stdlib surface (str.fromI64) land with the seed's emission
// fixtures.

func.func private @frk_rt_calque_print(!frk_bstr.str)
func.func private @frk_rt_calque_trap(!frk_bstr.str)
