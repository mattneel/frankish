//! The conformance dashboard (SPEC §8): conformance % per suite per
//! runner — a number, not a vibe. Denominators count applicable cases
//! only; a dash means the runner doesn't speak that suite's language.

use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

use crate::case::{self, CaseError};
use crate::golden::{Mode, Status, run_goldens};
use crate::runner::default_runners;

pub struct Dashboard {
    pub runner_names: Vec<&'static str>,
    /// suite → runner → (green, applicable).
    pub rows: BTreeMap<String, BTreeMap<&'static str, (usize, usize)>>,
}

impl fmt::Display for Dashboard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:<12} {:>6}", "suite", "cases")?;
        for name in &self.runner_names {
            write!(f, " {name:>8}")?;
        }
        writeln!(f)?;
        for (suite, per_runner) in &self.rows {
            let cases = per_runner
                .values()
                .map(|(_, applicable)| *applicable)
                .max()
                .unwrap_or(0);
            write!(f, "{suite:<12} {cases:>6}")?;
            for name in &self.runner_names {
                match per_runner.get(name) {
                    Some((green, applicable)) if *applicable > 0 => {
                        let percent = 100.0 * *green as f64 / *applicable as f64;
                        write!(f, " {percent:>7.1}%")?;
                    }
                    _ => write!(f, " {:>8}", "—")?,
                }
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

pub fn dashboard(root: &Path) -> Result<Dashboard, CaseError> {
    let cases = case::discover(root)?;
    let suite_of = |name: &str| -> String {
        name.split('/').next().unwrap_or(name).to_string()
    };

    let runners = default_runners();
    let mut rows: BTreeMap<String, BTreeMap<&'static str, (usize, usize)>> = BTreeMap::new();
    for case in &cases {
        rows.entry(suite_of(&case.name)).or_default();
    }

    for runner in &runners {
        let report = match run_goldens(root, runner.as_ref(), Mode::Check) {
            Ok(report) => report,
            // A runner with nothing applicable simply has no column data.
            Err(CaseError::NothingApplies { .. }) => continue,
            Err(other) => return Err(other),
        };
        for outcome in &report.outcomes {
            let suite = suite_of(&outcome.name);
            let entry = rows
                .entry(suite)
                .or_default()
                .entry(runner.name())
                .or_insert((0, 0));
            match outcome.status {
                Status::Skipped => {}
                Status::Pass | Status::Blessed { .. } => {
                    entry.0 += 1;
                    entry.1 += 1;
                }
                _ => entry.1 += 1,
            }
        }
    }

    Ok(Dashboard {
        runner_names: runners.iter().map(|runner| runner.name()).collect(),
        rows,
    })
}
