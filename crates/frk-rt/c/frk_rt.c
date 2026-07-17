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

/* The registered ABI (M17, D-062): including the generated contract
 * makes THIS compiler enforce every frk_rt_* signature, at every
 * compile, on every grid triple. `make abi` regenerates. */
#include "frk_rt_abi.h"

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

/* Rc strategy (D-041, header amended by D-057): three-word header
 * [layout u64 @ -24][size u64 @ -16][rcword @ -8]. The rcword packs
 * Bacon-Rajan state: bits 62..63 color (0 black 1 gray 2 white 3
 * purple), bit 61 buffered, bits 0..60 count. ALL rcword arithmetic
 * is UNSIGNED — the color lives in the sign bits, and an arithmetic
 * shift smears it (the Rust twin found this the hard way; D-057).
 * Layout encoding: bits 0..1 kind (0 wordmap, 1 table shell, 2
 * array); wordmap 2-bit codes from bit 4 (0 skip, 1 ptr, 2 dyn-tag);
 * array elem code in bits 2..3. Tier-0 targets are single-threaded.
 */

#define FRK_COLOR_SHIFT 62
#define FRK_COLOR_MASK (3ULL << FRK_COLOR_SHIFT)
#define FRK_BUFFERED (1ULL << 61)
#define FRK_COUNT_MASK ((1ULL << 61) - 1)
#define FRK_BLACK 0ULL
#define FRK_GRAY 1ULL
#define FRK_WHITE 2ULL
#define FRK_PURPLE 3ULL

static uint64_t frk_rc_frees;
uint64_t frk_rt_rc_free_count(void) { return frk_rc_frees; }

static uint64_t *frk_hdr_layout(unsigned char *p) { return (uint64_t *)(p - 24); }
static uint64_t *frk_hdr_size(unsigned char *p) { return (uint64_t *)(p - 16); }
static uint64_t *frk_hdr_rc(unsigned char *p) { return (uint64_t *)(p - 8); }
static uint64_t frk_count(uint64_t w) { return w & FRK_COUNT_MASK; }
static uint64_t frk_color(uint64_t w) { return (w & FRK_COLOR_MASK) >> FRK_COLOR_SHIFT; }
static uint64_t frk_with_color(uint64_t w, uint64_t c) {
    return (w & ~FRK_COLOR_MASK) | (c << FRK_COLOR_SHIFT);
}

static unsigned char **frk_cands;
static uint64_t frk_cand_len, frk_cand_cap;

static void frk_cand_push(unsigned char *p) {
    if (frk_cand_len == frk_cand_cap) {
        frk_cand_cap = frk_cand_cap ? frk_cand_cap * 2 : 64;
        frk_cands = realloc(frk_cands, (size_t)frk_cand_cap * sizeof *frk_cands);
    }
    frk_cands[frk_cand_len++] = p;
}

void *frk_rt_rc_alloc(uint64_t payload_bytes, uint64_t layout) {
    frk_rt_allocs += 1;
    uint64_t bytes = payload_bytes ? payload_bytes : 1;
    unsigned char *base = malloc((size_t)bytes + 24);
    if (!base) return base;
    unsigned char *payload = base + 24;
    *frk_hdr_layout(payload) = layout;
    *frk_hdr_size(payload) = bytes;
    *frk_hdr_rc(payload) = 1; /* count 1, black, unbuffered */
    return payload;
}

typedef void (*frk_visit_fn)(unsigned char *);

static void frk_for_each_child(unsigned char *payload, frk_visit_fn visit) {
    uint64_t layout = *frk_hdr_layout(payload);
    uint64_t size = *frk_hdr_size(payload);
    switch (layout & 3) {
        case 1: { /* table shell */
            int64_t *fields = (int64_t *)payload;
            int64_t cap = fields[0];
            int64_t *slots = (int64_t *)(intptr_t)fields[2];
            if (fields[3]) visit((unsigned char *)(intptr_t)fields[3]);
            if (slots)
                for (int64_t i = 0; i < cap; i++) {
                    int64_t *s = slots + i * 5;
                    if (s[0] != 1) continue;
                    if (s[1] >= 4 && s[1] <= 6 && s[2])
                        visit((unsigned char *)(intptr_t)s[2]);
                    if (s[3] >= 4 && s[3] <= 6 && s[4])
                        visit((unsigned char *)(intptr_t)s[4]);
                }
            break;
        }
        case 2: { /* array */
            uint64_t code = (layout >> 2) & 3;
            if (code == 0) break;
            int64_t *words = (int64_t *)payload;
            int64_t len = words[0];
            if (code == 1) {
                for (int64_t i = 0; i < len; i++)
                    if (words[1 + i]) visit((unsigned char *)(intptr_t)words[1 + i]);
            } else {
                for (int64_t i = 0; i + 1 < len * 2; i += 2) {
                    int64_t tag = words[1 + i];
                    if (tag >= 4 && tag <= 6 && words[2 + i])
                        visit((unsigned char *)(intptr_t)words[2 + i]);
                }
            }
            break;
        }
        default: { /* wordmap */
            int64_t *words = (int64_t *)payload;
            uint64_t count = size / 8;
            if (count > 30) count = 30;
            for (uint64_t w = 0; w < count;) {
                uint64_t code = (layout >> (4 + 2 * w)) & 3;
                if (code == 1) {
                    if (words[w]) visit((unsigned char *)(intptr_t)words[w]);
                    w += 1;
                } else if (code == 2) {
                    int64_t tag = words[w];
                    if (w + 1 < count && tag >= 4 && tag <= 6 && words[w + 1])
                        visit((unsigned char *)(intptr_t)words[w + 1]);
                    w += 2;
                } else {
                    w += 1;
                }
            }
        }
    }
}

static void frk_free_object(unsigned char *payload) {
    if ((*frk_hdr_layout(payload) & 3) == 1) {
        int64_t *fields = (int64_t *)payload;
        if (fields[2]) free((void *)(intptr_t)fields[2]);
    }
    free(payload - 24);
    frk_rc_frees += 1;
}

static void frk_release_inner(unsigned char *payload);
static void frk_release_visit(unsigned char *child) { frk_release_inner(child); }

void frk_rt_rc_retain(void *payload) {
    if (!payload) return;
    uint64_t *r = frk_hdr_rc(payload);
    *r = frk_with_color(*r + 1, FRK_BLACK);
}

static void frk_release_inner(unsigned char *payload) {
    uint64_t *r = frk_hdr_rc(payload);
    uint64_t w = *r;
    uint64_t count = frk_count(w) - 1;
    if (count == 0) {
        *r = frk_with_color(count | (w & FRK_BUFFERED), FRK_BLACK);
        if (!(w & FRK_BUFFERED)) {
            frk_for_each_child(payload, frk_release_visit);
            frk_free_object(payload);
        }
    } else {
        if (*frk_hdr_layout(payload) == 0) {
            *r = frk_with_color(count | (w & FRK_BUFFERED), FRK_BLACK);
        } else {
            uint64_t next = frk_with_color(count | (w & FRK_BUFFERED), FRK_PURPLE);
            if (!(w & FRK_BUFFERED)) {
                next |= FRK_BUFFERED;
                frk_cand_push(payload);
            }
            *r = next;
        }
    }
}

void frk_rt_rc_release(void *payload) {
    if (!payload) return;
    frk_rt_releases += 1;
    frk_release_inner(payload);
}

static void frk_mark_gray(unsigned char *payload);
static void frk_mark_gray_visit(unsigned char *child) {
    *frk_hdr_rc(child) -= 1;
    frk_mark_gray(child);
}
static void frk_mark_gray(unsigned char *payload) {
    uint64_t *r = frk_hdr_rc(payload);
    if (frk_color(*r) != FRK_GRAY) {
        *r = frk_with_color(*r, FRK_GRAY);
        frk_for_each_child(payload, frk_mark_gray_visit);
    }
}

static void frk_scan(unsigned char *payload);
static void frk_scan_black(unsigned char *payload);
static void frk_scan_black_visit(unsigned char *child) {
    *frk_hdr_rc(child) += 1;
    if (frk_color(*frk_hdr_rc(child)) != FRK_BLACK) frk_scan_black(child);
}
static void frk_scan_black(unsigned char *payload) {
    uint64_t *r = frk_hdr_rc(payload);
    *r = frk_with_color(*r, FRK_BLACK);
    frk_for_each_child(payload, frk_scan_black_visit);
}
static void frk_scan_visit(unsigned char *child) { frk_scan(child); }
static void frk_scan(unsigned char *payload) {
    uint64_t *r = frk_hdr_rc(payload);
    if (frk_color(*r) == FRK_GRAY) {
        if (frk_count(*r) > 0) {
            frk_scan_black(payload);
        } else {
            *r = frk_with_color(*r, FRK_WHITE);
            frk_for_each_child(payload, frk_scan_visit);
        }
    }
}

static void frk_collect_white(unsigned char *payload);
static void frk_collect_white_visit(unsigned char *child) { frk_collect_white(child); }
static void frk_collect_white(unsigned char *payload) {
    uint64_t *r = frk_hdr_rc(payload);
    if (frk_color(*r) == FRK_WHITE && !(*r & FRK_BUFFERED)) {
        *r = frk_with_color(*r, FRK_BLACK);
        frk_for_each_child(payload, frk_collect_white_visit);
        frk_free_object(payload);
    }
}

void frk_rt_rc_collect(void) {
    unsigned char **roots = frk_cands;
    uint64_t root_count = frk_cand_len;
    frk_cands = NULL;
    frk_cand_len = frk_cand_cap = 0;

    unsigned char **live = malloc((size_t)(root_count ? root_count : 1) * sizeof *live);
    uint64_t live_count = 0;
    for (uint64_t i = 0; i < root_count; i++) {
        unsigned char *p = roots[i];
        uint64_t *r = frk_hdr_rc(p);
        uint64_t w = *r;
        if (frk_color(w) == FRK_PURPLE && frk_count(w) > 0) {
            frk_mark_gray(p);
            live[live_count++] = p;
        } else {
            *r &= ~FRK_BUFFERED;
            /* Bacon-Rajan guard: only BLACK count-0 is a deferred
             * free; a GRAY count-0 is a sibling's TRIAL zero. */
            if (frk_color(w) == FRK_BLACK && frk_count(w) == 0) {
                frk_for_each_child(p, frk_release_visit);
                frk_free_object(p);
            }
        }
    }
    for (uint64_t i = 0; i < live_count; i++) frk_scan(live[i]);
    for (uint64_t i = 0; i < live_count; i++) {
        *frk_hdr_rc(live[i]) &= ~FRK_BUFFERED;
        frk_collect_white(live[i]);
    }
    free(live);
    free(roots);
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

void frk_rt_print_bool(int64_t value) {
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

/* The contract narrowing check (D-072): a demoted flow fact executes
 * here — mismatch prints the blame and aborts. */
void frk_rt_contract_check(int64_t actual, int64_t expected,
                           const uint8_t *blame, int64_t blame_len) {
    if (actual != expected) {
        fprintf(stderr,
                "frk: contract: narrowing refuted: expected variant %lld, "
                "got %lld — %.*s (D-072)\n",
                (long long)expected, (long long)actual,
                (int)(blame_len < 0 ? 0 : blame_len), (const char *)blame);
        abort();
    }
}

/* Lua runtime errors (D-056 helpers): print and abort. */
void frk_rt_lua_error(int64_t code) {
    fprintf(stderr, "frk: lua runtime error %lld (D-056)\n", (long long)code);
    abort();
}

/* ---- control effects (M15; κ_frk, D-060): the exact mirror of the
 * Rust twin's result-passing carrier. NO unwinder (this file targets
 * wasm32 among others). Single-threaded per run; a well-formed program
 * leaves pending=0 and the prompt stack empty. ---- */

static int64_t frk_ctl_next_token = 1;
static int64_t frk_ctl_pending;
static int64_t frk_ctl_target;
static int64_t frk_ctl_value_tag;
static int64_t frk_ctl_value_payload;
static int64_t *frk_ctl_prompts;
static uint64_t frk_ctl_len, frk_ctl_cap;

int64_t frk_rt_ctl_prompt_enter(void) {
    int64_t token = frk_ctl_next_token++;
    if (frk_ctl_len == frk_ctl_cap) {
        frk_ctl_cap = frk_ctl_cap ? frk_ctl_cap * 2 : 16;
        frk_ctl_prompts =
            realloc(frk_ctl_prompts, (size_t)frk_ctl_cap * sizeof *frk_ctl_prompts);
    }
    frk_ctl_prompts[frk_ctl_len++] = token;
    return token;
}

/* Pop `token` and anything nested above it (LIFO; defensive truncate). */
void frk_rt_ctl_prompt_exit(int64_t token) {
    uint64_t i = frk_ctl_len;
    while (i > 0) {
        i -= 1;
        if (frk_ctl_prompts[i] == token) {
            frk_ctl_len = i;
            return;
        }
    }
}

static int frk_ctl_live(int64_t token) {
    for (uint64_t i = 0; i < frk_ctl_len; i += 1) {
        if (frk_ctl_prompts[i] == token) return 1;
    }
    return 0;
}

void frk_rt_ctl_abort(int64_t token, int64_t tag, int64_t payload) {
    if (!frk_ctl_live(token)) {
        fprintf(stderr, "frk: escape past extent (\xce\xba_frk, D-060)\n");
        abort();
    }
    frk_ctl_target = token;
    frk_ctl_value_tag = tag;
    frk_ctl_value_payload = payload;
    frk_ctl_pending = 1;
}

int64_t frk_rt_ctl_pending(void) { return frk_ctl_pending; }

/* ---- effects-v1 (M24, D-069): the evidence stack, mirroring the
 * Rust twin. Labels are interned bstr pointers passed as words;
 * markers are one-shot (second resume_mark traps). ---- */

typedef struct {
    int64_t label, fn, env, token;
    int masked;
} frk_ctl_handler;

static frk_ctl_handler *frk_ctl_hs;
static uint64_t frk_ctl_hs_len, frk_ctl_hs_cap;
static int64_t *frk_ctl_used;
static uint64_t frk_ctl_used_len, frk_ctl_used_cap;

void frk_rt_ctl_handler_push(int64_t label, int64_t fn, int64_t env, int64_t token) {
    if (frk_ctl_hs_len == frk_ctl_hs_cap) {
        frk_ctl_hs_cap = frk_ctl_hs_cap ? frk_ctl_hs_cap * 2 : 8;
        frk_ctl_hs = realloc(frk_ctl_hs, (size_t)frk_ctl_hs_cap * sizeof *frk_ctl_hs);
    }
    frk_ctl_handler h = { label, fn, env, token, 0 };
    frk_ctl_hs[frk_ctl_hs_len++] = h;
}

void frk_rt_ctl_handler_pop(void) {
    if (frk_ctl_hs_len) frk_ctl_hs_len -= 1;
}

static int frk_ctl_was_consumed(int64_t marker) {
    for (uint64_t i = 0; i < frk_ctl_used_len; i += 1)
        if (frk_ctl_used[i] == marker) return 1;
    return 0;
}

int64_t frk_rt_ctl_perform_begin(int64_t label, int64_t *out) {
    uint64_t i = frk_ctl_hs_len;
    while (i > 0) {
        i -= 1;
        if (!frk_ctl_hs[i].masked && frk_ctl_hs[i].label == label) {
            frk_ctl_hs[i].masked = 1;
            int64_t marker = frk_ctl_next_token++;
            out[0] = frk_ctl_hs[i].fn;
            out[1] = frk_ctl_hs[i].env;
            out[2] = marker;
            out[3] = frk_ctl_hs[i].token;
            out[4] = (int64_t)i;
            return 1;
        }
    }
    {
        const unsigned char *base = (const unsigned char *)(intptr_t)label;
        uint64_t len = *(const uint64_t *)base;
        fprintf(stderr, "frk: unhandled effect \"");
        fwrite(base + 8, 1, (size_t)len, stderr);
        fprintf(stderr, "\" (Îº_frk, D-069)\n");
        abort();
    }
}

int64_t frk_rt_ctl_perform_end(int64_t entry, int64_t marker, int64_t token,
                               int64_t rpack, int64_t *out) {
    if ((uint64_t)entry < frk_ctl_hs_len) frk_ctl_hs[entry].masked = 0;
    {
        const int64_t *words = (const int64_t *)(intptr_t)rpack;
        int64_t rtag = 0, rpay = 0;
        if (words[0] > 0) { rtag = words[1]; rpay = words[2]; }
        out[0] = rtag;
        out[1] = rpay;
        if (frk_ctl_was_consumed(marker)) return 1;
        /* M26 (D-071 finding): an in-flight abort (the clause escaped
         * through an enclosing prompt) WINS — do not clobber it. */
        if (!frk_ctl_pending) frk_rt_ctl_abort(token, rtag, rpay);
        return 0;
    }
}

void frk_rt_ctl_pack_head(int64_t pack, int64_t *out) {
    const int64_t *words = (const int64_t *)(intptr_t)pack;
    if (words[0] > 0) { out[0] = words[1]; out[1] = words[2]; }
    else { out[0] = 0; out[1] = 0; }
}

void frk_rt_ctl_resume_mark(int64_t marker) {
    if (frk_ctl_was_consumed(marker)) {
        fprintf(stderr, "frk: one-shot violation (Îº_frk, D-069)\n");
        abort();
    }
    if (frk_ctl_used_len == frk_ctl_used_cap) {
        frk_ctl_used_cap = frk_ctl_used_cap ? frk_ctl_used_cap * 2 : 16;
        frk_ctl_used = realloc(frk_ctl_used, (size_t)frk_ctl_used_cap * sizeof *frk_ctl_used);
    }
    frk_ctl_used[frk_ctl_used_len++] = marker;
}

/* If an abort targets `token`, clear pending, write the parked dyn to
 * out[0]=tag,out[1]=payload, return 1; else return 0. */
int64_t frk_rt_ctl_resolve(int64_t token, int64_t *out) {
    if (!frk_ctl_pending || frk_ctl_target != token) return 0;
    frk_ctl_pending = 0;
    out[0] = frk_ctl_value_tag;
    out[1] = frk_ctl_value_payload;
    return 1;
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

static unsigned char *bstr_intern_bytes(const unsigned char *bytes, uint64_t len);

/* Iteration for pairs/next (D-058): slot-order scan. */
void frk_rt_table_next(int64_t shell, int64_t ktag, int64_t kpay,
                       int64_t *out) {
    int64_t *f = (int64_t *)(intptr_t)shell;
    int64_t cap = f[0];
    frk_table_slot *slots = (frk_table_slot *)(intptr_t)f[2];
    int64_t start = 0;
    if (ktag != 0 && cap > 0) {
        uint64_t mask = (uint64_t)cap - 1;
        uint64_t i = frk_table_hash(ktag, kpay) & mask;
        start = cap;
        for (;;) {
            frk_table_slot *s = &slots[i];
            if (s->state == 0) break;
            if (s->state == 1 && s->ktag == ktag && s->kpay == kpay) {
                start = (int64_t)i + 1;
                break;
            }
            i = (i + 1) & mask;
        }
    }
    for (int64_t i = start; i < cap; i++) {
        if (slots[i].state == 1) {
            out[0] = slots[i].ktag; out[1] = slots[i].kpay;
            out[2] = slots[i].vtag; out[3] = slots[i].vpay;
            return;
        }
    }
    out[0] = out[1] = out[2] = out[3] = 0;
}

/* Lua string.sub/rep (D-058). */
void *frk_rt_bstr_sub(const unsigned char *s, int64_t from, int64_t to) {
    uint64_t ulen = *(const uint64_t *)s;
    int64_t len = (int64_t)ulen;
    int64_t i = from < 0 ? len + from + 1 : from;
    int64_t j = to < 0 ? len + to + 1 : to;
    if (i < 1) i = 1;
    if (j > len) j = len;
    if (i > j) return bstr_intern_bytes((const unsigned char *)"", 0);
    return bstr_intern_bytes(s + 8 + (i - 1), (uint64_t)(j - i + 1));
}

void *frk_rt_bstr_rep(const unsigned char *s, int64_t count) {
    uint64_t len = *(const uint64_t *)s;
    if (count < 0) count = 0;
    uint64_t total = len * (uint64_t)count;
    unsigned char *tmp = malloc((size_t)total + 1);
    if (!tmp) return tmp;
    for (int64_t i = 0; i < count; i++)
        memcpy(tmp + (uint64_t)i * len, s + 8, (size_t)len);
    unsigned char *canonical = bstr_intern_bytes(tmp, total);
    free(tmp);
    return canonical;
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

/* scheme display protocol (M15, r7rs_core): no trailing newline; the
 * mirror of the Rust twin. */
void frk_rt_scm_display_num(double value) { printf("%.14g", value); }
void frk_rt_scm_display_bool(int64_t value) {
    printf("%s", value ? "#t" : "#f");
}
void frk_rt_scm_newline(void) { printf("\n"); }
void frk_rt_scm_display_str(const uint8_t *s) {
    uint64_t len = *(const uint64_t *)s;
    fwrite(s + 8, 1, (size_t)len, stdout);
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

