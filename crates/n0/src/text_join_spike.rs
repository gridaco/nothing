//! The D-M text-stage deciding spike: two real shaped-text producers pushed
//! at one candidate neutral shaped-text + font-key contract.
//!
//! The [join-point finding](../../../docs/wg/consolidation/n0-join-point.md)
//! gated the text stage on this exact experiment: when the Web family gains a
//! real shaped-text producer, push a text run from *both* it and n0 toward a
//! candidate neutral contract and observe whether a neutral font-key boundary
//! holds (text joins high) or forces the boundary down (text joins low).
//! `textlayout` is that second producer now, so this module runs the
//! experiment. Like [`super::vector_join_spike`], it exists only in `n0`'s
//! unit-test build, proposes no public contract, and survives as the witness
//! behind the taken decision, not as a proposal.
//!
//! What the arms measured:
//!
//! - **Glyph identity joins; placement and metrics are the scaler's.** Both
//!   shapers select identical glyphs from the same declared face on every
//!   platform (the content key matches by construction — n0's artifact
//!   cannot state it, so the host's declaration is joined back in). The Web
//!   producer's pen positions, advance, and metrics are exact font-unit
//!   arithmetic everywhere. n0's pass through its scaler backend, and
//!   *which* of them arrives exact is itself platform-dependent: the
//!   CoreText build states exact pen x with 2⁻¹⁴-quantized metrics; the
//!   FreeType build states exact metrics with each em advance one 2⁻¹⁶
//!   step short. One producer's facts are not even self-consistent across
//!   platforms — a neutral contract would have to legislate the
//!   derivation, rebuilding n0's oracle output, or carry one fact with two
//!   meanings.
//! - **Outlines are meaning; glyph replay is policy.** Two independent
//!   outline extractions of the same candidate fact (Skia `get_path` from
//!   environment bytes; the artifact's own ttf-parser stream) agree
//!   byte-identically through the one shared path fill at every probed
//!   anchor, on and off the integer lattice, and are bilevel on it — the
//!   admitted-domain law. n0's glyph replay through its oracle's live
//!   `Font` (hinting, subpixel, edging riding the instance) paints a
//!   measured non-bilevel fringe against those same outlines *on the
//!   lattice itself* — where the external oracle gates byte-exactly; a
//!   control with policy stripped (alias, unhinted, no subpixel) matches
//!   the outlines byte-exactly, attributing the fringe to the anti-alias
//!   mask policy. A contract that carried the fact but not the policy would
//!   make that fringe a silent wrong pixel; carrying the policy is the
//!   one-meaning tripwire.
//! - **The fact is a resource reference.** Realizing the candidate requires
//!   a declared digest→bytes environment and gains a lookup refusal — the
//!   exact boundary `rframe` refuses ("no fact that references a resource")
//!   and the `glyphless` route is named for. n0's own artifact carries
//!   process-local identity; the content digest never flows through its
//!   oracle and had to be joined back from the host's declaration.
//! - **Beyond the overlap the second producer still does not exist.** Line
//!   structure is a typed refusal in the Web family's v0 profile, and
//!   styled runs and variable instances are not even expressible in its
//!   input surface — everything a high join would exist to carry is still
//!   a one-producer fact, and one producer decides nothing.
//!
//! The decision taken on this evidence is recorded in the join-point finding
//! and the charter registry: **shaped text joins low**. Each engine keeps its
//! own text artifact, font registry, and glyph replay policy; sharing stops at
//! backend glyph/raster utilities, and content-digest font identity remains a
//! host-seam convention (the CLI `--font` surface, `textlayout::FontKey`, the
//! bake manifests), never a contract fact.

use std::collections::BTreeMap;
use std::sync::Arc;

use n0_model::model::{TextPayloadRef, TextStyleRec};
use n0_model::text_layout::{TextLayout, TextLayoutOracle};
use sha2::{Digest, Sha256};
use skia_safe::font::Edging;
use skia_safe::{
    surfaces, Canvas, Color as SkColor, Font, FontHinting, FontMgr, Matrix, Paint as SkPaint,
    PathBuilder, Point,
};
use textlayout::OutlineSink;

use crate::paint::{read_pixels, PaintCtx};
use crate::text_layout::SkiaTextLayoutOracle;

/// The pinned gate font, by exact bytes — the same identity the text cell
/// suite declares to Chromium and the CLI declares with `--font`.
const AHEM: &[u8] = include_bytes!("../../../fixtures/web-first/fonts/ahem.ttf");

/// The overlapping run both producers are pushed with: printable ASCII with
/// an interior space, one style, one face — the whole slice where two real
/// producers exist today.
const RUN_TEXT: &str = "AB C";
const FONT_SIZE: f32 = 20.0;

/// The text cells' posture: a 100×100 cell, anchor on the integer lattice.
const CELL: i32 = 100;
const LATTICE_ANCHOR: (f32, f32) = (10.0, 30.0);
const FRACTIONAL_ANCHOR: (f32, f32) = (10.5, 30.25);

// --- declared per-platform measurements ----------------------------------
//
// The corpus's ramp-quantization protocol: a divergence is declared with
// the bounds measured on each platform, and a platform without a
// declaration fails its arm loudly with the value to declare. One CI
// round-trip maps a new platform.

/// The two declared platforms — exactly the builds the values below were
/// measured on. Any other target (an Intel mac included) is undeclared and
/// reports its own measurement like any new platform.
fn darwin_arm64() -> bool {
    cfg!(all(target_os = "macos", target_arch = "aarch64"))
}

fn linux_x86_64() -> bool {
    cfg!(all(target_os = "linux", target_arch = "x86_64"))
}

/// The scaler quantum n0's metric facts arrive in: ascent, descent, and
/// baseline each land one step of this off the exact font-unit value on
/// the declared platform. Darwin-arm64 (the CoreText scaler): 2⁻¹⁴, a
/// 16.16 fixed-point step. Linux-x86_64 (the FreeType scaler): exactly
/// zero — its metrics are exact while its *advances* are not (below),
/// which is the finding: which facts arrive exact is itself
/// platform-dependent. A platform whose divergence has a different *shape*
/// restructures the arm rather than forcing a false declaration through
/// this scalar.
fn declared_metric_quantum() -> Option<f32> {
    if darwin_arm64() {
        Some(1.0 / 16384.0)
    } else if linux_x86_64() {
        Some(0.0)
    } else {
        None
    }
}

/// The em advance n0's producer states for one Ahem glyph at the probed
/// size: exact (20px) from the CoreText scaler, one 2⁻¹⁶ step short from
/// the FreeType scaler. The Web producer states exactly 20px on every
/// platform — its arithmetic never passes through a scaler.
fn declared_n0_em_advance() -> Option<f32> {
    if darwin_arm64() {
        Some(FONT_SIZE)
    } else if linux_x86_64() {
        Some(FONT_SIZE - 1.0 / 65536.0)
    } else {
        None
    }
}

/// Bytes by which n0's glyph replay of the candidate run differs from its
/// policy-stripped outline realization, at the lattice and fractional
/// anchors. Darwin-arm64: a 432-byte non-bilevel fringe on the lattice,
/// 843 bytes off it. Linux is undeclared *on purpose*: its first round
/// measured 60/657 against a consumer that still carried FreeType hinting
/// in `get_path`; the consumer now strips policy, so Linux re-declares
/// against the corrected baseline through the normal loud round-trip.
fn declared_replay_divergence() -> Option<(usize, usize)> {
    darwin_arm64().then_some((432, 843))
}

// --- the candidate neutral contract -------------------------------------
//
// Deliberately spike-local. These types are what `rframe` would have to
// gain for a high join; the decision not to promote them is the point of
// this module.

/// Candidate neutral font identity: the digest of the exact face bytes, the
/// face index, and the normalized variation coordinates — content identity,
/// with no live object, no registry address, and no raster-facing flags
/// (edging, hinting, subpixel are replay *policy*, and a meaning-only
/// contract may not carry policy).
#[derive(Debug, Clone, PartialEq)]
struct CandidateFontKey {
    digest: [u8; 32],
    face_index: u32,
    variations: Vec<(u32, f32)>,
}

/// One glyph of the candidate fact: identifier in the keyed face plus the
/// pen origin relative to the run's anchor on the baseline, in local px.
#[derive(Debug, Clone, Copy, PartialEq)]
struct CandidateGlyph {
    id: u16,
    x: f32,
    y: f32,
}

/// The candidate neutral shaped-text fact: everything a downstream would
/// need to replay the run, and nothing owned by either producer.
#[derive(Debug, Clone, PartialEq)]
struct CandidateShapedRun {
    key: CandidateFontKey,
    font_size: f32,
    glyphs: Vec<CandidateGlyph>,
    advance: f32,
    ascent: f32,
    descent: f32,
}

impl CandidateShapedRun {
    /// The facts the shapers own: identity, selection, and placement along
    /// the pen axis. The metric facts (`ascent`, `descent`, glyph `y`) are
    /// deliberately excluded — the metric arm shows they arrive
    /// producer-tinted.
    fn shaping_facts(&self) -> (&CandidateFontKey, f32, Vec<(u16, f32)>, f32) {
        (
            &self.key,
            self.font_size,
            self.glyphs
                .iter()
                .map(|glyph| (glyph.id, glyph.x))
                .collect(),
            self.advance,
        )
    }
}

fn ahem_digest() -> [u8; 32] {
    Sha256::digest(AHEM).into()
}

// --- producer A: the Web family (`textlayout`) ---------------------------

fn web_environment() -> textlayout::Environment {
    textlayout::Environment::new(vec![textlayout::FontResource {
        key: textlayout::FontKey::new(ahem_digest()),
        family: "Ahem".to_string(),
        face_index: 0,
        bytes: Arc::from(AHEM),
    }])
}

fn web_layout(text: &str) -> textlayout::ResolvedTextLayout {
    textlayout::resolve(
        &textlayout::AttributedText {
            text: text.to_string(),
            style: textlayout::Style {
                family: "Ahem".to_string(),
                size: FONT_SIZE,
            },
        },
        &web_environment(),
    )
    .expect("the overlapping run is inside the v0 profile")
}

/// Project the Web family's artifact onto the candidate. The projection is
/// direct: `FontKey` *is* the content digest, the pen already sits on the
/// baseline, and oracle v0 admits no variation input.
fn candidate_from_web(layout: &textlayout::ResolvedTextLayout) -> CandidateShapedRun {
    CandidateShapedRun {
        key: CandidateFontKey {
            digest: ahem_digest(),
            face_index: layout.face().face_index,
            variations: Vec::new(),
        },
        font_size: layout.font_size(),
        glyphs: layout
            .glyphs()
            .iter()
            .map(|glyph| CandidateGlyph {
                id: glyph.glyph_id,
                x: glyph.x,
                y: 0.0,
            })
            .collect(),
        advance: layout.advance(),
        ascent: layout.metrics().ascent,
        descent: layout.metrics().descent,
    }
}

// --- producer B: n0 (`SkiaTextLayoutOracle`) -----------------------------

fn ahem_paint_ctx() -> PaintCtx {
    let typeface = FontMgr::new()
        .new_from_data(AHEM, None)
        .expect("pinned Ahem bytes parse as a typeface");
    PaintCtx::new(Some(typeface))
}

fn n0_layout(ctx: &PaintCtx, text: &str) -> (Arc<TextLayout>, Font) {
    let oracle = SkiaTextLayoutOracle::new(ctx);
    let layout = oracle.layout(
        TextPayloadRef {
            text,
            default_style: TextStyleRec::from_font_size(FONT_SIZE),
            runs: None,
        },
        None,
    );
    let registry = oracle.font_registry();
    let font = registry.font(
        layout
            .glyph_runs
            .first()
            .expect("the overlapping run shapes glyphs")
            .font,
    );
    (layout, font)
}

/// Project n0's artifact onto the candidate. Two facts of this projection
/// are themselves evidence:
///
/// - The digest is *not read from the artifact* — n0's oracle output carries
///   only process-local identity, so content identity must be joined back
///   from the host's own declaration, which is possible only because
///   fallback is disabled and the declared environment is total.
/// - Glyph y positions are stated relative to the paragraph box; the
///   candidate wants baseline-relative placement, so the line's baseline is
///   subtracted — a lossless projection for the overlapping single line.
fn candidate_from_n0(
    layout: &TextLayout,
    oracle_font: &Font,
    digest: [u8; 32],
) -> CandidateShapedRun {
    assert_eq!(
        layout.lines.len(),
        1,
        "the overlapping run resolves to one line"
    );
    assert_eq!(
        layout.glyph_runs.len(),
        1,
        "the overlapping run resolves to one glyph run"
    );
    let line = &layout.lines[0];
    let run = &layout.glyph_runs[0];

    let mut variations = oracle_font
        .typeface()
        .variation_design_position()
        .unwrap_or_default()
        .iter()
        .map(|coordinate| (*coordinate.axis, coordinate.value))
        .collect::<Vec<_>>();
    variations.sort_by_key(|(axis, _)| *axis);

    CandidateShapedRun {
        key: CandidateFontKey {
            digest,
            face_index: 0,
            variations,
        },
        font_size: FONT_SIZE,
        glyphs: run
            .glyphs
            .iter()
            .map(|glyph| CandidateGlyph {
                id: glyph.id,
                x: glyph.x,
                y: glyph.y - line.baseline,
            })
            .collect(),
        advance: layout.width,
        ascent: line.ascent,
        descent: line.descent,
    }
}

// --- realizations of one candidate fact ----------------------------------

fn black_paint() -> SkPaint {
    let mut paint = SkPaint::default();
    paint.set_anti_alias(true);
    paint.set_color(SkColor::BLACK);
    paint
}

fn raster<F: FnOnce(&Canvas)>(draw: F) -> Vec<u8> {
    let mut surface = surfaces::raster_n32_premul((CELL, CELL)).expect("raster surface");
    let canvas = surface.canvas();
    canvas.clear(SkColor::WHITE);
    draw(canvas);
    read_pixels(&mut surface, CELL, CELL)
}

/// n0's replay policy: the oracle's exact live `Font` — raster-facing flags
/// and all — through the glyph pipeline, as `ItemKind::TextFill` replays it.
fn realize_glyph_replay(run: &CandidateShapedRun, font: &Font, anchor: (f32, f32)) -> Vec<u8> {
    let ids: Vec<u16> = run.glyphs.iter().map(|glyph| glyph.id).collect();
    let positions: Vec<Point> = run
        .glyphs
        .iter()
        .map(|glyph| Point::new(anchor.0 + glyph.x, anchor.1 + glyph.y))
        .collect();
    raster(|canvas| {
        canvas.draw_glyphs_at(
            ids.as_slice(),
            positions.as_slice(),
            Point::new(0.0, 0.0),
            font,
            &black_paint(),
        );
    })
}

/// A candidate consumer: everything it holds it got from the fact plus a
/// declared digest→bytes environment. The signature is the structural
/// evidence — realizing the candidate *requires* this environment parameter
/// and gains this refusal, which is the exact resource boundary `rframe`
/// refuses to express and the `glyphless` route is named for.
fn realize_candidate_outlines(
    run: &CandidateShapedRun,
    environment: &BTreeMap<[u8; 32], Vec<u8>>,
    anchor: (f32, f32),
) -> Result<Vec<u8>, String> {
    let bytes = environment.get(&run.key.digest).ok_or_else(|| {
        format!(
            "font sha256:{} is not in the declared environment",
            run.key
                .digest
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        )
    })?;
    let typeface = FontMgr::new()
        .new_from_data(bytes, run.key.face_index as usize)
        .ok_or("declared bytes do not parse as a face")?;
    let mut font = Font::new(typeface, run.font_size);
    // A backend `Font` is policy-tinted by default even for *outline
    // extraction*: the FreeType build hints `get_path` unless told not to
    // (measured — 60 differing bytes on the lattice in the first Linux
    // round). A consumer realizing meaning strips raster policy explicitly.
    font.set_hinting(FontHinting::None);
    let mut builder = PathBuilder::new();
    for glyph in &run.glyphs {
        if let Some(path) = font.get_path(glyph.id) {
            let path =
                path.make_transform(&Matrix::translate((anchor.0 + glyph.x, anchor.1 + glyph.y)));
            builder.add_path(&path, None);
        }
    }
    let path = builder.snapshot();
    Ok(raster(|canvas| {
        canvas.draw_path(&path, &black_paint());
    }))
}

/// The Web family's realization policy: the resolved artifact streams its
/// own outlines (the one home of the y-flip) and the consumer fills paths —
/// the same lowering `websem` performs before `rframe`.
fn realize_web_outlines(layout: &textlayout::ResolvedTextLayout, anchor: (f32, f32)) -> Vec<u8> {
    struct SkiaSink {
        builder: PathBuilder,
        dx: f32,
        dy: f32,
    }
    impl OutlineSink for SkiaSink {
        fn move_to(&mut self, x: f32, y: f32) {
            self.builder.move_to(Point::new(self.dx + x, self.dy + y));
        }
        fn line_to(&mut self, x: f32, y: f32) {
            self.builder.line_to(Point::new(self.dx + x, self.dy + y));
        }
        fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
            self.builder.quad_to(
                Point::new(self.dx + x1, self.dy + y1),
                Point::new(self.dx + x, self.dy + y),
            );
        }
        fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
            self.builder.cubic_to(
                Point::new(self.dx + x1, self.dy + y1),
                Point::new(self.dx + x2, self.dy + y2),
                Point::new(self.dx + x, self.dy + y),
            );
        }
        fn close(&mut self) {
            self.builder.close();
        }
    }

    let mut sink = SkiaSink {
        builder: PathBuilder::new(),
        dx: anchor.0,
        dy: anchor.1,
    };
    for index in 0..layout.glyphs().len() {
        layout.outline(index, &mut sink);
    }
    let path = sink.builder.snapshot();
    raster(|canvas| {
        canvas.draw_path(&path, &black_paint());
    })
}

fn differing_bytes(a: &[u8], b: &[u8]) -> usize {
    assert_eq!(a.len(), b.len());
    a.iter()
        .zip(b)
        .filter(|(left, right)| left != right)
        .count()
}

fn non_bilevel_bytes(image: &[u8]) -> usize {
    image
        .iter()
        .filter(|&&byte| byte != 0 && byte != 255)
        .count()
}

// --- the arms ------------------------------------------------------------

/// The platform-invariant join is glyph *identity*: both shapers select
/// the same glyphs from the same declared face at the same size, on every
/// platform. Placement splits by producer: the Web producer's pen
/// positions and advance are exact font-unit arithmetic everywhere, while
/// n0's pass through its scaler backend and arrive per-platform (exact
/// from CoreText; one 2⁻¹⁶ step short per em from FreeType). One
/// producer's placement facts are not even self-consistent across
/// platforms — which is the sharpest form of the metric finding, and the
/// content key still matches by construction only (n0's side is joined
/// back from the host declaration — see the process-identity arm).
#[test]
fn glyph_identity_joins_and_placement_carries_the_scaler() {
    let web = candidate_from_web(&web_layout(RUN_TEXT));

    let ctx = ahem_paint_ctx();
    let (layout, font) = n0_layout(&ctx, RUN_TEXT);
    let n0 = candidate_from_n0(&layout, &font, ahem_digest());

    // Identity joins everywhere. The run is not degenerate evidence: four
    // glyphs, including the space, which advances without ink.
    assert_eq!(web.key, n0.key);
    assert_eq!(web.font_size, n0.font_size);
    let ids =
        |run: &CandidateShapedRun| run.glyphs.iter().map(|glyph| glyph.id).collect::<Vec<_>>();
    assert_eq!(ids(&web), ids(&n0));
    assert_eq!(web.glyphs.len(), 4);

    // The Web producer's placement is exact on every platform: no scaler
    // ever touches its arithmetic.
    for (index, glyph) in web.glyphs.iter().enumerate() {
        assert_eq!(glyph.x, index as f32 * FONT_SIZE);
    }
    assert_eq!(web.advance, 4.0 * FONT_SIZE);

    // n0's placement is its scaler backend's, declared per platform.
    let Some(step) = declared_n0_em_advance() else {
        panic!(
            "undeclared platform: measured n0 pen x {:?}, advance {:?} — declare this \
             platform's em advance",
            n0.glyphs.iter().map(|glyph| glyph.x).collect::<Vec<_>>(),
            n0.advance,
        );
    };
    let mut pen = 0.0f32;
    for glyph in &n0.glyphs {
        assert_eq!(glyph.x, pen);
        pen += step;
    }
    assert_eq!(n0.advance, pen);
}

/// The metric facts of the same resolution do not join: the Web family's
/// font-unit arithmetic is exact, and n0's paragraph backend hands each of
/// ascent, descent, and baseline back one scaler quantum off the exact
/// value. A neutral contract would have to legislate metric derivation —
/// rebuilding n0's oracle output — or carry two meanings for one fact.
#[test]
fn the_metric_facts_carry_the_scaler_quantum() {
    let web = candidate_from_web(&web_layout(RUN_TEXT));
    assert_eq!(web.ascent, 0.8 * FONT_SIZE, "hhea ascender, exact");
    assert_eq!(web.descent, 0.2 * FONT_SIZE, "hhea descender, exact");

    let ctx = ahem_paint_ctx();
    let (layout, font) = n0_layout(&ctx, RUN_TEXT);
    let n0 = candidate_from_n0(&layout, &font, ahem_digest());

    let Some(quantum) = declared_metric_quantum() else {
        panic!(
            "undeclared platform: measured ascent {:?} ({:08x}), descent {:?} ({:08x}), \
             baseline-relative glyph y {:?} — declare this platform's scaler quantum",
            n0.ascent,
            n0.ascent.to_bits(),
            n0.descent,
            n0.descent.to_bits(),
            n0.glyphs[0].y,
        );
    };
    assert_eq!(n0.ascent, web.ascent + quantum);
    assert_eq!(n0.descent, web.descent - quantum);
    assert_eq!(n0.glyphs[0].y, -quantum);
}

/// Outline realization is resolved meaning, not policy: the candidate
/// consumer's environment-resolved outlines (Skia `get_path`) and the Web
/// family's artifact-streamed outlines (ttf-parser) are byte-identical at
/// every probed anchor — on the integer lattice, where they are also
/// bilevel (the admitted-domain law), and off it, where both carry the
/// identical analytic coverage. n0's quantized baseline sits below coverage
/// resolution and changes nothing.
#[test]
fn outline_realizations_agree_at_every_probed_anchor() {
    let ctx = ahem_paint_ctx();
    let (layout, font) = n0_layout(&ctx, RUN_TEXT);
    let n0_run = candidate_from_n0(&layout, &font, ahem_digest());
    let web_run = candidate_from_web(&web_layout(RUN_TEXT));
    let environment = BTreeMap::from([(ahem_digest(), AHEM.to_vec())]);

    for anchor in [LATTICE_ANCHOR, FRACTIONAL_ANCHOR] {
        let candidate = realize_candidate_outlines(&web_run, &environment, anchor)
            .expect("the declared environment carries the key");
        let quantized = realize_candidate_outlines(&n0_run, &environment, anchor)
            .expect("the declared environment carries the key");
        let web = realize_web_outlines(&web_layout(RUN_TEXT), anchor);

        assert_eq!(differing_bytes(&candidate, &web), 0, "at {anchor:?}");
        assert_eq!(differing_bytes(&candidate, &quantized), 0, "at {anchor:?}");
    }

    // Ink exists and the lattice render is bilevel — the comparisons are
    // neither vacuous nor smoothed. Sample inside the first em box.
    let lattice = realize_web_outlines(&web_layout(RUN_TEXT), LATTICE_ANCHOR);
    let index = ((20 * CELL + 15) * 4) as usize;
    assert_eq!(&lattice[index..index + 4], &[0, 0, 0, 255]);
    assert_eq!(non_bilevel_bytes(&lattice), 0);
}

/// Glyph replay is raster policy, not resolved meaning. The oracle's live
/// `Font` carries pinned raster-facing settings, and replaying the same
/// candidate fact through it paints a measured non-bilevel fringe against
/// the exact outlines — on the integer lattice itself, where the external
/// oracle gates byte-exactly. A contract carrying the fact without the
/// policy would make this fringe a silent wrong pixel; carrying the policy
/// would put raster flags in a meaning-only contract. Neither is
/// admissible, so the fact stays below the join.
#[test]
fn glyph_replay_is_raster_policy_not_resolved_meaning() {
    let ctx = ahem_paint_ctx();
    let (layout, font) = n0_layout(&ctx, RUN_TEXT);
    let run = candidate_from_n0(&layout, &font, ahem_digest());
    let environment = BTreeMap::from([(ahem_digest(), AHEM.to_vec())]);

    // The policy is real and rides the live instance — set by the pinned
    // paragraph backend, not chosen by this spike.
    assert_eq!(font.edging(), Edging::AntiAlias);
    assert_eq!(font.hinting(), FontHinting::Slight);
    assert!(font.is_subpixel());

    let Some((on_lattice, off_lattice)) = declared_replay_divergence() else {
        let lattice = differing_bytes(
            &realize_glyph_replay(&run, &font, LATTICE_ANCHOR),
            &realize_candidate_outlines(&run, &environment, LATTICE_ANCHOR).unwrap(),
        );
        let fractional = differing_bytes(
            &realize_glyph_replay(&run, &font, FRACTIONAL_ANCHOR),
            &realize_candidate_outlines(&run, &environment, FRACTIONAL_ANCHOR).unwrap(),
        );
        panic!(
            "undeclared platform: measured replay divergence {lattice} bytes on the lattice, \
             {fractional} off it — declare this platform's measurement"
        );
    };

    let replayed = realize_glyph_replay(&run, &font, LATTICE_ANCHOR);
    let resolved = realize_candidate_outlines(&run, &environment, LATTICE_ANCHOR).unwrap();
    assert_eq!(differing_bytes(&replayed, &resolved), on_lattice);
    // The divergence is a smoothing fringe, not displaced ink: every
    // differing byte is a partial-coverage value the outline render does
    // not contain.
    assert_eq!(non_bilevel_bytes(&replayed), on_lattice);

    let replayed = realize_glyph_replay(&run, &font, FRACTIONAL_ANCHOR);
    let resolved = realize_candidate_outlines(&run, &environment, FRACTIONAL_ANCHOR).unwrap();
    assert_eq!(differing_bytes(&replayed, &resolved), off_lattice);

    // Control isolation: with policy stripped — alias edging, no hinting,
    // no subpixel — the same glyph pipeline reproduces the outlines
    // byte-exactly at the lattice cell. The fringe above is therefore the
    // anti-alias glyph-mask policy itself, not a defect of glyph replay per
    // se; policy is exactly what the candidate key refuses to carry.
    let mut stripped = font.clone();
    stripped.set_edging(Edging::Alias);
    stripped.set_hinting(FontHinting::None);
    stripped.set_subpixel(false);
    let replayed = realize_glyph_replay(&run, &stripped, LATTICE_ANCHOR);
    let resolved = realize_candidate_outlines(&run, &environment, LATTICE_ANCHOR).unwrap();
    assert_eq!(differing_bytes(&replayed, &resolved), 0);
}

/// The candidate fact is unrealizable without a declared resource
/// environment: the consumer's signature carries digest→bytes, and an
/// undeclared key is a refusal. Admitting the fact into the shared contract
/// therefore deletes `rframe`'s load-bearing refusal — no fact that
/// references a resource — and un-names the `glyphless` route.
#[test]
fn the_candidate_fact_is_unrealizable_without_a_resource_environment() {
    let ctx = ahem_paint_ctx();
    let (layout, font) = n0_layout(&ctx, RUN_TEXT);
    let run = candidate_from_n0(&layout, &font, ahem_digest());

    let error = realize_candidate_outlines(&run, &BTreeMap::new(), LATTICE_ANCHOR)
        .expect_err("an undeclared font key must refuse, not substitute");
    assert!(
        error.contains("is not in the declared environment"),
        "{error}"
    );
}

/// n0's artifact carries process-local identity and raster-facing font
/// flags — replay policy riding the artifact. The content digest never
/// flows through its oracle; producer B's projection had to join it back
/// from the host's own declaration. A neutral font-key boundary therefore
/// sits at the *host seam*, not inside n0's shaped-text artifact.
#[test]
fn the_n0_artifact_carries_process_identity_not_content_identity() {
    let ctx = ahem_paint_ctx();
    let (layout, _) = n0_layout(&ctx, RUN_TEXT);

    assert!(
        layout.environment.starts_with("process-local:"),
        "{}",
        layout.environment
    );
    let identity = &layout.glyph_runs[0].font_identity;
    assert!(identity.starts_with("skia-process:"), "{identity}");
    for policy in ["edging=", "hinting=", "flags="] {
        assert!(
            identity.contains(policy),
            "raster-facing {policy:?} rides n0's font identity: {identity}"
        );
    }
}

/// Beyond the overlapping slice the second producer still does not exist:
/// the same sources n0 resolves — line structure here; styled runs and
/// variable instances equally — are refusals in the Web family's v0
/// profile. Every fact where a high join would have substance is still a
/// one-producer fact, and one producer decides nothing.
#[test]
fn beyond_the_overlap_the_second_producer_still_does_not_exist() {
    let ctx = ahem_paint_ctx();
    let oracle = SkiaTextLayoutOracle::new(&ctx);
    let layout = oracle.layout(
        TextPayloadRef {
            text: "A\nB",
            default_style: TextStyleRec::from_font_size(FONT_SIZE),
            runs: None,
        },
        None,
    );
    assert_eq!(layout.lines.len(), 2, "n0 resolves the two-line source");

    let error = textlayout::resolve(
        &textlayout::AttributedText {
            text: "A\nB".to_string(),
            style: textlayout::Style {
                family: "Ahem".to_string(),
                size: FONT_SIZE,
            },
        },
        &web_environment(),
    )
    .expect_err("the Web producer refuses the same source by profile");
    assert!(matches!(
        error,
        textlayout::ResolveError::UnsupportedCharacter { .. }
    ));
}

/// The decided perimeter, held in the spike itself: `textlayout` is this
/// crate's dev-dependency for this evidence alone. The text stage joined
/// low, so the Web family's producer never becomes a shipping dependency
/// of n0 — an engine-wide text service is exactly what the decision
/// declined to create.
#[test]
fn the_web_producer_is_not_a_shipping_dependency_of_n0() {
    // A section-membership parse, not a positional grep: every line naming
    // the crate — by dependency key (`textlayout = …`, which cannot confuse
    // skia-safe's same-named *feature* string) or by rename
    // (`package = "textlayout"`) — must sit inside `[dev-dependencies]`,
    // wherever the manifest puts its tables.
    let manifest = include_str!("../Cargo.toml");
    let mut section = String::new();
    let mut declared_in_dev = false;
    for line in manifest.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('[') {
            section = trimmed.to_string();
            continue;
        }
        let names_the_crate =
            trimmed.starts_with("textlayout =") || trimmed.contains("package = \"textlayout\"");
        if names_the_crate {
            assert_eq!(
                section, "[dev-dependencies]",
                "textlayout is declared under {section}; the text stage joined low, so the \
                 Web producer is dev-only evidence, never a shipping dependency"
            );
            declared_in_dev = true;
        }
    }
    assert!(
        declared_in_dev,
        "the spike's own dev-dependency is declared"
    );
}
