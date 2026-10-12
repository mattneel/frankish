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
fn cli_check_and_build() {
    let case = "../../goldens/calque/test_blocks/case.calq";
    let out = Command::new(bin())
        .args(["check", case])
        .output()
        .expect("spawn calque check");
    assert!(out.status.success(), "a valid unit checks quietly");
    assert_eq!(String::from_utf8_lossy(&out.stdout), "");

    // a build beside the source (file_stem.mlir), then check the
    // error path exits non-zero for a broken unit.
    let src = std::env::temp_dir().join("calque_cli_build.calq");
    std::fs::write(
        &src,
        "pub fn main() i64 {\n    io.print(str.fromI64(7));\n    io.print(\"\\n\");\n    0\n}\n",
    )
    .unwrap();
    let out2 = Command::new(bin())
        .args(["build", src.to_str().unwrap()])
        .output()
        .expect("spawn calque build");
    assert!(out2.status.success());
    let artifact = std::env::temp_dir().join("calque_cli_build.mlir");
    let text = std::fs::read_to_string(&artifact).expect("the artifact lands beside the source");
    assert!(text.starts_with("// calque-artifact v0"), "{}", &text[..80.min(text.len())]);

    let bad = std::env::temp_dir().join("calque_cli_bad.calq");
    std::fs::write(&bad, "pub fn main() i64 {\n    no_such_fn();\n    0\n}\n").unwrap();
    let out3 = Command::new(bin())
        .args(["check", bad.to_str().unwrap()])
        .output()
        .expect("spawn calque check (bad)");
    assert!(!out3.status.success(), "a broken unit exits non-zero");
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

#[test]
fn cli_bootstrap_uses_the_recorded_chain() {
    let dir = std::env::temp_dir().join(format!("calque-cli-bootstrap-{}", std::process::id()));
    std::fs::create_dir(&dir).expect("isolated bootstrap output directory");
    let out = Command::new(bin())
        .arg("bootstrap")
        .current_dir(&dir)
        .output()
        .expect("spawn calque bootstrap");
    assert!(
        out.status.success(),
        "bootstrap failed: {} {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let artifacts = dir.join("out/calque");
    let artifact1 = std::fs::read_to_string(artifacts.join("artifact1.mlir"))
        .expect("bootstrap emits the current compiler");
    let artifact2 = std::fs::read_to_string(artifacts.join("artifact2.mlir"))
        .expect("bootstrap emits the self-produced compiler");
    assert_eq!(artifact1, artifact2, "bootstrap reaches its byte-exact fixed point");
    let units = [
        calque::UnitFile {
            name: "str.calq".to_string(),
            ns: Some("str".to_string()),
            text: calque::stdlib_source().to_string(),
        },
        calque::UnitFile {
            name: "probe.calq".to_string(),
            ns: None,
            text: "pub fn main() i64 { io.print(str.fromI64(6 * 7)); io.print(\"\\n\"); 0 }\n"
                .to_string(),
        },
    ];
    let program = calque::run_artifact_with_input_cached(&artifact2, &units)
        .expect("the self-produced compiler compiles the probe");
    let result = calque::run_artifact(&program.output).expect("the probe runs");
    assert_eq!(result.output, "42\n");
    std::fs::remove_dir_all(&dir).expect("remove bootstrap outputs");
}
