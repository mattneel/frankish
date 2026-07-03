# The type kit, documented as reusable (M6; SPEC §6.4)

What ml_core's checker built, split by how far each piece travels.

## Travels to any frontend (the kit)

- **Unification over ena** (`frk_front::types` + the `Cx` core in
  `infer.rs`): `TyVid` keys an `InPlaceUnificationTable` with
  `Option<Ty>` values; `resolve` chases bindings, `occurs` guards
  infinite types, `unify` is structural with var-var union and
  var-value assignment through the *fallible* ena API
  (`unify_var_var`/`unify_var_value` — the infallible `union` demands
  `NoError` and does not fit `Option<Ty>`).
- **Schemes**: generalize-at-let with free-var-of-env subtraction;
  instantiation with fresh-variable substitution; **recorded
  instantiations** per binding id — the mechanism behind D-038's
  monomorphic-emission rule (0 uses → drop, 1 → concretize by unifying
  scheme vars with the use, >1 → error until monomorphization).
- **The value restriction** as a predicate on syntax (only `fun`
  right-hand sides generalize) — swap the predicate per language.
- **Zonking discipline**: emission consumes only fully-resolved types;
  leftover variables are ambiguity errors, not defaults.

## Deliberately per-frontend (not kit, and why)

- **The `Ty` language itself**: `Adt(String)` is nominal-by-name
  against a specimen-local constructor table; tuples/functions are
  ml-shaped. A shared core type language is *loanword's* question
  (M9) — the interchange format will force the vocabulary two
  frontends actually share. Do not abstract it earlier on one data
  point.
- **Constructor resolution** (ctor name → adt/tag/payload; OCaml's
  syntactic-tuple arity rule): specimen law, encoded per MANIFEST.
- **Kernel type spelling** (`Ty` → `!frk_adt.*`/`!frk_closure.fn`
  strings): the mapping is a frontend policy (ml erases `unit` to an
  empty product; another language may not).

## Standing debts (§6.5, ledgered)

Every emitted location is `unknown`: diagnostics cannot point at
source yet. Span threading from the lexer (offsets exist today)
through typed AST into MLIR `Location`s is scheduled with the next
frontend (M9), where two consumers justify the plumbing. The green
tree (§6.2) is deferred with it — see D-039.
