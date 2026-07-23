//! Architecture lock: csscascade has one DOM/cascade implementation.
//!
//! The production `dom` + `adapter` + `cascade` path replaced an older RcDom
//! wrapper, a stub styled tree, and an example-local copy of the Stylo
//! adapter. Keeping both paths made ownership ambiguous and let dead APIs look
//! production-ready.

use std::path::Path;

const RETIRED_PATHS: &[&str] = &[
    "src/main.rs",
    "src/rcdom/mod.rs",
    "src/tree/mod.rs",
    "examples/exp_impl_telement.rs",
    "examples/html2html.rs",
    "examples/print_rcdom.rs",
    "examples/print_tree.rs",
];

#[test]
fn csscascade_exports_only_the_live_implementation() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lib = include_str!("../src/lib.rs");

    for module in ["adapter", "cascade", "dom"] {
        assert!(
            lib.contains(&format!("pub mod {module};")),
            "the live {module} module must stay exported"
        );
    }
    for module in ["rcdom", "tree"] {
        assert!(
            !lib.contains(&format!("pub mod {module};")),
            "retired {module} implementation must not return"
        );
    }

    for relative in RETIRED_PATHS {
        assert!(
            !root.join(relative).exists(),
            "retired parallel implementation returned at {relative}"
        );
    }
}

#[test]
fn csscascade_has_no_implicit_product_binary() {
    let manifest = include_str!("../Cargo.toml");
    assert!(
        manifest.contains("autobins = false"),
        "csscascade is a library boundary; keep Cargo's implicit binary discovery disabled"
    );

    let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples");
    let mut names: Vec<_> = std::fs::read_dir(examples)
        .expect("read examples directory")
        .map(|entry| entry.expect("read example entry").file_name())
        .collect();
    names.sort();
    assert_eq!(
        names,
        [std::ffi::OsString::from("resolve_and_print.rs")],
        "examples must exercise the live implementation, not host a parallel one"
    );
}
