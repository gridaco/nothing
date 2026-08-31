//! The generated capability-status view and its freshness gate.
//!
//! The donor kept a hand-maintained property table and it drifted; this
//! path's tracker is the code and the corpora (per the D-N decision), and
//! this gate derives the *scannable* view from them instead of authoring a
//! second statement. Every refusal line below is the compiler's actual
//! departure, recompiled here — so `fixtures/web-first/STATUS.md` cannot
//! drift from behavior without this test failing, the same freshness
//! discipline the FlatBuffers codegen gate applies to `grida.rs`.
//!
//! Regenerate after a rung moves the corpus:
//!
//! ```sh
//! UPDATE_CAPABILITY_STATUS=1 cargo test -p websem --test capability_status
//! ```

#[path = "support/fixture_fonts.rs"]
mod fixture_fonts;

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use websem::{DegradationAction, InitialViewport, SvgFrameSource};

#[derive(Debug, Deserialize)]
struct PrimitiveSuite {
    fixtures: Vec<Primitive>,
}

#[derive(Debug, Deserialize)]
struct Primitive {
    id: String,
    source: String,
    entry: String,
}

fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/web-first")
}

/// A departure message rendered into one table cell: the register is a
/// GitHub-flavored table, so a pipe, newline, or emphasis marker in the
/// compiler's words must not change the row it reports.
fn cell(text: &str) -> String {
    text.replace('|', "\\|")
        .replace('*', "\\*")
        .replace('\n', " ")
}

#[test]
fn refusal_text_is_literal_inside_the_generated_table() {
    assert_eq!(cell("a | b\nc * d"), r"a \| b c \* d");
}

fn viewport() -> InitialViewport {
    InitialViewport::new(64.0, 64.0)
}

#[test]
fn the_committed_status_view_is_fresh() {
    let generated = generate();
    let path = corpus_root().join("STATUS.md");
    if std::env::var_os("UPDATE_CAPABILITY_STATUS").is_some() {
        fs::write(&path, &generated).expect("write STATUS.md");
        return;
    }
    let committed = fs::read_to_string(&path).unwrap_or_default();
    assert_eq!(
        committed, generated,
        "fixtures/web-first/STATUS.md is stale — regenerate with \
         UPDATE_CAPABILITY_STATUS=1 cargo test -p websem --test capability_status"
    );
}

fn generate() -> String {
    let root = corpus_root();
    let suite: PrimitiveSuite = serde_json::from_str(
        &fs::read_to_string(root.join("primitives.json")).expect("read primitives.json"),
    )
    .expect("parse primitives.json");

    let mut refusals: Vec<String> = fs::read_dir(root.join("unsupported"))
        .expect("read the unsupported corpus")
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".svg"))
        .map(|name| name.trim_end_matches(".svg").to_string())
        .collect();
    refusals.sort();

    let mut out = String::new();
    out.push_str("# Web-first capability status\n\n");
    out.push_str(
        "<!-- GENERATED FILE - do not edit. Regenerate:\n\
         UPDATE_CAPABILITY_STATUS=1 cargo test -p websem --test capability_status -->\n\n",
    );
    out.push_str(
        "A generated, freshness-gated view of the two corpora that track the\n\
         SVG engine of record's capability: what renders (the Chromium-baked\n\
         cells) and what departs by name (the refusal register). Every refusal\n\
         line is the compiler's actual departure, recompiled by the gate\n\
         (`crates/websem/tests/capability_status.rs`), so this file cannot\n\
         drift from behavior. The prose statement of record is\n\
         [crates/n0_cli/README.md](../../crates/n0_cli/README.md); the rung\n\
         history is the\n\
         [D-N register](../../docs/wg/consolidation/svg-engine-of-record.md);\n\
         the per-fixture rationale is\n\
         [unsupported/README.md](./unsupported/README.md).\n\n\
         Not a conformance claim: no score is computed or implied (FLIP is\n\
         unratified), and the corpus enumerates constructs, not the SVG\n\
         surface.\n\n",
    );

    writeln!(out, "## Chromium-baked cells ({})\n", suite.fixtures.len()).unwrap();
    out.push_str(
        "Each renders byte-exact against its committed Chromium oracle\n\
         (seven curved cells and four gradient ramps carry a declared, bounded\n\
         tolerance — see [README.md](./README.md)). Every thumbnail below\n\
         *is* that committed oracle, which byte-exactness makes this\n\
         engine's own render too; hover for the cell's name, click through\n\
         to its fixture source. No new image is committed for this view.\n\n",
    );
    for cell in &suite.fixtures {
        // One line per cell: the lines flow into one wrapping gallery
        // paragraph when rendered, and a rung's diff stays one line per
        // new cell. The alt text keeps every id greppable in the raw file.
        writeln!(
            out,
            "<a href=\"./{source}\" title=\"{id} ({entry})\"><img \
             src=\"./chromium/{id}.png\" width=\"56\" alt=\"{id}\"></a>",
            id = cell.id,
            source = cell.source,
            entry = cell.entry,
        )
        .unwrap();
    }
    out.push('\n');

    writeln!(out, "## The refusal register ({})\n", refusals.len()).unwrap();
    out.push_str(
        "What the slice refuses, by name, in the compiler's own words —\n\
         **both refuse** is a document-level contract; **declared** renders\n\
         the rest and names the hole. A rung that admits a construct moves\n\
         its row into the cells above.\n\n\
         | Fixture | Admission | The compiler's departure |\n\
         | --- | --- | --- |\n",
    );
    for id in &refusals {
        let source = fs::read_to_string(root.join("unsupported").join(format!("{id}.svg")))
            .unwrap_or_else(|error| panic!("{id}: read: {error}"));
        let strict = SvgFrameSource::from_standalone_svg_with_fonts(
            source.as_str(),
            viewport(),
            fixture_fonts::unsupported_environment(id),
        )
        .err()
        .unwrap_or_else(|| panic!("{id}: an unsupported fixture must refuse under strict"));
        match SvgFrameSource::from_standalone_svg_best_effort_with_fonts(
            source.as_str(),
            viewport(),
            fixture_fonts::unsupported_environment(id),
        ) {
            Err(_) => {
                writeln!(
                    out,
                    "| `{id}` | **both refuse** | {} |",
                    cell(&strict.to_string())
                )
                .unwrap();
            }
            Ok(best) => {
                let declared: Vec<String> = best
                    .degradations()
                    .iter()
                    .filter(|d| d.action() != DegradationAction::SamplesAsBase)
                    .map(|d| format!("{d}"))
                    .collect();
                assert!(
                    !declared.is_empty(),
                    "{id}: best-effort rendered it with nothing declared — a silent hole"
                );
                writeln!(
                    out,
                    "| `{id}` | declared | {} |",
                    cell(&declared.join("; "))
                )
                .unwrap();
            }
        }
    }

    out
}
