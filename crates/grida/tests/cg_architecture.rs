//! Architectural tests for the `cg` crate.
//!
//! `cg` is the model-agnostic leaf-type vocabulary — paints, colors,
//! strokes, text styles, and the SVG import IR value types. It is
//! extracted under the legacy seam program (gridaco/nothing#28): the
//! closed import set enforced here is exactly what the crate's
//! `Cargo.toml` allows —
//! `std`/`core`, `serde`, `math2`, and intra-`cg` items.
//!
//! In particular `cg` must not know skia exists: conversions into
//! `skia_safe` types belong at the consumer boundary (`shape/`,
//! `painter/`), never on the vocabulary types themselves.
//!
//! When this test fails, do not loosen the rule. Either move the
//! offending code to the right module, or restructure the import so
//! the type doesn't cross the boundary.

use std::fs;
use std::path::{Path, PathBuf};

const CG_ROOT_REL: &str = "../cg/src";

/// Modules of the `grida` crate (and external deps) that the cg
/// vocabulary must never reach into. Extraction turns each of these
/// into a compile error; this test makes them an error today.
const FORBIDDEN: &[&str] = &[
    "skia_safe",
    "crate::node",
    "node::schema",
    "crate::painter",
    "crate::runtime",
    "crate::cache",
    "crate::layout",
    "crate::htmlcss",
    "crate::import",
    "crate::io",
    "crate::window",
];

/// Known follow-ups: file-relative paths (under `crates/cg/src/`) where a
/// boundary violation is acknowledged but not yet fixed. Each entry
/// should reference an issue/comment explaining why it's deferred.
/// Keep this list **shrinking, not growing**.
const ALLOWLIST: &[(&str, &str)] = &[];

fn cg_root() -> PathBuf {
    let manifest = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest).join(CG_ROOT_REL)
}

fn rs_files_under(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return out;
    };
    for ent in entries.flatten() {
        let p = ent.path();
        if p.is_dir() {
            out.extend(rs_files_under(&p));
        } else if p.extension().and_then(|s| s.to_str()) == Some("rs") {
            out.push(p);
        }
    }
    out
}

fn is_allowlisted(file_rel: &str, forbidden: &str) -> bool {
    ALLOWLIST
        .iter()
        .any(|&(f, n)| file_rel.ends_with(f) && n == forbidden)
}

#[test]
fn cg_imports_are_closed() {
    let files = rs_files_under(&cg_root());
    assert!(
        !files.is_empty(),
        "no .rs files found under {} — did the module move?",
        CG_ROOT_REL
    );
    let mut violations: Vec<String> = Vec::new();
    for file in files {
        let content = fs::read_to_string(&file).unwrap_or_default();
        let rel = file
            .strip_prefix(cg_root())
            .unwrap_or(&file)
            .to_string_lossy()
            .to_string();
        for f in FORBIDDEN {
            if content.contains(f) && !is_allowlisted(&rel, f) {
                violations.push(format!("{}: references `{}`", rel, f));
            }
        }
    }
    if !violations.is_empty() {
        panic!(
            "architectural rule violated for `crates/cg/src/`:\n  {}\n\n\
             cg is the model-agnostic leaf vocabulary; its import set \
             is closed (std, serde, math2, intra-cg). See \
             tests/cg_architecture.rs and gridaco/nothing#28.\n\
             To allow a known temporary violation, add it to ALLOWLIST \
             with a comment explaining why and when it will be removed.",
            violations.join("\n  ")
        );
    }
}
