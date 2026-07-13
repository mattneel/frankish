# The Differential Law

One sentence carries the whole project's credibility:

> **The derived interpreter is the reference semantics. Every compiled
> path must agree with it byte-exactly on every golden. Specimen
> frontends add the upstream implementation as a third oracle.**

This chapter explains what runs, what "agree" means, and why the
arrangement catches the bugs it catches.

## The eight runners

`make diff` executes every golden case on every runner applicable to it
and holds the outputs in pairwise agreement:

| Runner | What it is | Applies to |
|---|---|---|
| `interp` | The derived reference interpreter, executing kernel dialects directly on a deep-stack thread | everything |
| `jit` | MLIR ExecutionEngine, arena memory strategy | everything |
| `jit-rc` | The same, reference-counting strategy — same IR, different lowering parameter | everything |
| `ocaml` | The real `ocaml`, running the *same source file* | ml_core cases |
| `node` | The real `node`, running the *same `.ts` file* | TS-0 cases |
| `lua` | The real `lua5.1`, running the *same `.lua` file* | femto_lua cases |
| `scheme` | The real `chibi-scheme -q`, running the *same `.scm` file* | r7rs_core cases |
| `repl` | Scripted REPL transcripts through the shell | transcript cases |

The AOT grid is the ninth path — the same corpus cross-compiled to five
architectures under both strategies and executed under qemu/wasmtime; it
runs as `make grid` rather than inside `make diff`, but it is held to the
same byte-exactness.

As of `m15-done`:

```
diff[interp,jit,jit-rc,ocaml,node,lua,scheme,repl]: 77 case(s), 0 divergent
```

## Why the interpreter is the reference

The interpreter is *derived*: each kernel dialect registers an evaluator
for its ops (the same K2 hook any dialect must provide), and the
interpreter executes the IR that the frontends actually emit — not a
parallel semantics written in prose. Three properties make it the right
anchor:

1. **Totality.** What native code leaves undefined, the interpreter
   defines as a deterministic trap (division by zero, out-of-bounds
   array access, call-depth exhaustion, dynamic-tag mismatch). The
   golden corpus is required to stay UB-free, and the interpreter is the
   instrument that proves it.
2. **Simplicity.** An evaluator per op, a frame of SSA values, a small
   step machine. When interp and JIT disagree, the interpreter is almost
   always right — and when it isn't, that is itself a first-rank finding
   about the semantics.
3. **Leading capability.** New semantics land in the interpreter first
   and native follows under test. Control effects (M15) are the clean
   example: the interpreter *really unwinds* aborts through the frame
   stack, while native code uses a result-passing lowering with no
   unwinder — two implementations that could hardly be more different,
   held equal by the corpus. The agreement *is* the proof that the
   lowering is correct.

## Why upstream oracles

Byte-agreement between interp and JIT proves internal consistency; it
cannot prove that femto_lua is *Lua*. So specimen cases are the same
source file the upstream implementation runs — not a transliteration.
`lua5.1` executes `case.lua`; frankish compiles the identical bytes. If
frankish's `%.14g` float formatting rounds a tie differently than PUC
Lua's, the corpus catches it (it did — the tie-rounding contract is
D-055, with a deliberate half-even tie case in the corpus).

Oracle versions are pinned in `versions.env` (`ocaml` 4.14.1 for the ml
corpus, node ≥ 20, `lua5.1` 5.1.5, `chibi-scheme` 0.9.1) because *the pin
is the spec*: "agrees with Lua" is not a claim about Lua-in-general but
about a named implementation at a named version.

## What "byte-exact" buys

The strictness is the point. A tolerance — "close enough on floats",
"same modulo iteration order" — is a place for bugs to hide, and worse, a
place for *semantics decisions to happen silently*. Where genuine
implementation freedom exists, frankish handles it the honest way: the
freedom is written into the canon as law, and the corpus is constructed
so the freedom is unobservable. Lua table iteration order is the worked
example: the interpreter iterates insertion order, native iterates slot
order — both legal per Lua 5.1 and per `docs/canon.md` — and the corpus
aggregates iteration results so both orders produce identical bytes.
The disagreement space is closed by *design*, not by fuzz.

## Divergence is a halt

The law's enforcement clause: a runner disagreement is a **first-rank
finding**. The feature that exposed it stops; the finding is filed in
`STATE.md`; the divergence is fixed or explicitly fenced before work
proceeds. Nothing is ever built on top of a known disagreement.

Fifteen milestones in, the standing result — 77 cases, zero divergent,
across eight paths and five architectures — is not evidence that the
system was written carefully. It is evidence that every careless stroke
was *caught*. The distinction matters: the M12 collector chapter records
three memory-corruption bugs, every one found as a corpus divergence or
harness trap, none by code review. The differential law is the reason
those stories have endings.
