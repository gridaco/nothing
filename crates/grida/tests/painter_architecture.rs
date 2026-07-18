//! Architectural tests for the painter's model seam.
//!
//! Enforces the painter-narrowing boundary (gridaco/nothing#31, part of
//! the legacy seam program gridaco/nothing#27):
//!
//! - **One compiler module owns the model.** Under `src/painter/`, only
//!   `compile.rs` (the model→display-list compiler) and `debug.rs` (the
//!   debug overlay — a distinct model-reading consumer, see its module
//!   docs) may reference the v1 node model: `node::schema`,
//!   `node::scene_graph` / `SceneGraph`, or `crate::node` in general.
//! - **`NodeId` is the sole model type in the display-list payload.**
//!   `crate::node::id` is the one blessed import everywhere in
//!   `painter/` — the draw loop consumes `PainterPictureLayer`s whose
//!   payload is `cg`-typed values, Skia geometry, and `NodeId`.
//! - **The draw loop never reads the model.** `painter.rs` must stay
//!   free of `Node::` matches entirely.
//!
//! When this test fails, do not loosen the rule. Move the offending code
//! into `painter/compile.rs` (if it compiles the model into the display
//! list) or out of `painter/` (if it is a new model consumer).
//!
//! Companion docs:
//! - `src/painter/compile.rs` — the seam's module docs.
//! - `src/painter/debug.rs` — why the overlay is permitted.

use std::fs;
use std::path::{Path, PathBuf};

const PAINTER_ROOT_REL: &str = "src/painter";

/// v1-model references forbidden outside the exempt modules.
/// `crate::node` intentionally also covers `node::schema` and
/// `node::scene_graph` when written with the `crate::` prefix; the
/// shorter forms catch `use super::…`-style and doc-comment leakage.
const MODEL_REFS: &[&str] = &[
    "node::schema",
    "node::scene_graph",
    "SceneGraph",
    "crate::node",
];

/// The one blessed model path: `NodeId` (and its id-module siblings).
/// Stripped from each line before scanning for `MODEL_REFS`.
const BLESSED: &str = "crate::node::id";

/// Modules under `src/painter/` permitted to read the model. Keep this
/// list **shrinking, not growing** — a third entry means a new model
/// consumer slipped into the painter.
const EXEMPT: &[&str] = &[
    // The model→display-list compiler — the seam itself
    // (gridaco/nothing#31). The only production-path model reader.
    "compile.rs",
    // The debug overlay — an overlay consumer wearing the painter's
    // name. It reads the model by design (draws single nodes without
    // the display-list pipeline) and is not part of the render
    // pipeline. See its module docs.
    "debug.rs",
];

fn painter_root() -> PathBuf {
    let manifest = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest).join(PAINTER_ROOT_REL)
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

fn rel(file: &Path) -> String {
    file.strip_prefix(painter_root())
        .unwrap_or(file)
        .to_string_lossy()
        .to_string()
}

fn is_exempt(file_rel: &str) -> bool {
    EXEMPT.iter().any(|&e| file_rel == e)
}

#[test]
fn model_access_is_confined_to_the_compiler_and_the_debug_overlay() {
    let files = rs_files_under(&painter_root());
    assert!(!files.is_empty(), "no files under {}", PAINTER_ROOT_REL);
    let mut violations: Vec<String> = Vec::new();
    for file in &files {
        let file_rel = rel(file);
        if is_exempt(&file_rel) {
            continue;
        }
        let content = fs::read_to_string(file).unwrap_or_default();
        for (lineno, line) in content.lines().enumerate() {
            // NodeId is the display list's sole model type — strip the
            // blessed path before scanning.
            let scanned = line.replace(BLESSED, "");
            for m in MODEL_REFS {
                if scanned.contains(m) {
                    violations.push(format!(
                        "{}:{}: references `{}` (model access outside the compiler)",
                        file_rel,
                        lineno + 1,
                        m,
                    ));
                }
            }
        }
    }
    if !violations.is_empty() {
        panic!(
            "painter model seam violated:\n  {}\n\n\
             Only `painter/compile.rs` (and the `painter/debug.rs` overlay) \
             may read the v1 node model — see gridaco/nothing#31 and \
             src/painter/compile.rs. Move the model-reading code into the \
             compiler; do not extend EXEMPT.",
            violations.join("\n  ")
        );
    }
}

#[test]
fn draw_loop_is_model_free() {
    // `painter.rs` — the draw loop — consumes the compiled display list
    // and must never match on the `Node` enum. Word-boundary check so
    // e.g. `LeafNode::` (still a violation, but a different string)
    // does not mask a plain `Node::` and identifiers like
    // `PainterPictureLayer::` never false-positive.
    let file = painter_root().join("painter.rs");
    let content = fs::read_to_string(&file).expect("painter.rs must exist");
    let mut violations: Vec<String> = Vec::new();
    for (lineno, line) in content.lines().enumerate() {
        let bytes = line.as_bytes();
        for (idx, _) in line.match_indices("Node::") {
            let boundary = idx == 0 || {
                let c = bytes[idx - 1] as char;
                !(c.is_ascii_alphanumeric() || c == '_')
            };
            if boundary {
                violations.push(format!("painter.rs:{}: `Node::` match", lineno + 1));
            }
        }
    }
    if !violations.is_empty() {
        panic!(
            "the draw loop reads the model:\n  {}\n\n\
             painter.rs consumes PainterPictureLayer (cg payload + NodeId) \
             only — model→display-list compilation belongs in \
             painter/compile.rs (gridaco/nothing#31).",
            violations.join("\n  ")
        );
    }
}
