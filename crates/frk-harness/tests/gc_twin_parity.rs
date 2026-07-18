//! M12's cross-twin collector verifiers (D-057.4): the C twin runs
//! the SAME hand-built cycle drills through zigcc — cascade,
//! dead-cycle collection, live-cycle survival — and must report the
//! same free counts the Rust twin's unit tests assert.

use std::process::Command;

#[test]
fn c_twin_collects_cycles_identically() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dir = root.join("target/gc-parity");
    std::fs::create_dir_all(&dir).unwrap();

    let driver = r#"
#include <stdio.h>
#include <stdint.h>
extern void *frk_rt_rc_alloc(uint64_t, uint64_t);
extern void frk_rt_rc_retain(void *);
extern void frk_rt_rc_release(void *);
extern void frk_rt_rc_collect(void);
extern uint64_t frk_rt_rc_free_count(void);
extern void frk_rt_table_init(int64_t);
extern void frk_rt_table_raw_set(int64_t, int64_t, int64_t, int64_t, int64_t);

/* wordmap: word 0 is a managed pointer -> code 1 at bit 4 */
#define WM1 (1ULL << 4)

int main(void) {
    /* cascade: outer -> inner */
    unsigned char *inner = frk_rt_rc_alloc(8, 0);
    unsigned char *outer = frk_rt_rc_alloc(8, WM1);
    *(uint64_t *)outer = (uint64_t)(uintptr_t)inner;
    frk_rt_rc_release(outer);
    printf("%llu\n", (unsigned long long)frk_rt_rc_free_count()); /* 2 */

    /* dead cycle */
    unsigned char *a = frk_rt_rc_alloc(8, WM1);
    unsigned char *b = frk_rt_rc_alloc(8, WM1);
    *(uint64_t *)a = (uint64_t)(uintptr_t)b; frk_rt_rc_retain(b);
    *(uint64_t *)b = (uint64_t)(uintptr_t)a; frk_rt_rc_retain(a);
    frk_rt_rc_release(a);
    frk_rt_rc_release(b);
    printf("%llu\n", (unsigned long long)frk_rt_rc_free_count()); /* still 2 */
    frk_rt_rc_collect();
    printf("%llu\n", (unsigned long long)frk_rt_rc_free_count()); /* 4 */

    /* live cycle survives, then dies */
    unsigned char *c = frk_rt_rc_alloc(8, WM1);
    unsigned char *d = frk_rt_rc_alloc(8, WM1);
    *(uint64_t *)c = (uint64_t)(uintptr_t)d; frk_rt_rc_retain(d);
    *(uint64_t *)d = (uint64_t)(uintptr_t)c; frk_rt_rc_retain(c);
    frk_rt_rc_release(d);
    frk_rt_rc_collect();
    printf("%llu\n", (unsigned long long)frk_rt_rc_free_count()); /* 4 */
    frk_rt_rc_release(c);
    frk_rt_rc_collect();
    printf("%llu\n", (unsigned long long)frk_rt_rc_free_count()); /* 6 */

    /* M28 (D-073): record shape — ptr at word 1, dead ring */
    unsigned char *n1 = frk_rt_rc_alloc(16, 1ULL << 6);
    unsigned char *n2 = frk_rt_rc_alloc(16, 1ULL << 6);
    ((uint64_t *)n1)[0] = 7; ((uint64_t *)n2)[0] = 11;
    ((uint64_t *)n1)[1] = (uint64_t)(uintptr_t)n2; frk_rt_rc_retain(n2);
    ((uint64_t *)n2)[1] = (uint64_t)(uintptr_t)n1; frk_rt_rc_retain(n1);
    frk_rt_rc_release(n1);
    frk_rt_rc_release(n2);
    frk_rt_rc_collect();
    printf("%llu\n", (unsigned long long)frk_rt_rc_free_count()); /* 8 */

    /* M31 (D-077): cyclic cons ring — dyn-pair wordmap [2,0,2,0] */
    /* codes: word0 tag, word1 pay, word2 tag, word3 pay -> 2,0,2,0 */
    uint64_t pair_wm = (2ULL << 4) | (0ULL << 6) | (2ULL << 8) | (0ULL << 10);
    unsigned char *q1 = frk_rt_rc_alloc(32, pair_wm);
    unsigned char *q2 = frk_rt_rc_alloc(32, pair_wm);
    ((int64_t *)q1)[0] = 2; ((int64_t *)q2)[0] = 2;
    ((int64_t *)q1)[2] = 6; ((int64_t *)q1)[3] = (int64_t)(intptr_t)q2; frk_rt_rc_retain(q2);
    ((int64_t *)q2)[2] = 6; ((int64_t *)q2)[3] = (int64_t)(intptr_t)q1; frk_rt_rc_retain(q1);
    frk_rt_rc_release(q1);
    frk_rt_rc_release(q2);
    frk_rt_rc_collect();
    printf("%llu\n", (unsigned long long)frk_rt_rc_free_count()); /* 10 */

    /* And a vector (arr<dyn>) holding a pair: cascade crosses arms */
    unsigned char *pp = frk_rt_rc_alloc(32, pair_wm);
    ((int64_t *)pp)[0] = 2; ((int64_t *)pp)[2] = 0;
    unsigned char *vv = frk_rt_rc_alloc(8 + 32, 2ULL | (2ULL << 2));
    ((int64_t *)vv)[0] = 2;
    ((int64_t *)vv)[1] = 2; ((int64_t *)vv)[2] = 41;
    ((int64_t *)vv)[3] = 6; ((int64_t *)vv)[4] = (int64_t)(intptr_t)pp;
    frk_rt_rc_release(vv);
    printf("%llu\n", (unsigned long long)frk_rt_rc_free_count()); /* 12 */

    /* M35 (D-084): the TAG_THREAD = 8 widening drill — a dead ring
     * closing through a table SLOT holding a tag-8 dyn:
     * thread -> chain(pair) -> vector -> table -> slot(tag 8) -> thread.
     * Red if any tracer arm misses tag 8. */
    unsigned char *th = frk_rt_rc_alloc(32, pair_wm);
    unsigned char *ch = frk_rt_rc_alloc(32, pair_wm);
    unsigned char *v8 = frk_rt_rc_alloc(8 + 16, 2ULL | (2ULL << 2));
    unsigned char *sh = frk_rt_rc_alloc(32, 1ULL); /* TABLE_SHELL */
    frk_rt_table_init((int64_t)(intptr_t)sh);
    ((int64_t *)th)[0] = 6; ((int64_t *)th)[1] = (int64_t)(intptr_t)ch; frk_rt_rc_retain(ch);
    ((int64_t *)th)[2] = 0; ((int64_t *)th)[3] = 0;
    ((int64_t *)ch)[0] = 7; ((int64_t *)ch)[1] = (int64_t)(intptr_t)v8; frk_rt_rc_retain(v8);
    ((int64_t *)ch)[2] = 0; ((int64_t *)ch)[3] = 0;
    ((int64_t *)v8)[0] = 1;
    ((int64_t *)v8)[1] = 4; ((int64_t *)v8)[2] = (int64_t)(intptr_t)sh; frk_rt_rc_retain(sh);
    {
        union { double d; int64_t i; } one; one.d = 1.0;
        frk_rt_table_raw_set((int64_t)(intptr_t)sh, 2, one.i, 8, (int64_t)(intptr_t)th);
    }
    frk_rt_rc_retain(th);
    frk_rt_rc_release(th);
    frk_rt_rc_release(ch);
    frk_rt_rc_release(v8);
    frk_rt_rc_release(sh);
    frk_rt_rc_collect();
    printf("%llu\n", (unsigned long long)frk_rt_rc_free_count()); /* 16 */
    return 0;
}
"#;
    std::fs::write(dir.join("driver.c"), driver).unwrap();
    let exe = dir.join("driver");
    let status = Command::new("sh")
        .arg(root.join("scripts/zigcc.sh"))
        .args(["-O1", "-o"])
        .arg(&exe)
        .arg(dir.join("driver.c"))
        .arg(root.join("crates/frk-rt/c/frk_rt.c"))
        .current_dir(&root)
        .status()
        .expect("zigcc");
    assert!(status.success());
    let output = Command::new(&exe).output().expect("driver run");
    assert!(output.status.success());
    let lines: Vec<&str> = std::str::from_utf8(&output.stdout).unwrap().lines().collect();
    assert_eq!(
        lines,
        ["2", "2", "4", "4", "6", "8", "10", "12", "16"],
        "the C twin's cascade/cycle/record-ring/pair-ring/vector/thread-ring story matches the Rust twin's"
    );
}
