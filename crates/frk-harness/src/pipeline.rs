//! The lowering pipeline shared by every consumer in the harness: the JIT
//! runner and the stage dumper must see the *same* passes in the same
//! order, so it is defined exactly once, here, as (name, constructor)
//! pairs — the names feed stage-dump file naming (docs/stages.md).

use melior::ir::Module;
use melior::pass::{self, Pass, PassManager};
use melior::{Context, Error};

/// Kernel + upstream dialects → LLVM dialect. Kernel lowerings run
/// first (they emit upstream/llvm ops), then the upstream conversions
/// validated against mlir-opt. Every golden lowers through this table.
pub const UPSTREAM_TO_LLVM: &[(&str, fn() -> Pass)] = &[
    ("lower-frk-adt", frk_dialects::lower_adt_pass),
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
