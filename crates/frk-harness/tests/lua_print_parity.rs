//! The cross-twin %.14g parity rig (D-055.2): the C twin printing
//! natively via zigcc against the Rust emulation, on bit-identical
//! values — the tie pair included, so half-even rounding parity is
//! PROVEN, not assumed. (The lua5.1 oracle joins as the third
//! printer through the corpus goldens.)

use std::process::Command;

#[test]
fn c_and_rust_print_lua_numbers_byte_identically() {
    let values: Vec<f64> = vec![
        42.0,
        -0.0,
        0.1,
        1.0 / 3.0,
        0.0001,
        2.5,
        100.125,
        0.007,
        -1.5,
        // The deliberate tie pair (15th significant digit exactly 5,
        // binary-exact): half-even at digit 14.
        12345678901234.5,
        12345678901233.5,
        9007199254740.5,
    ];

    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dir = root.join("target/lua-print-parity");
    std::fs::create_dir_all(&dir).unwrap();

    // Bit-identical value transport: u64 patterns memcpy'd in C.
    let mut driver = String::from(
        "#include <stdio.h>\n#include <string.h>\n#include <stdint.h>\n\
         int main(void) {\n    uint64_t bits[] = {",
    );
    for value in &values {
        driver.push_str(&format!("{}ULL,", value.to_bits()));
    }
    driver.push_str(
        "};\n    for (unsigned i = 0; i < sizeof bits / sizeof *bits; i++) {\n\
         double v; memcpy(&v, &bits[i], 8);\n        printf(\"%.14g\\n\", v);\n    }\n\
         return 0;\n}\n",
    );
    std::fs::write(dir.join("driver.c"), driver).unwrap();

    let exe = dir.join("driver");
    let status = Command::new("sh")
        .arg(root.join("scripts/zigcc.sh"))
        .args(["-O1", "-o"])
        .arg(&exe)
        .arg(dir.join("driver.c"))
        .current_dir(&root)
        .status()
        .expect("zigcc");
    assert!(status.success());

    let output = Command::new(&exe).output().expect("driver run");
    assert!(output.status.success());
    let c_lines: Vec<String> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(str::to_string)
        .collect();

    let rust_lines: Vec<String> = values.iter().map(|v| frk_rt::format_lua_num(*v)).collect();
    assert_eq!(
        c_lines, rust_lines,
        "the two twins must print %.14g byte-identically (D-055.2)"
    );
}
