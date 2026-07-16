//! Materializes the registry's Rust-twin check (M17, D-062): one typed
//! fn-pointer assertion per frk-abi entry, included by lib.rs. A
//! Rust-twin signature that drifts from the registry is a COMPILE
//! error.
fn main() {
    let out = std::env::var("OUT_DIR").expect("OUT_DIR");
    std::fs::write(
        std::path::Path::new(&out).join("abi_assertions.rs"),
        frk_abi::rust_assertions(),
    )
    .expect("writing abi_assertions.rs");
    println!("cargo::rerun-if-changed=build.rs");
}
