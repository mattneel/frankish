//! The lowering pipeline shared by every consumer in the harness: the JIT
//! runner and the stage dumper must see the *same* passes in the same
//! order, so it is defined exactly once, here, as (name, constructor)
//! pairs — the names feed stage-dump file naming (docs/stages.md).

use melior::ir::Module;
use melior::pass::{self, Pass, PassManager};
use melior::{Context, Error};

/// Upstream dialects (func/arith/scf/cf) → LLVM dialect. Validated against
/// mlir-opt with the same pass names; goldens/upstream/* all lower through
/// this.
pub const UPSTREAM_TO_LLVM: &[(&str, fn() -> Pass)] = &[
    ("convert-scf-to-cf", pass::conversion::create_scf_to_control_flow),
    ("convert-to-llvm", pass::conversion::create_to_llvm),
    (
        "reconcile-unrealized-casts",
        pass::conversion::create_reconcile_unrealized_casts,
    ),
];

/// Runs the whole [`UPSTREAM_TO_LLVM`] pipeline over a module in one pass
/// manager.
pub fn lower_to_llvm(context: &Context, module: &mut Module) -> Result<(), Error> {
    let manager = PassManager::new(context);
    for (_, constructor) in UPSTREAM_TO_LLVM {
        manager.add_pass(constructor());
    }
    manager.run(module)
}
