# Introduction

Every language implementer rebuilds the same middle layer.

MLIR solved the bottom of the stack: if your problem looks like linear
algebra, loops over buffers, or straight-line arithmetic, the upstream
dialects (`arith`, `scf`, `cf`, `llvm`) and their lowering pipelines are
excellent, shared, and battle-tested. And language *frontends* are
well-trodden territory — parsing, name resolution, and type checking have
textbooks.

The layer in between has neither. Algebraic data types with efficient
pattern-match compilation. Closures with capture layout and calling
conventions. Memory management that can be swapped without touching the
frontend. Dynamic values, tables, and metatables. Interned strings. Proper
tail calls. Escape continuations and, eventually, effect handlers. Every
new language on MLIR re-derives these from scratch, makes the same
mistakes, and buries the design decisions in code nobody can audit.

frankish's bet is that this middle layer can be built **once, as a curated
library of MLIR dialects**, and that the way to build it honestly is to
make real languages depend on it.

## The forcing loop

The project does not design dialects in the abstract. It picks a real
language, freezes a small but honest subset (the **specimen**), and
implements it against the upstream implementation as an oracle:

1. A feature is admitted to a specimen **only** if it carries an idiom the
   kernel library lacks. This is law ([L5](method/laws.md)); feature lists
   are fences, not TODOs.
2. Implementing the feature forces a kernel dialect into existence — or
   extends one — under verifier-first discipline: the test lands with or
   before the code ([L1](method/laws.md)).
3. Whatever the specimen built privately gets **promoted** down into the
   kernel, and the specimen is re-based onto the promoted form. The
   decision-tree pattern-match compiler, for example, was born inside the
   ML frontend and now serves any frontend with a `match`.

Four specimens have run this loop: **ml_core** (an OCaml slice — sums,
products, HM inference, match compilation), **TS-0** (a TypeScript subset,
fed through a content-addressed frontend artifact called *loanword*),
**femto_lua** (Lua 5.1 — dynamic values, tables, metatables, multiple
returns), and **r7rs_core** (Scheme — proper tail calls and escape
continuations, which forced the control-effects dialect `frk_ctl` into
existence).

## The credibility mechanism

A dialect library is only as good as its semantics, and semantics claimed
in prose are worthless. frankish's answer is mechanical:

- A **derived reference interpreter** executes the kernel dialects
  directly. It is the semantics; the compiled paths must agree with it
  byte-for-byte ([the differential law](method/differential.md)).
- Every golden program runs on **eight execution paths**: the interpreter,
  two JIT configurations (one per memory strategy), the AOT grid, and —
  for specimen programs — the language's *upstream implementation*
  (`ocaml`, `node`, `lua5.1`, `chibi-scheme`). As of `m15-done` the
  matrix is 77 cases with zero divergence.
- The AOT grid cross-compiles every golden to **five architectures**
  (x86_64, aarch64, riscv64, wasm32-wasi, and big-endian s390x) under
  **both** memory strategies, and executes them under qemu and wasmtime.
  The grid is not ceremony: it has caught ABI bugs the host could never
  see — a wasm import-signature mismatch, an endianness assumption, a
  calling-convention drift between the two runtime twins.

When this book says the garbage collector works, it means: the collector
is implemented twice — once in Rust for the in-process JIT, once in C,
cross-compiled per architecture — and every golden program produces
identical bytes through both, on five architectures, with allocation and
free counters checked against the reference. That is the standard of
evidence throughout.

## What kind of book this is

This is the deep documentation of a working system, written from its
sources. It is organized so that a reader can start from the laws and
descend as far as they care to — down to header bit layouts, calling
conventions, and the exact reduction rules of the control calculus. Wherever
the prose and the repository could disagree, the repository wins; the book
cites files, decision numbers (`D-nnn`), and milestone tags (`mN-done`)
so the claims can be audited.

It is also, unapologetically, a record of method: small laws, mechanical
enforcement, an append-only decision ledger, and milestone notes that say
what was learned — including the bugs. The [collector war
stories](memory/war-stories.md) chapter exists because the three memory
bugs found at M12 are more instructive than the clean design that emerged
from them.
