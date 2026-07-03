# specimen: r7rs_core — stub (ratify post-M10)

Pin: R7RS-small core; chibi-scheme as readable reference, chez as ceiling.
Role: tortures frk.ctl — proper tail calls are a hard law (verifier: deep
mutual recursion goldens at fixed stack), one-shot/escape continuations
first (full call/cc FENCED per SPEC §14), dynamic-wind semantics decision,
hygienic macros exercising the expander (sets-of-scopes crib).
Status: not started; do not ratify before the ctl effects design lands.
