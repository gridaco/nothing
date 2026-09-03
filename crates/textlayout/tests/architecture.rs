//! The crate's identity, enforced: `textlayout` owns shaping, metrics, and
//! the resolved artifact — and refuses font discovery, render contracts,
//! backends, clocks, and every ambient input. A dependency or source line
//! that violates the refusal fails here before it can ship.

use std::fs;
use std::path::Path;

/// The complete permitted dependency perimeter, across every dependency
/// table shape a manifest can declare (plain, dev, build, target-specific).
/// Growing it is a decision, not a convenience — state the refusal the new
/// edge does not violate.
#[test]
fn dependency_perimeter_is_exactly_the_pinned_oracle_tables() {
    let manifest = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml")).unwrap();

    let mut in_dependency_section = false;
    let mut deps = Vec::new();
    for raw_line in manifest.lines() {
        let line = raw_line.trim();
        if line.starts_with('[') {
            // Any section whose header names dependencies counts:
            // [dependencies], [dev-dependencies], [build-dependencies],
            // [target.'cfg(...)'.dependencies], and their dotted-key forms.
            in_dependency_section = line.contains("dependencies");
            continue;
        }
        if !in_dependency_section || line.is_empty() || line.starts_with('#') {
            continue;
        }
        // Formatter-wrapped arrays and inline tables are indented continuations,
        // not additional dependency declarations.
        if raw_line.len() == raw_line.trim_start().len()
            && let Some((name, _)) = line.split_once('=')
        {
            deps.push(name.trim().to_string());
        }
    }
    assert_eq!(
        deps,
        vec!["rustybuzz".to_string(), "regex-syntax".to_string()],
        "textlayout's dependency perimeter changed; every edge must be a decision"
    );

    // A build script is a dependency-shaped hole the manifest scan cannot
    // see; the crate declares none.
    assert!(
        !Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/build.rs")).exists(),
        "textlayout must not gain a build script"
    );
}

/// No ambient font access, no render contract, no backend, no clock, no I/O
/// — enforced as a whole-file scan like the sibling locks, so nothing hides
/// in a comment, a doctest, or a brace import. Prose in `src/` therefore
/// names refusals descriptively, never by token.
#[test]
fn source_refuses_ambient_and_backend_reach() {
    const FORBIDDEN: &[&str] = &[
        // Ambient font discovery — the environment is a manifest of bytes.
        "fontdb",
        "core_text",
        "CoreText",
        "dwrite",
        "fontconfig",
        // Render contracts and backends stay out of the resolver.
        "rframe",
        "websem",
        "n0_model",
        "csscascade",
        "stylo",
        "skia",
        // Ambient inputs: resolution is a pure function of declared inputs.
        "std::fs",
        "std::net",
        "std::env",
        "std::time",
        "std::io",
        "SystemTime",
        "Instant",
        // Brace and glob imports could smuggle any of the above past a
        // path-shaped needle; single-path imports only.
        "use std::{",
        "use std::*",
    ];

    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut stack = vec![src];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().is_some_and(|ext| ext == "rs") {
                let source = fs::read_to_string(&path).unwrap();
                for token in FORBIDDEN {
                    assert!(
                        !source.contains(token),
                        "{} reaches for {token}, which textlayout refuses",
                        path.display()
                    );
                }
            }
        }
    }
}
