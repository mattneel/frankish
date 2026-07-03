//! The golden engine: run a corpus through one runner and byte-compare
//! canonicalized output against blessed expectations (law L2), or bless
//! new expectations. Blessing is mechanical here; the *justification*
//! requirement lives in the commit message (AGENTS.md L2) — never bless a
//! diff you don't understand.

use std::fmt;
use std::fs;
use std::path::Path;

use crate::canon;
use crate::case::{self, Case, CaseError};
use crate::runner::Runner;

/// What to do with the corpus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Compare against expected.out; mismatches write output.actual.
    Check,
    /// Overwrite expected.out with current canonical output (L2 applies).
    Bless,
}

#[derive(Debug)]
pub enum Status {
    Pass,
    /// Bless wrote expected.out; `changed` is false when bytes were
    /// already identical.
    Blessed { changed: bool },
    /// The case's `runners=` directive excludes this runner. Reported,
    /// never silent; a corpus where *everything* skips is an error.
    Skipped,
    /// Canonical output differs from expected.out; the actual bytes were
    /// written next to it as output.actual (gitignored).
    Mismatch,
    /// expected.out does not exist — an unblessed case is red, not green.
    MissingExpected,
    /// The runner failed (parse/verify/lower/invoke/io).
    Error(String),
}

#[derive(Debug)]
pub struct CaseOutcome {
    pub name: String,
    pub status: Status,
}

#[derive(Debug)]
pub struct Report {
    pub runner: &'static str,
    pub outcomes: Vec<CaseOutcome>,
}

impl Report {
    pub fn is_green(&self) -> bool {
        self.outcomes.iter().all(|outcome| {
            matches!(
                outcome.status,
                Status::Pass | Status::Blessed { .. } | Status::Skipped
            )
        })
    }
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut passed = 0usize;
        let mut red = 0usize;
        for outcome in &self.outcomes {
            let line = match &outcome.status {
                Status::Pass => {
                    passed += 1;
                    format!("ok      {}", outcome.name)
                }
                Status::Blessed { changed: true } => {
                    passed += 1;
                    format!("blessed {} (bytes changed)", outcome.name)
                }
                Status::Blessed { changed: false } => {
                    passed += 1;
                    format!("blessed {} (unchanged)", outcome.name)
                }
                Status::Skipped => {
                    format!("skip    {} (not applicable to this runner)", outcome.name)
                }
                Status::Mismatch => {
                    red += 1;
                    format!(
                        "FAIL    {} — output.actual written; diff it against expected.out",
                        outcome.name
                    )
                }
                Status::MissingExpected => {
                    red += 1;
                    format!(
                        "FAIL    {} — no expected.out (write it by hand or `make bless` \
                         with an L2 justification)",
                        outcome.name
                    )
                }
                Status::Error(message) => {
                    red += 1;
                    format!("ERROR   {} — {message}", outcome.name)
                }
            };
            writeln!(f, "{line}")?;
        }
        write!(
            f,
            "goldens[{}]: {} case(s), {} green, {} red",
            self.runner,
            self.outcomes.len(),
            passed,
            red
        )
    }
}

/// Name of the failure artifact written next to expected.out. Matches the
/// `/goldens/**/*.actual` gitignore pattern.
const ACTUAL_FILE: &str = "output.actual";

/// Runs every applicable case under `root` through `runner` in `mode`.
/// A corpus where nothing applies to this runner is an error (a typo'd
/// `runners=` directive must not read as green).
pub fn run_goldens(root: &Path, runner: &dyn Runner, mode: Mode) -> Result<Report, CaseError> {
    let cases = case::discover(root)?;
    let outcomes: Vec<CaseOutcome> = cases
        .iter()
        .map(|case| CaseOutcome {
            name: case.name.clone(),
            status: if case.applies_to(runner.name()) {
                run_case(case, runner, mode)
            } else {
                Status::Skipped
            },
        })
        .collect();
    if outcomes
        .iter()
        .all(|outcome| matches!(outcome.status, Status::Skipped))
    {
        return Err(CaseError::NothingApplies {
            root: root.to_path_buf(),
            runner: runner.name().to_string(),
        });
    }
    Ok(Report {
        runner: runner.name(),
        outcomes,
    })
}

fn run_case(case: &Case, runner: &dyn Runner, mode: Mode) -> Status {
    let raw = match runner.run(case) {
        Ok(raw) => raw,
        Err(error) => return Status::Error(error.to_string()),
    };
    let actual = canon::canonicalize(&raw);
    let actual_path = case.dir.join(ACTUAL_FILE);

    match mode {
        Mode::Bless => {
            let previous = fs::read_to_string(&case.expected_path).ok();
            let changed = previous.as_deref() != Some(actual.as_str());
            if let Err(error) = fs::write(&case.expected_path, &actual) {
                return Status::Error(format!("writing expected.out: {error}"));
            }
            let _ = fs::remove_file(&actual_path);
            Status::Blessed { changed }
        }
        Mode::Check => match fs::read_to_string(&case.expected_path) {
            Ok(expected) if expected == actual => {
                let _ = fs::remove_file(&actual_path);
                Status::Pass
            }
            Ok(_) => {
                if let Err(error) = fs::write(&actual_path, &actual) {
                    return Status::Error(format!("writing output.actual: {error}"));
                }
                Status::Mismatch
            }
            Err(_) => Status::MissingExpected,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{FakeRunner, TempCorpus};

    #[test]
    fn check_passes_when_canonical_output_matches() {
        let corpus = TempCorpus::new();
        corpus.add_case("s/a", "irrelevant", Some("42\n"));
        // Raw output lacks the trailing LF; canon adds it (canon.md §1).
        let runner = FakeRunner::fixed("42");
        let report = run_goldens(corpus.root(), &runner, Mode::Check).unwrap();
        assert!(report.is_green(), "{report}");
    }

    #[test]
    fn check_mismatch_is_red_and_writes_actual() {
        let corpus = TempCorpus::new();
        corpus.add_case("s/a", "irrelevant", Some("42\n"));
        let runner = FakeRunner::fixed("41");
        let report = run_goldens(corpus.root(), &runner, Mode::Check).unwrap();
        assert!(!report.is_green(), "{report}");
        let actual = corpus.read("s/a", "output.actual");
        assert_eq!(actual, "41\n");
    }

    #[test]
    fn check_missing_expected_is_red() {
        let corpus = TempCorpus::new();
        corpus.add_case("s/a", "irrelevant", None);
        let runner = FakeRunner::fixed("42");
        let report = run_goldens(corpus.root(), &runner, Mode::Check).unwrap();
        assert!(!report.is_green(), "{report}");
        assert!(matches!(
            report.outcomes[0].status,
            Status::MissingExpected
        ));
    }

    #[test]
    fn runner_error_is_red_not_a_panic() {
        let corpus = TempCorpus::new();
        corpus.add_case("s/a", "irrelevant", Some("42\n"));
        let runner = FakeRunner::failing("boom");
        let report = run_goldens(corpus.root(), &runner, Mode::Check).unwrap();
        assert!(!report.is_green(), "{report}");
        assert!(matches!(report.outcomes[0].status, Status::Error(_)));
    }

    #[test]
    fn bless_writes_canonical_bytes_and_clears_actual() {
        let corpus = TempCorpus::new();
        corpus.add_case("s/a", "irrelevant", Some("41\n"));
        // Leave a stale failure artifact behind, then bless over it.
        let failing = FakeRunner::fixed("42");
        run_goldens(corpus.root(), &failing, Mode::Check).unwrap();
        assert!(corpus.exists("s/a", "output.actual"));

        let report = run_goldens(corpus.root(), &failing, Mode::Bless).unwrap();
        assert!(report.is_green(), "{report}");
        assert!(matches!(
            report.outcomes[0].status,
            Status::Blessed { changed: true }
        ));
        assert_eq!(corpus.read("s/a", "expected.out"), "42\n");
        assert!(!corpus.exists("s/a", "output.actual"));

        // Blessing again reports unchanged — the L2 smell test for
        // pointless blesses.
        let report = run_goldens(corpus.root(), &failing, Mode::Bless).unwrap();
        assert!(matches!(
            report.outcomes[0].status,
            Status::Blessed { changed: false }
        ));
    }

    #[test]
    fn non_applicable_cases_skip_visibly_and_stay_green() {
        let corpus = TempCorpus::new();
        corpus.add_case("s/runs", "irrelevant", Some("42\n"));
        corpus.add_case(
            "s/skips",
            "// frk-case: runners=some_other_runner\n",
            Some("99\n"),
        );
        let runner = FakeRunner::fixed("42");
        let report = run_goldens(corpus.root(), &runner, Mode::Check).unwrap();
        assert!(report.is_green(), "{report}");
        assert!(report.to_string().contains("skip    s/skips"), "{report}");
    }

    #[test]
    fn a_corpus_where_everything_skips_is_an_error() {
        let corpus = TempCorpus::new();
        corpus.add_case("s/a", "// frk-case: runners=elsewhere\n", Some("1\n"));
        let runner = FakeRunner::fixed("1");
        let error = run_goldens(corpus.root(), &runner, Mode::Check).unwrap_err();
        assert!(
            matches!(error, crate::case::CaseError::NothingApplies { .. }),
            "{error}"
        );
    }

    #[test]
    fn pass_clears_stale_actual() {
        let corpus = TempCorpus::new();
        corpus.add_case("s/a", "irrelevant", Some("42\n"));
        let bad = FakeRunner::fixed("41");
        run_goldens(corpus.root(), &bad, Mode::Check).unwrap();
        assert!(corpus.exists("s/a", "output.actual"));

        let good = FakeRunner::fixed("42");
        let report = run_goldens(corpus.root(), &good, Mode::Check).unwrap();
        assert!(report.is_green(), "{report}");
        assert!(!corpus.exists("s/a", "output.actual"));
    }
}
