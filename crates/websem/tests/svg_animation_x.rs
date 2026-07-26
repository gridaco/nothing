//! Exact retained-source gates for the Web SVG animation slice.
//!
//! The independent Chromium baker owns timeline control and committed oracle
//! pixels. These tests consume only those artifacts; they never run the sealed
//! consolidation scoreboard or compute a similarity score.
//!
//! The slice itself is one `<animate>` on a top-level `<rect>`'s `x`. Two
//! fixtures exercise it: a bare rect on a backdrop, and `svg-scene-cub` — a
//! whole composition whose animated rect is one node among seventeen, so the
//! sampling path is gated over a scene rather than only over a single shape.

mod support;

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use animation_sampling::SampleTime;
use math2::Rectangle;
use rframe::{Frame, Geometry};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use support::{decode_png, render_through_n0};
use websem::{InitialViewport, SvgFrameSource};

/// The host-established initial viewport for this file's *inline* laws — the
/// ones that author their own source instead of reading the corpus. Every such
/// source authors explicit root dimensions, making this value inert, except
/// `sampled_overrides_stay_local_under_a_scaling_viewport`, which deliberately
/// leaves its height unauthored and takes 32 from here (auto -> 100% of the
/// initial viewport), so its expected transform depends on these values. Corpus
/// fixtures use their own declared dims (see [`Fixture::viewport`]); the
/// viewport laws proper live in `viewport_contract.rs`.
fn host_viewport() -> InitialViewport {
    InitialViewport::new(64.0, 32.0)
}

#[derive(Debug, Deserialize)]
struct Suite {
    schema_version: u32,
    fixtures: Vec<Fixture>,
}

#[derive(Debug, Deserialize)]
struct Fixture {
    id: String,
    width: i32,
    height: i32,
    authored_base_x: f32,
    frame: FrameShape,
    base: BaseCase,
    animation: AnimationCase,
}

impl Fixture {
    /// The declared dims are the initial viewport the fixture was baked at —
    /// the browser window in `bake_chromium.ts`. A fixture that leaves the root
    /// `width`/`height` unauthored (the cub does) resolves them from this, so
    /// passing anything else would render a differently-sized scene than the
    /// oracle.
    fn viewport(&self) -> InitialViewport {
        InitialViewport::new(self.width as f32, self.height as f32)
    }
}

/// The resolved frame each fixture must compile to.
///
/// Declared rather than discovered: a fixture that silently stops
/// materializing an element, or that materializes its animated rect at another
/// index, fails here instead of quietly weakening every law that reads the
/// animated node.
#[derive(Debug, Deserialize)]
struct FrameShape {
    node_count: usize,
    animated_node_index: usize,
}

#[derive(Debug, Deserialize)]
struct BaseCase {
    source: String,
    oracle: String,
    expected_x: f32,
}

#[derive(Debug, Deserialize)]
struct AnimationCase {
    source: String,
    samples: Vec<SampleCase>,
    retained_seek_order_ns: Vec<i64>,
}

#[derive(Debug, Deserialize)]
struct SampleCase {
    time_ns: i64,
    oracle: String,
    expected_x: f32,
}

#[derive(Debug, Deserialize)]
struct BakeManifest {
    schema_version: u32,
    kind: String,
    bake_script_sha256: String,
    suite: String,
    suite_sha256: String,
    capture: CapturePolicy,
    fixtures: Vec<BakeFixture>,
}

#[derive(Debug, Deserialize)]
struct CapturePolicy {
    viewport: String,
    device_scale_factor: u32,
    network: String,
    timeline_control: String,
    comparison: String,
    fresh_captures_per_case: u32,
}

#[derive(Debug, Deserialize)]
struct BakeFixture {
    id: String,
    width: i32,
    height: i32,
    authored_base_x: f32,
    retained_seek_order_ns: Vec<i64>,
    retained_seek_count: usize,
    cases: Vec<BakeRecord>,
}

#[derive(Debug, Deserialize)]
struct BakeRecord {
    id: String,
    policy: String,
    time_ns: Option<i64>,
    source: String,
    source_sha256: String,
    oracle: String,
    oracle_sha256: String,
    rgba_sha256: String,
    expected_x: f32,
    observed: Observation,
    fresh_capture_count: u32,
}

#[derive(Debug, Deserialize)]
struct Observation {
    base_x: f32,
    anim_x: f32,
    bbox_x: f32,
    current_time_seconds: f64,
    animation_element_count: usize,
    animations_paused: bool,
}

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/web-first/animation")
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> T {
    let bytes = fs::read(path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    serde_json::from_slice(&bytes)
        .unwrap_or_else(|error| panic!("parse {}: {error}", path.display()))
}

fn suite() -> Suite {
    read_json(&fixture_root().join("cases.json"))
}

fn sha256_file(path: &Path) -> String {
    let bytes = fs::read(path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    format!("{:x}", Sha256::digest(bytes))
}

fn rgba_sha256_file(path: &Path) -> String {
    let png = decode_png(
        &fs::read(path).unwrap_or_else(|error| panic!("read {}: {error}", path.display())),
    )
    .unwrap_or_else(|| panic!("decode {}", path.display()));
    format!("{:x}", Sha256::digest(png.pixels))
}

/// The `x` of the fixture's animated node, which the suite declares by index.
fn probe_x(frame: &Frame, fixture: &Fixture) -> f32 {
    assert_eq!(
        frame.nodes.len(),
        fixture.frame.node_count,
        "{} materializes exactly its declared node count",
        fixture.id
    );
    let node = frame
        .nodes
        .get(fixture.frame.animated_node_index)
        .unwrap_or_else(|| panic!("{} declares an out-of-range animated node", fixture.id));
    let Geometry::Rect(rect) = &node.geometry else {
        panic!("{}: only a rect's x is animated in this slice", fixture.id);
    };
    rect.x
}

fn assert_exact_oracle(frame: Frame, oracle: &Path, width: i32, height: i32, label: &str) {
    let actual = render_through_n0(&frame, width, height);
    let expected = decode_png(
        &fs::read(oracle).unwrap_or_else(|error| panic!("read {}: {error}", oracle.display())),
    )
    .unwrap_or_else(|| panic!("decode Chromium oracle {}", oracle.display()));
    assert_eq!(
        (expected.width, expected.height),
        (width, height),
        "{label}"
    );

    let first = actual
        .chunks_exact(4)
        .zip(expected.pixels.chunks_exact(4))
        .enumerate()
        .find(|(_, (actual, expected))| actual != expected);
    assert!(
        first.is_none(),
        "{label} differs from Chromium at {first:?}"
    );
}

#[test]
fn chromium_animation_oracle_provenance_is_current() {
    let root = fixture_root();
    let suite = suite();
    assert_eq!(suite.schema_version, 1);
    let manifest: BakeManifest = read_json(&root.join("oracle-bake.json"));
    assert_eq!(manifest.schema_version, 1);
    assert_eq!(manifest.kind, "chromium-svg-animation-oracle");
    assert_eq!(manifest.suite, "cases.json");
    assert_eq!(manifest.suite_sha256, sha256_file(&root.join("cases.json")));
    assert_eq!(
        manifest.bake_script_sha256,
        sha256_file(&root.join("bake_chromium.ts"))
    );
    assert_eq!(
        manifest.capture.viewport,
        "per-fixture declared dims (the initial viewport)"
    );
    assert_eq!(manifest.capture.device_scale_factor, 1);
    assert_eq!(
        manifest.capture.network,
        "all http(s) requests aborted; zero attempted"
    );
    assert_eq!(
        manifest.capture.timeline_control,
        "pauseAnimations() then setCurrentTime(ns / 1e9)"
    );
    assert_eq!(
        manifest.capture.comparison,
        "exact decoded RGBA; fresh PNG encodings also byte-identical"
    );
    assert_eq!(manifest.capture.fresh_captures_per_case, 2);
    assert_eq!(manifest.fixtures.len(), suite.fixtures.len());

    let mut oracle_paths = BTreeSet::new();
    for (fixture, baked) in suite.fixtures.iter().zip(&manifest.fixtures) {
        assert_eq!(baked.id, fixture.id, "fixture order");
        assert_eq!((baked.width, baked.height), (fixture.width, fixture.height));
        assert_eq!(baked.authored_base_x, fixture.authored_base_x);
        assert_eq!(
            baked.retained_seek_order_ns,
            fixture.animation.retained_seek_order_ns
        );
        assert_eq!(
            baked.retained_seek_count,
            fixture.animation.retained_seek_order_ns.len()
        );
        assert_eq!(baked.cases.len(), 1 + fixture.animation.samples.len());

        let mut record_ids = BTreeSet::new();
        let mut sample_times = BTreeSet::new();
        let mut saw_base = false;
        for record in &baked.cases {
            assert!(
                record_ids.insert(record.id.as_str()),
                "duplicate record id in {}",
                fixture.id
            );
            assert!(
                oracle_paths.insert(record.oracle.as_str()),
                "two records claim oracle {}",
                record.oracle
            );
            assert_eq!(record.fresh_capture_count, 2);
            assert!(record.observed.animations_paused);
            assert_eq!(
                record.source_sha256,
                sha256_file(&root.join(&record.source)),
                "{} source provenance",
                record.id
            );
            assert_eq!(
                record.oracle_sha256,
                sha256_file(&root.join(&record.oracle)),
                "{} oracle provenance",
                record.id
            );
            assert_eq!(
                record.rgba_sha256,
                rgba_sha256_file(&root.join(&record.oracle)),
                "{} decoded RGBA provenance",
                record.id
            );
            match record.time_ns {
                None => {
                    assert!(!saw_base, "duplicate Base record in {}", fixture.id);
                    saw_base = true;
                    assert_eq!(record.id, format!("{}/base", fixture.id));
                    assert_eq!(record.policy, "base-static-projection");
                    assert_eq!(record.source, fixture.base.source);
                    assert_eq!(record.oracle, fixture.base.oracle);
                    assert_eq!(record.expected_x, fixture.base.expected_x);
                    assert_eq!(record.observed.base_x, fixture.authored_base_x);
                    assert_eq!(record.observed.anim_x, fixture.base.expected_x);
                    assert_eq!(record.observed.bbox_x, fixture.base.expected_x);
                    assert_eq!(record.observed.current_time_seconds, 0.0);
                    assert_eq!(record.observed.animation_element_count, 0);
                }
                Some(time) => {
                    assert!(
                        sample_times.insert(time),
                        "duplicate sample {time}ns in {}",
                        fixture.id
                    );
                    assert_eq!(record.id, format!("{}/sample-{time}ns", fixture.id));
                    assert_eq!(record.policy, "sample");
                    let sample = fixture
                        .animation
                        .samples
                        .iter()
                        .find(|sample| sample.time_ns == time)
                        .unwrap_or_else(|| {
                            panic!("{} manifest has undeclared sample {time}ns", fixture.id)
                        });
                    assert_eq!(record.source, fixture.animation.source);
                    assert_eq!(record.oracle, sample.oracle);
                    assert_eq!(record.expected_x, sample.expected_x);
                    assert_eq!(record.observed.base_x, fixture.authored_base_x);
                    assert_eq!(record.observed.anim_x, sample.expected_x);
                    assert_eq!(record.observed.bbox_x, sample.expected_x);
                    assert_eq!(
                        record.observed.current_time_seconds,
                        time as f64 / 1_000_000_000.0
                    );
                    assert_eq!(record.observed.animation_element_count, 1);
                }
            }
        }
        assert!(saw_base, "{} is missing its Base record", fixture.id);
        assert_eq!(
            sample_times,
            fixture
                .animation
                .samples
                .iter()
                .map(|sample| sample.time_ns)
                .collect(),
            "{} must cover every declared sample exactly once",
            fixture.id
        );
    }
}

#[test]
fn retained_samples_are_exact_to_chromium_and_seek_order_independent() {
    let root = fixture_root();
    for fixture in suite().fixtures {
        let id = &fixture.id;
        let animated_text = fs::read_to_string(root.join(&fixture.animation.source))
            .unwrap_or_else(|error| panic!("read {} animated source: {error}", id));
        let static_text = fs::read_to_string(root.join(&fixture.base.source))
            .unwrap_or_else(|error| panic!("read {} Base source: {error}", id));
        let source = SvgFrameSource::from_standalone_svg(animated_text.clone(), fixture.viewport())
            .unwrap_or_else(|error| panic!("retain {id} animated: {error}"));
        let static_source = SvgFrameSource::from_standalone_svg(static_text, fixture.viewport())
            .unwrap_or_else(|error| panic!("retain {id} Base: {error}"));

        assert_eq!(source.source(), animated_text);
        assert!(source.has_animation_elements(), "{id} carries an animation");
        assert!(
            !static_source.has_animation_elements(),
            "{id} Base projection carries none"
        );
        let base = source.base_frame();
        assert_eq!(
            base,
            static_source.base_frame(),
            "{id}: the Base view IS the static projection"
        );
        assert_eq!(probe_x(&base, &fixture), fixture.authored_base_x);
        assert_eq!(probe_x(&base, &fixture), fixture.base.expected_x);
        assert_exact_oracle(
            base.clone(),
            &root.join(&fixture.base.oracle),
            fixture.width,
            fixture.height,
            &format!("{id} Base"),
        );

        let pre_roll = source
            .sample_frame(SampleTime::from_nanoseconds(-1))
            .unwrap_or_else(|error| panic!("{id} pre-roll: {error}"));
        assert_eq!(
            pre_roll, base,
            "{id} pre-roll reveals the authored Base value"
        );

        for sample in &fixture.animation.samples {
            let first = source
                .sample_frame(SampleTime::from_nanoseconds(sample.time_ns))
                .unwrap_or_else(|error| panic!("{id} sample {}ns: {error}", sample.time_ns));
            let second = source
                .sample_frame(SampleTime::from_nanoseconds(sample.time_ns))
                .unwrap_or_else(|error| panic!("{id} repeat sample {}ns: {error}", sample.time_ns));
            assert_eq!(first, second, "{id} {}ns frame determinism", sample.time_ns);
            assert_eq!(
                probe_x(&first, &fixture),
                sample.expected_x,
                "{id} {}ns x",
                sample.time_ns
            );
            let animated = fixture.frame.animated_node_index;
            assert_eq!(
                first.nodes[animated].owner, base.nodes[animated].owner,
                "{id} {}ns stable visual identity",
                sample.time_ns
            );
            assert_exact_oracle(
                first,
                &root.join(&sample.oracle),
                fixture.width,
                fixture.height,
                &format!("{id} Sample({}ns)", sample.time_ns),
            );
        }

        assert_ne!(
            source
                .sample_frame(SampleTime::ZERO)
                .unwrap_or_else(|error| panic!("{id} zero sample: {error}")),
            base,
            "{id}: Base is not shorthand for Sample(0)"
        );
        for &time in &fixture.animation.retained_seek_order_ns {
            let frame = source
                .sample_frame(SampleTime::from_nanoseconds(time))
                .unwrap_or_else(|error| panic!("{id} shuffled seek {time}ns: {error}"));
            let expected = fixture
                .animation
                .samples
                .iter()
                .find(|sample| sample.time_ns == time)
                .expect("seek order is closed over declared samples");
            assert_eq!(probe_x(&frame, &fixture), expected.expected_x);
        }
        assert_eq!(
            source.base_frame(),
            base,
            "{id}: sampling never mutates retained authored state"
        );
        assert_eq!(source.source(), animated_text);
    }
}

/// Damage from a sample is the animated node and nothing else, over the union
/// of where it was and where it went — including in a seventeen-node scene,
/// where every other node must stay untouched.
#[test]
fn chassis_damage_observes_the_same_stable_animated_visual() {
    let root = fixture_root();
    for fixture in suite().fixtures {
        let id = &fixture.id;
        let source = SvgFrameSource::from_standalone_svg(
            fs::read_to_string(root.join(&fixture.animation.source))
                .unwrap_or_else(|error| panic!("read {id}: {error}")),
            fixture.viewport(),
        )
        .unwrap_or_else(|error| panic!("retain {id}: {error}"));
        let base = source.base_frame();
        let midpoint = source
            .sample_frame(SampleTime::from_nanoseconds(1_000_000_000))
            .unwrap_or_else(|error| panic!("{id} midpoint sample: {error}"));
        let animated = fixture.frame.animated_node_index;
        let owner = base.nodes[animated].owner;
        let expected_union =
            math2::union(&[base.nodes[animated].bounds, midpoint.nodes[animated].bounds]);

        let before = n0::glyphless::compile(base).expect("compile Base product");
        let after = n0::glyphless::compile(midpoint).expect("compile sample product");
        let damage = n0::glyphless::diff_frame(&before, &after);
        assert_eq!(damage.changed, vec![owner], "{id} changed exactly one node");
        assert_eq!(
            damage.union_frame,
            Some(expected_union),
            "{id} damage covers both positions"
        );
        assert!(n0::glyphless::diff_frame(&after, &after).is_empty());

        // The rect-x fixture's damage rectangle is also pinned by hand: the
        // union above is derived from the same frames the diff came from, so a
        // bounds regression that moved both would satisfy it.
        if id == "svg-rect-x-animation" {
            assert_eq!(expected_union, Rectangle::from_xywh(4.0, 8.0, 36.0, 16.0));
        }
    }
}

#[test]
fn unsupported_animation_is_retained_for_base_and_refused_for_sample() {
    const CASES: &[(&str, &str)] = &[
        (
            r#"<animate attributeName="y" from="1" to="2" dur="1s" fill="freeze"/>"#,
            "only attributeName=\"x\"",
        ),
        (
            r#"<animate attributeName="x" from="1" to="2" begin="0s" dur="1s" fill="freeze"/>"#,
            "attribute \"begin\"",
        ),
        (
            r#"<animate attributeName="x" from="1" to="2" dur="1s"/>"#,
            "required attribute \"fill\"",
        ),
        (
            r#"<animateTransform attributeName="transform" from="0" to="1" dur="1s"/>"#,
            "animation element <animateTransform>",
        ),
        (
            r##"<animateColor attributeName="fill" from="#000" to="#fff" dur="1s"/>"##,
            "animation element <animateColor>",
        ),
        (
            r#"<animate attributeName="x" from="1" to="2" dur="1.5s" fill="freeze"/>"#,
            "positive integer in ms or s",
        ),
    ];

    for (markup, expected) in CASES {
        let svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16">
                 <rect x="4" y="4" width="4" height="4">{markup}</rect>
               </svg>"#
        );
        let source =
            SvgFrameSource::from_standalone_svg(svg, host_viewport()).expect("Base materializes");
        assert!(source.has_animation_elements());
        assert_eq!(probe_single_x(&source.base_frame()), 4.0);
        let error = source
            .sample_frame(SampleTime::ZERO)
            .expect_err("unsupported animation must refuse the whole sample");
        assert!(
            error.reason().contains(expected),
            "expected {expected:?}, got {error}"
        );
        assert!(
            matches!(
                error.path(),
                "svg/rect[1]/animate[1]"
                    | "svg/rect[1]/animateTransform[1]"
                    | "svg/rect[1]/animateColor[1]"
            ),
            "unexpected structural path: {}",
            error.path()
        );
    }
}

#[test]
fn sampling_refuses_dynamic_side_channels_and_unclosed_inline_html() {
    for (rect_attributes, child, expected) in [
        (
            r#"onclick="window.changed = true""#,
            "",
            "event-handler attribute",
        ),
        (r#"style="animation: spin 1s""#, "", "style attribute"),
        (
            "",
            "<style>@keyframes spin { to { opacity: 0 } }</style>",
            "<style>",
        ),
    ] {
        let svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16">
                 <rect x="4" y="4" width="4" height="4" {rect_attributes}>{child}</rect>
               </svg>"#
        );
        let source =
            SvgFrameSource::from_standalone_svg(svg, host_viewport()).expect("Base materializes");
        let error = source
            .sample_frame(SampleTime::ZERO)
            .expect_err("dynamic side channel must refuse Sample");
        assert!(
            error.reason().contains(expected),
            "expected {expected:?}, got {error}"
        );
    }

    // `<script>` in a standalone document refuses at construction, in both
    // admissions: xml5ever suspends the XML parse at the element, so
    // content after it would be silently absent — an undeclared hole.
    let script_svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16">
         <rect x="4" y="4" width="4" height="4"><script>window.changed = true</script></rect>
       </svg>"#;
    for (label, result) in [
        (
            "strict",
            SvgFrameSource::from_standalone_svg(script_svg, host_viewport()),
        ),
        (
            "best-effort",
            SvgFrameSource::from_standalone_svg_best_effort(script_svg, host_viewport()),
        ),
    ] {
        let error = result.expect_err("script-bearing standalone document must refuse");
        assert!(
            matches!(error, websem::CompileError::ScriptSuspendsParse),
            "{label}: expected ScriptSuspendsParse, got {error}"
        );
    }

    let html = r#"<html><body><svg width="16" height="16">
                    <rect x="4" y="4" width="4" height="4"/>
                  </svg></body></html>"#;
    let inline = SvgFrameSource::from_html_inline_svg(html).expect("inline Base materializes");
    assert!(!inline.has_animation_elements());
    let error = inline
        .sample_frame(SampleTime::ZERO)
        .expect_err("inline Sample waits for document-wide inventory");
    assert_eq!(error.path(), "document");
    assert!(
        error
            .reason()
            .contains("document-wide CSS and script inventory")
    );
}

#[test]
fn static_sample_equals_base_and_web_timing_observes_final_boundary() {
    let static_source = SvgFrameSource::from_standalone_svg(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16">
             <rect x="4" y="4" width="4" height="4"/>
           </svg>"#,
        host_viewport(),
    )
    .expect("retain static SVG");
    assert_eq!(
        static_source
            .sample_frame(SampleTime::from_nanoseconds(i64::MIN))
            .expect("static pre-roll"),
        static_source.base_frame()
    );
    assert_eq!(
        static_source
            .sample_frame(SampleTime::from_nanoseconds(i64::MAX))
            .expect("static late sample"),
        static_source.base_frame()
    );

    let animated = SvgFrameSource::from_standalone_svg(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16">
             <rect x="4" y="4" width="4" height="4">
               <animate attributeName="x" from="0" to="1" dur="1ms" fill="freeze"/>
             </rect>
           </svg>"#,
        host_viewport(),
    )
    .expect("retain boundary SVG");
    let before = probe_single_x(
        &animated
            .sample_frame(SampleTime::from_nanoseconds(999_999))
            .expect("one nanosecond before end"),
    );
    let at = probe_single_x(
        &animated
            .sample_frame(SampleTime::from_nanoseconds(1_000_000))
            .expect("exact end"),
    );
    let after = probe_single_x(
        &animated
            .sample_frame(SampleTime::from_nanoseconds(1_000_001))
            .expect("one nanosecond after end"),
    );
    assert!(before < 1.0, "active sample immediately before final end");
    assert_eq!(at, 1.0, "freeze observes exact terminal endpoint");
    assert_eq!(after, 1.0, "freeze remains terminal after active end");
}

/// An admitted `x` animation composes with a non-identity root viewport:
/// the sampled override lands in local (viewBox user-unit) space before
/// the viewport transform, so a sampled node keeps Base's transform and
/// its geometry carries the sampled local value. No Chromium bake — this
/// pins the composition order, not pixels. (The source also exercises
/// mixed sizing: authored width, auto height from the initial viewport.)
#[test]
fn sampled_overrides_stay_local_under_a_scaling_viewport() {
    let animated = SvgFrameSource::from_standalone_svg(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="64" viewBox="0 0 32 16">
             <rect x="4" y="4" width="4" height="4">
               <animate attributeName="x" from="4" to="12" dur="2s" fill="freeze"/>
             </rect>
           </svg>"#,
        // 64x32: width authored, height auto -> 32, so sx = sy = 2.
        host_viewport(),
    )
    .expect("retain scaled animated SVG");
    let viewport = math2::transform::AffineTransform::from_acebdf(2.0, 0.0, 0.0, 0.0, 2.0, 0.0);
    let base = animated.base_frame();
    assert_eq!(base.bounds, Rectangle::from_xywh(0.0, 0.0, 64.0, 32.0));
    assert_eq!(base.nodes[0].transform, viewport);

    let sampled = animated
        .sample_frame(SampleTime::from_nanoseconds(1_000_000_000))
        .expect("midpoint sample");
    assert_eq!(
        sampled.nodes[0].transform, viewport,
        "time never touches the viewport mapping"
    );
    let Geometry::Rect(rect) = &sampled.nodes[0].geometry else {
        panic!("these animation fixtures materialize rects only");
    };
    assert_eq!(rect.x, 8.0, "the sampled override is local, pre-transform");
    assert_eq!(
        sampled.nodes[0].bounds,
        math2::rect_transform(*rect, &viewport),
        "bounds follow the one transform"
    );
}

fn probe_single_x(frame: &Frame) -> f32 {
    let Geometry::Rect(rect) = &frame.nodes[0].geometry else {
        panic!("these animation fixtures materialize rects only");
    };
    rect.x
}
