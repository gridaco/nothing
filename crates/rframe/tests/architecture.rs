//! Dependency-direction lock for the shared kernel.
//!
//! The resolved contract (`frame.rs`) and the private drawlist (`drawlist.rs`)
//! must stay backend-free and producer-free: only `paint.rs` may touch Skia,
//! and the contract must carry no serialization. This is the executable form
//! of the Web-First Amendment's shared-boundary discipline. The pattern
//! mirrors `crates/grida/tests/*_architecture.rs`.

use std::fs;
use std::path::Path;

/// The resolved contract and the private drawlist — the two modules the
/// shared boundary is made of. `paint.rs` (the backend) and `lib.rs` (the
/// crate root, which documents and re-exports the backend) are intentionally
/// out of scope.
const CONTRACT_FILES: &[&str] = &["frame.rs", "drawlist.rs"];

/// Import-level substrings that must not appear in the contract/drawlist
/// source (chosen so ordinary prose in doc comments cannot trip the gate).
const FORBIDDEN_IN_CONTRACT: &[&str] = &[
    "skia_safe",  // no backend objects in the shared boundary
    "csscascade", // no Web front-end coupling
    "stylo",      // no cascade-engine coupling
    "n0_model",   // no producer coupling (the contract is source-neutral)
    "Serialize",
    "Deserialize",
    "serde", // no serialization / round-trip promise
];

#[test]
fn contract_and_drawlist_are_backend_and_producer_free() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    for file in CONTRACT_FILES {
        let path = src.join(file);
        let content = fs::read_to_string(&path).unwrap_or_else(|_| panic!("read {file}"));
        for needle in FORBIDDEN_IN_CONTRACT {
            assert!(
                !content.contains(needle),
                "{file} references {needle:?}; the resolved contract and drawlist must stay \
                 source-neutral and backend-free (see docs/wg/consolidation/web-first.md)"
            );
        }
    }
}

#[test]
fn private_drawlist_is_not_an_external_join_point() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs");
    let content = fs::read_to_string(root).expect("read crate root");
    assert!(
        content.contains("mod drawlist;") && !content.contains("pub mod drawlist;"),
        "the provisional drawlist stays crate-private until the owner chooses a lower join point"
    );
    assert!(
        !content.contains("pub use drawlist"),
        "private drawlist types must not be re-exported"
    );
}
