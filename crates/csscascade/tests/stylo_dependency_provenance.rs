//! Dependency-provenance lock for the shared Stylo family.
//!
//! The Web cascade must resolve one coherent official-upstream crate family.
//! A mixed registry/Git graph, a floating branch, or a per-consumer source
//! override would make the cascade depend on Cargo's source unification details
//! rather than on one reviewed upstream state.

use std::fs;
use std::path::{Path, PathBuf};

const OFFICIAL_REPOSITORY: &str = "https://github.com/servo/stylo.git";
const STYLO_REV: &str = "a64923b5d5c67313c81c5056f5e30ec0babb04d6";

/// Workspace dependency keys for the directly consumed Stylo crates.
const DIRECT_STYLO_FAMILY: &[&str] = &[
    "selectors",
    "stylo",
    "stylo_atoms",
    "stylo_dom",
    "stylo_static_prefs",
    "style_traits",
];

/// Every package Cargo resolves from the official Stylo repository, including
/// transitive implementation crates whose Rust types must not split by source.
const ALL_UPSTREAM_PACKAGES: &[&str] = &[
    "selectors",
    "servo_arc",
    "stylo",
    "stylo_atoms",
    "stylo_derive",
    "stylo_dom",
    "stylo_malloc_size_of",
    "stylo_static_prefs",
    "stylo_traits",
    "to_shmem",
    "to_shmem_derive",
];

#[test]
fn stylo_family_has_one_immutable_official_upstream_source() {
    assert!(
        STYLO_REV.len() == 40 && STYLO_REV.bytes().all(|byte| byte.is_ascii_hexdigit()),
        "the reviewed Stylo revision must be a full immutable Git object id"
    );

    let root = workspace_root();
    let root_manifest = read(&root.join("Cargo.toml"));
    let workspace_dependencies = section(&root_manifest, "[workspace.dependencies]");

    assert!(
        !workspace_dependencies.contains("branch ="),
        "the shared Stylo family must never float on a Git branch"
    );

    for dependency_key in DIRECT_STYLO_FAMILY {
        let declarations = assignments(workspace_dependencies, dependency_key);
        assert_eq!(
            declarations.len(),
            1,
            "workspace dependency {dependency_key:?} must have exactly one source declaration"
        );
        let declaration = declarations[0];
        assert_eq!(
            quoted_field(declaration, "git"),
            Some(OFFICIAL_REPOSITORY),
            "workspace dependency {dependency_key:?} must come directly from official servo/stylo"
        );
        assert_eq!(
            quoted_field(declaration, "rev"),
            Some(STYLO_REV),
            "workspace dependency {dependency_key:?} must use the reviewed immutable revision"
        );
        assert_eq!(
            quoted_field(declaration, "version"),
            None,
            "workspace dependency {dependency_key:?} must not declare a registry fallback while the reviewed revision is required"
        );
        assert!(
            !declaration.contains("branch =") && !declaration.contains("tag ="),
            "workspace dependency {dependency_key:?} must not dilute the exact revision pin"
        );
    }

    assert_all_consumers_inherit_the_workspace_source(&root);
    assert_git_pinned_cascade_is_not_publishable(&root);
    assert_lockfile_contains_only_the_reviewed_source(&root);
}

fn assert_git_pinned_cascade_is_not_publishable(root: &Path) {
    let manifest_path = root.join("crates/csscascade/Cargo.toml");
    let manifest = read(&manifest_path);
    assert!(
        manifest
            .lines()
            .any(|line| line.trim() == "publish = false"),
        "{} must refuse packaging while its required Stylo semantics exist only at a Git revision",
        manifest_path.display()
    );
}

fn assert_all_consumers_inherit_the_workspace_source(root: &Path) {
    let crates = root.join("crates");
    let mut consumer_edges = 0;

    for entry in fs::read_dir(crates).expect("read workspace crates") {
        let manifest_path = entry
            .expect("workspace crate entry")
            .path()
            .join("Cargo.toml");
        if !manifest_path.is_file() {
            continue;
        }

        let manifest = read(&manifest_path);
        for dependency_key in DIRECT_STYLO_FAMILY {
            for declaration in assignments(&manifest, dependency_key) {
                consumer_edges += 1;
                assert_eq!(
                    declaration,
                    format!("{dependency_key}.workspace = true"),
                    "{} must inherit {dependency_key:?} from [workspace.dependencies]; \
                     per-consumer versions or sources can split the Stylo family",
                    manifest_path.display()
                );
            }
        }
    }

    assert!(
        consumer_edges > 0,
        "expected at least one workspace crate to consume the shared Stylo family"
    );
}

fn assert_lockfile_contains_only_the_reviewed_source(root: &Path) {
    let lockfile = read(&root.join("Cargo.lock"));
    let expected_source = format!("git+{OFFICIAL_REPOSITORY}?rev={STYLO_REV}#{STYLO_REV}");

    for package_name in ALL_UPSTREAM_PACKAGES {
        let matching: Vec<_> = lockfile
            .split("[[package]]")
            .filter(|package| package.contains(&format!("\nname = \"{package_name}\"\n")))
            .collect();
        assert_eq!(
            matching.len(),
            1,
            "Cargo.lock must contain exactly one {package_name:?} package"
        );
        assert!(
            matching[0].contains(&format!("\nsource = \"{expected_source}\"\n")),
            "Cargo.lock package {package_name:?} must resolve from the reviewed official revision, \
             with no registry or alternate-Git copy"
        );
    }
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("csscascade must live under <workspace>/crates")
        .to_path_buf()
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
}

fn section<'a>(manifest: &'a str, heading: &str) -> &'a str {
    let start = manifest
        .find(heading)
        .unwrap_or_else(|| panic!("missing {heading}"));
    let body = &manifest[start + heading.len()..];
    let end = body.find("\n[").unwrap_or(body.len());
    &body[..end]
}

fn assignments<'a>(manifest: &'a str, key: &str) -> Vec<&'a str> {
    let direct = format!("{key} =");
    let inherited = format!("{key}.workspace =");
    manifest
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with(&direct) || line.starts_with(&inherited))
        .collect()
}

fn quoted_field<'a>(declaration: &'a str, field: &str) -> Option<&'a str> {
    let marker = format!("{field} = \"");
    let value = declaration.split_once(&marker)?.1;
    value.split_once('"').map(|(value, _)| value)
}
