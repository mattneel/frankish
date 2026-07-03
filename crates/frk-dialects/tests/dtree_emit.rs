//! Verifiers for the promoted dispatch emitter (M6; law L1). A module
//! is built by hand: packed Option construction, a compiled decision
//! tree, arm bodies supplied by the test's callback — then interpreted.
//! No frontend anywhere in sight, which is the point of the promotion.

use frk_dialects::adt_dtree::{Matrix, Pattern, ValueType, compile};
use frk_dialects::dtree_emit::emit_dispatch;
use frk_interp::Interp;
use melior::Context;
use melior::ir::attribute::{
    Attribute, FlatSymbolRefAttribute, IntegerAttribute, StringAttribute, TypeAttribute,
};
use melior::ir::operation::{OperationBuilder, OperationLike};
use melior::ir::r#type::{FunctionType, IntegerType};
use melior::ir::{
    Block, BlockLike, BlockRef, Identifier, Location, Module, Region, RegionLike, Type, Value,
    ValueLike,
};

const OPTION_I64: &str = "!frk_adt.sum<[[], [i64]]>";

fn result_of<'c, 'r>(inserted: melior::ir::OperationRef<'c, 'r>) -> Value<'c, 'r> {
    let raw = inserted.result(0).unwrap().to_raw();
    unsafe { Value::from_raw(raw) }
}

/// Builds `@main() -> i64` that constructs Option variant `variant`
/// (payload 41 when Some), dispatches [Some x -> x + 1 | None -> 0]
/// through the promoted emitter, and returns the result.
fn build_option_match(context: &Context, variant: usize) -> Module<'_> {
    let location = Location::unknown(context);
    let module = Module::new(location);
    let i64_type: Type = IntegerType::new(context, 64).into();
    let option = Type::parse(context, OPTION_I64).unwrap();
    let empty = Type::parse(context, "!frk_adt.product<[]>").unwrap();
    let p_i64 = Type::parse(context, "!frk_adt.product<[i64]>").unwrap();

    let region = Region::new();
    let entry = region.append_block(Block::new(&[]));

    // Construct the scrutinee.
    let new = result_of(entry.append_operation(
        OperationBuilder::new("frk_adt.product_new", location)
            .add_results(&[empty])
            .build()
            .unwrap(),
    ));
    let payload = if variant == 1 {
        let forty_one = result_of(entry.append_operation(melior::dialect::arith::constant(
            context,
            IntegerAttribute::new(i64_type, 41).into(),
            location,
        )));
        result_of(entry.append_operation(
            OperationBuilder::new("frk_adt.product_snoc", location)
                .add_operands(&[new, forty_one])
                .add_results(&[p_i64])
                .build()
                .unwrap(),
        ))
    } else {
        new
    };
    let scrutinee = result_of(entry.append_operation(
        OperationBuilder::new("frk_adt.make_sum", location)
            .add_attributes(&[(
                Identifier::new(context, "variant"),
                IntegerAttribute::new(i64_type, variant as i64).into(),
            )])
            .add_operands(&[payload])
            .add_results(&[option])
            .build()
            .unwrap(),
    ));

    // Compile the match [Some(bind x) -> arm0 | None -> arm1].
    let compiled = compile(Matrix::over_scrutinee(
        ValueType::Sum(vec![vec![], vec![ValueType::Int]]),
        vec![
            Pattern::Variant { tag: 1, fields: vec![Pattern::Binding("x".into())] },
            Pattern::Variant { tag: 0, fields: vec![] },
        ],
    ))
    .unwrap();
    assert!(compiled.diagnostics.inexhaustive.is_none());

    let merge = region.append_block(Block::new(&[(i64_type, location)]));

    emit_dispatch(
        context,
        &region,
        entry,
        &compiled.tree,
        scrutinee,
        option,
        merge,
        &mut |arm_entry: BlockRef, arm: usize, bindings: &[(String, Value)]| {
            let value = match arm {
                0 => {
                    // Some x -> x + 1
                    let x = bindings
                        .iter()
                        .find(|(name, _)| name == "x")
                        .map(|(_, value)| *value)
                        .expect("binding x");
                    let one = result_of(arm_entry.append_operation(
                        melior::dialect::arith::constant(
                            context,
                            IntegerAttribute::new(i64_type, 1).into(),
                            location,
                        ),
                    ));
                    result_of(arm_entry.append_operation(
                        OperationBuilder::new("arith.addi", location)
                            .add_operands(&[x, one])
                            .add_results(&[i64_type])
                            .build()
                            .unwrap(),
                    ))
                }
                _ => result_of(arm_entry.append_operation(
                    melior::dialect::arith::constant(
                        context,
                        IntegerAttribute::new(i64_type, 0).into(),
                        location,
                    ),
                )),
            };
            Ok((value, arm_entry))
        },
    )
    .unwrap();

    // merge: return its block arg.
    let out = {
        let raw = merge.argument(0).unwrap().to_raw();
        unsafe { Value::from_raw(raw) }
    };
    merge.append_operation(
        OperationBuilder::new("func.return", location)
            .add_operands(&[out])
            .build()
            .unwrap(),
    );

    let function = melior::dialect::func::func(
        context,
        StringAttribute::new(context, "main"),
        TypeAttribute::new(FunctionType::new(context, &[], &[i64_type]).into()),
        region,
        &[(
            Identifier::new(context, "llvm.emit_c_interface"),
            Attribute::unit(context),
        )],
        location,
    );
    module.body().append_operation(function);
    module
}

fn interpret(module: &Module) -> i64 {
    let mut interp = Interp::new(module).unwrap();
    frk_dialects::register_eval(&mut interp);
    let values = interp.eval_function("main", &[]).unwrap();
    values[0].as_signed().unwrap()
}

#[test]
fn promoted_dispatch_runs_without_a_frontend() {
    let context = frk_core::context();
    frk_dialects::register(&context).unwrap();

    let some = build_option_match(&context, 1);
    assert!(some.as_operation().verify());
    frk_dialects::verify(&context, &some).unwrap();
    assert_eq!(interpret(&some), 42); // Some 41 -> 41 + 1

    let none = build_option_match(&context, 0);
    assert!(none.as_operation().verify());
    frk_dialects::verify(&context, &none).unwrap();
    assert_eq!(interpret(&none), 0);

    // The frk_verify + FlatSymbolRefAttribute imports stay honest.
    let _ = FlatSymbolRefAttribute::new(&context, "unused");
}
