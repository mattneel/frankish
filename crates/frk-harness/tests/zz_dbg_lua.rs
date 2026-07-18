#[test]
fn dump_case() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let case = std::env::var("DBG_CASE").unwrap_or_else(|_| "co_stackful".into());
    let source = std::fs::read_to_string(
        root.join(format!("goldens/lua/{case}/case.lua")),
    )
    .unwrap();
    let context = frk_core::context();
    frk_dialects::register(&context).expect("register");
    match frk_front::lua::compile_lua(&context, "case.lua", &source) {
        Ok(_) => eprintln!("COMPILED OK"),
        Err(e) => {
            std::fs::write("/tmp/claude-1000/-home-autark-src-frankish/cdff9e51-7ddf-4590-aa6c-df320a0da740/scratchpad/dbg_module.txt", e.to_string()).unwrap();
            eprintln!("written {} chars", e.to_string().len());
        }
    }
}
