//! Dependency-direction lock for the resolved render contract.
//!
//! Since the vector join retired the temporary proving drawlist and
//! painter, the whole crate is the shared boundary: every module must stay
//! backend-free and producer-free, and the manifest must not reintroduce a
//! backend, a feature gate, or a serialization surface. This is the
//! executable form of the Web-First Amendment's shared-boundary discipline.
//! The pattern mirrors `crates/grida/tests/*_architecture.rs`.

use std::fs;
use std::path::Path;

/// Import-level substrings that must not appear anywhere in the crate source
/// (chosen so ordinary prose in doc comments cannot trip the gate).
const FORBIDDEN: &[&str] = &[
    "skia_safe",  // no backend objects in the shared boundary
    "csscascade", // no Web front-end coupling
    "stylo",      // no cascade-engine coupling
    "n0_model",   // no producer coupling (the contract is source-neutral)
    "Serialize",
    "Deserialize",
    "serde", // no serialization / round-trip promise
];

#[test]
fn the_whole_crate_is_backend_and_producer_free() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut checked = 0;
    walk(&src, &mut checked);
    assert!(checked >= 2, "expected to check lib.rs and frame.rs");
}

fn walk(dir: &Path, checked: &mut usize) {
    for entry in fs::read_dir(dir).expect("read dir") {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            walk(&path, checked);
            continue;
        }
        if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
            continue;
        }
        let name = path.file_name().unwrap().to_str().unwrap().to_owned();
        let content = fs::read_to_string(&path).expect("read source");
        for needle in FORBIDDEN {
            assert!(
                !content.contains(needle),
                "{name} references {needle:?}; the resolved contract must stay source-neutral \
                 and backend-free (see docs/wg/consolidation/web-first.md)"
            );
        }
        *checked += 1;
    }
}

#[test]
fn manifest_reintroduces_no_backend_or_feature_gate() {
    let manifest = include_str!("../Cargo.toml");
    for needle in ["skia", "[features]", "serde"] {
        assert!(
            !manifest.contains(needle),
            "rframe's manifest contains {needle:?}; the contract crate stays backend-free, \
             feature-free, and serialization-free until the owner decides otherwise"
        );
    }
}
