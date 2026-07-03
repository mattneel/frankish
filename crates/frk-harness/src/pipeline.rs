//! The lowering pipeline shared by every consumer in the harness: the
//! JIT runners and the stage dumper must see the *same* passes in the
//! same order, so it is defined exactly once, here. The memory strategy
//! (D-041) parameterizes only the kernel stage; upstream conversions
//! are strategy-blind.

use frk_dialects::Strategy;
use melior::ir::Module;
use melior::pass::{self, Pass, PassManager};
use melior::{Context, Error};

/// Stage names + fresh Pass objects for one strategy, in order.
pub fn stages(strategy: Strategy) -> Vec<(&'static str, Pass)> {
    vec![
        ("lower-frk-kernel", frk_dialects::lower_kernel_pass(strategy)),
        (
            "convert-scf-to-cf",
            pass::conversion::create_scf_to_control_flow(),
        ),
        ("convert-to-llvm", pass::conversion::create_to_llvm()),
        (
            "reconcile-unrealized-casts",
            pass::conversion::create_reconcile_unrealized_casts(),
        ),
    ]
}

/// Runs the whole pipeline over a module in one pass manager.
pub fn lower_to_llvm(
    context: &Context,
    module: &mut Module,
    strategy: Strategy,
) -> Result<(), Error> {
    let manager = PassManager::new(context);
    for (_, pass) in stages(strategy) {
        manager.add_pass(pass);
    }
    manager.run(module)
}
