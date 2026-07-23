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

#[test]
fn document_selection_is_owned_and_never_process_global() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let protected_paths = [
        "src/lib.rs",
        "src/adapter.rs",
        "src/cascade.rs",
        "src/dom.rs",
        "README.md",
        "ARCHITECTURE.md",
    ];
    let corpus = protected_paths
        .iter()
        .map(|path| {
            std::fs::read_to_string(root.join(path))
                .unwrap_or_else(|error| panic!("read {path}: {error}"))
        })
        .collect::<Vec<_>>()
        .join("\n");

    for forbidden in [
        "static DEMO_DOM",
        "AtomicPtr<DemoDom>",
        "bootstrap_dom",
        "pub fn dom() -> &'static DemoDom",
    ] {
        assert!(
            !corpus.contains(forbidden),
            "document selection must not use the retired ambient mechanism {forbidden:?}"
        );
    }

    assert!(
        corpus.contains("pub struct DocumentSession"),
        "document ownership must remain explicit"
    );
    assert!(
        corpus.contains("node: &'session SessionNode"),
        "Stylo handles must remain lifetime-bound to their owning session"
    );
    assert!(
        corpus.contains("inner: Box<SessionInner>")
            && corpus.contains("handles: Box<[SessionNode]>"),
        "session identity records must stay in stable owned allocations"
    );

    for forbidden in [
        "unsafe impl Send for DemoDom",
        "unsafe impl Sync for DemoDom",
    ] {
        assert!(
            !corpus.contains(forbidden),
            "the frozen DOM must not claim thread safety around Stylo's interior data: {forbidden}"
        );
    }
}
