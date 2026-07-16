//! The pack-reclamation witness (M22, D-067; law L1): packs are
//! callee-owned — arg packs die by dispose, received result packs die
//! by the extended die_at (with the derived-borrow locality gate) —
//! so a call-heavy rc program reclaims O(calls) allocations instead
//! of leaking 2 per call (the D-064 evidence).
//!
//! ONE test in its own file: the twin counters are process-global,
//! and a solo integration binary keeps the deltas clean.

#[test]
fn packs_reclaim_under_rc() {
    let source = r#"
local function f(a, b) return a + b end
local total = 0
for i = 1, 1000 do total = f(total, i) end
print(total)
"#;
    let allocs0 = frk_rt::frk_rt_alloc_count();
    let frees0 = frk_rt::frk_rt_rc_free_count();
    let context = frk_core::context();
    frk_dialects::register(&context).unwrap();
    let module = frk_front::lua::compile_lua(&context, "m.lua", source).unwrap();
    let mut module = module;
    frk_harness::pipeline::lower_to_llvm(&context, &mut module, frk_dialects::Strategy::Rc)
        .unwrap();
    let engine = melior::ExecutionEngine::new(&module, 2, &[], false, false);
    unsafe {
        for entry in frk_abi::RT_ABI {
            if entry.jit == frk_abi::JitBinding::NotLinked {
                continue;
            }
            engine.register_symbol(
                entry.name,
                frk_harness::runner::jit_symbol_for_test(entry.name).unwrap(),
            );
        }
        engine.invoke_packed("main", &mut []).unwrap();
        frk_rt::frk_rt_rc_collect();
    }
    let allocs = frk_rt::frk_rt_alloc_count() - allocs0;
    let frees = frk_rt::frk_rt_rc_free_count() - frees0;
    let leaked = allocs - frees;
    // 1000 calls; before D-067 this leaked ~2026 (2/call). After: only
    // the process-lifetime stdlib seeding survives. Generous bound so
    // stdlib growth doesn't flake the witness; the REGRESSION this
    // guards is any O(calls) term reappearing.
    assert!(
        leaked < 100,
        "pack leak regressed: {allocs} allocs, {frees} frees, {leaked} leaked \
         (O(calls) leak = a D-067 regression)"
    );
}
