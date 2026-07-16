//! `make abi`: prints the generated C twin contract to stdout.
//! Usage: cargo run -q -p frk-abi --bin gen-header > crates/frk-rt/c/frk_rt_abi.h
fn main() {
    print!("{}", frk_abi::c_header());
}
