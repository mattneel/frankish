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
static uint64_t frk_rt_releases;

uint64_t frk_rt_alloc_count(void) { return frk_rt_allocs; }
uint64_t frk_rt_rc_release_count(void) { return frk_rt_releases; }

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

/* JS-faithful f64 printing (D-047): shortest round-trip digits via
 * precision search (1..17, first %.{p}g whose strtod round-trips
 * bit-exactly); integer-valued doubles in range print as integers.
 * The corpus fence (canon): values are 0 or |v| in [1e-4, 1e15),
 * finite — %g stays positional there and matches JS ToString. */
#include <stdio.h>
#include <math.h>

void frk_rt_print_f64(double value) {
    if (value == (long long)value && fabs(value) < 1e15) {
        /* Integer-valued: JS prints no decimal point. Covers -0
         * correctly? JS prints "-0" for negative zero: */
        if (value == 0.0 && signbit(value)) {
            printf("-0\n");
            return;
        }
        printf("%lld\n", (long long)value);
        return;
    }
    char buffer[64];
    for (int precision = 1; precision <= 17; precision++) {
        snprintf(buffer, sizeof buffer, "%.*g", precision, value);
        double back;
        sscanf(buffer, "%lf", &back);
        if (back == value) break;
    }
    printf("%s\n", buffer);
}

void frk_rt_print_bool(unsigned char value) {
    printf("%s\n", value ? "true" : "false");
}

/* The dyn tag check (D-051/D-054): mismatch prints and aborts. */
void frk_rt_dyn_check(int64_t actual, int64_t expected) {
    if (actual != expected) {
        fprintf(stderr,
                "frk: dyn tag mismatch: expected %lld, got %lld (D-051)\n",
                (long long)expected, (long long)actual);
        abort();
    }
}

/* ---- strings (M9, D-049): {u64 len; u16 units[]}; plain malloc,
 * strategy-independent. UTF-16 code-unit semantics throughout. ---- */

static unsigned char *frk_str_alloc(uint64_t len_units) {
    unsigned char *base = malloc(8 + (size_t)len_units * 2);
    if (base) *(uint64_t *)base = len_units;
    return base;
}

void *frk_rt_str_from_units(const uint16_t *units, uint64_t len) {
    unsigned char *base = frk_str_alloc(len);
    if (base && len) memcpy(base + 8, units, (size_t)len * 2);
    return base;
}

void *frk_rt_str_concat(const unsigned char *a, const unsigned char *b) {
    uint64_t alen = *(const uint64_t *)a, blen = *(const uint64_t *)b;
    unsigned char *base = frk_str_alloc(alen + blen);
    if (!base) return base;
    memcpy(base + 8, a + 8, (size_t)alen * 2);
    memcpy(base + 8 + (size_t)alen * 2, b + 8, (size_t)blen * 2);
    return base;
}

int64_t frk_rt_str_eq(const unsigned char *a, const unsigned char *b) {
    uint64_t alen = *(const uint64_t *)a, blen = *(const uint64_t *)b;
    if (alen != blen) return 0;
    return memcmp(a + 8, b + 8, (size_t)alen * 2) == 0;
}

uint64_t frk_rt_str_len(const unsigned char *s) { return *(const uint64_t *)s; }

/* UTF-16 → UTF-8, lone surrogates to U+FFFD (matches Rust's
 * from_utf16_lossy — the two printers must agree byte-for-byte). */
static void put_code_point(uint32_t cp) {
    if (cp < 0x80) putchar(cp);
    else if (cp < 0x800) {
        putchar(0xC0 | (cp >> 6));
        putchar(0x80 | (cp & 0x3F));
    } else if (cp < 0x10000) {
        putchar(0xE0 | (cp >> 12));
        putchar(0x80 | ((cp >> 6) & 0x3F));
        putchar(0x80 | (cp & 0x3F));
    } else {
        putchar(0xF0 | (cp >> 18));
        putchar(0x80 | ((cp >> 12) & 0x3F));
        putchar(0x80 | ((cp >> 6) & 0x3F));
        putchar(0x80 | (cp & 0x3F));
    }
}

void frk_rt_print_str(const unsigned char *s) {
    uint64_t len = *(const uint64_t *)s;
    const uint16_t *units = (const uint16_t *)(s + 8);
    for (uint64_t i = 0; i < len; i++) {
        uint16_t unit = units[i];
        if (unit >= 0xD800 && unit <= 0xDBFF && i + 1 < len &&
            units[i + 1] >= 0xDC00 && units[i + 1] <= 0xDFFF) {
            uint32_t cp = 0x10000 + (((uint32_t)(unit - 0xD800)) << 10) +
                          (units[i + 1] - 0xDC00);
            put_code_point(cp);
            i++;
        } else if (unit >= 0xD800 && unit <= 0xDFFF) {
            put_code_point(0xFFFD);
        } else {
            put_code_point(unit);
        }
    }
    putchar('\n');
}

void frk_rt_rc_retain(void *payload) {
    if (!payload) return;
    int64_t *header = (int64_t *)((unsigned char *)payload - 8);
    *header += 1;
}

void frk_rt_rc_release(void *payload) {
    if (!payload) return;
    frk_rt_releases += 1;
    int64_t *header = (int64_t *)((unsigned char *)payload - 8);
    *header -= 1;
}
