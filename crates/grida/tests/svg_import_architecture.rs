//! Architectural tests for the SVG import seam.
//!
//! The SVG import subsystem's product is the `IRSVG` tree
//! (`SVGPackedScene`); the v1 node model is one *consumer* of that IR
//! through the adapter (`import/svg/pack.rs` + `grida.rs`). These tests
//! enforce the seam (the SVG sink inversion, gridaco/nothing#29, under
//! the legacy seam program gridaco/nothing#27):
//!
//! - **The IR layer must not know the node model.** `crates/cg/src/svg.rs`,
//!   `import/svg/packed_scene.rs`, `import/svg/from_usvg.rs`, and
//!   everything under `formats/svg/` may not reference `node::schema`,
//!   the factory, or the scene graph.
//! - **The IR must not carry runtime-paint policy.** The
//!   SVG→runtime-`Paint` projection (baked opacity, UV-space gradient
//!   normalization) lives in `import/svg/paint.rs` on the adapter side;
//!   `crates/cg/src/svg.rs` stays spec-faithful vocabulary.
//! - **Only the adapter touches the model.** Within `import/svg/`, only
//!   `pack.rs` and `grida.rs` may reference `crate::node`.
//!
//! When this test fails, do not loosen the rule. Either move the
//! offending code to the right side of the seam, or restructure the
//! import so the type doesn't cross the boundary.

use std::fs;
use std::path::{Path, PathBuf};

/// Tokens that mean "this file knows the v1 node model".
const MODEL_TOKENS: &[&str] = &[
    "node::schema",
    "crate::node",
    "NodeFactory",
    "SceneGraph",
    "scene_graph",
];

/// Tokens that mean "this file projects into the runtime paint model".
/// Banned in `crates/cg/src/svg.rs` so the IR vocabulary stays spec-faithful.
/// Checked after stripping the legitimate `SVG*`-prefixed IR type names,
/// so `SVGSolidPaint`/`SVGPaint::` don't false-positive.
const PAINT_PROJECTION_TOKENS: &[&str] = &[
    "into_paint",
    "SolidPaint",
    "LinearGradientPaint",
    "RadialGradientPaint",
    "Paint::",
];

/// The IR's own spec-faithful type names, removed from the text before
/// scanning for the runtime-paint tokens above. Longest first.
const IR_TYPE_NAMES: &[&str] = &[
    "SVGLinearGradientPaint",
    "SVGRadialGradientPaint",
    "SVGSolidPaint",
    "SVGPaint",
];

/// Known follow-ups; keep this list **shrinking, not growing**.
const ALLOWLIST: &[(&str, &str)] = &[];

fn crate_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn cg_svg() -> PathBuf {
    crate_root().join("../cg/src/svg.rs")
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

fn check_files(files: &[PathBuf], forbidden: &[&str], rule: &str) {
    let root = crate_root();
    let mut violations: Vec<String> = Vec::new();
    for file in files {
        let content = fs::read_to_string(file).unwrap_or_default();
        let rel = file
            .strip_prefix(&root)
            .unwrap_or(file)
            .to_string_lossy()
            .to_string();
        for f in forbidden {
            if content.contains(f) && !is_allowlisted(&rel, f) {
                violations.push(format!("{rel}: references `{f}`"));
            }
        }
    }
    if !violations.is_empty() {
        panic!(
            "SVG import seam violated ({rule}):\n  {}\n\n\
             See tests/svg_import_architecture.rs and gridaco/nothing#29.\n\
             To allow a known temporary violation, add it to ALLOWLIST \
             with a comment explaining why and when it will be removed.",
            violations.join("\n  ")
        );
    }
}

#[test]
fn ir_layer_does_not_know_the_node_model() {
    let root = crate_root();
    let mut files = vec![
        cg_svg(),
        root.join("src/import/svg/packed_scene.rs"),
        root.join("src/import/svg/from_usvg.rs"),
    ];
    files.extend(rs_files_under(&root.join("src/formats/svg")));
    assert!(files.iter().all(|f| f.exists()), "IR layer files moved?");
    check_files(&files, MODEL_TOKENS, "IR layer must not import the model");
}

#[test]
fn ir_vocabulary_carries_no_paint_projection() {
    let path = cg_svg();
    let mut content = fs::read_to_string(&path).expect("crates/cg/src/svg.rs moved?");
    for name in IR_TYPE_NAMES {
        content = content.replace(name, "");
    }
    let violations: Vec<String> = PAINT_PROJECTION_TOKENS
        .iter()
        .filter(|t| content.contains(**t))
        .map(|t| format!("crates/cg/src/svg.rs: references `{t}`"))
        .collect();
    if !violations.is_empty() {
        panic!(
            "SVG import seam violated (crates/cg/src/svg.rs is spec-faithful vocabulary; \
             the projection lives in import/svg/paint.rs):\n  {}\n\n\
             See tests/svg_import_architecture.rs and gridaco/nothing#29.",
            violations.join("\n  ")
        );
    }
}

#[test]
fn only_the_adapter_touches_the_model() {
    let root = crate_root();
    let svg_dir = root.join("src/import/svg");
    let files: Vec<PathBuf> = rs_files_under(&svg_dir)
        .into_iter()
        .filter(|p| {
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            name != "pack.rs" && name != "grida.rs"
        })
        .collect();
    assert!(!files.is_empty(), "no non-adapter files under import/svg?");
    check_files(
        &files,
        &["crate::node"],
        "only pack.rs and grida.rs may consume the node model",
    );
}
