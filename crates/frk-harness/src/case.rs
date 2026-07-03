//! Golden case discovery and the `// frk-case:` directive format.
//! The corpus layout and directive vocabulary are law-adjacent: they are
//! documented in goldens/README.md and ruled in D-027.

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// The result type an entry function returns. v0 supports exactly one
/// (docs/canon.md §2); widening this enum widens the canon contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResultKind {
    I64,
}

/// One golden case: a directory holding `case.mlir` + `expected.out`.
#[derive(Clone, Debug)]
pub struct Case {
    /// Root-relative name with `/` separators, e.g. `upstream/add_i64`.
    pub name: String,
    pub dir: PathBuf,
    pub source_path: PathBuf,
    pub expected_path: PathBuf,
    /// Entry function symbol (directive `entry=`, default `main`).
    pub entry: String,
    /// Entry result rendering (directive `result=`, default `i64`).
    pub result: ResultKind,
    /// Runner names this case applies to (directive `runners=a,b`);
    /// None = every runner (SPEC §7.2 "all applicable runners"). Used
    /// while an op set is ahead of some execution path (e.g. adt before
    /// its K3 lowering); a skip is reported, never silent.
    pub runners: Option<Vec<String>>,
}

impl Case {
    pub fn applies_to(&self, runner: &str) -> bool {
        self.runners
            .as_ref()
            .is_none_or(|list| list.iter().any(|name| name == runner))
    }
}

#[derive(Debug)]
pub enum CaseError {
    Io(PathBuf, io::Error),
    Directive { case: PathBuf, message: String },
    EmptyCorpus(PathBuf),
    /// Every case's `runners=` directive excluded this runner — almost
    /// certainly a typo'd runner name, never a green suite.
    NothingApplies { root: PathBuf, runner: String },
}

impl fmt::Display for CaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(path, error) => write!(f, "{}: {error}", path.display()),
            Self::Directive { case, message } => {
                write!(f, "{}: bad frk-case directive: {message}", case.display())
            }
            Self::EmptyCorpus(root) => write!(
                f,
                "no cases found under {} (a corpus with zero cases is a wrong path, \
                 not a green suite)",
                root.display()
            ),
            Self::NothingApplies { root, runner } => write!(
                f,
                "every case under {} excludes runner {runner:?} via runners= \
                 directives — typo'd runner name?",
                root.display()
            ),
        }
    }
}

impl std::error::Error for CaseError {}

const SOURCE_FILE: &str = "case.mlir";
const EXPECTED_FILE: &str = "expected.out";
const DIRECTIVE_PREFIX: &str = "// frk-case:";

/// Walks `root` and returns every directory containing a `case.mlir`,
/// sorted by name so reports and diffs are deterministic (canon §3 spirit).
/// Zero cases is an error, never a vacuous green.
pub fn discover(root: &Path) -> Result<Vec<Case>, CaseError> {
    let mut cases = Vec::new();
    walk(root, root, &mut cases)?;
    if cases.is_empty() {
        return Err(CaseError::EmptyCorpus(root.to_path_buf()));
    }
    cases.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(cases)
}

fn walk(root: &Path, dir: &Path, cases: &mut Vec<Case>) -> Result<(), CaseError> {
    let source_path = dir.join(SOURCE_FILE);
    if source_path.is_file() {
        cases.push(load(root, dir, source_path)?);
        return Ok(());
    }
    let entries = fs::read_dir(dir).map_err(|e| CaseError::Io(dir.to_path_buf(), e))?;
    for entry in entries {
        let entry = entry.map_err(|e| CaseError::Io(dir.to_path_buf(), e))?;
        let path = entry.path();
        if path.is_dir() {
            walk(root, &path, cases)?;
        }
    }
    Ok(())
}

fn load(root: &Path, dir: &Path, source_path: PathBuf) -> Result<Case, CaseError> {
    let source = fs::read_to_string(&source_path)
        .map_err(|e| CaseError::Io(source_path.clone(), e))?;

    let mut entry = String::from("main");
    let mut result = ResultKind::I64;
    let mut runners = None;

    for line in source.lines() {
        let Some(directive) = line.trim().strip_prefix(DIRECTIVE_PREFIX) else {
            continue;
        };
        let directive = directive.trim();
        let Some((key, value)) = directive.split_once('=') else {
            return Err(CaseError::Directive {
                case: source_path.clone(),
                message: format!("expected key=value, got {directive:?}"),
            });
        };
        match (key.trim(), value.trim()) {
            ("entry", v) if !v.is_empty() => entry = v.to_string(),
            ("result", "i64") => result = ResultKind::I64,
            ("result", other) => {
                return Err(CaseError::Directive {
                    case: source_path.clone(),
                    message: format!("unsupported result type {other:?} (v0: i64)"),
                });
            }
            ("runners", list) => {
                let names: Vec<String> =
                    list.split(',').map(|name| name.trim().to_string()).collect();
                if names.is_empty() || names.iter().any(String::is_empty) {
                    return Err(CaseError::Directive {
                        case: source_path.clone(),
                        message: format!("runners needs a comma-separated list, got {list:?}"),
                    });
                }
                runners = Some(names);
            }
            (other, _) => {
                return Err(CaseError::Directive {
                    case: source_path.clone(),
                    message: format!("unknown key {other:?} (known: entry, result, runners)"),
                });
            }
        }
    }

    let name = dir
        .strip_prefix(root)
        .unwrap_or(dir)
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/");

    Ok(Case {
        name,
        dir: dir.to_path_buf(),
        source_path,
        expected_path: dir.join(EXPECTED_FILE),
        entry,
        result,
        runners,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempCorpus;

    #[test]
    fn discovers_cases_sorted_with_defaults() {
        let corpus = TempCorpus::new();
        corpus.add_case("suite/zeta", "func.func @main() { return }", Some("0\n"));
        corpus.add_case("suite/alpha", "func.func @main() { return }", Some("0\n"));

        let cases = discover(corpus.root()).unwrap();
        let names: Vec<_> = cases.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["suite/alpha", "suite/zeta"]);
        assert_eq!(cases[0].entry, "main");
        assert_eq!(cases[0].result, ResultKind::I64);
    }

    #[test]
    fn directives_override_defaults() {
        let corpus = TempCorpus::new();
        corpus.add_case(
            "s/c",
            "// frk-case: entry=start\n// frk-case: result=i64\nfunc.func @start() { return }",
            Some("0\n"),
        );
        let cases = discover(corpus.root()).unwrap();
        assert_eq!(cases[0].entry, "start");
    }

    #[test]
    fn runners_directive_parses_and_gates_applicability() {
        let corpus = TempCorpus::new();
        corpus.add_case(
            "s/c",
            "// frk-case: runners=interp, jit\nfunc.func @main() { return }",
            Some("0\n"),
        );
        let cases = discover(corpus.root()).unwrap();
        assert!(cases[0].applies_to("interp"));
        assert!(cases[0].applies_to("jit"));
        assert!(!cases[0].applies_to("aot"));

        let unrestricted = {
            let corpus = TempCorpus::new();
            corpus.add_case("s/c", "func.func @main() { return }", None);
            discover(corpus.root()).unwrap().remove(0)
        };
        assert!(unrestricted.applies_to("anything"));
    }

    #[test]
    fn empty_runners_directive_is_an_error() {
        let corpus = TempCorpus::new();
        corpus.add_case("s/c", "// frk-case: runners=\n", Some("0\n"));
        let error = discover(corpus.root()).unwrap_err();
        assert!(matches!(error, CaseError::Directive { .. }), "{error}");
    }

    #[test]
    fn unknown_directive_key_is_an_error() {
        let corpus = TempCorpus::new();
        corpus.add_case("s/c", "// frk-case: entree=main\n", Some("0\n"));
        let error = discover(corpus.root()).unwrap_err();
        assert!(matches!(error, CaseError::Directive { .. }), "{error}");
    }

    #[test]
    fn unsupported_result_type_is_an_error() {
        let corpus = TempCorpus::new();
        corpus.add_case("s/c", "// frk-case: result=f64\n", Some("0\n"));
        let error = discover(corpus.root()).unwrap_err();
        assert!(matches!(error, CaseError::Directive { .. }), "{error}");
    }

    #[test]
    fn empty_corpus_is_an_error_not_a_green_suite() {
        let corpus = TempCorpus::new();
        let error = discover(corpus.root()).unwrap_err();
        assert!(matches!(error, CaseError::EmptyCorpus(_)), "{error}");
    }
}
