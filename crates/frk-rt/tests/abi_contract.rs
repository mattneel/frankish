//! The registered-ABI enforcement witnesses (M17, D-062; law L1 — the
//! refusal must be proven, not assumed).
//!
//! The two ENFORCEMENT points are compile-time and cannot themselves
//! be unit-tested by running: the Rust twin is checked by build.rs's
//! generated fn-pointer assertions (this crate does not build if a
//! signature drifts), and the C twin is checked by the generated
//! header at every compile. What CAN and MUST be witnessed at test
//! time: (1) the checked-in header has not drifted from the registry;
//! (2) the C compiler really does REFUSE a signature that contradicts
//! the header (the tamper witness).

use std::process::Command;

fn repo_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root")
        .to_path_buf()
}

fn cc() -> String {
    // The pinned clang the harness uses; plain cc as fallback for
    // dev machines without the LLVM prefix (the header is compiler-
    // agnostic C99).
    let pinned = "/usr/lib/llvm-22/bin/clang";
    if std::path::Path::new(pinned).exists() {
        pinned.to_string()
    } else {
        "cc".to_string()
    }
}

#[test]
fn checked_in_header_matches_the_registry() {
    let expected = frk_abi::c_header();
    let actual = include_str!("../c/frk_rt_abi.h");
    assert_eq!(
        actual, expected,
        "crates/frk-rt/c/frk_rt_abi.h drifted from frk-abi — run `make abi`"
    );
}

#[test]
fn c_twin_compiles_against_the_contract() {
    let out = Command::new(cc())
        .args(["-c", "-o", "/dev/null"])
        .arg(repo_root().join("crates/frk-rt/c/frk_rt.c"))
        .output()
        .expect("running cc");
    assert!(
        out.status.success(),
        "C twin no longer satisfies the registered ABI:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn tampered_c_signature_is_refused() {
    // The M15 display_bool bug, replayed on purpose: a definition
    // whose signature contradicts the registered contract must FAIL
    // to compile. This is the witness that the header has teeth.
    let dir = std::env::temp_dir().join(format!("frk-abi-tamper-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("tamper dir");
    let tamper = dir.join("tamper.c");
    std::fs::write(
        &tamper,
        r#"#include "frk_rt_abi.h"
/* wrong width: the registry says int64_t */
void frk_rt_scm_display_bool(uint8_t value) { (void)value; }
"#,
    )
    .expect("writing tamper.c");
    let out = Command::new(cc())
        .args(["-c", "-o", "/dev/null"])
        .arg("-I")
        .arg(repo_root().join("crates/frk-rt/c"))
        .arg(&tamper)
        .output()
        .expect("running cc");
    std::fs::remove_dir_all(&dir).ok();
    assert!(
        !out.status.success(),
        "a signature contradicting the registered ABI COMPILED — the contract has no teeth"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("conflicting types"),
        "expected a conflicting-types refusal, got:\n{stderr}"
    );
}
