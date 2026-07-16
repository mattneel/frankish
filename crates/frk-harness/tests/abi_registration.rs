//! Coverage witnesses for registry-driven registration (D-062,
//! finished at M21; law L1): the frk-abi table drives the JIT symbol
//! set and the interp builtin set — these tests prove every row has
//! its pointer/behavior, and that the tables carry nothing stale.

use melior::ir::Module;

#[test]
fn every_registered_jit_row_has_a_pointer_and_nothing_is_stale() {
    for entry in frk_abi::RT_ABI {
        let bound = frk_harness::runner::jit_symbol_for_test(entry.name).is_some();
        match entry.jit {
            frk_abi::JitBinding::NotLinked => assert!(
                !bound,
                "{} is NotLinked in frk-abi but jit_symbol still binds it (stale row?)",
                entry.name
            ),
            _ => assert!(
                bound,
                "{} is {:?} in frk-abi but jit_symbol has no pointer",
                entry.name, entry.jit
            ),
        }
    }
}

#[test]
fn every_builtin_row_registers_and_nothing_is_stale() {
    let context = frk_core::context();
    frk_dialects::register(&context).expect("registration");
    let module = Module::parse(
        &context,
        "func.func @main() -> i64 { %z = arith.constant 0 : i64 return %z : i64 }",
    )
    .expect("module");
    let mut interp = frk_interp::Interp::new(&module).expect("interp");
    frk_harness::runner::register_protocol_builtins(&mut interp);
    for entry in frk_abi::RT_ABI {
        let registered = interp.has_builtin(entry.name);
        match entry.interp {
            frk_abi::InterpDisposition::Builtin => assert!(
                registered,
                "{} is Builtin in frk-abi but no behavior registered",
                entry.name
            ),
            _ => assert!(
                !registered,
                "{} is not Builtin in frk-abi but a behavior is registered (stale?)",
                entry.name
            ),
        }
    }
}
