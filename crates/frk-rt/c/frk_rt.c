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

/* Lua runtime errors (D-056 helpers): print and abort. */
void frk_rt_lua_error(int64_t code) {
    fprintf(stderr, "frk: lua runtime error %lld (D-056)\n", (long long)code);
    abort();
}

/* ---- tables (M11 bar 3; D-056): pure-hash dyn-keyed maps, mirror
 * of the Rust twin. Shell [cap,count,slots,meta] strategy-allocated
 * by the lowering; slots malloc'd. All-i64 ABI, out-pointer returns. */

typedef struct {
    int64_t state; /* 0 empty, 1 full, 2 tombstone */
    int64_t ktag, kpay, vtag, vpay;
} frk_table_slot;

static uint64_t frk_table_hash(int64_t ktag, int64_t kpay) {
    uint64_t h = (uint64_t)ktag * 0x9E3779B97F4A7C15ULL ^ (uint64_t)kpay;
    h ^= h >> 30;
    h *= 0xBF58476D1CE4E5B9ULL;
    h ^= h >> 27;
    return h;
}

void frk_rt_table_init(int64_t shell) {
    int64_t *f = (int64_t *)(intptr_t)shell;
    f[0] = f[1] = f[2] = f[3] = 0;
}

void frk_rt_table_raw_set(int64_t shell, int64_t ktag, int64_t kpay,
                          int64_t vtag, int64_t vpay);

static void frk_table_grow(int64_t shell) {
    int64_t *f = (int64_t *)(intptr_t)shell;
    int64_t old_cap = f[0];
    frk_table_slot *old = (frk_table_slot *)(intptr_t)f[2];
    int64_t new_cap = old_cap ? old_cap * 2 : 8;
    f[0] = new_cap;
    f[1] = 0;
    f[2] = (int64_t)(intptr_t)calloc((size_t)new_cap, sizeof(frk_table_slot));
    for (int64_t i = 0; i < old_cap; i++)
        if (old && old[i].state == 1)
            frk_rt_table_raw_set(shell, old[i].ktag, old[i].kpay, old[i].vtag,
                                 old[i].vpay);
    free(old);
}

void frk_rt_table_raw_set(int64_t shell, int64_t ktag, int64_t kpay,
                          int64_t vtag, int64_t vpay) {
    int64_t *f = (int64_t *)(intptr_t)shell;
    if (f[0] == 0 || f[1] * 10 >= f[0] * 7) frk_table_grow(shell);
    f = (int64_t *)(intptr_t)shell;
    frk_table_slot *slots = (frk_table_slot *)(intptr_t)f[2];
    uint64_t mask = (uint64_t)f[0] - 1;
    uint64_t i = frk_table_hash(ktag, kpay) & mask;
    int64_t first_tomb = -1;
    for (;;) {
        frk_table_slot *s = &slots[i];
        if (s->state == 1 && s->ktag == ktag && s->kpay == kpay) {
            if (vtag == 0) s->state = 2; /* nil deletes */
            else { s->vtag = vtag; s->vpay = vpay; }
            return;
        }
        if (s->state == 2 && first_tomb < 0) first_tomb = (int64_t)i;
        if (s->state == 0) {
            if (vtag == 0) return; /* deleting absent: no-op */
            frk_table_slot *t = first_tomb >= 0 ? &slots[first_tomb] : s;
            t->state = 1; t->ktag = ktag; t->kpay = kpay;
            t->vtag = vtag; t->vpay = vpay;
            f[1] += 1;
            return;
        }
        i = (i + 1) & mask;
    }
}

void frk_rt_table_raw_get(int64_t shell, int64_t ktag, int64_t kpay,
                          int64_t *out) {
    int64_t *f = (int64_t *)(intptr_t)shell;
    if (f[0] != 0) {
        frk_table_slot *slots = (frk_table_slot *)(intptr_t)f[2];
        uint64_t mask = (uint64_t)f[0] - 1;
        uint64_t i = frk_table_hash(ktag, kpay) & mask;
        for (;;) {
            frk_table_slot *s = &slots[i];
            if (s->state == 0) break;
            if (s->state == 1 && s->ktag == ktag && s->kpay == kpay) {
                out[0] = s->vtag; out[1] = s->vpay;
                return;
            }
            i = (i + 1) & mask;
        }
    }
    out[0] = 0; out[1] = 0; /* nil */
}

int64_t frk_rt_table_len(int64_t shell) {
    int64_t out[2];
    int64_t n = 0;
    for (;;) {
        double probe = (double)(n + 1);
        int64_t bits; memcpy(&bits, &probe, 8);
        frk_rt_table_raw_get(shell, 2, bits, out);
        if (out[0] == 0) return n;
        n++;
    }
}

/* ---- byte strings (M11 bar 3; D-052/D-056): interned, identity-
 * equal. Layout {u64 len, bytes}. FNV-1a open addressing; canonical
 * pointers live for the process (rt values, outside the strategy
 * axis until the tracer). Tier-0 targets are single-threaded. ---- */

static unsigned char **bstr_slots;
static uint64_t bstr_cap, bstr_count;

static uint64_t bstr_hash(const unsigned char *bytes, uint64_t len) {
    uint64_t hash = 1469598103934665603ULL;
    for (uint64_t i = 0; i < len; i++) {
        hash ^= bytes[i];
        hash *= 1099511628211ULL;
    }
    return hash;
}

static void bstr_grow(void);

static unsigned char *bstr_intern_bytes(const unsigned char *bytes, uint64_t len) {
    if (bstr_cap == 0 || bstr_count * 10 >= bstr_cap * 7) bstr_grow();
    uint64_t mask = bstr_cap - 1;
    uint64_t slot = bstr_hash(bytes, len) & mask;
    for (;;) {
        unsigned char *entry = bstr_slots[slot];
        if (!entry) break;
        uint64_t entry_len = *(uint64_t *)entry;
        if (entry_len == len && (len == 0 || memcmp(entry + 8, bytes, (size_t)len) == 0))
            return entry;
        slot = (slot + 1) & mask;
    }
    unsigned char *base = malloc(8 + (size_t)len);
    if (!base) return base;
    *(uint64_t *)base = len;
    if (len) memcpy(base + 8, bytes, (size_t)len);
    bstr_slots[slot] = base;
    bstr_count++;
    return base;
}

static void bstr_grow(void) {
    uint64_t old_cap = bstr_cap;
    unsigned char **old = bstr_slots;
    bstr_cap = old_cap ? old_cap * 2 : 64;
    bstr_slots = calloc((size_t)bstr_cap, sizeof *bstr_slots);
    bstr_count = 0;
    for (uint64_t i = 0; i < old_cap; i++) {
        if (!old || !old[i]) continue;
        /* Re-slot the existing canonical block (pointer preserved). */
        uint64_t len = *(uint64_t *)old[i];
        uint64_t mask = bstr_cap - 1;
        uint64_t slot = bstr_hash(old[i] + 8, len) & mask;
        while (bstr_slots[slot]) slot = (slot + 1) & mask;
        bstr_slots[slot] = old[i];
        bstr_count++;
    }
    free(old);
}

void *frk_rt_bstr_intern(const unsigned char *bytes, uint64_t len) {
    return bstr_intern_bytes(bytes, len);
}

void *frk_rt_bstr_concat(const unsigned char *a, const unsigned char *b) {
    uint64_t alen = *(const uint64_t *)a, blen = *(const uint64_t *)b;
    unsigned char *tmp = malloc((size_t)(alen + blen) + 1);
    if (!tmp) return tmp;
    memcpy(tmp, a + 8, (size_t)alen);
    memcpy(tmp + alen, b + 8, (size_t)blen);
    unsigned char *canonical = bstr_intern_bytes(tmp, alen + blen);
    free(tmp);
    return canonical;
}

void *frk_rt_bstr_from_num(double value) {
    char buffer[40];
    int written = snprintf(buffer, sizeof buffer, "%.14g", value);
    return bstr_intern_bytes((const unsigned char *)buffer, (uint64_t)written);
}

void frk_rt_print_lua_str(const unsigned char *s) {
    uint64_t len = *(const uint64_t *)s;
    fwrite(s + 8, 1, (size_t)len, stdout);
    putchar('\n');
}

/* ---- Lua printing (M11 bar 4; D-052/D-055): native %.14g. ---- */
void frk_rt_print_lua_num(double value) { printf("%.14g\n", value); }
void frk_rt_print_lua_bool(unsigned char value) {
    printf("%s\n", value ? "true" : "false");
}
void frk_rt_print_lua_nil(void) { printf("nil\n"); }

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
