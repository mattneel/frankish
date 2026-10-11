//! The `calque` CLI's smoke gates (F30's slice 1): `test` reports the
//! playbook's PASS/FAIL lines and the exit code follows; `run`
//! streams main's prints.

use std::process::Command;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_calque")
}

#[test]
fn cli_runs_the_test_blocks_and_main() {
    let case = "../../goldens/calque/test_blocks/case.calq";
    let out = Command::new(bin())
        .args(["test", case])
        .output()
        .expect("spawn calque test");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "the passing unit exits 0: {stdout} {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(stdout.contains("PASS the answer is 42"), "{stdout}");
    assert!(stdout.contains("PASS strings agree"), "{stdout}");
    assert!(stdout.contains("PASS bools agree"), "{stdout}");

    let out2 = Command::new(bin())
        .args(["run", case])
        .output()
        .expect("spawn calque run");
    assert!(out2.status.success());
    assert_eq!(String::from_utf8_lossy(&out2.stdout), "42\n");
}

#[test]
fn cli_test_failure_exits_nonzero() {
    let path = std::env::temp_dir().join("calque_cli_fail.calq");
    std::fs::write(
        &path,
        "test \"bad math\" {\n    assert.equal(1, 2);\n}\n\npub fn main() i64 {\n    0\n}\n",
    )
    .unwrap();
    let out = Command::new(bin())
        .args(["test", path.to_str().unwrap()])
        .output()
        .expect("spawn calque test");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(!out.status.success(), "a failing unit exits non-zero: {stdout}");
    assert!(stdout.contains("FAIL bad math"), "{stdout}");
}
