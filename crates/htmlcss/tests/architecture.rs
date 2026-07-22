//! Dependency-perimeter locks for the extracted mature Web renderer.

use std::fs;
use std::path::{Path, PathBuf};

fn rust_files_under(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir).expect("read source directory") {
        let path = entry.expect("read source entry").path();
        if path.is_dir() {
            files.extend(rust_files_under(&path));
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            files.push(path);
        }
    }
    files
}

#[test]
fn styled_dom_seam_keeps_implementation_modules_private() {
    let source_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let library = fs::read_to_string(source_root.join("lib.rs")).expect("read crate root");
    assert!(library.contains("mod collect;"));
    assert!(library.contains("mod frontend;"));
    assert!(!library.contains("pub mod collect;"));
    assert!(!library.contains("pub mod frontend;"));

    let seam = fs::read_to_string(source_root.join("styled_dom.rs"))
        .expect("read styled-DOM compatibility seam");
    let exports: Vec<&str> = seam
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("pub "))
        .collect();
    assert_eq!(
        exports,
        [
            "pub use crate::collect::styled_of;",
            "pub use crate::frontend::parse_and_style;",
        ]
    );
}

#[test]
fn production_markdown_css_is_owned_by_the_crate() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source = fs::read_to_string(crate_root.join("src/github_markdown.rs"))
        .expect("read Markdown stylesheet module");
    assert!(!source.contains("fixtures/"));
    assert!(crate_root.join("assets/css/grida-markdown.css").is_file());
}

#[test]
fn manifest_does_not_depend_on_engine_models_or_resolved_contract() {
    let manifest = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
        .expect("read htmlcss manifest");

    for forbidden in [
        "grida =",
        "package = \"grida\"",
        "../grida",
        "n0 =",
        "package = \"n0\"",
        "../n0",
        "n0-model =",
        "package = \"n0-model\"",
        "rframe =",
        "package = \"rframe\"",
        "../rframe",
    ] {
        assert!(
            !manifest.contains(forbidden),
            "htmlcss must not depend on `{forbidden}`"
        );
    }
}

#[test]
fn source_does_not_reach_into_legacy_or_chassis_models() {
    let source_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let forbidden = [
        "grida::",
        "crate::node",
        "SceneGraph",
        "import::svg",
        "grida_generated",
        "io_grida",
        "grida.fbs",
        "n0::",
        "n0_model::",
        "rframe::",
    ];
    let mut violations = Vec::new();

    for path in rust_files_under(&source_root) {
        let source = fs::read_to_string(&path).expect("read Rust source");
        for needle in forbidden {
            if source.contains(needle) {
                let relative = path.strip_prefix(&source_root).unwrap_or(&path);
                violations.push(format!("{}: `{needle}`", relative.display()));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "htmlcss crossed its dependency perimeter:\n{}",
        violations.join("\n")
    );
}
