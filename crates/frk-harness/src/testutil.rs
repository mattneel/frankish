//! Test-only helpers: throwaway corpora on disk and canned runners. No
//! external dependencies — a temp dir with a Drop impl and a runner that
//! returns fixed bytes are all the harness self-tests need.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::case::Case;
use crate::runner::{RunError, Runner};

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

/// A corpus root under the system temp dir, removed on drop.
pub struct TempCorpus {
    root: PathBuf,
}

impl TempCorpus {
    pub fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "frk-harness-selftest-{}-{}",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).expect("creating temp corpus root");
        Self { root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Creates `<name>/case.mlir` (with `source`) and, when given,
    /// `<name>/expected.out`.
    pub fn add_case(&self, name: &str, source: &str, expected: Option<&str>) {
        let dir = self.root.join(name);
        fs::create_dir_all(&dir).expect("creating case dir");
        fs::write(dir.join("case.mlir"), source).expect("writing case.mlir");
        if let Some(expected) = expected {
            fs::write(dir.join("expected.out"), expected).expect("writing expected.out");
        }
    }

    pub fn read(&self, case: &str, file: &str) -> String {
        fs::read_to_string(self.root.join(case).join(file)).expect("reading corpus file")
    }

    pub fn exists(&self, case: &str, file: &str) -> bool {
        self.root.join(case).join(file).exists()
    }
}

impl Drop for TempCorpus {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// A runner returning canned raw output (or a canned failure) for every
/// case — exercises engine/diff machinery without MLIR in the loop.
pub struct FakeRunner {
    name: &'static str,
    output: Result<String, String>,
}

impl FakeRunner {
    pub fn fixed(raw: &str) -> Self {
        Self {
            name: "fake",
            output: Ok(raw.to_string()),
        }
    }

    pub fn failing(message: &str) -> Self {
        Self {
            name: "fake",
            output: Err(message.to_string()),
        }
    }
}

impl Runner for FakeRunner {
    fn name(&self) -> &'static str {
        self.name
    }

    fn run(&self, _case: &Case) -> Result<String, RunError> {
        self.output
            .clone()
            .map_err(RunError::Invoke)
    }
}
