# Specimens and Fences

A specimen is a real language, frozen to a small honest subset, implemented
against its upstream as an oracle. Specimens are how frankish forces kernel
dialects into existence without designing them in a vacuum — and the rules
that keep them honest are law.

## The admission rule (L5, D-010)

> No feature enters a specimen unless it carries an idiom the kernel dialect
> library lacks.

This is the whole discipline in one sentence. A feature is not admitted
because it would be convenient, familiar, or expected — it is admitted when
the *idiom* it forces is one the kernel cannot yet express. femto_lua's
metatables entered because dynamic dispatch was a missing idiom;
`call/cc` entered r7rs_core because escape continuations were. A feature
that carries only idioms the kernel already has stays fenced, because
implementing it would teach the kernel nothing.

The consequence: **fence lists in manifests are boundaries, not backlogs.**
A `## Fences` section is not a roadmap of things coming soon; it is the
frozen edge of the subset, and crossing it requires a ledger entry.

## The order (D-009, confirmed at D-040)

Specimens were sequenced deliberately, from most-abstract to
most-runtime-heavy, so each retired a class of risk before the next
depended on it:

```
ml_core  →  TS-0  →  femto_lua  →  r7rs_core
```

`ml_core` first retired *abstraction risk* — could the kernel dialects
carry a statically-typed functional language at all? TS-0 exercised the
loanword frontend interchange and introduced the first floating-point
idiom. femto_lua brought the dynamic runtime — fat values, tables,
garbage collection. r7rs_core brought control effects. A fifth entry,
`c_oracle`, exists as an *oracle rig* rather than a frontend (D-009): a
way to check C-level runtime behavior, not a language riding the kernel.

## The oracle table

Every specimen is held byte-identical to a **pinned** upstream
implementation. The pin is the spec: "agrees with Lua" means "agrees with
this named binary at this version."

| Specimen | Subset | Upstream oracle | Pin (`versions.env`) | Forces |
|---|---|---|---|---|
| ml_core | MinCaml-shaped core ML | `ocaml` | 4.14.1 | `frk_adt`, closures, HM inference, match compilation |
| TS-0 | monomorphic TypeScript | `node` / V8 | node ≥ 20, TS 6.0.3 | f64 numbers, UTF-16 strings, arrays, the loanword interchange |
| femto_lua | Lua 5.1 subset | `lua5.1` (PUC-Rio) | 5.1.5 | `frk_dyn`, `frk_bstr`, tables, GC, the pack convention |
| r7rs_core | R7RS-small core | `chibi-scheme -q` | 0.9.1 | `frk_ctl`, proper tail calls, escape continuations |

Four languages, four families (statically-typed functional,
gradually-typed OO, dynamically-typed imperative, and a Lisp), one kernel
underneath — held at zero divergence across the whole matrix.

## Manifests are the scope

Each specimen's scope lives *solely* in its `specimens/<name>/MANIFEST.md`
(D-010) — not in the code, not in the spec, not in this book. A manifest
names the subset, the frozen upstream, the admission justifications, the
fence list, the oracle protocol, and the status. The chapters that follow
tour each specimen's manifest and the kernel work it forced; where they
state a fence, the manifest is the authority.

The recurring shape to watch for, specimen by specimen, is the **forcing
loop closing**: a feature is admitted, it forces private machinery inside
the frontend, and the machinery is later *promoted* into the kernel so the
next specimen inherits it. The decision-tree pattern-match compiler
(born in ml_core, promoted at M6) and the pack calling convention (born
in femto_lua) are the clearest cases, and each has its own section ahead.
