//! Stage dumps (SPEC §7.3): numbered per-pass IR snapshots into a
//! directory — diffable, the pedagogy artifact. Format: docs/stages.md.
//!
//! Ruled in D-028: v0 runs one single-pass PassManager per pipeline entry
//! so each snapshot is exactly "the module after pass N", at the cost of
//! not exercising multi-pass manager scheduling here (the JIT runner
//! covers that path over the same table).

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use melior::ir::Module;
use melior::ir::operation::OperationLike;
use melior::pass::PassManager;

use crate::pipeline;

#[derive(Debug)]
pub enum StageError {
    Io(PathBuf, io::Error),
    Parse(PathBuf),
    Verify(PathBuf),
    Lower { stage: String, message: String },
}

impl fmt::Display for StageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(path, error) => write!(f, "{}: {error}", path.display()),
            Self::Parse(path) => write!(f, "{}: parse failed", path.display()),
            Self::Verify(path) => {
                write!(f, "{}: module failed MLIR verification", path.display())
            }
            Self::Lower { stage, message } => write!(f, "stage {stage}: {message}"),
        }
    }
}

impl std::error::Error for StageError {}

/// Parses `source_path`, then writes `00-parsed.mlir` and one
/// `NN-<pass-name>.mlir` per entry of [`pipeline::UPSTREAM_TO_LLVM`] into
/// `out_dir`, which is removed and recreated whole (stale snapshots must
/// not linger). Returns the written paths in order.
pub fn dump_stages(source_path: &Path, out_dir: &Path) -> Result<Vec<PathBuf>, StageError> {
    let source = fs::read_to_string(source_path)
        .map_err(|e| StageError::Io(source_path.to_path_buf(), e))?;

    let context = frk_core::context();
    let mut module = Module::parse(&context, &source)
        .ok_or_else(|| StageError::Parse(source_path.to_path_buf()))?;
    if !module.as_operation().verify() {
        return Err(StageError::Verify(source_path.to_path_buf()));
    }

    if out_dir.exists() {
        fs::remove_dir_all(out_dir).map_err(|e| StageError::Io(out_dir.to_path_buf(), e))?;
    }
    fs::create_dir_all(out_dir).map_err(|e| StageError::Io(out_dir.to_path_buf(), e))?;

    let mut written = Vec::with_capacity(1 + pipeline::UPSTREAM_TO_LLVM.len());
    written.push(snapshot(out_dir, 0, "parsed", &module)?);

    for (index, (name, constructor)) in pipeline::UPSTREAM_TO_LLVM.iter().enumerate() {
        let manager = PassManager::new(&context);
        manager.add_pass(constructor());
        manager.run(&mut module).map_err(|e| StageError::Lower {
            stage: (*name).to_string(),
            message: format!("{e}"),
        })?;
        written.push(snapshot(out_dir, index + 1, name, &module)?);
    }

    Ok(written)
}

fn snapshot(
    out_dir: &Path,
    index: usize,
    name: &str,
    module: &Module,
) -> Result<PathBuf, StageError> {
    let path = out_dir.join(format!("{index:02}-{name}.mlir"));
    fs::write(&path, format!("{}", module.as_operation()))
        .map_err(|e| StageError::Io(path.clone(), e))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempCorpus;

    const ADD_SOURCE: &str = "func.func @main() -> i64 {\n  \
        %a = arith.constant 40 : i64\n  %b = arith.constant 2 : i64\n  \
        %s = arith.addi %a, %b : i64\n  return %s : i64\n}\n";

    #[test]
    fn dumps_are_numbered_named_and_lowered() {
        let corpus = TempCorpus::new();
        corpus.add_case("s/a", ADD_SOURCE, None);
        let source = corpus.root().join("s/a/case.mlir");
        let out = corpus.root().join("out");

        let written = dump_stages(&source, &out).unwrap();
        let names: Vec<_> = written
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            names,
            [
                "00-parsed.mlir",
                "01-convert-scf-to-cf.mlir",
                "02-convert-to-llvm.mlir",
                "03-reconcile-unrealized-casts.mlir",
            ]
        );

        let parsed = fs::read_to_string(&written[0]).unwrap();
        assert!(parsed.contains("func.func"), "{parsed}");
        let lowered = fs::read_to_string(&written[2]).unwrap();
        assert!(lowered.contains("llvm.func"), "{lowered}");
    }

    #[test]
    fn out_dir_is_recreated_whole() {
        let corpus = TempCorpus::new();
        corpus.add_case("s/a", ADD_SOURCE, None);
        let source = corpus.root().join("s/a/case.mlir");
        let out = corpus.root().join("out");

        fs::create_dir_all(&out).unwrap();
        fs::write(out.join("99-stale.mlir"), "junk").unwrap();

        dump_stages(&source, &out).unwrap();
        assert!(!out.join("99-stale.mlir").exists());
    }
}
