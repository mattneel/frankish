# The Tier-0 Grid

D-017 ruled that portability is not a porting effort but a CI grid:
specimen × triple, executed via qemu-user and wasmtime, with s390x as the
big-endian canary. D-042 executed the ruling. The grid is the full golden
corpus, compiled ahead-of-time for four triples under both memory
strategies, plus the canary leg — and it must be byte-identical to the
interpreter and the JIT on every case (L3).

```rust
pub const GRID: [Triple; 4] = [
    Triple::X86_64Linux,
    Triple::Aarch64Linux,
    Triple::Riscv64Linux,
    Triple::Wasm32Wasi,
];
```

| Triple | Executor | What the slot buys |
|---|---|---|
| `x86_64-linux-musl` | native exec | the host baseline; `make ci`'s continuous AOT slice (D-042) |
| `aarch64-linux-musl` | `qemu-aarch64` | the mainstream non-x86 64-bit ISA, same corpus on a second register file — sysroot-free because zig bundles musl |
| `riscv64-linux-musl` | `qemu-riscv64` | a third ISA; D-051 cites its address-space growth ("riscv64 sv48+ looms") among the reasons frk_dyn refuses 48-bit pointer games |
| `wasm32-wasi` | `wasmtime run` | 32-bit pointers plus *exact* import-signature enforcement — the strictest ABI checker in the grid (D-042/D-051) |
| `s390x-linux-musl` | `qemu-s390x` | the big-endian canary (D-017): the slot model is same-width load/store symmetric, which is exactly what the canary proves (D-042); nightly, not per-push |

All five link musl-static (zig bundles libc), so qemu-user runs the
binaries with no sysroot at all. Runner names are `aot-<short>` and
`aot-<short>-rc` — ten AOT runners over one pipeline definition.

## The AOT path, step by step

`AotRunner::run` in `crates/frk-harness/src/runner.rs`:

1. **Front half.** Same as every runner: parse, MLIR verify, frankish
   semantic verify.
2. **Entry rename.** The entry `func.func`'s `sym_name` becomes
   `frk_entry` *before* lowering — the C shim owns `main()`. Valid
   because corpus protocol makes entry functions externally-invoked-only,
   so the rename is reference-free (D-042; goldens/README.md).
3. **The shared pipeline** at the runner's strategy — the exact five
   stages the JIT runs.
4. **Translate.** `mlir-translate --mlir-to-llvmir` produces `case.ll`.
5. **Compile with the pinned clang.**
   `{LLVM-22 prefix}/bin/clang -target <triple> -O1 -c` — the pinned
   LLVM's clang, not zig's, because the IR may be newer than zig's bundled
   LLVM. On `wasm32-wasi` the compile adds `-mtail-call`: `musttail`
   needs the wasm tail-call feature, and wasmtime 46 has the proposal on
   by default (D-059).
6. **Generate the shim**, keyed on the case kind. Specimen cases
   (ts/lua/scheme) have a void entry whose output is the linked runtime's
   prints; kernel cases return one i64:

   ```c
   /* ts / lua / scheme */
   extern void frk_entry(void);
   int main(void) { frk_entry(); return 0; }

   /* everything else */
   extern long long frk_entry(void);
   int main(void) { printf("%lld\n", frk_entry()); return 0; }
   ```

7. **Link with zig.** `scripts/zigcc.sh -target <triple> -O1` links
   `{case.o, shim.c, crates/frk-rt/c/frk_rt.c}` into `case.exe` — the C
   runtime twin is compiled per triple right here, every run.
8. **Execute** natively, under `qemu-<arch>`, or under `wasmtime run`
   (the harness checks `PATH`, then `~/.wasmtime/bin/wasmtime`).
9. **Canonicalize and compare** stdout against the blessed golden.

The zig driver is eleven lines of POSIX sh, and handles both a plain zig
install and an anyzig-style version-manager shim, pinned by
`ZIG_VERSION=0.16.0` in `versions.env`:

```sh
. "$script_dir/../versions.env"

if zig version >/dev/null 2>&1; then
    exec zig cc "$@"
else
    exec zig "$ZIG_VERSION" cc "$@"
fi
```

## Invocations

```sh
make grid          # full corpus × {x86_64,aarch64,riscv64,wasm32} × {arena,rc}
make canary        # the s390x big-endian leg
make grid-native   # host triple only — the slice scripts/ci.sh runs
```

All three shell out to `frnksh grid`, which prints one row per triple with
`ok/total` cells for each strategy and exits on
`grid: GREEN (both strategies)` or `grid: RED`. Placement is a ruling, not
an accident (D-042): the dev loop (`make test` / `make diff`) keeps the
fast in-process runners and oracles, AOT lives in `make grid`, `make ci`
runs the native slice for continuous L3 coverage, and the canary is
scheduled nightly. Current standing: the full 77-case corpus is green
across the grid and the canary; the newest suite's exit line reads
"6/6 × x86_64/aarch64/riscv64/wasm32 × 2 + s390x canary" (m15-done), and
the single `ctl` golden yields 42 on all eight grid cells plus interp and
both JITs (D-061).

## What the grid has actually caught

The grid is not an insurance policy; it has a kill list.

**The wasm `signature_mismatch` (M7, first wasm run).** The runtime's
allocator originally took `size_t`. The kernel lowering passes `i64`
unconditionally, and wasm enforces exact import signatures — so the link
trapped. The fix became ABI law: sizes are u64 on *every* target, both
twins cast down (D-042). A 64-bit-only grid would have shipped the
mismatch indefinitely.

**The scheme `display_bool` trap (M15).** From the commit that closed the
r7rs_core corpus (`da1093a`):

> Fix found by the grid: `frk_rt_scm_display_bool` took u8, but the
> lowering passes the extended i1 as i64 — wasm's exact-import-signature
> rule trapped it (the LANDSCAPE-pinned lesson). i64 in both twins + the
> JIT capture now.

Same rule, second catch, eight milestones apart. Both twins now declare
`frk_rt_scm_display_bool(value: i64)`. Note what did *not* catch it: the
interpreter (builtins have no C ABI), the JIT (in-process symbol
resolution checks nothing), and the three native triples (relaxed calling
conventions absorb a u8/i64 disagreement silently). Only wasm's linker
refuses — which is precisely why the 32-bit slot earns its place.

**The canary's standing proof.** s390x has produced no divergence — and
that is the deliverable: every green nightly run is evidence that the slot
model stays same-width load/store symmetric and that no lowering has
smuggled in a byte-order assumption. D-051 chose fat `{tag, payload}` dyn
values over NaN-boxing partly *for* this leg ("no bit games on the
big-endian canary"); the canary is what keeps that bargain checkable.
D-059 also routes an open question here: "s390x musttail behavior is the
canary's to report."

The general lesson is the two-twin chapter's lesson restated at grid
scale: none of these were portability bugs found by users on exotic
hardware. They were ABI contract violations, found in CI, by the cheapest
five machines that disagree with each other in the right ways.
