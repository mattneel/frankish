//! The TS-1 trust-but-verify witnesses (D-072; law L1), over the REAL
//! corpus artifacts:
//! - every narrow fact in the direct-narrowing cases PROMOTES (the
//!   dominance pass re-derives tsc's flow analysis in full);
//! - the aliased-discriminant case DEMOTES (our pass honestly cannot
//!   see through the alias — the fact stays a runtime check);
//! - a TAMPERED fact — the artifact claims the wrong variant — is
//!   caught by that runtime check, with blame naming the cast site.

use frk_interp::Interp;
use serde_json::Value as Json;
use sha2::{Digest, Sha256};

fn produce(path: &str) -> String {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = std::process::Command::new("node")
        .arg(root.join("tools/loanword-ts/src/main.ts"))
        .arg(root.join(path))
        .output()
        .expect("node producer");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

/// (promoted, surviving) after compiling the artifact and running the
/// D-072 promotion pass.
fn promote_counts(artifact: &str) -> (usize, usize) {
    let context = frk_core::context();
    frk_dialects::register(&context).unwrap();
    let module = frk_front::loanword::compile_loanword(&context, artifact).unwrap();
    frk_dialects::verify(&context, &module).unwrap();
    use melior::ir::operation::OperationLike;
    frk_dialects::contract::promote_narrows(module.as_operation()).unwrap()
}

#[test]
fn direct_narrowing_promotes_every_fact() {
    let (promoted, surviving) = promote_counts(&produce("goldens/ts1/unions_basic/case.ts"));
    assert!(promoted >= 4, "unions_basic exports at least 4 facts, saw {promoted}");
    assert_eq!(surviving, 0, "every direct fact must promote");

    let (promoted, surviving) = promote_counts(&produce("goldens/ts1/unions_three/case.ts"));
    assert!(promoted >= 5, "unions_three exports at least 5 facts, saw {promoted}");
    assert_eq!(surviving, 0, "chain + !== facts must promote");
}

#[test]
fn aliased_discriminant_demotes_to_a_runtime_check() {
    let (promoted, surviving) = promote_counts(&produce("goldens/ts1/alias_demote/case.ts"));
    assert_eq!(
        (promoted, surviving),
        (0, 1),
        "the aliased fact is invisible to the dominance pass — it must survive"
    );
}

// ---- the tampered-fact witness ----

/// Canonical JSON (recursively sorted keys, no whitespace) — must
/// match the producer/consumer byte for byte.
fn canonical(value: &Json, out: &mut String) {
    match value {
        Json::Object(map) => {
            out.push('{');
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            for (index, key) in keys.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                canonical(&Json::String((*key).clone()), out);
                out.push(':');
                canonical(&map[*key], out);
            }
            out.push('}');
        }
        Json::Array(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                canonical(item, out);
            }
            out.push(']');
        }
        other => out.push_str(&other.to_string()),
    }
}

/// Flips every narrow fact's claimed variant in place.
fn flip_narrows(node: &mut Json) -> usize {
    let mut flipped = 0;
    match node {
        Json::Object(map) => {
            if map.get("k").and_then(Json::as_str) == Some("narrow") {
                let v = map["v"].as_u64().unwrap();
                map.insert("v".into(), Json::from(1 - v));
                flipped += 1;
            }
            for (_, value) in map.iter_mut() {
                flipped += flip_narrows(value);
            }
        }
        Json::Array(items) => {
            for item in items {
                flipped += flip_narrows(item);
            }
        }
        _ => {}
    }
    flipped
}

#[test]
fn a_false_fact_is_caught_at_runtime_with_blame() {
    // The fixture's variants share the field shape (x: number in
    // both), so a flipped fact still TYPECHECKS downstream — only the
    // demoted runtime check can object. (alias_demote itself refuses
    // the flip at emission: 'square' has no 'radius' — the consumer's
    // own typing is the first line of defense.)
    let artifact = produce("crates/frk-front/tests/fixtures/false_fact.ts");
    let mut document: Json = serde_json::from_str(&artifact).unwrap();

    // Tamper: the aliased-discriminant fact claims 'a' (v=0) at a
    // site the dominance pass cannot verify. Claim 'b' instead.
    assert_eq!(flip_narrows(&mut document), 1, "false_fact has one fact");

    // Re-seal the content id (the tamper-refusal test covers broken
    // seals; here the seal is valid and the FACT is the lie).
    let map = document.as_object_mut().unwrap();
    map.remove("sha256");
    let mut bytes = String::new();
    canonical(&Json::Object(map.clone()), &mut bytes);
    let sha = format!("{:x}", Sha256::digest(bytes.as_bytes()));
    map.insert("sha256".into(), Json::from(sha));
    let mut tampered = String::new();
    canonical(&document, &mut tampered);

    let context = frk_core::context();
    frk_dialects::register(&context).unwrap();
    let module = frk_front::loanword::compile_loanword(&context, &tampered).unwrap();
    frk_dialects::verify(&context, &module).unwrap();
    let mut interp = Interp::new(&module).unwrap();
    frk_dialects::register_eval(&mut interp);
    let error = interp
        .eval_function("main", &[])
        .expect_err("the false fact must trap at the demoted check");
    let message = format!("{error:?}");
    assert!(message.contains("narrowing refuted"), "{message}");
    assert!(
        message.contains("cast to 'b'") && message.contains("false_fact.ts:7:"),
        "blame must name the claimed kind and the cast site: {message}"
    );
}
