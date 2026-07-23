//! Dependency-direction locks for n0's source-neutral frame ingress.

const MANIFEST: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));

#[test]
fn rframe_edge_is_contract_only_and_backend_free() {
    assert!(
        MANIFEST.contains("rframe = { path = \"../rframe\", default-features = false }"),
        "n0 must consume only rframe's backend-free resolved contract"
    );
}

#[test]
fn n0_has_no_web_frontend_dependency() {
    let dependencies = dependency_lines(MANIFEST).collect::<Vec<_>>();
    for forbidden in ["websem", "htmlcss", "csscascade"] {
        assert!(
            !dependencies.iter().any(|line| {
                line.split_once('=')
                    .is_some_and(|(name, _)| name.trim() == forbidden)
            }),
            "n0 must remain downstream of source-neutral rframe, not depend on Web frontend crate {forbidden}"
        );
    }
}

fn dependency_lines(manifest: &str) -> impl Iterator<Item = &str> {
    let mut dependency_section = false;
    manifest.lines().filter(move |line| {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            dependency_section = trimmed.ends_with("dependencies]")
                || trimmed.ends_with("dev-dependencies]")
                || trimmed.ends_with("build-dependencies]");
            return false;
        }
        dependency_section && !trimmed.is_empty() && !trimmed.starts_with('#')
    })
}
