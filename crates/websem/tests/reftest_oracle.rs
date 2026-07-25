//! Chromium-backed exact reftests for every root-level Web-first primitive.
//!
//! `fixtures/web-first/primitives.json` is the closed enumeration. The tests
//! fail if a root `.html`/`.svg` input is not listed, if bake provenance drifts,
//! if any RGBA pixel differs from Chromium, or if CPU/PNG output changes across
//! two identical runs. Every pixel renders through n0's one downstream —
//! rframe's proving painter retired with the D-M vector join. No similarity
//! score is computed and the sealed scoreboard is never invoked.

mod support;

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use n0::paint::PaintCtx;
use rframe::{Frame, FrameNode, Geometry, Identity, Provenance, SolidPaintStack, VisualRef};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use skia_safe::{Color, surfaces};
use support::{decode_png, render_through_n0};
use websem::{InitialViewport, compile_html_inline_svg, compile_standalone_svg};

#[derive(Debug, Deserialize)]
struct PrimitiveSuite {
    schema_version: u32,
    fixtures: Vec<Primitive>,
}

#[derive(Debug, Deserialize)]
struct Primitive {
    id: String,
    source: String,
    entry: String,
    oracle: String,
    width: i32,
    height: i32,
}

#[derive(Debug, Deserialize)]
struct BakeManifest {
    schema_version: u32,
    bake_script_sha256: String,
    suite: String,
    suite_sha256: String,
    fixtures: Vec<BakeRecord>,
}

#[derive(Debug, Deserialize)]
struct BakeRecord {
    id: String,
    source: String,
    source_sha256: String,
    oracle: String,
    oracle_sha256: String,
    width: i32,
    height: i32,
}

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/web-first")
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> T {
    let bytes = fs::read(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    serde_json::from_slice(&bytes).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()))
}

fn suite() -> PrimitiveSuite {
    read_json(&fixture_root().join("primitives.json"))
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn sha256_file(path: &Path) -> String {
    let bytes = fs::read(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    sha256(&bytes)
}

/// Render the frame through n0 and encode PNG bytes — the downstream plus
/// encoding, mirroring the host's output path.
fn render_png_through_n0(frame: &Frame, width: i32, height: i32) -> Vec<u8> {
    let context = PaintCtx::new(None);
    let product = n0::glyphless::compile(frame.clone()).expect("compile admitted Web frame");
    let mut surface = surfaces::raster_n32_premul((width, height)).expect("CPU raster surface");
    surface.canvas().clear(Color::TRANSPARENT);
    product
        .execute(
            surface.canvas(),
            &math2::transform::AffineTransform::identity(),
            &context,
        )
        .expect("execute admitted Web frame through n0");
    let image = surface.image_snapshot();
    skia_safe::encode::image(None, &image, skia_safe::EncodedImageFormat::PNG, None)
        .expect("encode PNG")
        .as_bytes()
        .to_vec()
}

/// The `[r, g, b, a]` at `(x, y)` of an RGBA8888 row-major buffer.
fn pixel(pixels: &[u8], width: i32, x: i32, y: i32) -> [u8; 4] {
    let height = pixels.len() as i32 / (width * 4);
    assert!(
        (0..width).contains(&x) && (0..height).contains(&y),
        "probe ({x}, {y}) outside {width}x{height}"
    );
    let offset = ((y * width + x) * 4) as usize;
    pixels[offset..offset + 4].try_into().expect("RGBA pixel")
}

fn contract_frame(paints: SolidPaintStack, transform: math2::transform::AffineTransform) -> Frame {
    let rect = math2::Rectangle::from_xywh(2.0, 3.0, 8.0, 6.0);
    Frame {
        owner: VisualRef::new(Identity::new(1), Provenance::new(10)),
        bounds: math2::Rectangle::from_xywh(0.0, 0.0, 32.0, 24.0),
        nodes: vec![FrameNode {
            owner: VisualRef::new(Identity::new(2), Provenance::new(20)),
            transform,
            geometry: Geometry::Rect(rect),
            bounds: math2::rect_transform(rect, &transform),
            paints,
        }],
    }
}

#[test]
fn primitive_suite_enumerates_every_root_input() {
    let root = fixture_root();
    let suite = suite();
    assert_eq!(
        suite.schema_version, 0,
        "unsupported primitive suite schema"
    );

    let disk: BTreeSet<String> = fs::read_dir(&root)
        .expect("read primitive fixture root")
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .filter_map(|entry| {
            let path = entry.path();
            matches!(
                path.extension().and_then(|ext| ext.to_str()),
                Some("html" | "svg")
            )
            .then(|| entry.file_name().to_string_lossy().into_owned())
        })
        .collect();
    let declared: BTreeSet<String> = suite
        .fixtures
        .iter()
        .map(|fixture| fixture.source.clone())
        .collect();

    assert_eq!(
        declared, disk,
        "every root Web-first HTML/SVG primitive must be enumerated exactly once"
    );
    assert_eq!(
        suite.fixtures.len(),
        declared.len(),
        "primitive source entries must be unique"
    );
}

#[test]
fn primitive_oracle_provenance_is_current() {
    let root = fixture_root();
    let suite = suite();
    let manifest: BakeManifest = read_json(&root.join("oracle-bake.json"));
    assert_eq!(manifest.schema_version, 1, "unsupported bake schema");
    assert_eq!(manifest.suite, "primitives.json");
    assert_eq!(
        manifest.suite_sha256,
        sha256_file(&root.join("primitives.json")),
        "primitive suite changed without rebaking Chromium provenance"
    );
    assert_eq!(
        manifest.bake_script_sha256,
        sha256_file(&root.join("bake_chromium.ts")),
        "Chromium baker changed without refreshing provenance"
    );
    assert_eq!(manifest.fixtures.len(), suite.fixtures.len());

    for (fixture, record) in suite.fixtures.iter().zip(&manifest.fixtures) {
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

#[test]
fn every_primitive_is_pixel_exact_to_chromium_and_deterministic() {
    let root = fixture_root();
    for fixture in suite().fixtures {
        let source = fs::read_to_string(root.join(&fixture.source))
            .unwrap_or_else(|e| panic!("read {}: {e}", fixture.source));
        let frame = match fixture.entry.as_str() {
            "html-inline-svg" => compile_html_inline_svg(&source),
            // The declared fixture dimensions serve as the initial
            // viewport. Today every committed standalone fixture authors
            // explicit root dimensions, so this value is inert, and the
            // baker's element-box assertion (bake_chromium.ts, captureSvg)
            // pins the captured size to exactly these dims. Auto-sized
            // fixtures become bakeable when the corpus step sizes the
            // browser window per fixture.
            "standalone-svg" => compile_standalone_svg(
                &source,
                InitialViewport::new(fixture.width as f32, fixture.height as f32),
            ),
            other => panic!("{} has unknown entry {other:?}", fixture.id),
        }
        .unwrap_or_else(|e| panic!("compile {}: {e}", fixture.id));
        assert_eq!(
            (frame.bounds.width, frame.bounds.height),
            (fixture.width as f32, fixture.height as f32),
            "{} resolved viewport",
            fixture.id
        );

        let actual = render_through_n0(&frame, fixture.width, fixture.height);
        let oracle_bytes = fs::read(root.join(&fixture.oracle))
            .unwrap_or_else(|e| panic!("read {}: {e}", fixture.oracle));
        let oracle = decode_png(&oracle_bytes)
            .unwrap_or_else(|| panic!("decode Chromium oracle for {}", fixture.id));
        assert_eq!(
            (oracle.width, oracle.height),
            (fixture.width, fixture.height),
            "{} oracle dimensions",
            fixture.id
        );

        let mut first_difference = None;
        let mut differing_pixels = 0usize;
        for (index, (actual_pixel, oracle_pixel)) in actual
            .chunks_exact(4)
            .zip(oracle.pixels.chunks_exact(4))
            .enumerate()
        {
            if actual_pixel != oracle_pixel {
                differing_pixels += 1;
                first_difference.get_or_insert((index, actual_pixel, oracle_pixel));
            }
        }
        assert_eq!(
            differing_pixels, 0,
            "{} has {differing_pixels} pixels differing from Chromium; first: {first_difference:?}",
            fixture.id
        );

        let second = render_through_n0(&frame, fixture.width, fixture.height);
        assert_eq!(
            actual, second,
            "{} n0 CPU raster must be byte-deterministic",
            fixture.id
        );
        let first_png = render_png_through_n0(&frame, fixture.width, fixture.height);
        let second_png = render_png_through_n0(&frame, fixture.width, fixture.height);
        assert!(
            first_png == second_png,
            "{} encoded PNG must be byte-deterministic",
            fixture.id
        );
    }
}

/// Formerly the both-downstreams byte-identity gate. Its replacement purpose
/// completed with the D-M vector join: the proving downstream is gone, and
/// the surviving laws are contract admission, geometry/clip coverage, and
/// byte determinism through the one n0 downstream.
#[test]
fn admitted_frame_contract_shapes_raster_deterministically_through_n0() {
    use cg::{CGColor, Paint, Paints, SolidPaint};
    use math2::transform::AffineTransform;

    let transparent = SolidPaintStack::try_from_paints(Paints::new([Paint::Solid(
        SolidPaint::new_color(CGColor::TRANSPARENT),
    )]))
    .expect("transparent paint normalizes to no visual paint");
    let translucent = SolidPaintStack::solid(CGColor::from_rgba(0x16, 0xa3, 0x4a, 0x80));
    let layered = SolidPaintStack::try_from_paints(Paints::new([
        Paint::Solid(SolidPaint::new_color(CGColor::from_rgba(
            0xef, 0x44, 0x44, 0x80,
        ))),
        Paint::Solid(SolidPaint::new_color(CGColor::from_rgba(
            0x25, 0x63, 0xeb, 0x80,
        ))),
    ]))
    .expect("ordinary solid stack");
    let transformed = AffineTransform::from_acebdf(2.0, 0.0, 4.0, 0.0, 1.0, 5.0);

    // Coverage probes derive from the contract alone: the node rect is
    // (2,3,8,6); the transformed case maps it by x' = 2x + 4, y' = y + 5 to
    // cover x ∈ [8,24), y ∈ [8,14) inside the 32×24 viewport clip.
    for (name, frame, coverage) in [
        (
            "empty",
            contract_frame(SolidPaintStack::empty(), AffineTransform::identity()),
            None,
        ),
        (
            "transparent",
            contract_frame(transparent, AffineTransform::identity()),
            None,
        ),
        (
            "translucent",
            contract_frame(translucent, AffineTransform::identity()),
            Some(((5, 5), (20, 20))),
        ),
        (
            "layered-transformed",
            contract_frame(layered, transformed),
            Some(((10, 10), (5, 5))),
        ),
    ] {
        let first = render_through_n0(&frame, 32, 24);
        let second = render_through_n0(&frame, 32, 24);
        assert_eq!(
            first, second,
            "{name} must raster byte-deterministically through n0"
        );
        match coverage {
            None => assert!(
                first.iter().all(|&byte| byte == 0),
                "{name} admits no visible paint and must raster fully transparent"
            ),
            Some(((inside_x, inside_y), (outside_x, outside_y))) => {
                assert_ne!(
                    pixel(&first, 32, inside_x, inside_y)[3],
                    0,
                    "{name} must paint its resolved geometry"
                );
                assert_eq!(
                    pixel(&first, 32, outside_x, outside_y),
                    [0, 0, 0, 0],
                    "{name} must leave uncovered viewport pixels clear"
                );
            }
        }
    }
}
