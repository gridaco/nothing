//! Dependency-direction lock for the thin `n0` command host.
//!
//! The host may perform file I/O and use a backend. It must not regain source
//! semantics from the legacy engine or route through the Web proving shell.

use std::fs;
use std::path::Path;

const FORBIDDEN_SOURCE_PATHS: &[&str] = &[
    "grida::",
    "grida_generated",
    "import::svg",
    "node::schema",
    "SceneGraph",
    "websem::",
];

#[test]
fn host_touches_no_legacy_or_proving_shell_path() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut checked = 0;
    walk(&src, &mut checked);
    assert!(checked >= 1, "expected to check at least one source file");
}

#[test]
fn manifest_uses_the_extracted_web_renderer_directly() {
    let manifest = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));
    assert!(
        manifest.contains("publish = false"),
        "the CLI is not published as a crate"
    );
    assert!(
        manifest.contains("[[bin]]\nname = \"n0\""),
        "the package must build the product command named n0"
    );
    assert!(
        manifest.contains("htmlcss = { path = \"../htmlcss\" }"),
        "the current Web source path must enter through htmlcss"
    );
    for forbidden in ["path = \"../grida\"", "path = \"../websem\""] {
        assert!(
            !manifest.contains(forbidden),
            "the product host must not depend on the legacy engine or the proving shell: {forbidden}"
        );
    }
}

#[test]
fn producers_and_cores_do_not_depend_back_on_the_host() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    for relative in [
        "htmlcss/Cargo.toml",
        "websem/Cargo.toml",
        "rframe/Cargo.toml",
        "n0/Cargo.toml",
    ] {
        let manifest_path = root.join(relative);
        let manifest = fs::read_to_string(&manifest_path)
            .unwrap_or_else(|error| panic!("read {}: {error}", manifest_path.display()));
        let has_host_dependency = manifest
            .lines()
            .map(str::trim)
            .any(|line| line.starts_with("n0_cli =") || line.contains("path = \"../n0_cli\""));
        assert!(
            !has_host_dependency,
            "{relative} depends back on the product host"
        );
    }
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
        let name = path.file_name().unwrap().to_str().unwrap();
        let content = fs::read_to_string(&path).expect("read source");
        for needle in FORBIDDEN_SOURCE_PATHS {
            assert!(
                !content.contains(needle),
                "{name} references {needle:?}; n0_cli must stay a thin host"
            );
        }
        *checked += 1;
    }
}
