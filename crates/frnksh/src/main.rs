//! frnksh — the frankish driver (SPEC §9). Bare invocation becomes the
//! REPL ("the frankish shell") at M8; the `frankish` alias symlink
//! (D-002) ships with packaging. The harness-facing subcommands arrived
//! with M1:
//!
//! ```text
//! frnksh test  [--goldens DIR]           golden corpus, every runner
//! frnksh bless [--goldens DIR]           rewrite goldens (law L2 applies)
//! frnksh diff  [--goldens DIR]           runner-agreement matrix (law L3)
//! frnksh emit --stages FILE [--out DIR]  per-pass IR snapshots (docs/stages.md)
//! ```
//!
//! Argument parsing is hand-rolled on purpose: three flags don't earn a
//! dependency. Revisit when the M8 surface lands.

use std::path::PathBuf;
use std::process::ExitCode;

use frk_harness::diff::diff_corpus;
use frk_harness::golden::{Mode, run_goldens};
use frk_harness::runner::{default_runners, reference_runner};
use frk_harness::stages::dump_stages;

const USAGE: &str = "usage:
  frnksh                                 the frankish shell (REPL, D-002)
  frnksh run FILE                        execute .ml/.mlir on the reference interpreter
  frnksh test  [--goldens DIR]           run the golden corpus (default DIR: goldens)
  frnksh bless [--goldens DIR]           rewrite expected outputs — commit message
                                         must justify the change (AGENTS.md L2)
  frnksh diff  [--goldens DIR]           compare all runners pairwise (AGENTS.md L3)
  frnksh dashboard [--goldens DIR]       conformance % per suite per runner (SPEC §8)
  frnksh grid [--goldens DIR] [--canary|--native]
                                         AOT cross grid, both strategies (SPEC §10);
                                         --canary: the s390x big-endian leg;
                                         --native: host triple only (the CI slice)
  frnksh emit --stages FILE [--out DIR]  write per-pass IR snapshots
                                         (default DIR: out/stages/<FILE stem>)";

#[derive(Debug, PartialEq, Eq)]
enum Command {
    /// Bare invocation = the frankish shell (D-002, M8).
    Repl,
    /// Execute a source file end to end on the reference interpreter.
    Run { source: PathBuf },
    Test { goldens: PathBuf },
    Bless { goldens: PathBuf },
    Diff { goldens: PathBuf },
    Dashboard { goldens: PathBuf },
    Grid { goldens: PathBuf, canary: bool, native_only: bool },
    Emit { source: PathBuf, out: Option<PathBuf> },
}

fn parse(args: &[String]) -> Result<Command, String> {
    let mut words = args.iter().map(String::as_str);
    let Some(subcommand) = words.next() else {
        return Ok(Command::Repl);
    };

    match subcommand {
        "run" => match (words.next(), words.next()) {
            (Some(file), None) => Ok(Command::Run { source: PathBuf::from(file) }),
            _ => Err("run: expected exactly one FILE (.ml or .mlir)".into()),
        },
        "grid" => {
            let mut goldens = PathBuf::from("goldens");
            let mut canary = false;
            let mut native_only = false;
            loop {
                match words.next() {
                    None => break,
                    Some("--goldens") => match words.next() {
                        Some(dir) => goldens = PathBuf::from(dir),
                        None => return Err("grid: --goldens needs a DIR".into()),
                    },
                    Some("--canary") => canary = true,
                    Some("--native") => native_only = true,
                    Some(other) => return Err(format!("grid: unknown argument {other:?}")),
                }
            }
            Ok(Command::Grid { goldens, canary, native_only })
        }
        "test" | "bless" | "diff" | "dashboard" => {
            let mut goldens = PathBuf::from("goldens");
            match (words.next(), words.next(), words.next()) {
                (None, ..) => {}
                (Some("--goldens"), Some(dir), None) => goldens = PathBuf::from(dir),
                _ => return Err(format!("{subcommand}: expected only --goldens DIR")),
            }
            Ok(match subcommand {
                "test" => Command::Test { goldens },
                "bless" => Command::Bless { goldens },
                "dashboard" => Command::Dashboard { goldens },
                _ => Command::Diff { goldens },
            })
        }
        "emit" => {
            let (mut source, mut out) = (None, None);
            loop {
                match words.next() {
                    None => break,
                    Some("--stages") => match words.next() {
                        Some(file) if source.is_none() => source = Some(PathBuf::from(file)),
                        _ => return Err("emit: --stages needs exactly one FILE".into()),
                    },
                    Some("--out") => match words.next() {
                        Some(dir) if out.is_none() => out = Some(PathBuf::from(dir)),
                        _ => return Err("emit: --out needs exactly one DIR".into()),
                    },
                    Some(other) => return Err(format!("emit: unknown argument {other:?}")),
                }
            }
            let Some(source) = source else {
                return Err("emit: --stages FILE is required (plain emit arrives with \
                            the shell milestones)"
                    .into());
            };
            Ok(Command::Emit { source, out })
        }
        other => Err(format!("unknown subcommand {other:?}")),
    }
}

fn version_line() -> String {
    format!(
        "frnksh {} (pre-M8 skeleton; the shell lands at M8 — docs/SPEC.md §9)",
        env!("CARGO_PKG_VERSION")
    )
}

fn run(command: Command) -> ExitCode {
    match command {
        Command::Repl => {
            use std::io::{BufRead, Write};
            println!("{}", version_line());
            println!("the frankish shell — ml_core on the reference interpreter (:help, :q)");
            let mut engine = frk_repl::Engine::new();
            if let Ok(cwd) = std::env::current_dir() {
                engine.load_base = cwd;
            }
            let stdin = std::io::stdin();
            let mut stdout = std::io::stdout();
            loop {
                print!("{}", engine.prompt());
                let _ = stdout.flush();
                let mut line = String::new();
                match stdin.lock().read_line(&mut line) {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {}
                }
                let response = engine.feed(&line);
                if !response.is_empty() {
                    println!("{response}");
                }
                if engine.done {
                    break;
                }
            }
            ExitCode::SUCCESS
        }
        Command::Run { source } => {
            // End-to-end on the reference interpreter (D-008): .ml via
            // the ml_core frontend; .mlir parsed directly.
            let context = frk_core::context();
            if let Err(error) = frk_dialects::register(&context) {
                eprintln!("frnksh run: {error}");
                return ExitCode::from(2);
            }
            let text = match std::fs::read_to_string(&source) {
                Ok(text) => text,
                Err(error) => {
                    eprintln!("frnksh run: {}: {error}", source.display());
                    return ExitCode::from(2);
                }
            };
            let is_ml = source.extension().is_some_and(|ext| ext == "ml");
            let module = if is_ml {
                match frk_front::compile_ml(&context, &text) {
                    Ok(module) => module,
                    Err(error) => {
                        eprintln!("frnksh run: {error}");
                        return ExitCode::FAILURE;
                    }
                }
            } else {
                match melior::ir::Module::parse(&context, &text) {
                    Some(module) => module,
                    None => {
                        eprintln!("frnksh run: {}: parse failed", source.display());
                        return ExitCode::FAILURE;
                    }
                }
            };
            if let Err(error) = frk_dialects::verify(&context, &module) {
                eprintln!("frnksh run: verify: {error}");
                return ExitCode::FAILURE;
            }
            let mut interp = match frk_interp::Interp::new(&module) {
                Ok(interp) => interp,
                Err(error) => {
                    eprintln!("frnksh run: {error}");
                    return ExitCode::FAILURE;
                }
            };
            frk_dialects::register_eval(&mut interp);
            match interp.eval_function("main", &[]) {
                Ok(values) => match values[0].as_signed() {
                    Ok(value) => {
                        println!("{value}");
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("frnksh run: {error}");
                        ExitCode::FAILURE
                    }
                },
                Err(error) => {
                    eprintln!("frnksh run: {error}");
                    ExitCode::FAILURE
                }
            }
        }
        Command::Test { goldens } => {
            let mut green = true;
            for runner in default_runners() {
                match run_goldens(&goldens, runner.as_ref(), Mode::Check) {
                    Ok(report) => {
                        println!("{report}");
                        green &= report.is_green();
                    }
                    // A runner with nothing applicable is normal on
                    // kind-homogeneous subsets (e.g. goldens/repl);
                    // the full corpus exercises every runner.
                    Err(frk_harness::case::CaseError::NothingApplies { .. }) => {
                        println!("goldens[{}]: nothing applicable, skipped", runner.name());
                    }
                    Err(error) => {
                        eprintln!("frnksh test: {error}");
                        return ExitCode::from(2);
                    }
                }
            }
            if green { ExitCode::SUCCESS } else { ExitCode::FAILURE }
        }
        Command::Bless { goldens } => {
            let runner = reference_runner();
            match run_goldens(&goldens, runner.as_ref(), Mode::Bless) {
                Ok(report) => {
                    println!("{report}");
                    println!(
                        "L2: blessing requires a commit-message line explaining why \
                         the bytes changed."
                    );
                    if report.is_green() { ExitCode::SUCCESS } else { ExitCode::FAILURE }
                }
                Err(error) => {
                    eprintln!("frnksh bless: {error}");
                    ExitCode::from(2)
                }
            }
        }
        Command::Diff { goldens } => {
            let runners = default_runners();
            let refs: Vec<&dyn frk_harness::runner::Runner> =
                runners.iter().map(|boxed| boxed.as_ref()).collect();
            match diff_corpus(&goldens, &refs) {
                Ok(report) => {
                    println!("{report}");
                    if report.is_green() { ExitCode::SUCCESS } else { ExitCode::FAILURE }
                }
                Err(error) => {
                    eprintln!("frnksh diff: {error}");
                    ExitCode::from(2)
                }
            }
        }
        Command::Dashboard { goldens } => {
            match frk_harness::dashboard::dashboard(&goldens) {
                Ok(board) => {
                    print!("{board}");
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("frnksh dashboard: {error}");
                    ExitCode::from(2)
                }
            }
        }
        Command::Grid { goldens, canary, native_only } => {
            use frk_dialects::Strategy;
            use frk_harness::runner::{AotRunner, Triple};
            let triples: Vec<Triple> = if canary {
                vec![Triple::S390xLinux]
            } else if native_only {
                vec![Triple::X86_64Linux]
            } else {
                Triple::GRID.to_vec()
            };
            let mut green = true;
            println!("{:<16} {:>8} {:>8}", "triple", "arena", "rc");
            for triple in triples {
                let mut cells = Vec::new();
                for strategy in [Strategy::Arena, Strategy::Rc] {
                    let runner = AotRunner::new(triple, strategy);
                    match run_goldens(&goldens, &runner, Mode::Check) {
                        Ok(report) => {
                            let total = report
                                .outcomes
                                .iter()
                                .filter(|o| {
                                    !matches!(o.status, frk_harness::golden::Status::Skipped)
                                })
                                .count();
                            let ok = report
                                .outcomes
                                .iter()
                                .filter(|o| {
                                    matches!(o.status, frk_harness::golden::Status::Pass)
                                })
                                .count();
                            if !report.is_green() {
                                green = false;
                                eprintln!("{report}");
                            }
                            cells.push(format!("{ok}/{total}"));
                        }
                        Err(error) => {
                            green = false;
                            cells.push("ERR".into());
                            eprintln!("frnksh grid [{}]: {error}", triple.short());
                        }
                    }
                }
                println!("{:<16} {:>8} {:>8}", triple.target(), cells[0], cells[1]);
            }
            if green {
                println!("grid: GREEN (both strategies)");
                ExitCode::SUCCESS
            } else {
                println!("grid: RED");
                ExitCode::FAILURE
            }
        }
        Command::Emit { source, out } => {
            let out = out.unwrap_or_else(|| {
                let stem = source
                    .file_stem()
                    .map(|stem| stem.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "module".to_string());
                PathBuf::from("out/stages").join(stem)
            });
            match dump_stages(&source, &out) {
                Ok(written) => {
                    for path in &written {
                        println!("{}", path.display());
                    }
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("frnksh emit: {error}");
                    ExitCode::FAILURE
                }
            }
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match parse(&args) {
        Ok(command) => run(command),
        Err(message) => {
            eprintln!("frnksh: {message}\n{USAGE}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn version_line_names_the_binary_and_version() {
        let line = version_line();
        assert!(line.starts_with("frnksh "));
        assert!(line.contains(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn bare_invocation_is_the_placeholder() {
        assert_eq!(parse(&[]).unwrap(), Command::Repl);
    }

    #[test]
    fn corpus_subcommands_default_to_goldens_dir() {
        assert_eq!(
            parse(&args(&["test"])).unwrap(),
            Command::Test { goldens: "goldens".into() }
        );
        assert_eq!(
            parse(&args(&["bless"])).unwrap(),
            Command::Bless { goldens: "goldens".into() }
        );
        assert_eq!(
            parse(&args(&["diff"])).unwrap(),
            Command::Diff { goldens: "goldens".into() }
        );
    }

    #[test]
    fn goldens_flag_overrides_dir() {
        assert_eq!(
            parse(&args(&["test", "--goldens", "elsewhere"])).unwrap(),
            Command::Test { goldens: "elsewhere".into() }
        );
    }

    #[test]
    fn emit_requires_stages_file() {
        assert!(parse(&args(&["emit"])).is_err());
        assert_eq!(
            parse(&args(&["emit", "--stages", "x.mlir"])).unwrap(),
            Command::Emit { source: "x.mlir".into(), out: None }
        );
        assert_eq!(
            parse(&args(&["emit", "--stages", "x.mlir", "--out", "d"])).unwrap(),
            Command::Emit { source: "x.mlir".into(), out: Some("d".into()) }
        );
    }

    #[test]
    fn junk_is_a_usage_error() {
        assert!(parse(&args(&["frobnicate"])).is_err());
        assert!(parse(&args(&["test", "--bogus"])).is_err());
        assert!(parse(&args(&["emit", "--stages"])).is_err());
    }
}
