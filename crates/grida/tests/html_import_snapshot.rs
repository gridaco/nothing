//! Snapshot gate for the HTML importer.
//!
//! `import/html` is about to be refactored onto the shared htmlcss
//! front-end (the legacy seam program — gridaco/nothing#27, seam issue
//! gridaco/nothing#30). Its only coverage was 31 inline unit tests, so
//! this suite pins the importer's complete observable output over the
//! `fixtures/test-html/L0` corpus *before* any refactor lands:
//!
//! - the full v1 record of every emitted node (`{:?}`, DFS pre-order),
//!   locking every property the importer sets, and
//! - the computed layout (`LayoutEngine`, 600×800 canvas-md viewport)
//!   for every node — the pixel-relevant gate without rasterization.
//!
//! The refactor's bar is **zero golden diffs**. If a representation-
//! equivalent diff is ever unavoidable, the layout section must stay
//! byte-identical and the field-level diff must be ledgered in the
//! commit message that regenerates the goldens.
//!
//! Regenerate with: `GRIDA_UPDATE_GOLDENS=1 cargo test -p grida --test
//! html_import_snapshot`.
//!
//! One `#[test]` on purpose: `from_html_str` drives the process-global
//! Stylo DOM slot and is not thread-safe; a single test in its own
//! integration-test process serializes all fixtures.

use grida::import::html::from_html_str;
use grida::layout::engine::LayoutEngine;
use grida::layout::ComputedLayout;
use grida::node::schema::{NodeId, Scene, Size};
use std::collections::HashMap;
use std::fs;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};

const VIEWPORT: (f32, f32) = (600.0, 800.0); // the `canvas-md` preset

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn goldens_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/goldens/html-import")
}

fn fixtures() -> Vec<PathBuf> {
    let dir = repo_root().join("fixtures/test-html/L0");
    let mut out: Vec<PathBuf> = fs::read_dir(&dir)
        .expect("fixtures/test-html/L0 missing")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("html"))
        .collect();
    out.sort();
    out
}

/// One full import + layout pass, rendered as the snapshot text.
fn dump(html: &str) -> String {
    let result = catch_unwind(AssertUnwindSafe(|| from_html_str(html)));
    let graph = match result {
        Err(_) => return "PANIC\n".to_string(),
        Ok(Err(e)) => return format!("ERROR: {e}\n"),
        Ok(Ok(graph)) => graph,
    };

    // DFS pre-order over the emitted graph.
    fn walk(
        graph: &grida::node::scene_graph::SceneGraph,
        id: &NodeId,
        depth: usize,
        order: &mut Vec<(usize, NodeId)>,
    ) {
        order.push((depth, *id));
        if let Some(children) = graph.get_children(id) {
            for child in children {
                walk(graph, child, depth + 1, order);
            }
        }
    }
    let mut order: Vec<(usize, NodeId)> = Vec::new();
    for root in graph.roots() {
        walk(&graph, root, 0, &mut order);
    }

    let mut out = String::new();
    out.push_str("## scene\n");
    for (depth, id) in &order {
        let node = graph.get_node(id).expect("dfs id must resolve");
        out.push_str(&format!("{}{:?}\n", "  ".repeat(*depth), node));
    }

    let scene = Scene {
        name: "html-import-snapshot".to_string(),
        graph,
        background_color: None,
    };
    let mut engine = LayoutEngine::new();
    let result = engine.compute(
        &scene,
        Size {
            width: VIEWPORT.0,
            height: VIEWPORT.1,
        },
        None,
    );
    let map: HashMap<NodeId, ComputedLayout> = result.iter().map(|(k, v)| (k, *v)).collect();

    out.push_str(&format!(
        "## layout {}x{}\n",
        VIEWPORT.0 as u32, VIEWPORT.1 as u32
    ));
    for (depth, id) in &order {
        match map.get(id) {
            Some(l) => out.push_str(&format!(
                "{}{:?} [{:?} {:?} {:?} {:?}]\n",
                "  ".repeat(*depth),
                id,
                l.x,
                l.y,
                l.width,
                l.height
            )),
            None => out.push_str(&format!("{}{:?} [no layout]\n", "  ".repeat(*depth), id)),
        }
    }
    out
}

#[test]
fn html_import_snapshots() {
    let update = std::env::var("GRIDA_UPDATE_GOLDENS").is_ok();
    let goldens = goldens_dir();
    if update {
        fs::create_dir_all(&goldens).expect("create goldens dir");
    }

    let mut failures: Vec<String> = Vec::new();
    for fixture in fixtures() {
        let name = fixture
            .file_stem()
            .and_then(|s| s.to_str())
            .expect("fixture name")
            .to_string();
        let html = fs::read_to_string(&fixture).expect("read fixture");

        // Determinism self-check: two full passes must agree before a
        // golden is trusted or compared.
        let first = dump(&html);
        let second = dump(&html);
        assert_eq!(
            first, second,
            "nondeterministic import/layout dump for {name}"
        );

        let golden_path = goldens.join(format!("{name}.snap.txt"));
        if update {
            fs::write(&golden_path, &first).expect("write golden");
            continue;
        }
        match fs::read_to_string(&golden_path) {
            Ok(expected) if expected == first => {}
            Ok(_) => failures.push(format!("{name}: differs from golden")),
            Err(_) => failures.push(format!("{name}: golden missing")),
        }
    }

    if !failures.is_empty() {
        panic!(
            "HTML import snapshots changed:\n  {}\n\n\
             The importer's observable output is pinned (gridaco/nothing#30). \
             If the change is intended, regenerate with \
             GRIDA_UPDATE_GOLDENS=1 and ledger the field-level diff in the \
             regenerating commit's message; the layout section must stay \
             byte-identical for behavior-preserving refactors.",
            failures.join("\n  ")
        );
    }
}
