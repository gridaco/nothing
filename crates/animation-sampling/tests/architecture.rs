use std::collections::BTreeSet;

#[test]
fn dependency_perimeter_is_exact_math_only() {
    let manifest = include_str!("../Cargo.toml");
    let mut dependencies = BTreeSet::new();
    let mut dependency_section = false;
    for line in manifest.lines() {
        let line = line.split('#').next().unwrap().trim();
        if line.starts_with('[') {
            dependency_section = line
                .strip_suffix(']')
                .is_some_and(|section| section.ends_with("dependencies"));
            continue;
        }
        if dependency_section && !line.is_empty() {
            dependencies.insert(line.split('=').next().unwrap().trim());
        }
    }
    assert_eq!(
        dependencies,
        BTreeSet::from(["num-bigint", "num-rational", "num-traits"])
    );
}

#[test]
fn source_has_no_producer_backend_clock_or_io_dependency() {
    let source = include_str!("../src/lib.rs");
    for forbidden in [
        "n0_model",
        "websem",
        "rframe",
        "csscascade",
        "std::fs",
        "std::io",
        "std::net",
        "std::time",
        "tokio",
        "skia",
    ] {
        assert!(
            !source.contains(forbidden),
            "source-neutral sampling kernel contains forbidden dependency `{forbidden}`"
        );
    }
}
