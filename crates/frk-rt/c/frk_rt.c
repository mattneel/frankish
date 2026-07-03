/* frk-rt, C mirror (D-042). The AOT/cross grid compiles THIS file per
 * triple with zig cc — no Rust cross-toolchain needed. The Rust crate
 * (../src/lib.rs) stays canonical for the in-process JIT; the two
 * implementations are held behaviorally equal by the grid itself:
 * aot output must byte-match jit output on every golden (law L3).
 *
 * ABI (D-041/D-042): sizes are uint64_t ON EVERY TARGET — the kernel
 * lowering passes i64 unconditionally, and 32-bit-word targets (wasm)
 * enforce exact import signatures, so size_t here would trap at link
 * (signature_mismatch — found by the first wasm grid run). The
 * runtime casts down; 8-aligned; rc header is an i64 refcount at
 * ptr-8, count starts 1; v0 release decrements only (no free until
 * sized releases land with the M10 GC-gate work).
 */

#include <stdint.h>
#include <stdlib.h>
#include <string.h>

/* Allocation counter (D-041 ratification rider): the M10 pass's
 * measurable target. Tier-0 targets run single-threaded; a plain
 * increment suffices until a threaded target joins the grid. */
static uint64_t frk_rt_allocs;

uint64_t frk_rt_alloc_count(void) { return frk_rt_allocs; }

void *frk_rt_arena_alloc(uint64_t bytes) {
    frk_rt_allocs += 1;
    if (bytes == 0) bytes = 1;
    /* malloc alignment is >= 8 on every Tier-0 triple (musl, wasi). */
    return malloc((size_t)bytes);
}

void *frk_rt_rc_alloc(uint64_t payload_bytes) {
    frk_rt_allocs += 1;
    if (payload_bytes == 0) payload_bytes = 1;
    unsigned char *base = malloc((size_t)payload_bytes + 8);
    if (!base) return base;
    *(int64_t *)base = 1;
    return base + 8;
}

void frk_rt_rc_retain(void *payload) {
    if (!payload) return;
    int64_t *header = (int64_t *)((unsigned char *)payload - 8);
    *header += 1;
}

void frk_rt_rc_release(void *payload) {
    if (!payload) return;
    int64_t *header = (int64_t *)((unsigned char *)payload - 8);
    *header -= 1;
}
