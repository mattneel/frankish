//! The loanword freeze contract's MUSTs, refused as well as obeyed
//! (D-046/D-050): a tampered artifact — one bit of source flipped —
//! is rejected with a content-id error naming both hashes. Plus the
//! §6.5 witness: the first genuine TS-0 runtime trap (array OOB,
//! D-049) carries a source location through span threading.

use frk_interp::Interp;

fn produce(path: &str) -> String {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = std::process::Command::new("node")
        .arg(root.join("tools/loanword-ts/src/main.ts"))
        .arg(root.join(path))
        .output()
        .expect("node producer");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn a_tampered_artifact_is_refused_with_both_hashes() {
    let artifact = produce("goldens/ts0/fib/case.ts");
    // Flip one bit: fib's `2` becomes `3` inside the embedded source.
    let tampered = artifact.replace("n < 2", "n < 3");
    assert_ne!(artifact, tampered, "tampering must change bytes");

    let context = frk_core::context();
    frk_dialects::register(&context).unwrap();
    let error = frk_front::loanword::compile_loanword(&context, &tampered)
        .expect_err("a bit-flipped artifact must be refused");
    let message = error.to_string();
    assert!(message.contains("content id mismatch"), "{message}");
    assert!(
        message.contains("claims") && message.contains("hash to"),
        "the refusal names both hashes: {message}"
    );

    // And the untampered artifact still compiles.
    frk_front::loanword::compile_loanword(&context, &artifact).expect("original verifies");
}

#[test]
fn the_first_runtime_trap_carries_a_source_location() {
    // The §6.5 witness (D-050): OOB is stricter-than-JS by ruling
    // (D-049, D-038 precedent) and the trap names file:line:col.
    let artifact = produce("crates/frk-front/tests/fixtures/oob.ts");
    let context = frk_core::context();
    frk_dialects::register(&context).unwrap();
    let module = frk_front::loanword::compile_loanword(&context, &artifact).unwrap();
    let mut interp = Interp::new(&module).unwrap();
    frk_dialects::register_eval(&mut interp);
    let error = interp
        .eval_function("main", &[])
        .expect_err("out-of-bounds must trap (D-049)");
    let message = error.to_string();
    assert!(message.contains("out of bounds"), "{message}");
    assert!(
        message.contains("oob.ts") && message.contains(":4:"),
        "the trap points at source (§6.5): {message}"
    );
}
