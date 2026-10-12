# c5v1 — the compiler-source snapshot post-D-128 (module surface)

Status: the hop that lets calq0's own sources USE the module +
receiver-call surface (D-129: the emitter-API migration).

## What this is

A copy of the canonical compiler's source set at the state where the
compiler IMPLEMENTS modules/receiver calls but uses neither in its own
code. Built by the previous hop's compiler (c4v1's, via the fold); NOT
seed-buildable (no SEED_BUILDABLE marker - the seed's surface closed
at D-124).

    ordered fold: seed -> c2v1 -> c3v1 -> c4v1 -> c5v1 -> CURRENT

- b0 cutoff: every feature through D-128 (`pub module`, primary
  structs, `Self`, receiver calls).
- unit set: `std/str.calq`, `compiler/{lex,report,parse,emit,compile}`
  (the flat v0 model; module-owned fns carry their module ns).
- Byte-pinned by `MANIFEST.sha256`; never edited after this point.
