//! Architectural tests for the HTML importer seam.
//!
//! The importer (`import/html`) consumes the shared htmlcss front-end
//! and the per-element `StyledElement` record; it must not read Stylo
//! styles directly. The DOM walk (`csscascade::adapter` / `dom`) stays
//! on the importer side by design — only *style* resolution is shared.
//! (The HTML importer seam, gridaco/nothing#30, under the legacy seam
//! program gridaco/nothing#27.)
//!
//! When this test fails, do not loosen the rule. Extend the
//! `StyledElement` record (renderer-neutrally) and map the new field in
//! `import/html/from_styled.rs` instead of reaching back into Stylo.

use std::fs;
use std::path::{Path, PathBuf};

/// Direct Stylo style reads — owned by `htmlcss/frontend.rs` +
/// `htmlcss/collect.rs`, never by the importer.
const FORBIDDEN: &[&str] = &[
    "use style::",
    "ComputedValues",
    "CascadeDriver",
    "csscascade::cascade",
];

/// Known follow-ups; keep this list **shrinking, not growing**.
const ALLOWLIST: &[(&str, &str)] = &[];

fn import_html_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/import/html")
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
fn importer_does_not_read_stylo_styles() {
    let root = import_html_root();
    let files = rs_files_under(&root);
    assert!(!files.is_empty(), "no .rs files under src/import/html?");
    let mut violations: Vec<String> = Vec::new();
    for file in files {
        let content = fs::read_to_string(&file).unwrap_or_default();
        let rel = file
            .strip_prefix(&root)
            .unwrap_or(&file)
            .to_string_lossy()
            .to_string();
        for f in FORBIDDEN {
            if content.contains(f) && !is_allowlisted(&rel, f) {
                violations.push(format!("{rel}: references `{f}`"));
            }
        }
    }
    if !violations.is_empty() {
        panic!(
            "HTML importer seam violated:\n  {}\n\n\
             Style resolution is owned by the shared front-end \
             (htmlcss/frontend.rs) and the StyledElement record; the \
             importer maps the record in import/html/from_styled.rs. \
             See tests/html_import_architecture.rs and \
             gridaco/nothing#30.",
            violations.join("\n  ")
        );
    }
}
