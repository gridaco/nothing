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

/// Scan every source file for forbidden token substrings, with a
/// provenance-specific message. The same walk as the backend lock above;
/// each refusal below names the decision or law that owns it, so deleting
/// one is visibly re-opening *that* record and no other.
fn assert_source_free_of(needles: &[&str], provenance: &str) {
    fn walk_for(dir: &Path, needles: &[&str], provenance: &str, checked: &mut usize) {
        for entry in fs::read_dir(dir).expect("read dir") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                walk_for(&path, needles, provenance, checked);
                continue;
            }
            if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
                continue;
            }
            let name = path.file_name().unwrap().to_str().unwrap().to_owned();
            let content = fs::read_to_string(&path).expect("read source");
            for needle in needles {
                assert!(
                    !content.contains(needle),
                    "{name} references {needle:?}; {provenance}"
                );
            }
            *checked += 1;
        }
    }
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut checked = 0;
    walk_for(&src, needles, provenance, &mut checked);
    assert!(checked >= 2, "expected to check lib.rs and frame.rs");
}

/// The D-M text-stage decision (taken low, 2026-08-05 — see
/// docs/wg/consolidation/n0-join-point.md#the-text-stage-evidence): the
/// contract gains no shaped-text fact. Web text enters as resolved outline
/// geometry; n0 text stays in its private tier. Deleting this lock is
/// re-opening that decision.
#[test]
fn the_contract_carries_no_shaped_text_fact() {
    assert_source_free_of(
        &["Glyph", "FontKey", "ShapedText", "Typeface"],
        "the D-M text stage joined low, so the resolved contract admits no shaped-text \
         fact (docs/wg/consolidation/n0-join-point.md#the-text-stage-evidence)",
    );
}

/// The contract's standing identity: no fact that references a resource.
/// This refusal predates and stands apart from the D-M text decision — the
/// charter's phase 3 (images through a declared resource environment) may
/// revisit it on its own evidence, and editing this lock then is *that*
/// decision's business, not a re-opening of the text stage.
#[test]
fn the_contract_carries_no_resource_reference() {
    assert_source_free_of(
        &["Resource"],
        "the resolved contract's standing identity refuses facts that reference a \
         resource (docs/wg/consolidation/web-first.md); phase 3 revisits this on its \
         own image evidence, by its own registered decision",
    );
}
