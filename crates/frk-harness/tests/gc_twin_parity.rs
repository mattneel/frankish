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
        ["2", "2", "4", "4", "6"],
        "the C twin's cascade/dead-cycle/live-cycle story matches the Rust twin's"
    );
}
