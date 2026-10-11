//! The `calque` CLI (F30's surface, slice 1): `run` executes a unit's
//! main and streams its prints; `test` runs the unit's `test` blocks
//! and reports `PASS <name>` / `FAIL <name>` (the demo playbook's
//! format), exiting non-zero on any failure. The front-end is the
//! canonical compiler (seed -> calq0) through the harness glue.

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: calque run|test FILE.calq");
        return ExitCode::from(2);
    }
    let cmd = args[1].as_str();
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
            eprintln!("calque: unknown subcommand `{other}` (run|test)");
            ExitCode::from(2)
        }
    }
}
