//! Rung-B/T3 SVG text: real-font artifact geometry, combining clusters,
//! glyph placement offsets, and UTF-16 query mapping,
//! before outline rasterization.
//!
//! Chromium directly grades the SVG character-cell facts it exposes. Glyph
//! ids, cluster source/glyph spans, and ink bounds are separately pinned to
//! the exact font bytes because the browser API exposes no glyph identifier
//! or outline.
//! Engine pixels are tested only for their own determinism and admission
//! identity; this suite makes no Chromium real-font pixel claim.

#[allow(dead_code)]
mod support;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use rframe::{FillRule, Geometry, PathCommand, PathData};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use textlayout::OutlineSink;
use websem::{DegradationAction, InitialViewport, SvgFrameSource};

const AHEM_BYTES: &[u8] = include_bytes!("../../../fixtures/web-first/fonts/ahem.ttf");
const AHEM_SHA256: &str = "b719ecb31c5b21fc573c03f6421c74ac63c271a5a3ff841e34f9705fb94b8448";
const BUNGEE_BYTES: &[u8] = include_bytes!("../../../fixtures/fonts/Bungee/Bungee-Regular.ttf");
const BUNGEE_SHA256: &str = "b90c3ca443713b070cb1dec6a3bb1ef7572c2b565c431d9a85d74bbfa07e24cc";
const PT_SERIF_BYTES: &[u8] =
    include_bytes!("../../../fixtures/fonts/PT_Serif/PTSerif-Regular.ttf");
const PT_SERIF_SHA256: &str = "13d9f82f41fcd7d2813dc0a44a9639dec0c1e9a922ab96c7de8dec467c3dec55";

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/web-first/text/geometry")
}

fn is_textlayout_v3_direct_scalar(character: char) -> bool {
    matches!(
        character,
        ' '..='~'
            | '\u{00C0}'..='\u{00C5}'
            | '\u{00C7}'..='\u{00CF}'
            | '\u{00D1}'..='\u{00D6}'
            | '\u{00D9}'..='\u{00DD}'
            | '\u{00E0}'..='\u{00E5}'
            | '\u{00E7}'..='\u{00EF}'
            | '\u{00F1}'..='\u{00F6}'
            | '\u{00F9}'..='\u{00FD}'
            | '\u{00FF}'
    )
}

#[derive(Deserialize)]
struct Suite {
    schema_version: u32,
    fonts: Vec<SuiteFont>,
    cases: Vec<SuiteCase>,
}

#[derive(Deserialize)]
struct SuiteFont {
    family: String,
    path: String,
    sha256: String,
    face_index: u32,
    license: String,
    license_sha256: String,
}

#[derive(Deserialize)]
struct SuiteCase {
    id: String,
    source: String,
    oracle: String,
    width: i32,
    height: i32,
    text: String,
    x: String,
    y: String,
    text_anchor: String,
    font_family: String,
    font_size: String,
    fill: String,
    font_facts: FontFacts,
}

#[derive(Deserialize)]
struct FontFacts {
    #[serde(default)]
    schema_version: Option<u32>,
    units_per_em: u16,
    glyphs: Vec<GlyphFact>,
    #[serde(default)]
    clusters: Option<Vec<ClusterFact>>,
    ink_bounds: NumberRect,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum GlyphFact {
    Direct(DirectGlyphFact),
    Placed(PlacedGlyphFact),
}

#[derive(Deserialize)]
struct DirectGlyphFact {
    source_utf8_byte: u32,
    source_utf16_index: u32,
    scalar: String,
    glyph_id: u16,
    cluster: u32,
}

#[derive(Deserialize)]
struct PlacedGlyphFact {
    glyph_id: u16,
    cluster_index: usize,
    x: f64,
    offset_x: f64,
    offset_y: f64,
    advance: f64,
}

#[derive(Deserialize)]
struct ClusterFact {
    source_utf8: [usize; 2],
    source_utf16: [usize; 2],
    source_scalars: [usize; 2],
    glyphs: [usize; 2],
}

#[derive(Deserialize)]
struct GeometryOracle {
    schema_version: u32,
    kind: String,
    measurement: Measurement,
}

#[derive(Deserialize)]
struct Measurement {
    text_content: String,
    computed_font_family: String,
    computed_font_size: String,
    computed_text_anchor: String,
    font_ready: bool,
    number_of_chars: usize,
    computed_text_length: f64,
    substring_length: f64,
    characters: Vec<CharacterGeometry>,
}

#[derive(Deserialize)]
struct CharacterGeometry {
    utf16_code_unit: u32,
    substring_length: f64,
    start: NumberPoint,
    end: NumberPoint,
    extent: NumberRect,
    rotation: f64,
}

#[derive(Deserialize)]
struct NumberPoint {
    x: f64,
    y: f64,
}

#[derive(Deserialize)]
struct NumberRect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[derive(Deserialize)]
struct BakeManifest {
    schema_version: u32,
    kind: String,
    browser_version: String,
    suite: String,
    suite_sha256: String,
    bake_script: String,
    bake_script_sha256: String,
    capture_module: String,
    capture_module_sha256: String,
    fonts: Vec<BakeFont>,
    records: Vec<BakeRecord>,
}

#[derive(Deserialize)]
struct BakeFont {
    family: String,
    sha256: String,
    face_index: u32,
    license_sha256: String,
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
    font_family: String,
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> T {
    let bytes =
        std::fs::read(path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    serde_json::from_slice(&bytes)
        .unwrap_or_else(|error| panic!("parse {}: {error}", path.display()))
}

fn suite() -> Suite {
    let suite: Suite = read_json(&fixture_root().join("cases.json"));
    assert_eq!(suite.schema_version, 2);
    assert!(!suite.fonts.is_empty());
    assert!(!suite.cases.is_empty());
    suite
}

fn sha256_file(path: &Path) -> String {
    let bytes =
        std::fs::read(path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    format!("{:x}", Sha256::digest(bytes))
}

fn hex_bytes(hex: &str) -> [u8; 32] {
    assert_eq!(hex.len(), 64);
    let mut out = [0; 32];
    for (index, slot) in out.iter_mut().enumerate() {
        *slot = u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16).expect("hex digest");
    }
    out
}

fn suite_font<'a>(suite: &'a Suite, family: &str) -> &'a SuiteFont {
    suite
        .fonts
        .iter()
        .find(|font| font.family == family)
        .unwrap_or_else(|| panic!("undeclared suite font {family}"))
}

fn font_bytes(font: &SuiteFont) -> Arc<[u8]> {
    std::fs::read(fixture_root().join(&font.path))
        .expect("pinned real-font bytes")
        .into()
}

fn environment(font: &SuiteFont, bytes: &Arc<[u8]>) -> textlayout::Environment {
    textlayout::Environment::new(vec![textlayout::FontResource {
        key: textlayout::FontKey::new(hex_bytes(&font.sha256)),
        family: font.family.clone(),
        face_index: font.face_index,
        bytes: Arc::clone(bytes),
    }])
}

fn bungee_environment() -> textlayout::Environment {
    let digest = Sha256::digest(BUNGEE_BYTES);
    assert_eq!(format!("{digest:x}"), BUNGEE_SHA256);
    textlayout::Environment::new(vec![textlayout::FontResource {
        key: textlayout::FontKey::new(digest.into()),
        family: "Bungee".to_string(),
        face_index: 0,
        bytes: Arc::from(BUNGEE_BYTES),
    }])
}

fn ahem_environment() -> textlayout::Environment {
    let digest = Sha256::digest(AHEM_BYTES);
    assert_eq!(format!("{digest:x}"), AHEM_SHA256);
    textlayout::Environment::new(vec![textlayout::FontResource {
        key: textlayout::FontKey::new(digest.into()),
        family: "Ahem".to_string(),
        face_index: 0,
        bytes: Arc::from(AHEM_BYTES),
    }])
}

fn pt_serif_environment() -> textlayout::Environment {
    let digest = Sha256::digest(PT_SERIF_BYTES);
    assert_eq!(format!("{digest:x}"), PT_SERIF_SHA256);
    textlayout::Environment::new(vec![textlayout::FontResource {
        key: textlayout::FontKey::new(digest.into()),
        family: "PT Serif".to_string(),
        face_index: 0,
        bytes: Arc::from(PT_SERIF_BYTES),
    }])
}

fn canonical_source(case: &SuiteCase) -> String {
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\">\n  \
         <text x=\"{}\" y=\"{}\" text-anchor=\"{}\" font-family=\"{}\" \
         font-size=\"{}\" fill=\"{}\">{}</text>\n</svg>\n",
        case.width,
        case.height,
        case.x,
        case.y,
        case.text_anchor,
        case.font_family,
        case.font_size,
        case.fill,
        case.text
    )
}

fn exact(label: &str, artifact: f32, chromium: f64) {
    assert_eq!(
        f64::from(artifact),
        chromium,
        "{label}: artifact binary32 promoted to binary64 must equal Chromium exactly"
    );
}

fn parse_number(label: &str, source: &str) -> f32 {
    source
        .parse::<f32>()
        .unwrap_or_else(|error| panic!("{label} {source:?}: {error}"))
}

#[derive(Default)]
struct LocalPathSink {
    commands: Vec<PathCommand>,
}

impl OutlineSink for LocalPathSink {
    fn move_to(&mut self, x: f32, y: f32) {
        self.commands.push(PathCommand::MoveTo { x, y });
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.commands.push(PathCommand::LineTo { x, y });
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.commands.push(PathCommand::QuadTo { x1, y1, x, y });
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.commands.push(PathCommand::CubicTo {
            x1,
            y1,
            x2,
            y2,
            x,
            y,
        });
    }

    fn close(&mut self) {
        self.commands.push(PathCommand::Close);
    }
}

fn local_outline_path(layout: &textlayout::ResolvedTextLayout) -> PathData {
    let mut sink = LocalPathSink::default();
    for glyph_index in 0..layout.glyphs().len() {
        layout.outline(glyph_index, &mut sink);
    }
    PathData::new(sink.commands, FillRule::NonZero).expect("resolved outline is a checked path")
}

fn assert_text_refusal(
    text: &str,
    family: &str,
    environment: textlayout::Environment,
    expected_reason: &str,
) {
    let source = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="3000" height="1500"><text x="100" y="1150" font-family="{family}" font-size="1000" fill="#111827">{text}</text></svg>"##
    );
    let strict = SvgFrameSource::from_standalone_svg_with_fonts(
        source.as_str(),
        InitialViewport::new(3000.0, 1500.0),
        environment.clone(),
    )
    .expect_err("strict must refuse the out-of-profile text");
    assert!(
        strict.to_string().contains(expected_reason),
        "strict reason {strict} did not contain {expected_reason:?}"
    );

    let best = SvgFrameSource::from_standalone_svg_best_effort_with_fonts(
        source.as_str(),
        InitialViewport::new(3000.0, 1500.0),
        environment,
    )
    .expect("best effort must declare and skip the text node");
    assert!(best.base_frame().nodes().is_empty());
    let substantive: Vec<_> = best
        .degradations()
        .iter()
        .filter(|item| item.action() != DegradationAction::SamplesAsBase)
        .collect();
    assert_eq!(substantive.len(), 1);
    assert!(substantive[0].path().ends_with("/text[1]"));
    assert!(substantive[0].reason().contains(expected_reason));
}

#[test]
fn geometry_suite_is_closed_and_hash_pinned() {
    let root = fixture_root();
    let suite = suite();
    let manifest: BakeManifest = read_json(&root.join("oracle-bake.json"));
    assert_eq!(manifest.schema_version, 2);
    assert_eq!(manifest.kind, "chromium-svg-text-geometry-oracle");
    assert_eq!(manifest.browser_version, "149.0.7827.55");
    assert_eq!(manifest.suite, "cases.json");
    assert_eq!(manifest.bake_script, "bake_chromium.ts");
    assert_eq!(manifest.capture_module, "../../chromium_capture.ts");
    assert_eq!(
        manifest.suite_sha256,
        sha256_file(&root.join(&manifest.suite))
    );
    assert_eq!(
        manifest.bake_script_sha256,
        sha256_file(&root.join(&manifest.bake_script))
    );
    assert_eq!(
        manifest.capture_module_sha256,
        sha256_file(&root.join(&manifest.capture_module))
    );
    assert!(
        suite
            .fonts
            .windows(2)
            .all(|pair| pair[0].family < pair[1].family)
    );
    assert_eq!(manifest.fonts.len(), suite.fonts.len());
    for (font, baked) in suite.fonts.iter().zip(&manifest.fonts) {
        assert_eq!(font.sha256, sha256_file(&root.join(&font.path)));
        assert_eq!(font.license_sha256, sha256_file(&root.join(&font.license)));
        assert_eq!(baked.family, font.family);
        assert_eq!(baked.sha256, font.sha256);
        assert_eq!(baked.face_index, font.face_index);
        assert_eq!(baked.license_sha256, font.license_sha256);
    }
    assert_eq!(manifest.records.len(), suite.cases.len());

    let sources: BTreeSet<String> = std::fs::read_dir(&root)
        .expect("geometry root")
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().and_then(|ext| ext.to_str()) == Some("svg"))
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    let declared_sources: BTreeSet<String> =
        suite.cases.iter().map(|case| case.source.clone()).collect();
    assert_eq!(
        sources, declared_sources,
        "every geometry source is enumerated"
    );

    let oracle_root = root.join("chromium");
    let oracles: BTreeSet<String> = std::fs::read_dir(&oracle_root)
        .expect("geometry oracle root")
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().and_then(|ext| ext.to_str()) == Some("json"))
        .map(|entry| format!("chromium/{}", entry.file_name().to_string_lossy()))
        .collect();
    let declared_oracles: BTreeSet<String> =
        suite.cases.iter().map(|case| case.oracle.clone()).collect();
    assert_eq!(
        oracles, declared_oracles,
        "every geometry oracle is enumerated"
    );

    assert!(suite.cases.windows(2).all(|pair| pair[0].id < pair[1].id));
    for (case, record) in suite.cases.iter().zip(&manifest.records) {
        assert_eq!(case.source, record.source);
        assert_eq!(case.oracle, record.oracle);
        assert_eq!(case.id, record.id);
        assert_eq!((case.width, case.height), (record.width, record.height));
        assert_eq!(case.font_family, record.font_family);
        assert_eq!(
            canonical_source(case),
            std::fs::read_to_string(root.join(&case.source)).expect("canonical source")
        );
        assert_eq!(record.source_sha256, sha256_file(&root.join(&case.source)));
        assert_eq!(record.oracle_sha256, sha256_file(&root.join(&case.oracle)));
    }
}

#[test]
fn resolved_artifacts_match_chromium_geometry_exactly() {
    let suite = suite();
    for case in &suite.cases {
        let font = suite_font(&suite, &case.font_family);
        let bytes = font_bytes(font);
        let environment = environment(font, &bytes);
        let face = rustybuzz::ttf_parser::Face::parse(&bytes, font.face_index)
            .expect("pinned face parses for font-fact checks");
        let oracle: GeometryOracle = read_json(&fixture_root().join(&case.oracle));
        assert_eq!(oracle.schema_version, 1);
        assert_eq!(oracle.kind, "chromium-svg-text-geometry");
        let measured = oracle.measurement;
        assert!(measured.font_ready);
        assert_eq!(measured.text_content, case.text);
        assert_eq!(measured.computed_font_family, font.family);
        assert_eq!(measured.computed_font_size, format!("{}px", case.font_size));
        assert_eq!(measured.computed_text_anchor, case.text_anchor);

        let font_size = parse_number("font-size", &case.font_size);
        let x = parse_number("x", &case.x);
        let y = parse_number("y", &case.y);
        let layout = textlayout::resolve(
            &textlayout::AttributedText {
                text: case.text.clone(),
                style: textlayout::Style {
                    family: case.font_family.clone(),
                    size: font_size,
                },
            },
            &environment,
        )
        .expect("rung-B artifact resolves");
        assert_eq!(
            layout.face().key,
            textlayout::FontKey::new(hex_bytes(&font.sha256))
        );
        assert_eq!(layout.face().face_index, font.face_index);
        assert_eq!(layout.face().units_per_em, case.font_facts.units_per_em);
        assert_eq!(layout.source(), case.text);
        assert_eq!(layout.font_size(), font_size);
        assert_eq!(layout.glyphs().len(), case.font_facts.glyphs.len());
        let source_utf16: Vec<u16> = case.text.encode_utf16().collect();
        assert_eq!(measured.number_of_chars, source_utf16.len());
        assert_eq!(measured.characters.len(), source_utf16.len());
        exact(
            "run advance",
            layout.advance(),
            measured.computed_text_length,
        );
        exact(
            "whole substring",
            layout.advance(),
            measured.substring_length,
        );

        let anchor_start = match case.text_anchor.as_str() {
            "start" => x,
            "middle" => x - layout.advance() / 2.0,
            "end" => x - layout.advance(),
            other => panic!("unrecognized anchor {other}"),
        };
        let metrics = layout.metrics();
        match case.font_facts.schema_version {
            None => {
                assert!(case.font_facts.clusters.is_none());
                assert_eq!(layout.clusters().len(), case.font_facts.glyphs.len());
                let source_scalars: Vec<(usize, char)> = case.text.char_indices().collect();
                assert_eq!(source_scalars.len(), case.font_facts.glyphs.len());
                let mut source_utf16_index = 0;
                for (index, ((glyph, cluster), fact)) in layout
                    .glyphs()
                    .iter()
                    .zip(layout.clusters())
                    .zip(&case.font_facts.glyphs)
                    .enumerate()
                {
                    let GlyphFact::Direct(fact) = fact else {
                        panic!("legacy direct facts must retain their direct shape")
                    };
                    let scalar = fact.scalar.chars().next().expect("one scalar fact");
                    assert_eq!(fact.scalar.chars().count(), 1);
                    let (source_utf8_byte, source_scalar) = source_scalars[index];
                    assert_eq!(scalar, source_scalar);
                    assert!(is_textlayout_v3_direct_scalar(scalar));
                    assert_eq!(fact.source_utf8_byte as usize, source_utf8_byte);
                    assert_eq!(fact.source_utf16_index as usize, source_utf16_index);
                    assert_eq!(fact.source_utf8_byte, fact.cluster);
                    assert_eq!(glyph.cluster_index, index);
                    assert_eq!(
                        cluster.source_utf8(),
                        source_utf8_byte..source_utf8_byte + scalar.len_utf8()
                    );
                    assert_eq!(
                        cluster.source_utf16(),
                        source_utf16_index..source_utf16_index + scalar.len_utf16()
                    );
                    assert_eq!(cluster.source_scalars(), index..index + 1);
                    assert_eq!(cluster.glyphs(), index..index + 1);
                    assert_eq!(cluster.source_utf8().start as u32, fact.cluster);
                    assert_eq!(glyph.glyph_id, fact.glyph_id);
                    assert_eq!((glyph.offset_x, glyph.offset_y), (0.0, 0.0));
                    assert_eq!(
                        face.glyph_index(scalar).expect("direct cmap glyph").0,
                        fact.glyph_id,
                        "glyph identity is inferred from the pinned cmap, not a browser API"
                    );
                    source_utf16_index += scalar.len_utf16();
                }
                assert_eq!(source_utf16_index, source_utf16.len());
            }
            Some(2) => {
                let facts = case
                    .font_facts
                    .clusters
                    .as_ref()
                    .expect("v2 font facts carry clusters");
                assert_eq!(layout.clusters().len(), facts.len());
                for (cluster, fact) in layout.clusters().iter().zip(facts) {
                    assert_eq!(
                        cluster.source_utf8(),
                        fact.source_utf8[0]..fact.source_utf8[1]
                    );
                    assert_eq!(
                        cluster.source_utf16(),
                        fact.source_utf16[0]..fact.source_utf16[1]
                    );
                    assert_eq!(
                        cluster.source_scalars(),
                        fact.source_scalars[0]..fact.source_scalars[1]
                    );
                    assert_eq!(cluster.glyphs(), fact.glyphs[0]..fact.glyphs[1]);
                }
                for (glyph, fact) in layout.glyphs().iter().zip(&case.font_facts.glyphs) {
                    let GlyphFact::Placed(fact) = fact else {
                        panic!("v2 font facts carry placed glyphs")
                    };
                    assert_eq!(glyph.glyph_id, fact.glyph_id);
                    assert_eq!(glyph.cluster_index, fact.cluster_index);
                    exact("glyph pen x", glyph.x, fact.x);
                    exact("glyph x offset", glyph.offset_x, fact.offset_x);
                    exact("glyph y offset", glyph.offset_y, fact.offset_y);
                    exact("glyph advance", glyph.advance, fact.advance);
                }
            }
            other => panic!("unsupported font-fact schema {other:?}"),
        }

        // SVG DOM methods address UTF-16 code units, but every unit in one
        // typographic/shaping cluster reports that cluster's complete cell.
        for (cluster_index, cluster) in layout.clusters().iter().enumerate() {
            let glyphs = &layout.glyphs()[cluster.glyphs()];
            let cluster_x = glyphs[0].x;
            let cluster_advance: f32 = glyphs.iter().map(|glyph| glyph.advance).sum();
            for character_index in cluster.source_utf16() {
                let character = &measured.characters[character_index];
                assert_eq!(
                    character.utf16_code_unit,
                    u32::from(source_utf16[character_index]),
                    "cluster {cluster_index} UTF-16 identity"
                );
                exact(
                    "character cluster advance",
                    cluster_advance,
                    character.substring_length,
                );
                exact(
                    "character cluster start x",
                    anchor_start + cluster_x,
                    character.start.x,
                );
                exact("character baseline y", y, character.start.y);
                exact(
                    "character cluster end x",
                    anchor_start + cluster_x + cluster_advance,
                    character.end.x,
                );
                exact("character end y", y, character.end.y);
                exact(
                    "character cell x",
                    anchor_start + cluster_x,
                    character.extent.x,
                );
                exact("character cell y", y - metrics.ascent, character.extent.y);
                exact(
                    "character cell width",
                    cluster_advance,
                    character.extent.width,
                );
                exact(
                    "character cell height",
                    metrics.ascent + metrics.descent,
                    character.extent.height,
                );
                assert_eq!(character.rotation, 0.0);
            }
        }

        if case.id == "svg-text-allerta-latin-precomposed" {
            assert!(case.font_facts.glyphs.iter().any(|fact| {
                matches!(
                    fact,
                    GlyphFact::Direct(fact)
                        if fact.source_utf8_byte != fact.source_utf16_index
                )
            }));
        } else if case.id == "svg-text-allerta-decomposed-acute" {
            assert_eq!(layout.clusters()[1].source_scalars().len(), 2);
            assert_eq!(layout.clusters()[1].glyphs().len(), 1);
        } else if case.id == "svg-text-bungee-acute-offset" {
            assert_ne!(layout.glyphs()[2].offset_x, 0.0);
        } else if case.id == "svg-text-bungee-double-acute-offset" {
            assert_ne!(layout.glyphs()[2].offset_x, 0.0);
            assert_ne!(layout.glyphs()[2].offset_y, 0.0);
        }

        let ink = layout.ink_bounds().expect("real face has outline ink");
        exact("ink x", ink.x, case.font_facts.ink_bounds.x);
        exact("ink y", ink.y, case.font_facts.ink_bounds.y);
        exact("ink width", ink.width, case.font_facts.ink_bounds.width);
        exact("ink height", ink.height, case.font_facts.ink_bounds.height);
        let streamed = local_outline_path(&layout).local_bounds();
        assert_eq!(
            (streamed.x, streamed.y, streamed.width, streamed.height),
            (ink.x, ink.y, ink.width, ink.height),
            "{}: artifact bounds must be the exact streamed-outline bounds",
            case.id
        );
    }
}

#[test]
fn real_font_pixels_obey_only_the_engine_laws() {
    let suite = suite();
    for case in &suite.cases {
        let font = suite_font(&suite, &case.font_family);
        let bytes = font_bytes(font);
        let environment = environment(font, &bytes);
        let source = std::fs::read_to_string(fixture_root().join(&case.source)).unwrap();
        let font_size = parse_number("font-size", &case.font_size);
        let layout = textlayout::resolve(
            &textlayout::AttributedText {
                text: case.text.clone(),
                style: textlayout::Style {
                    family: case.font_family.clone(),
                    size: font_size,
                },
            },
            &environment,
        )
        .expect("rung-B artifact resolves for frame projection");
        assert!(layout.ink_bounds().is_some());
        let strict = SvgFrameSource::from_standalone_svg_with_fonts(
            source.as_str(),
            InitialViewport::new(case.width as f32, case.height as f32),
            environment.clone(),
        )
        .unwrap_or_else(|error| panic!("{} strict compile: {error}", case.id))
        .base_frame();
        let best = SvgFrameSource::from_standalone_svg_best_effort_with_fonts(
            source.as_str(),
            InitialViewport::new(case.width as f32, case.height as f32),
            environment,
        )
        .unwrap_or_else(|error| panic!("{} best compile: {error}", case.id));
        let substantive: Vec<_> = best
            .degradations()
            .iter()
            .filter(|item| item.action() != DegradationAction::SamplesAsBase)
            .collect();
        assert!(substantive.is_empty(), "{}: {substantive:?}", case.id);

        assert_eq!(strict.nodes().len(), 1);
        let node = &strict.nodes()[0];
        let Geometry::Path(path) = &node.geometry else {
            panic!("{}: text must lower to path geometry", case.id)
        };
        assert_eq!(
            node.bounds,
            path.local_bounds(),
            "{}: the identity-mapped node must state its exact lowered path bounds",
            case.id
        );
        let first = support::render_through_n0(&strict, case.width, case.height);
        let again = support::render_through_n0(&strict, case.width, case.height);
        let best_pixels = support::render_through_n0(&best.base_frame(), case.width, case.height);
        assert_eq!(first, again, "{}: fresh renders differ", case.id);
        assert_eq!(first, best_pixels, "{}: admissions differ", case.id);
        assert!(
            first.chunks_exact(4).any(|pixel| pixel[3] != 0),
            "{}: the clipped real-font witness must paint",
            case.id
        );
    }
}

#[test]
fn vertical_query_metric_grid_refuses_when_horizontal_boundaries_are_exact() {
    let suite = suite();
    let font = suite_font(&suite, "Allerta");
    let bytes = font_bytes(font);
    let source = r##"<svg xmlns="http://www.w3.org/2000/svg" width="400" height="100"><text x="0" y="80" text-anchor="start" font-family="Allerta" font-size="80" fill="#000">Hxi</text></svg>"##;
    let strict = SvgFrameSource::from_standalone_svg_with_fonts(
        source,
        InitialViewport::new(400.0, 100.0),
        environment(font, &bytes),
    )
    .expect_err("fractional query metrics must not lower silently");
    assert!(strict.to_string().contains("query metrics"));

    let best = SvgFrameSource::from_standalone_svg_best_effort_with_fonts(
        source,
        InitialViewport::new(400.0, 100.0),
        environment(font, &bytes),
    )
    .expect("best effort declares and skips the run");
    assert!(best.base_frame().nodes().is_empty());
    assert!(best.degradations().iter().any(|item| {
        item.path().ends_with("/text[1]") && item.reason().contains("query metrics")
    }));
}

#[test]
fn horizontal_query_grid_refuses_when_vertical_metrics_are_integral() {
    let source = r##"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="100"><text x="0" y="60" text-anchor="start" font-family="Bungee" font-size="50" fill="#000">Hxi</text></svg>"##;
    let strict = SvgFrameSource::from_standalone_svg_with_fonts(
        source,
        InitialViewport::new(200.0, 100.0),
        bungee_environment(),
    )
    .expect_err("off-grid horizontal boundaries must not lower silently");
    assert!(strict.to_string().contains("1/64 SVG text query grid"));

    let best = SvgFrameSource::from_standalone_svg_best_effort_with_fonts(
        source,
        InitialViewport::new(200.0, 100.0),
        bungee_environment(),
    )
    .expect("best effort declares and skips the run");
    assert!(best.base_frame().nodes().is_empty());
    assert!(best.degradations().iter().any(|item| {
        item.path().ends_with("/text[1]") && item.reason().contains("1/64 SVG text query grid")
    }));
}

#[test]
fn merged_ligature_cluster_refuses_in_both_admissions_before_lowering() {
    let source = r##"<svg xmlns="http://www.w3.org/2000/svg" width="5000" height="4100"><text x="0" y="4000" text-anchor="start" font-family="PT Serif" font-size="5000" fill="#000">fi</text></svg>"##;
    let strict = SvgFrameSource::from_standalone_svg_with_fonts(
        source,
        InitialViewport::new(5000.0, 4100.0),
        pt_serif_environment(),
    )
    .expect_err("a merged source cluster must not lower silently");
    assert!(strict.to_string().contains("shaping cluster mapping"));

    let best = SvgFrameSource::from_standalone_svg_best_effort_with_fonts(
        source,
        InitialViewport::new(5000.0, 4100.0),
        pt_serif_environment(),
    )
    .expect("best effort declares and skips the run");
    assert!(best.base_frame().nodes().is_empty());
    assert!(best.degradations().iter().any(|item| {
        item.path().ends_with("/text[1]") && item.reason().contains("shaping cluster mapping")
    }));
}

#[test]
fn combining_profile_boundaries_refuse_at_the_same_text_node_in_both_admissions() {
    assert_text_refusal(
        "Ax\u{0300}Z",
        "Bungee",
        bungee_environment(),
        "outside textlayout-v3's admitted",
    );
    for source in [
        "\u{0301}AX",
        "Ax\u{0301}\u{0301}Z",
        "A1\u{0301}Z",
        "Aé\u{0301}Z",
    ] {
        assert_text_refusal(
            source,
            "Bungee",
            bungee_environment(),
            "is not the sole admitted mark",
        );
    }
    assert_text_refusal(
        "Ax\u{0301}Z",
        "Ahem",
        ahem_environment(),
        "no glyph for '\\u{301}' at byte 2",
    );
}
