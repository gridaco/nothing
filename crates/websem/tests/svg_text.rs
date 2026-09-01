//! The `<text>` slice, against the Chromium oracle and its own refusals.
//!
//! Every admitted cell here renders through the one downstream
//! (`websem → rframe → n0`) and is compared byte-for-byte against the
//! committed Chromium capture, inside the numeric domain the ratified
//! [text-oracle method](../../../docs/wg/consolidation/text-oracle.md)
//! admits. The refusal cases are the other half of the same law: a
//! construct outside the slice names itself rather than painting.

mod support;

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::Deserialize;
use websem::{CompileError, DegradationAction, InitialViewport, SvgFrameSource};

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/web-first/text")
}

#[derive(Deserialize)]
struct Suite {
    schema_version: u32,
    font: SuiteFont,
    cases: Vec<SuiteCase>,
}

#[derive(Deserialize)]
struct SuiteFont {
    family: String,
    path: String,
    sha256: String,
}

#[derive(Deserialize)]
struct SuiteCase {
    id: String,
    source: String,
    oracle: String,
    width: i32,
    height: i32,
}

#[derive(Deserialize)]
struct BakeManifest {
    schema_version: u32,
    suite: String,
    suite_sha256: String,
    bake_script: String,
    bake_script_sha256: String,
    capture_module: String,
    capture_module_sha256: String,
    records: Vec<BakeRecord>,
}

#[derive(Deserialize)]
struct BakeRecord {
    id: String,
    source: String,
    source_sha256: String,
    oracle: String,
    oracle_sha256: String,
    width: i32,
    height: i32,
}

fn suite() -> Suite {
    let bytes = std::fs::read(fixture_root().join("cases.json")).expect("text suite manifest");
    let suite: Suite = serde_json::from_slice(&bytes).expect("well-formed text suite");
    assert_eq!(suite.schema_version, 1);
    assert!(!suite.cases.is_empty());
    suite
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> T {
    let bytes = fs::read(path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    serde_json::from_slice(&bytes)
        .unwrap_or_else(|error| panic!("parse {}: {error}", path.display()))
}

fn sha256_file(path: &Path) -> String {
    use sha2::{Digest, Sha256};
    let bytes = fs::read(path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    format!("{:x}", Sha256::digest(bytes))
}

const AHEM_BYTES: &[u8] = include_bytes!("../../../fixtures/web-first/fonts/ahem.ttf");

/// The pinned gate font's digest, as recorded in
/// `fixtures/web-first/fonts/README.md`. Verified here because this test is
/// the host: the environment carries verified identity, and a host that
/// declares one without checking is the hole the method closes.
const AHEM_SHA256: [u8; 32] = [
    0xb7, 0x19, 0xec, 0xb3, 0x1c, 0x5b, 0x21, 0xfc, 0x57, 0x3c, 0x03, 0xf6, 0x42, 0x1c, 0x74, 0xac,
    0x63, 0xc2, 0x71, 0xa5, 0xa3, 0xff, 0x84, 0x1e, 0x34, 0xf9, 0x70, 0x5f, 0xb9, 0x4b, 0x84, 0x48,
];

fn ahem_environment() -> textlayout::Environment {
    textlayout::Environment::new(vec![textlayout::FontResource {
        key: textlayout::FontKey::new(AHEM_SHA256),
        family: "Ahem".to_string(),
        face_index: 0,
        bytes: Arc::from(AHEM_BYTES),
    }])
}

fn compile(source: &str) -> Result<rframe::Frame, CompileError> {
    SvgFrameSource::from_standalone_svg_with_fonts(
        source,
        InitialViewport::new(100.0, 100.0),
        ahem_environment(),
    )
    .map(|source| source.base_frame())
}

fn compile_best(source: &str) -> Result<SvgFrameSource, CompileError> {
    SvgFrameSource::from_standalone_svg_best_effort_with_fonts(
        source,
        InitialViewport::new(100.0, 100.0),
        ahem_environment(),
    )
}

fn svg(body: &str) -> String {
    format!(r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">{body}</svg>"##)
}

/// One resolved run must be one path node carrying the glyph outlines.
#[test]
fn a_run_resolves_to_one_path_node() {
    let frame = compile(&svg(
        r##"<text x="25" y="60" font-family="Ahem" font-size="50" fill="#000">X</text>"##,
    ))
    .expect("the run is inside the admitted slice");
    let nodes = frame.nodes();
    assert_eq!(nodes.len(), 1);
    assert!(matches!(nodes[0].geometry, rframe::Geometry::Path(_)));
    // The em box, measured: 0.8em above the baseline, 0.2em below.
    let bounds = nodes[0].bounds;
    assert_eq!(
        (bounds.x, bounds.y, bounds.width, bounds.height),
        (25.0, 20.0, 50.0, 50.0)
    );
}

/// The space advances without contributing geometry, and the anchor is a
/// projection of the resolved advance — both measured against Chromium.
#[test]
fn advance_and_anchor_place_the_run() {
    let spaced = compile(&svg(
        r##"<text x="10" y="60" font-family="Ahem" font-size="20" fill="#000">X X</text>"##,
    ))
    .expect("admitted");
    let bounds = spaced.nodes()[0].bounds;
    assert_eq!((bounds.x, bounds.width), (10.0, 60.0));

    let middled = compile(&svg(
        r##"<text x="50" y="60" text-anchor="middle" font-family="Ahem" font-size="20" fill="#000">XXX</text>"##,
    ))
    .expect("admitted");
    let bounds = middled.nodes()[0].bounds;
    assert_eq!((bounds.x, bounds.width), (20.0, 60.0));

    let ended = compile(&svg(
        r##"<text x="90" y="60" text-anchor="end" font-family="Ahem" font-size="20" fill="#000">XXX</text>"##,
    ))
    .expect("admitted");
    let bounds = ended.nodes()[0].bounds;
    assert_eq!((bounds.x, bounds.width), (30.0, 60.0));
}

/// XML whitespace collapsing is the document's semantics, not the shaper's:
/// the indented spelling resolves to the same run as the inline one.
#[test]
fn indented_content_collapses_like_the_inline_spelling() {
    let inline = compile(&svg(
        r##"<text x="10" y="60" font-family="Ahem" font-size="20" fill="#000">X X</text>"##,
    ))
    .expect("admitted");
    let indented = compile(&svg(
        "<text x=\"10\" y=\"60\" font-family=\"Ahem\" font-size=\"20\" fill=\"#000\">\n    X   X\n  </text>",
    ))
    .expect("admitted");
    assert_eq!(inline.nodes()[0].geometry, indented.nodes()[0].geometry);
}

/// A run that collapses to nothing is an admitted nothing — not a node, and
/// not a refusal.
#[test]
fn whitespace_only_content_is_an_admitted_nothing() {
    let frame = compile(&svg(
        "<text x=\"10\" y=\"60\" font-family=\"Ahem\" font-size=\"20\" fill=\"#000\">   </text>",
    ))
    .expect("admitted");
    assert!(frame.nodes().is_empty());
}

/// The hermetic default: with no declared font, text refuses by name rather
/// than reaching for an ambient face.
#[test]
fn an_undeclared_font_refuses_by_name() {
    let error = SvgFrameSource::from_standalone_svg(
        svg(r##"<text x="10" y="60" font-family="Ahem" font-size="20" fill="#000">X</text>"##),
        InitialViewport::new(100.0, 100.0),
    )
    .expect_err("an empty environment resolves no text");
    let message = error.to_string();
    assert!(
        message.contains("Ahem") && message.contains("not in the declared environment"),
        "the refusal must name the family: {message}"
    );
}

/// Everything outside the ratified numeric domain refuses by name — the
/// engine declines to codify the rasterizer-internal rule Chromium snaps by.
#[test]
fn geometry_outside_the_numeric_domain_refuses() {
    for (body, expected) in [
        (
            r##"<text x="25.5" y="60" font-family="Ahem" font-size="50" fill="#000">X</text>"##,
            "not integral",
        ),
        (
            r##"<text x="25" y="60.5" font-family="Ahem" font-size="50" fill="#000">X</text>"##,
            "not integral",
        ),
        (
            r##"<text x="25" y="60" font-family="Ahem" font-size="48" fill="#000">X</text>"##,
            "not an integer multiple of 5",
        ),
        (
            // A middle anchor whose half-advance is fractional.
            r##"<text x="50" y="60" text-anchor="middle" font-family="Ahem" font-size="15" fill="#000">XXX</text>"##,
            "not integral",
        ),
    ] {
        let error = compile(&svg(body)).expect_err("outside the numeric domain");
        let message = error.to_string();
        assert!(
            message.contains(expected),
            "expected {expected:?} in the refusal, got: {message}"
        );
    }
}

/// Constructs the slice does not resolve name themselves rather than
/// painting a run that ignores them.
#[test]
fn beyond_slice_text_constructs_refuse_by_name() {
    for (body, expected) in [
        (
            r##"<text x="10" y="60" font-family="Ahem" font-size="20" dx="5" fill="#000">X</text>"##,
            "dx",
        ),
        (
            r##"<text x="10" y="60" font-family="Ahem" font-size="20" textLength="40" fill="#000">X</text>"##,
            "textLength",
        ),
        (
            r##"<text x="10" y="60" font-family="Ahem" font-size="20" fill="#000"><tspan>X</tspan></text>"##,
            "tspan",
        ),
        (
            r##"<text x="10" y="60" font-family="Ahem" font-size="20" text-anchor="centre" fill="#000">X</text>"##,
            "text-anchor",
        ),
        (
            r##"<text x="10" y="60" font-family="Ahem" font-size="20" fill="#000" stroke="#f00" stroke-width="2">X</text>"##,
            "stroke on <text>",
        ),
        (
            // Outside textlayout-v2's explicit repertoire.
            r##"<text x="10" y="60" font-family="Ahem" font-size="20" fill="#000">X&#x5D0;</text>"##,
            "outside textlayout-v2's admitted",
        ),
    ] {
        let error = compile(&svg(body)).expect_err("outside the admitted text slice");
        let message = error.to_string();
        assert!(
            message.contains(expected),
            "expected {expected:?} in the refusal, got: {message}"
        );
    }
}

/// A `<style>` rule declaring `text-anchor` would be a silent drop under the
/// pinned cascade — Chromium applies it, the servo build has no such
/// longhand — so the sheet patrol refuses it by name.
#[test]
fn text_anchor_in_a_sheet_refuses() {
    let error = compile(&svg(
        r##"<style>text { text-anchor: middle; }</style><text x="50" y="60" font-family="Ahem" font-size="20" fill="#000">XXX</text>"##,
    ))
    .expect_err("a sheet declaring text-anchor must not silently drop");
    assert!(error.to_string().contains("text-anchor"));
}

/// `font-family` arrives through the cascade, so an author rule beats the
/// presentation attribute exactly as Chromium measured.
#[test]
fn an_author_rule_selects_the_family() {
    // The rule names the declared family; the attribute names an undeclared
    // one. If the rule did not win, this would refuse.
    let frame = compile(&svg(
        r##"<style>text { font-family: Ahem; }</style><text x="25" y="60" font-family="NotDeclared" font-size="50" fill="#000">X</text>"##,
    ))
    .expect("the author rule selects the declared family");
    assert_eq!(frame.nodes().len(), 1);
}

/// The computed font-size is not sufficient provenance for text. The pinned
/// cascade resolves viewport and metric units against a different environment,
/// and quantizes large values before the old numeric check sees them. Every
/// such authored route now refuses before shaping rather than emitting the
/// probe's silent wrong pixels.
#[test]
fn authored_font_size_sources_are_guarded_before_shaping() {
    for (body, expected) in [
        (
            r##"<text x="10" y="60" font-family="Ahem" font-size="3.125vw" fill="#000">X</text>"##,
            "font-size basis",
        ),
        (
            r##"<text x="10" y="60" font-family="Ahem" style="font-size:3.125vw" fill="#000">X</text>"##,
            "font-size basis",
        ),
        (
            r##"<style>text { font-size: 3.125vw }</style><text x="10" y="60" font-family="Ahem" fill="#000">X</text>"##,
            "font-size basis",
        ),
        (
            r##"<g font-size="2ex"><text x="10" y="60" font-family="Ahem" font-size="inherit" fill="#000">X</text></g>"##,
            "font-size basis",
        ),
        (
            r##"<text x="10" y="60" font-family="Ahem" style="--s:20px;font-size:var(--s)" fill="#000">X</text>"##,
            "var()",
        ),
        (
            r##"<text x="10" y="60" font-family="Ahem" font-size="20\70 x" fill="#000">X</text>"##,
            "CSS escape",
        ),
        (
            r##"<text x="10" y="60" font-family="Ahem" font-size="125%" fill="#000">X</text>"##,
            "direct number/px source profile",
        ),
        (
            r##"<text x="10" y="60" font-family="Ahem" font-size="1em" fill="#000">X</text>"##,
            "direct number/px source profile",
        ),
        (
            r##"<text x="10" y="60" font-family="Ahem" font-size="calc(10px + 10px)" fill="#000">X</text>"##,
            "direct number/px source profile",
        ),
        (
            r##"<text x="-5070" y="4096" font-family="Ahem" font-size="5119px" fill="#000">X</text>"##,
            "Stylo font-size quantization",
        ),
        (
            r##"<text x="10" y="60" font-family="Ahem" font-size="1e-50px" fill="#000">X</text>"##,
            "loses decimal provenance",
        ),
        (
            r##"<text x="10" y="60" style="font:20px Ahem" fill="#000">X</text>"##,
            "text layout property font",
        ),
        (
            r##"<text x="-5070" y="4096" font-family="Ahem" fill="#000" style="--sentinel:'/*';font-size:5119px">X</text>"##,
            "quoted text and CSS comment delimiters",
        ),
        (
            r##"<text x="-5070" y="4096" font-family="Ahem" fill="#000" style="font-\73 ize:5119px">X</text>"##,
            "font-\\73 ize",
        ),
    ] {
        let source = svg(body);
        let error = compile(&source).expect_err("unproved font-size source must refuse");
        assert!(
            error.to_string().contains(expected),
            "expected {expected:?}, got {error}"
        );

        let best = compile_best(&source).expect("best effort declares and skips the text");
        assert!(best.base_frame().nodes().is_empty());
        assert!(
            best.degradations().iter().any(|degradation| {
                degradation.path().ends_with("/text[1]") && degradation.reason().contains(expected)
            }),
            "expected best effort to declare {expected:?} for {body}; got {:?}",
            best.degradations()
        );
    }
}

/// The narrow source profile still reaches the one cascade through every
/// ingress it claims: direct number/px values, author precedence, and exact
/// inheritance all resolve to the same glyph geometry.
#[test]
fn direct_font_size_sources_remain_admitted_across_the_cascade() {
    for body in [
        r##"<text x="10" y="60" font-family="Ahem" font-size="20" fill="#000">X</text>"##,
        r##"<text x="10" y="60" font-family="Ahem" style="font-size:20px" fill="#000">X</text>"##,
        r##"<style>text { font-size: 20px }</style><text x="10" y="60" font-family="Ahem" font-size="35" fill="#000">X</text>"##,
        r##"<g font-size="20px"><text x="10" y="60" font-family="Ahem" font-size="inherit" fill="#000">X</text></g>"##,
    ] {
        let frame = compile(&svg(body)).expect("direct source profile is admitted");
        let bounds = frame.nodes()[0].bounds;
        assert_eq!(
            (bounds.x, bounds.y, bounds.width, bounds.height),
            (10.0, 44.0, 20.0, 20.0)
        );
    }
}

/// Text semantics represented by Stylo but absent from oracle v2 must not
/// become defaults silently. This includes the font shorthand and inherited
/// declarations from ancestors, not only direct presentation attributes.
#[test]
fn unconsumed_text_layout_css_refuses_at_the_text_node() {
    for (body, expected) in [
        (
            r##"<text x="10" y="60" style="font:italic 20px Ahem" fill="#000">X</text>"##,
            "font",
        ),
        (
            r##"<text x="10" y="60" font-family="Ahem" font-size="20" style="letter-spacing:5px" fill="#000">XX</text>"##,
            "letter-spacing",
        ),
        (
            r##"<style>text { writing-mode: vertical-rl }</style><text x="20" y="20" font-family="Ahem" font-size="20" fill="#000">XX</text>"##,
            "writing-mode",
        ),
        (
            r##"<g font-weight="bold"><text x="10" y="60" font-family="Ahem" font-size="20" fill="#000">X</text></g>"##,
            "font-weight",
        ),
        (
            r##"<g style="dominant-baseline:middle"><text x="10" y="60" font-family="Ahem" font-size="20" fill="#000">X</text></g>"##,
            "dominant-baseline",
        ),
        (
            r##"<g text-anchor="end"><text x="90" y="60" font-family="Ahem" font-size="20" fill="#000">XXX</text></g>"##,
            "text-anchor",
        ),
    ] {
        let source = svg(body);
        let error = compile(&source).expect_err("unconsumed text layout must refuse");
        assert!(
            error.to_string().contains(expected),
            "expected {expected:?}, got {error}"
        );
        let best = compile_best(&source).expect("best effort declares and skips the text");
        assert!(best.degradations().iter().any(|degradation| {
            degradation.path().ends_with("/text[1]") && degradation.reason().contains(expected)
        }));
    }
}

/// Rung A is a final-device promise. Identity linear mappings plus integer
/// translation are admitted after composing text, groups, viewBox, and use;
/// fractional translation and every non-identity linear map refuse even when
/// a sampled Ahem box happened to raster exactly.
#[test]
fn the_numeric_domain_is_enforced_on_the_final_ctm() {
    for source in [
        svg(r##"<text x="10" y="60" transform="translate(5)" font-family="Ahem" font-size="20" fill="#000">X</text>"##),
        svg(r##"<g transform="translate(5)"><text x="10" y="60" font-family="Ahem" font-size="20" fill="#000">X</text></g>"##),
        svg(r##"<g transform="scale(2)"><text x="10" y="60" transform="scale(.5)" font-family="Ahem" font-size="20" fill="#000">X</text></g>"##),
        svg(r##"<g transform="translate(.5)"><text x="10" y="60" transform="translate(-.5)" font-family="Ahem" font-size="20" fill="#000">X</text></g>"##),
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100" viewBox="-5 0 100 100" preserveAspectRatio="none"><text x="10" y="60" font-family="Ahem" font-size="20" fill="#000">X</text></svg>"##.to_string(),
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><defs><text id="t" x="10" y="60" font-family="Ahem" font-size="20" fill="#000">X</text></defs><use href="#t" x="5"/></svg>"##.to_string(),
    ] {
        compile(&source).expect("the composed final CTM is an integer translation");
    }

    for source in [
        svg(r##"<text x="10" y="60" transform="translate(.5)" font-family="Ahem" font-size="20" fill="#000">X</text>"##),
        svg(r##"<g transform="scale(2)"><text x="5" y="30" font-family="Ahem" font-size="10" fill="#000">X</text></g>"##),
        svg(r##"<text x="10" y="60" transform="matrix(0 1 -1 0 100 0)" font-family="Ahem" font-size="20" fill="#000">X</text>"##),
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100" viewBox="0 0 50 50" preserveAspectRatio="none"><text x="5" y="30" font-family="Ahem" font-size="10" fill="#000">X</text></svg>"##.to_string(),
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><defs><text id="t" x="10" y="60" font-family="Ahem" font-size="20" fill="#000">X</text></defs><use href="#t" x=".5"/></svg>"##.to_string(),
    ] {
        let error = compile(&source).expect_err("final CTM is outside rung A");
        let message = error.to_string();
        assert!(message.contains("text final CTM") && message.contains("numeric domain"), "{message}");
        let best = compile_best(&source).expect("best effort declares and skips the text");
        assert!(best.degradations().iter().any(|degradation| {
            !degradation.path().is_empty()
                && degradation.reason().contains("text final CTM")
        }));
    }
}

/// The manifest is the complete text corpus, not a convenient subset. A new
/// source cannot bypass review by appearing beside it without a case row, and
/// duplicate rows cannot make the apparent cell count larger than the gate.
#[test]
fn text_suite_enumerates_every_svg_input() {
    let root = fixture_root();
    let suite = suite();
    let disk: BTreeSet<String> = fs::read_dir(&root)
        .expect("read text fixture root")
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .filter_map(|entry| {
            let path = entry.path();
            (path.extension().and_then(|ext| ext.to_str()) == Some("svg"))
                .then(|| entry.file_name().to_string_lossy().into_owned())
        })
        .collect();
    let declared: BTreeSet<String> = suite
        .cases
        .iter()
        .map(|fixture| fixture.source.clone())
        .collect();
    let ids: BTreeSet<&str> = suite
        .cases
        .iter()
        .map(|fixture| fixture.id.as_str())
        .collect();
    let oracles: BTreeSet<&str> = suite
        .cases
        .iter()
        .map(|fixture| fixture.oracle.as_str())
        .collect();

    assert_eq!(
        declared, disk,
        "every text SVG input must be enumerated exactly once"
    );
    assert_eq!(
        suite.cases.len(),
        declared.len(),
        "text source entries must be unique"
    );
    assert_eq!(suite.cases.len(), ids.len(), "text ids must be unique");
    assert_eq!(
        suite.cases.len(),
        oracles.len(),
        "text oracle entries must be unique"
    );
    assert!(
        suite.cases.windows(2).all(|pair| pair[0].id < pair[1].id),
        "text cases must remain sorted by id"
    );
}

/// Hash-pin every input to the bake: suite, baker, the one shared capture
/// posture, fixture bytes, and oracle bytes. Editing any part without a fresh
/// deterministic Chromium verification therefore fails the Rust gate.
#[test]
fn text_oracle_provenance_is_current() {
    let root = fixture_root();
    let suite = suite();
    let manifest: BakeManifest = read_json(&root.join("oracle-bake.json"));

    assert_eq!(manifest.schema_version, 1, "unsupported text bake schema");
    assert_eq!(manifest.suite, "cases.json");
    assert_eq!(manifest.bake_script, "bake_chromium.ts");
    assert_eq!(manifest.capture_module, "../chromium_capture.ts");
    assert_eq!(
        manifest.suite_sha256,
        sha256_file(&root.join(&manifest.suite)),
        "text suite changed without rebaking Chromium provenance"
    );
    assert_eq!(
        manifest.bake_script_sha256,
        sha256_file(&root.join(&manifest.bake_script)),
        "text baker changed without refreshing provenance"
    );
    assert_eq!(
        manifest.capture_module_sha256,
        sha256_file(&root.join(&manifest.capture_module)),
        "shared Chromium capture posture changed without refreshing text provenance"
    );
    assert_eq!(manifest.records.len(), suite.cases.len());

    for (fixture, record) in suite.cases.iter().zip(&manifest.records) {
        assert_eq!(record.id, fixture.id);
        assert_eq!(record.source, fixture.source);
        assert_eq!(record.oracle, fixture.oracle);
        assert_eq!(
            (record.width, record.height),
            (fixture.width, fixture.height)
        );
        assert_eq!(
            record.source_sha256,
            sha256_file(&root.join(&fixture.source)),
            "{} source changed without rebaking provenance",
            fixture.id
        );
        assert_eq!(
            record.oracle_sha256,
            sha256_file(&root.join(&fixture.oracle)),
            "{} oracle changed without rebaking provenance",
            fixture.id
        );
    }
}

/// The rung's pixel law: the resolved run rasters byte-identically to the
/// Chromium capture, and does so deterministically.
#[test]
fn admitted_runs_match_the_chromium_oracle() {
    let suite = suite();
    let root = fixture_root();
    let mut divergences = Vec::new();
    for case in &suite.cases {
        let source = std::fs::read_to_string(root.join(&case.source)).expect("fixture source");
        let frame = compile(&source).expect("admitted cell");
        let rendered = support::render_through_n0(&frame, case.width, case.height);
        let best = compile_best(&source).expect("best effort admits the same text cell");
        let substantive: Vec<_> = best
            .degradations()
            .iter()
            .filter(|degradation| degradation.action() != DegradationAction::SamplesAsBase)
            .collect();
        if !substantive.is_empty() {
            divergences.push(format!(
                "{}: best effort declared an admitted cell: {:?}",
                case.id, substantive
            ));
            continue;
        }
        let best_rendered = support::render_through_n0(&best.base_frame(), case.width, case.height);
        if best_rendered != rendered {
            divergences.push(format!("{}: strict and best-effort pixels differ", case.id));
            continue;
        }
        let again = support::render_through_n0(&frame, case.width, case.height);
        if rendered != again {
            divergences.push(format!("{}: render is not deterministic", case.id));
            continue;
        }
        let oracle_bytes = std::fs::read(root.join(&case.oracle)).expect("committed oracle");
        let oracle = support::decode_png(&oracle_bytes).expect("decodable oracle");
        if oracle.width != case.width || oracle.height != case.height {
            divergences.push(format!("{}: oracle dimensions differ", case.id));
            continue;
        }
        let differing = rendered
            .chunks_exact(4)
            .zip(oracle.pixels.chunks_exact(4))
            .filter(|(a, b)| a != b)
            .count();
        if differing != 0 {
            divergences.push(format!("{}: {differing} differing pixels", case.id));
        }
    }
    assert!(divergences.is_empty(), "{}", divergences.join("\n"));
}

/// The suite's font declaration is an identity the host verifies, not a path
/// it trusts: this test is the host for the Rust-side gate, so the bytes it
/// declares must be the bytes the manifest names.
#[test]
fn the_declared_font_identity_is_verified() {
    use sha2::{Digest, Sha256};
    let suite = suite();
    let digest = format!("{:x}", Sha256::digest(AHEM_BYTES));
    assert_eq!(
        digest, suite.font.sha256,
        "the gate font is not the pinned identity"
    );
    assert_eq!(
        hex_bytes(&digest),
        AHEM_SHA256,
        "the constant this test declares to the engine must be the same digest"
    );
    assert_eq!(suite.font.family, "Ahem");
    assert_eq!(suite.font.path, "../fonts/ahem.ttf");
}

fn hex_bytes(hex: &str) -> [u8; 32] {
    let mut out = [0u8; 32];
    for (index, slot) in out.iter_mut().enumerate() {
        *slot = u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16).expect("hex digest");
    }
    out
}
