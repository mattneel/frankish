//! The M9 startup-number rig (recorded, not gated — SPEC §13): builds
//! goldens/ts0/fib persistently at out/fib-native via the exact AOT
//! steps, so `time out/fib-native` vs `time node goldens/ts0/fib/case.ts`
//! is an apples-to-apples startup+compute comparison.

use std::process::Command;

use melior::ir::operation::OperationLike;

fn main() {
    let out_dir = std::path::Path::new("out");
    std::fs::create_dir_all(out_dir).unwrap();

    let artifact = Command::new("node")
        .arg("tools/loanword-ts/src/main.ts")
        .arg("goldens/ts0/fib/case.ts")
        .output()
        .expect("producer");
    assert!(artifact.status.success());
    let text = String::from_utf8(artifact.stdout).unwrap();

    let context = frk_core::context();
    frk_dialects::register(&context).unwrap();
    let mut module = frk_front::loanword::compile_loanword(&context, &text).unwrap();

    // Entry rename (D-042) then the arena pipeline.
    // The module-level walk mirrors the AOT runner.
    {
        use melior::ir::BlockLike;
        use melior::ir::attribute::StringAttribute;
        use melior::ir::operation::OperationMutLike;
        let body = module.body();
        let mut next = body.first_operation_mut();
        while let Some(mut op) = next {
            let following = op.next_in_block_mut();
            let matches = op
                .attribute("sym_name")
                .ok()
                .and_then(|a| StringAttribute::try_from(a).ok())
                .is_some_and(|a| a.value() == "main");
            if matches {
                op.set_attribute("sym_name", StringAttribute::new(&context, "frk_entry").into());
                break;
            }
            next = following;
        }
    }
    frk_harness::pipeline::lower_to_llvm(&context, &mut module, frk_dialects::Strategy::Arena)
        .unwrap();

    let mlir_path = out_dir.join("fib.lowered.mlir");
    std::fs::write(&mlir_path, module.as_operation().to_string()).unwrap();

    let prefix = std::env::var("MLIR_SYS_220_PREFIX").unwrap_or("/usr/lib/llvm-22".into());
    let ll = out_dir.join("fib.ll");
    assert!(
        Command::new(format!("{prefix}/bin/mlir-translate"))
            .args(["--mlir-to-llvmir"])
            .arg(&mlir_path)
            .arg("-o")
            .arg(&ll)
            .status()
            .unwrap()
            .success()
    );
    let obj = out_dir.join("fib.o");
    assert!(
        Command::new(format!("{prefix}/bin/clang"))
            .args(["-target", "x86_64-linux-musl", "-O2", "-c"])
            .arg(&ll)
            .arg("-o")
            .arg(&obj)
            .status()
            .unwrap()
            .success()
    );
    let shim = out_dir.join("fib_shim.c");
    std::fs::write(&shim, "extern void frk_entry(void);\nint main(void){ frk_entry(); return 0; }\n").unwrap();
    assert!(
        Command::new("sh")
            .arg("scripts/zigcc.sh")
            .args(["-target", "x86_64-linux-musl", "-O2"])
            .arg(&obj)
            .arg(&shim)
            .arg("crates/frk-rt/c/frk_rt.c")
            .args(["-o"])
            .arg(out_dir.join("fib-native"))
            .status()
            .unwrap()
            .success()
    );
    println!("built out/fib-native");
}
