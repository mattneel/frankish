//! M35 (D-084) verifiers that are never differential:
//! (1) THE FORCED-TRANSFORM LICENSE GATE — every pre-M35 lua golden,
//!     compiled with the resumable-frame transform forced ON, still
//!     produces its pinned bytes on the reference interpreter
//!     (including tail_recursion at 100k frames: the transform must
//!     never guard a tail site, or the depth cap trips loudly).
//! (2) THE TRAP MATRIX — yield-from-main, the intrinsic/metamethod
//!     boundary, and the iterator-head boundary, all named messages
//!     (chibi... lua5.1 refuses the same shapes catchably; ours abort
//!     deterministically — the D-084.5 fences, unit-only).

use frk_harness::runner::{InterpRunner, Runner};

fn interp_forced(path: &std::path::Path) -> Result<String, String> {
    let source = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let context = frk_core::context();
    frk_dialects::register(&context).map_err(|e| e.to_string())?;
    let module = frk_front::lua::compile_lua_forced(&context, "case.lua", &source)
        .map_err(|e| format!("compile: {}", &e[..e.len().min(300)]))?;
    frk_dialects::verify(&context, &module).map_err(|e| e.to_string())?;
    let mut interp = frk_interp::Interp::new(&module).map_err(|e| e.to_string())?;
    frk_dialects::register_eval(&mut interp);
    frk_harness::runner::register_protocol_builtins(&mut interp);
    interp
        .eval_function("main", &[])
        .map_err(|e| e.to_string())?;
    Ok(frk_harness::canon::canonicalize(&interp.take_output()))
}

#[test]
fn forced_transform_keeps_every_pre_m35_case_byte_identical() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let goldens = root.join("goldens/lua");
    let mut checked = 0;
    for entry in std::fs::read_dir(&goldens).unwrap() {
        let dir = entry.unwrap().path();
        let case = dir.join("case.lua");
        if !case.is_file() {
            continue;
        }
        let source = std::fs::read_to_string(&case).unwrap();
        if source.contains("coroutine") {
            continue; // already licensed — the differential corpus owns it
        }
        let expected = std::fs::read_to_string(dir.join("expected.out")).unwrap();
        let actual = interp_forced(&case)
            .unwrap_or_else(|e| panic!("{}: forced compile/run failed: {e}", dir.display()));
        assert_eq!(
            actual,
            frk_harness::canon::canonicalize(&expected),
            "{}: forced transform changed the bytes",
            dir.display()
        );
        checked += 1;
    }
    assert!(checked >= 18, "expected the whole pre-M35 corpus, got {checked}");
}

fn run_lua_source(name: &str, source: &str) -> Result<String, String> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dir = root.join(format!("target/{name}-fixture"));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("case.lua"), source).unwrap();
    std::fs::write(dir.join("expected.out"), "unreachable\n").unwrap();
    let cases = frk_harness::case::discover(&dir).unwrap();
    InterpRunner.run(&cases[0]).map_err(|e| e.to_string())
}

#[test]
fn yield_from_main_traps() {
    let error = run_lua_source(
        "lua-yield-main",
        "print(1)\ncoroutine.yield(2)\nprint(3)\n",
    )
    .expect_err("yield from the main chunk must trap");
    assert!(error.contains("yield from the main chunk"), "{error}");
    assert!(error.contains("D-084"), "{error}");
}

#[test]
fn yield_across_iterator_head_traps() {
    let error = run_lua_source(
        "lua-yield-iter",
        "local function baditer(s, c)\n  coroutine.yield(\"nope\")\n  return nil\nend\nlocal co = coroutine.create(function()\n  for x in baditer, false, false do\n    print(x)\n  end\nend)\nprint(coroutine.resume(co))\n",
    )
    .expect_err("a yielding iterator at the generic-for head must trap");
    assert!(error.contains("intrinsic boundary"), "{error}");
}

#[test]
fn yield_across_metamethod_traps() {
    let error = run_lua_source(
        "lua-yield-meta",
        "local t = setmetatable({}, { __index = function(tab, k)\n  coroutine.yield(k)\n  return 0\nend })\nlocal co = coroutine.create(function()\n  return t.missing\nend)\nprint(coroutine.resume(co))\n",
    )
    .expect_err("a yielding __index metamethod must trap");
    assert!(error.contains("intrinsic boundary"), "{error}");
}
