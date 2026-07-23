//! Dependency-direction lock for the Web semantic front-end.
//!
//! `websem` produces the source-neutral contract; it must not reach the legacy
//! import path, the legacy node model, the `.grida` codec, or any backend, and
//! it must not do I/O. This is the executable form of the Web-First Amendment's
//! forbidden-path list. Pattern mirrors `crates/grida/tests/*_architecture.rs`.

use std::fs;
use std::path::Path;

/// Import-level substrings forbidden anywhere in `websem/src`.
const FORBIDDEN: &[&str] = &[
    // No backend / escape hatch — websem emits the contract, it does not paint.
    "skia_safe",
    "SkPicture",
    // No legacy engine, import adapters, node model, or `.grida` codec.
    "grida::",
    "import::svg",
    "node::schema",
    "io_grida",
    "grida_generated",
    // No I/O or network policy in the front-end.
    "std::fs",
    "std::net",
    "reqwest",
];

#[test]
fn websem_touches_no_forbidden_path() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut checked = 0;
    walk(&src, &mut checked);
    assert!(checked >= 1, "expected to check at least one source file");
}

#[test]
fn websem_normal_edge_keeps_rframe_backend_free() {
    let manifest = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));
    assert!(
        manifest.contains("rframe = { path = \"../rframe\", default-features = false }"),
        "websem's normal rframe edge must disable backend features; pixel hosts opt in only as dev/host targets"
    );
}

fn walk(dir: &Path, checked: &mut usize) {
    for entry in fs::read_dir(dir).expect("read dir") {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            walk(&path, checked);
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let name = path.file_name().unwrap().to_str().unwrap();
        let content = fs::read_to_string(&path).expect("read source");
        for needle in FORBIDDEN {
            assert!(
                !content.contains(needle),
                "{name} references {needle:?}; the Web front-end must not touch legacy import, \
                 the node model, the .grida codec, a backend, or I/O \
                 (see docs/wg/consolidation/web-first.md)"
            );
        }
        *checked += 1;
    }
}
