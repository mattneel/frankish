//! Materializes the capture-shim signature check (D-062, lens-1
//! finding): the JIT registers shims by type-erased pointer, so the
//! registry's types are asserted here at compile time instead.
fn main() {
    let out = std::env::var("OUT_DIR").expect("OUT_DIR");
    std::fs::write(
        std::path::Path::new(&out).join("capture_assertions.rs"),
        frk_abi::capture_shim_assertions(),
    )
    .expect("writing capture_assertions.rs");
    println!("cargo::rerun-if-changed=build.rs");
}
