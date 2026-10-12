//! The `calque` CLI (F30's surface, slice 1): `run` executes a unit's
//! main and streams its prints; `test` runs the unit's `test` blocks
//! and reports `PASS <name>` / `FAIL <name>` (the demo playbook's
//! format), exiting non-zero on any failure. The front-end is the
//! canonical compiler (seed -> calq0) through the harness glue.

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: calque run|test FILE.calq | calque bootstrap [SNAPSHOT_DIR]");
        return ExitCode::from(2);
    }
    let cmd = args[1].as_str();
    if cmd == "bootstrap" {
        let dir = args
            .get(2)
            .cloned()
            .unwrap_or_else(|| "tools/calque/bootstrap/snapshots/c3v1".to_string());
        return bootstrap(&dir);
    }
    if args.len() < 3 {
        eprintln!("usage: calque run|test FILE.calq | calque bootstrap [SNAPSHOT_DIR]");
        return ExitCode::from(2);
    }
    let path = args[2].as_str();
    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("calque: cannot read {path}: {e}");
            return ExitCode::from(2);
        }
    };
    let name = std::path::Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "unit.calq".to_string());
    let artifact = match calque::compile_source(&name, &source) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("calque: {name}: {e}");
            return ExitCode::from(1);
        }
    };
    match cmd {
        "check" => ExitCode::SUCCESS,
        "build" => {
            let stem = std::path::Path::new(path)
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "unit".to_string());
            let out_path = std::path::Path::new(path).with_file_name(format!("{stem}.mlir"));
            match std::fs::write(&out_path, &artifact) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("calque: cannot write {}: {e}", out_path.display());
                    ExitCode::from(1)
                }
            }
        }
        "run" => match calque::run_artifact(&artifact) {
            Ok(outcome) => {
                print!("{}", outcome.output);
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("calque: run: {e}");
                ExitCode::from(1)
            }
        },
        "test" => match calque::run_test_blocks(&artifact) {
            Ok(results) => {
                let mut failed = false;
                for (label, ok) in results {
                    if ok {
                        println!("PASS {label}");
                    } else {
                        println!("FAIL {label}");
                        failed = true;
                    }
                }
                if failed {
                    ExitCode::from(1)
                } else {
                    ExitCode::SUCCESS
                }
            }
            Err(e) => {
                eprintln!("calque: test: {e}");
                ExitCode::from(1)
            }
        },
        other => {
            eprintln!("calque: unknown subcommand `{other}` (run|test|bootstrap)");
            ExitCode::from(2)
        }
    }
}

/// The recorded bootstrap chain (BOOTSTRAP.md §4, D-119): the FROZEN
/// seed builds the c2v1 snapshot -> artifact0; artifact0 compiles the
/// CURRENT compiler sources -> artifact1; artifact1 does it again ->
/// artifact2. Writes the artifacts + sha256s of record under
/// out/calque/ and reports the fixed point (artifact1 == artifact2).
fn progress(msg: &str) {
    use std::io::Write;
    eprintln!("bootstrap: {msg} ...");
    let _ = std::io::stderr().flush();
}

fn bootstrap(snapshot_dir: &str) -> ExitCode {
    use sha2::{Digest, Sha256};

    let snap = match calque::snapshot_sources(snapshot_dir) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("bootstrap: {e}");
            return ExitCode::from(2);
        }
    };

    let current = calque::compiler_sources();

    progress("stage 1: seed -> snapshot artifact (JIT)");
    let artifact0 = match calque::seed_translate_cached(&snap) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("bootstrap: the seed failed on the snapshot: {e}");
            return ExitCode::from(1);
        }
    };
    let run = |artifact: &str, label: &str| -> Result<String, String> {
        let outcome = calque::run_artifact_with_input_cached(artifact, &current)
            .map_err(|e| format!("bootstrap: {label} run failed: {e}"))?;
        if outcome.output.contains("error ") && outcome.output.starts_with("error ") {
            return Err(format!("bootstrap: {label} reported: {}", &outcome.output[..outcome.output.len().min(200)]));
        }
        Ok(outcome.output)
    };
    progress("stage 2: artifact0 compiles the current sources");
    let artifact1 = match run(&artifact0, "artifact0 -> artifact1") {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(1);
        }
    };
    progress("stage 3: artifact1 compiles the current sources");
    let artifact2 = match run(&artifact1, "artifact1 -> artifact2") {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(1);
        }
    };
    progress("stage 4: snapshot vs current sources (exact diff)");
    let mut same_sources = true;
    for u in &current {
        let snap_path = u
            .name
            .strip_prefix("tools/calque/")
            .map(|rel| format!("{snapshot_dir}/{rel}"));
        let snap_text = snap_path
            .and_then(|p| std::fs::read_to_string(p).ok())
            .unwrap_or_default();
        if snap_text != u.text {
            same_sources = false;
        }
    }

    let sha = |s: &str| format!("{:x}", Sha256::digest(s.as_bytes()));
    let out_dir = std::path::Path::new("out/calque");
    if let Err(e) = std::fs::create_dir_all(out_dir) {
        eprintln!("bootstrap: cannot create out/calque: {e}");
        return ExitCode::from(1);
    }
    for (name, text) in [
        ("artifact0.mlir", &artifact0),
        ("artifact1.mlir", &artifact1),
        ("artifact2.mlir", &artifact2),
    ] {
        if let Err(e) = std::fs::write(out_dir.join(name), text) {
            eprintln!("bootstrap: cannot write {name}: {e}");
            return ExitCode::from(1);
        }
    }
    let fixed = artifact1 == artifact2;
    println!("bootstrap: artifact0 sha256 {}", sha(&artifact0));
    println!("bootstrap: artifact1 sha256 {}", sha(&artifact1));
    println!("bootstrap: artifact2 sha256 {}", sha(&artifact2));
    println!("bootstrap: snapshot == current sources: {same_sources}");
    println!("bootstrap: fixed point (artifact1 == artifact2): {fixed}");
    if fixed {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
