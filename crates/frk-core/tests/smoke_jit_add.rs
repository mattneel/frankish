//! M0 smoke verifier (SPEC §13, M0): construct `add(i64, i64) -> i64` with
//! the melior builder API, lower to the LLVM dialect, JIT it through
//! ExecutionEngine, and assert the result — proving the pinned toolchain
//! (versions.env → melior → libMLIR → ORC JIT) end to end.
//!
//! Law L1: this test is the milestone's verifier and lands in the same
//! commit as the melior pin it verifies.

use melior::{
    ExecutionEngine,
    dialect::{arith, func},
    ir::{
        Attribute, Block, BlockLike, Identifier, Location, Module, Region, RegionLike, Type,
        attribute::{StringAttribute, TypeAttribute},
        operation::OperationLike,
        r#type::{FunctionType, IntegerType},
    },
    pass::{self, PassManager},
};

#[test]
fn jit_add_i64() {
    let context = frk_core::context();
    let location = Location::unknown(&context);
    let mut module = Module::new(location);

    let i64_type: Type = IntegerType::new(&context, 64).into();

    let function = {
        let block = Block::new(&[(i64_type, location), (i64_type, location)]);
        let sum = block.append_operation(arith::addi(
            block.argument(0).unwrap().into(),
            block.argument(1).unwrap().into(),
            location,
        ));
        block.append_operation(func::r#return(&[sum.result(0).unwrap().into()], location));

        let region = Region::new();
        region.append_block(block);

        func::func(
            &context,
            StringAttribute::new(&context, "add"),
            TypeAttribute::new(
                FunctionType::new(&context, &[i64_type, i64_type], &[i64_type]).into(),
            ),
            region,
            // invoke_packed calls through the C-interface wrapper; without
            // this attribute the JIT has no `_mlir_ciface_add` to find.
            &[(
                Identifier::new(&context, "llvm.emit_c_interface"),
                Attribute::unit(&context),
            )],
            location,
        )
    };
    module.body().append_operation(function);
    assert!(module.as_operation().verify(), "IR failed MLIR verification");

    let pass_manager = PassManager::new(&context);
    pass_manager.add_pass(pass::conversion::create_to_llvm());
    pass_manager
        .run(&mut module)
        .expect("lowering to the llvm dialect failed");

    let engine = ExecutionEngine::new(&module, 2, &[], false, false);

    let mut lhs: i64 = 40;
    let mut rhs: i64 = 2;
    let mut result: i64 = i64::MIN;

    unsafe {
        engine
            .invoke_packed(
                "add",
                &mut [
                    &mut lhs as *mut i64 as *mut (),
                    &mut rhs as *mut i64 as *mut (),
                    &mut result as *mut i64 as *mut (),
                ],
            )
            .expect("JIT invocation failed");
    }

    assert_eq!(result, 42);
}
