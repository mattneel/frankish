# The Two-Twin Runtime

frk-rt exists twice, on purpose. The Rust crate
(`crates/frk-rt/src/lib.rs`) is canonical and hosts the in-process JIT
runners; the C mirror (`crates/frk-rt/c/frk_rt.c`, 728 lines) is what the
AOT grid compiles per triple with `zig cc`. The C file's header comment is
the design in full:

```c
/* frk-rt, C mirror (D-042). The AOT/cross grid compiles THIS file per
 * triple with zig cc — no Rust cross-toolchain needed. The Rust crate
 * (../src/lib.rs) stays canonical for the in-process JIT; the two
 * implementations are held behaviorally equal by the grid itself:
 * aot output must byte-match jit output on every golden (law L3).
 *
 * ABI (D-041/D-042): sizes are uint64_t ON EVERY TARGET — the kernel
 * lowering passes i64 unconditionally, and 32-bit-word targets (wasm)
 * enforce exact import signatures, so size_t here would trap at link
 * (signature_mismatch — found by the first wasm grid run). */
```

## Why two implementations

The alternative was cross-building the Rust crate for five triples —
rustup target setup, per-triple sysroots, a second toolchain axis to pin.
D-042 refused it: the grid compiles one C file with the already-pinned
`zig cc` (which bundles musl and wasi libc), and the Rust crate never
leaves the host. The cost is a mirror to keep honest; the payment
mechanism is L3 itself. There is no "keep the twins in sync" checklist —
every golden run *is* the sync check, because `aot-*` output must
byte-match `jit` output on every case. Where byte equality is subtle, a
dedicated rig exists: the `%.14g` number formatter is proven by a
cross-twin test that compiles the C twin via zigcc and diffs it against
the Rust emulation on deliberate round-half-even tie values (D-055).

The Rust twin is still `std` today; it goes `#![no_std]`-capable when the
grid demands it — the ABI won't change (module doc).

## The ABI laws

| Law | Rule | Origin |
|---|---|---|
| u64 sizes | allocation sizes are `uint64_t` on every target; the runtime casts down | first wasm grid run: `size_t` trapped at link (`signature_mismatch`), D-042 |
| all-i64 arguments | table and dyn entry points pass tags, payloads, and shell pointers as `i64` words | the slot model; D-056 |
| out-pointer returns | multi-word results are written through a caller-alloca out-pointer, never struct-returned | "struct-return conventions across five triples are exactly the ABI risk the wasm signature_mismatch taught us to refuse" (D-056) |
| 8-alignment | both allocators return 8-aligned payloads | D-041 |

Two representative signatures, identical in both twins:

```rust
pub extern "C" fn frk_rt_table_raw_get(shell: i64, ktag: i64, kpay: i64, out: *mut i64)
pub unsafe extern "C" fn frk_rt_ctl_resolve(token: i64, out: *mut i64) -> i64
```

The out-pointer recipe proved itself twice: first for table gets, then
reused verbatim as the ctl prompt's result slot — the D-061 panel judge
scored it "Tier-0 strongest" precisely because it was already grid-proven.

## Residents

| Subsystem | Entry points | Notes |
|---|---|---|
| counters | `frk_rt_alloc_count`, `frk_rt_rc_release_count`, `frk_rt_rc_free_count` | the measurable targets ratified with D-041/D-053; leak canaries assert against them |
| arena | `frk_rt_arena_alloc(bytes)` | process-lifetime bump (v0 never resets); no headers, never traces |
| rc + cycles | `frk_rt_rc_alloc(payload_bytes, layout)`, `_retain`, `_release`, `_collect` | Bacon–Rajan trial deletion over an explicit candidate buffer (D-053/D-057) |
| tables | `frk_rt_table_init`, `_raw_get`, `_raw_set`, `_len`, `_next` | pure-hash dyn-keyed maps; 4-word shell `[cap, count, slots, meta]`; shell is strategy-allocated, slots traced and freed via the layout word |
| byte strings | `frk_rt_bstr_intern`, `_concat`, `_from_num`, `_sub`, `_rep` | global intern table owns canonical pointers; equality *is* pointer identity (D-052/D-056) |
| UTF-16 strings | `frk_rt_str_from_units`, `_concat`, `_eq`, `_len` | rt-owned immutable values, layout `{len: u64, units: u16×len}` (D-049) |
| prints | `frk_rt_print_f64/_bool/_str/_lua_*`, `frk_rt_scm_display_num/_bool`, `frk_rt_scm_newline` | one protocol per specimen; `format_lua_num` is the `%.14g` contract (D-055) |
| ctl pending cell | `frk_rt_ctl_prompt_enter/_exit`, `_abort`, `_pending`, `_resolve` | the result-passing carrier (D-060/D-061) |

The rc header is three words, written by `frk_rt_rc_alloc` in both twins:

```text
[layout: u64 @ ptr-24] [size: u64 @ ptr-16] [rcword @ ptr-8]
rcword: bits 62..63 color (0 black, 1 gray, 2 white, 3 purple),
        bit 61 buffered, bits 0..60 count
```

The layout word (D-057) makes compiler knowledge runtime-visible: bits
0..1 select the kind (wordmap / table shell / array); a wordmap carries
2-bit per-word trace codes from bit 4 (0 skip, 1 managed pointer,
2 dyn-tag paired with the next word). The lowering computes it per
allocation site, and a dev-dependency parity test holds the Rust and C
encodings in lockstep.

One war story lives in both twins' comments because both paid for it: the
color field occupies the rcword's sign bits, and an *arithmetic* right
shift smears them — purple read back as −1 and never matched. The Rust
twin shifts logically through `u64`; the C twin declares "ALL rcword
arithmetic is UNSIGNED — the Rust twin found this the hard way; D-057".

## The ctl pending cell

Native control effects have no unwinder — Tier-0 includes wasm, so
result-passing is the default lowering (D-011, executed by D-061). The
runtime's share is deliberately small (frk-rt section comment): abort sets
a process-global pending cell; every non-tail caller checks it after a
call and returns; the matching prompt resolves it.

```rust
pub extern "C" fn frk_rt_ctl_abort(token: i64, tag: i64, payload: i64) {
    if !ctl_prompts().lock().unwrap().contains(&token) {
        eprintln!("frk: escape past extent (κ_frk, D-060)");
        std::process::abort();
    }
    // park {tag, payload}, record the target, set pending
}
```

`frk_rt_ctl_resolve(token, out)` is the prompt's catch test: if the
pending abort targets `token`, clear pending, write the parked
`{tag, payload}` through `out`, return 1; otherwise return 0 and leave the
abort for an outer prompt. Prompt tokens are monotonic and never reused
within a run (no ABA), mirroring the interpreter's cells exactly — the
same state machine, one unwinding and one threading returns.

A well-formed program leaves `pending = 0` and the prompt stack empty, so
state never leaks between goldens sharing a JIT process — which matters,
because the harness runs the whole corpus through one process per runner.

## How each execution path binds the runtime

The JIT runner hands the ORC engine every symbol out of the host process
(`engine.register_symbol("frk_rt_rc_alloc", frk_rt::frk_rt_rc_alloc as *mut ())`
and ~30 siblings in `runner.rs`), substituting capturing print shims so
JIT output lands in the harness buffer rather than interleaving with its
stdout (D-047). The AOT runner links `frk_rt.c` into the executable
alongside the generated shim. The interpreter needs no runtime at all —
its builtins reimplement the observable protocol directly, sharing only
the formatters (`frk_rt::format_lua_num`, `format_f64`) so that all three
paths print the same bytes.

Threading is a non-feature by ruling: Tier-0 targets run single-threaded,
so the C twin uses plain counters ("a plain increment suffices until a
threaded target joins the grid"), and the Rust twin's atomics are host
hygiene, not a concurrency contract.
