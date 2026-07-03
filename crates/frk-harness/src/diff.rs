//! Differential runner (SPEC §7.2; law L3): run every case through every
//! applicable runner, canonicalize each output, and byte-compare
//! pairwise. A disagreement between runners is a first-rank finding —
//! halt the feature, file it in STATE.md, fix or fence before proceeding.
//!
//! M1 honesty: [`crate::runner::default_runners`] registers only `jit`,
//! so the matrix is trivially in agreement until M2's interpreter joins
//! (and becomes the reference semantics, D-008). The mechanics below are
//! fully live and self-tested with canned runners.

use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

use crate::canon;
use crate::case::{self, CaseError};
use crate::runner::Runner;

/// One case's canonical output (or error text) per runner. Keyed by
/// runner name in a BTreeMap so iteration is deterministic (canon §3).
#[derive(Debug)]
pub struct CaseDiff {
    pub name: String,
    pub outputs: BTreeMap<&'static str, Result<String, String>>,
}

impl CaseDiff {
    /// Agreement = every runner succeeded and produced identical bytes.
    pub fn agrees(&self) -> bool {
        let mut reference: Option<&str> = None;
        for output in self.outputs.values() {
            match output {
                Err(_) => return false,
                Ok(text) => match reference {
                    None => reference = Some(text),
                    Some(seen) if seen == text => {}
                    Some(_) => return false,
                },
            }
        }
        true
    }
}

#[derive(Debug)]
pub struct DiffReport {
    pub runner_names: Vec<&'static str>,
    pub diffs: Vec<CaseDiff>,
}

impl DiffReport {
    pub fn is_green(&self) -> bool {
        self.diffs.iter().all(CaseDiff::agrees)
    }
}

impl fmt::Display for DiffReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut divergent = 0usize;
        for diff in &self.diffs {
            if diff.agrees() {
                writeln!(f, "agree    {}", diff.name)?;
            } else {
                divergent += 1;
                writeln!(f, "DIVERGE  {}", diff.name)?;
                for (runner, output) in &diff.outputs {
                    match output {
                        Ok(text) => writeln!(f, "         {runner}: {text:?}")?,
                        Err(error) => writeln!(f, "         {runner}: ERROR {error}")?,
                    }
                }
            }
        }
        write!(
            f,
            "diff[{}]: {} case(s), {} divergent",
            self.runner_names.join(","),
            self.diffs.len(),
            divergent
        )?;
        if divergent > 0 {
            write!(
                f,
                "\nL3: a runner disagreement is a first-rank finding — halt the \
                 feature, file it in STATE.md, fix or fence before proceeding."
            )?;
        } else if self.runner_names.len() < 2 {
            write!(
                f,
                "\nnote: single runner registered; pairwise comparison bites when \
                 the interpreter joins at M2."
            )?;
        }
        Ok(())
    }
}

/// Runs the corpus under `root` through all `runners`. Runner names must
/// be distinct (they key the matrix).
pub fn diff_corpus(root: &Path, runners: &[&dyn Runner]) -> Result<DiffReport, CaseError> {
    assert!(!runners.is_empty(), "diff_corpus needs at least one runner");
    let cases = case::discover(root)?;

    let mut diffs = Vec::with_capacity(cases.len());
    for case in &cases {
        let mut outputs = BTreeMap::new();
        for runner in runners {
            if !case.applies_to(runner.name()) {
                continue;
            }
            let output = runner
                .run(case)
                .map(|raw| canon::canonicalize(&raw))
                .map_err(|error| error.to_string());
            let clobbered = outputs.insert(runner.name(), output).is_some();
            assert!(!clobbered, "duplicate runner name {:?}", runner.name());
        }
        if outputs.is_empty() {
            // A case no registered runner can execute is red, not
            // invisible — the likely cause is a typo'd runners= list.
            outputs.insert(
                "(none)",
                Err("no registered runner applies to this case".to_string()),
            );
        }
        diffs.push(CaseDiff {
            name: case.name.clone(),
            outputs,
        });
    }

    Ok(DiffReport {
        runner_names: runners.iter().map(|runner| runner.name()).collect(),
        diffs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{FakeRunner, TempCorpus};

    #[test]
    fn agreeing_runners_are_green() {
        let corpus = TempCorpus::new();
        corpus.add_case("s/a", "irrelevant", None);
        // Different raw flavors, identical canonical bytes — agreement is
        // judged inside the canon contract, never outside it (§7.4).
        let alpha = FakeRunner::named("alpha", "42");
        let beta = FakeRunner::named("beta", "42\n");
        let report = diff_corpus(corpus.root(), &[&alpha, &beta]).unwrap();
        assert!(report.is_green(), "{report}");
    }

    #[test]
    fn divergence_is_red_and_names_both_outputs() {
        let corpus = TempCorpus::new();
        corpus.add_case("s/a", "irrelevant", None);
        let alpha = FakeRunner::named("alpha", "42\n");
        let beta = FakeRunner::named("beta", "41\n");
        let report = diff_corpus(corpus.root(), &[&alpha, &beta]).unwrap();
        assert!(!report.is_green(), "{report}");
        let rendered = report.to_string();
        assert!(rendered.contains("DIVERGE"), "{rendered}");
        assert!(rendered.contains("first-rank finding"), "{rendered}");
    }

    #[test]
    fn a_runner_error_is_divergence() {
        let corpus = TempCorpus::new();
        corpus.add_case("s/a", "irrelevant", None);
        let ok = FakeRunner::named("ok", "42\n");
        let bad = FakeRunner::failing("engine exploded");
        let report = diff_corpus(corpus.root(), &[&ok, &bad]).unwrap();
        assert!(!report.is_green(), "{report}");
    }

    #[test]
    fn runner_directives_narrow_the_matrix_without_reddening_it() {
        let corpus = TempCorpus::new();
        corpus.add_case("s/both", "irrelevant", None);
        corpus.add_case("s/only_alpha", "// frk-case: runners=alpha\n", None);
        let alpha = FakeRunner::named("alpha", "1\n");
        let beta = FakeRunner::named("beta", "1\n");
        let report = diff_corpus(corpus.root(), &[&alpha, &beta]).unwrap();
        assert!(report.is_green(), "{report}");
    }

    #[test]
    fn a_case_no_runner_can_execute_is_red() {
        let corpus = TempCorpus::new();
        corpus.add_case("s/orphan", "// frk-case: runners=ghost\n", None);
        let alpha = FakeRunner::named("alpha", "1\n");
        let report = diff_corpus(corpus.root(), &[&alpha]).unwrap();
        assert!(!report.is_green(), "{report}");
        assert!(
            report.to_string().contains("no registered runner"),
            "{report}"
        );
    }

    #[test]
    fn single_runner_agrees_trivially_and_says_so() {
        let corpus = TempCorpus::new();
        corpus.add_case("s/a", "irrelevant", None);
        let only = FakeRunner::named("jit", "42\n");
        let report = diff_corpus(corpus.root(), &[&only]).unwrap();
        assert!(report.is_green(), "{report}");
        assert!(report.to_string().contains("single runner"), "{report}");
    }
}
