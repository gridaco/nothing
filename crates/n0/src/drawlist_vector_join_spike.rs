//! Disposable, bounded D-M vector-join evidence.
//!
//! This is only the drawlist/painter plus mixed-private-text arm. It exists
//! only in `n0`'s unit-test build and deliberately does not propose a public
//! contract: the paint and stroke leaves below are test-local witnesses while
//! D-C is open, and corner smoothing is rejected because it has not yet been
//! resolved to source-neutral geometry.
//!
//! Candidate identity and provenance are consumed by this local compiler,
//! orchestration proof, and the engine's private complete-frame damage policy
//! as one opaque arm-local owner key. This does not define their future public
//! relationship. The shared cache and stable runtime identity do not consume
//! the candidate yet, so those missing policy arms prevent this spike from
//! completing D-M.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use n0_model::math::{Affine, RectF};
use n0_model::model::{
    BlendMode, Color, CornerSmoothing, DocBuilder, Document, Header, LayoutBehavior, NodeId, Paint,
    Paints, Payload, Radius, RectangularCornerRadius, RectangularStrokeWidth, ShapeDesc,
    SizeIntent, SolidPaint, Stroke, StrokeAlign, StrokeCap, StrokeJoin, StrokeWidth,
};
use n0_model::path::{self, FillRule, PathCommand, ResolvedPathArtifact};
use n0_model::properties::{PropertyKey, PropertyTarget, PropertyValue, PropertyValues, ValueView};
use n0_model::resolve::{resolve, ResolveOptions, Resolved};
use skia_safe::FontMgr;

use super::{DrawList, DrawValues, Item, ItemKind};
use crate::damage::{diff_inputs, DamageOwner, FrameDamage, FrameDamageInput};
use crate::frame;
use crate::paint::{raster_to_bytes_unchecked, PaintCtx, PaintEnvironmentKey};

const WIDTH: i32 = 180;
const HEIGHT: i32 = 100;
const INTER: &[u8] =
    include_bytes!("../../../fixtures/fonts/Inter/Inter-VariableFont_opsz,wght.ttf");

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct VisualIdentity(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct VisualProvenance(u64);

/// Candidate identity and provenance are opaque and independent from the n0
/// arena. The complete pair is the lookup key for visual facts, draw order,
/// n0-node projection, and private-text insertion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct VisualRef {
    identity: VisualIdentity,
    provenance: VisualProvenance,
}

// Deliberately unrelated to n0's arena order: attribution must not inherit
// incidental NodeId ordering.
const FRAME_REF: VisualRef = visual_ref(60, 100);
const RECT_REF: VisualRef = visual_ref(20, 200);
const ELLIPSE_REF: VisualRef = visual_ref(50, 300);
const TEXT_REF: VisualRef = visual_ref(10, 350);
const PATH_REF: VisualRef = visual_ref(40, 400);
const LINE_REF: VisualRef = visual_ref(30, 500);

const fn visual_ref(identity: u64, provenance: u64) -> VisualRef {
    VisualRef {
        identity: VisualIdentity(identity),
        provenance: VisualProvenance(provenance),
    }
}

/// The n0-side bridge is deliberately outside the candidate input. Neither
/// arena identity nor generation state enters the normalized visual facts.
#[derive(Debug)]
struct IdentityProjection {
    by_node: BTreeMap<NodeId, VisualRef>,
    by_visual: BTreeMap<VisualRef, NodeId>,
}

impl IdentityProjection {
    fn new(entries: impl IntoIterator<Item = (NodeId, VisualRef)>) -> Self {
        let mut by_node = BTreeMap::new();
        let mut by_visual = BTreeMap::new();
        for (node, visual) in entries {
            assert!(by_node.insert(node, visual).is_none());
            assert!(by_visual.insert(visual, node).is_none());
        }
        Self { by_node, by_visual }
    }

    fn visual(&self, node: NodeId) -> Result<VisualRef, EvidenceError> {
        self.by_node
            .get(&node)
            .copied()
            .ok_or(EvidenceError::MissingNodeIdentity(node))
    }

    fn node(&self, visual: VisualRef) -> Result<NodeId, EvidenceError> {
        self.by_visual
            .get(&visual)
            .copied()
            .ok_or(EvidenceError::MissingVisualTarget(visual))
    }
}

/// A deliberately narrow, disposable paint witness. It is neither cg nor
/// n0-model's permanent paint seat; it exists only to make independent literal
/// construction possible while D-C remains open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EvidenceSolid {
    active: bool,
    argb: u32,
}

impl EvidenceSolid {
    const fn opaque(argb: u32) -> Self {
        Self { active: true, argb }
    }

    const fn inactive(argb: u32) -> Self {
        Self {
            active: false,
            argb,
        }
    }

    fn visible(self) -> bool {
        self.active && (self.argb >> 24) != 0
    }

    fn model(self) -> Paint {
        let mut solid = SolidPaint::new(Color(self.argb));
        solid.active = self.active;
        Paint::Solid(solid)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EvidenceStrokeAlign {
    Inside,
    Center,
    Outside,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EvidenceStrokeCap {
    Butt,
    Round,
    Square,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EvidenceStrokeJoin {
    Miter,
    Round,
    Bevel,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum EvidenceStrokeWidth {
    None,
    Uniform(f32),
    Rectangular([f32; 4]),
}

impl EvidenceStrokeWidth {
    fn normalized(self) -> Self {
        match self {
            Self::None | Self::Uniform(0.0) => Self::None,
            Self::Uniform(width) => Self::Uniform(width),
            Self::Rectangular([top, right, bottom, left])
                if top == right && right == bottom && bottom == left =>
            {
                Self::Uniform(top).normalized()
            }
            Self::Rectangular(widths) => Self::Rectangular(widths),
        }
    }

    fn max(self) -> f32 {
        match self {
            Self::None => 0.0,
            Self::Uniform(width) => width,
            Self::Rectangular(widths) => widths.into_iter().fold(0.0, f32::max),
        }
    }

    fn model(self) -> StrokeWidth {
        match self {
            Self::None => StrokeWidth::None,
            Self::Uniform(width) => StrokeWidth::Uniform(width),
            Self::Rectangular([top, right, bottom, left]) => {
                StrokeWidth::Rectangular(RectangularStrokeWidth {
                    stroke_top_width: top,
                    stroke_right_width: right,
                    stroke_bottom_width: bottom,
                    stroke_left_width: left,
                })
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct EvidenceStroke {
    paints: Vec<EvidenceSolid>,
    width: EvidenceStrokeWidth,
    align: EvidenceStrokeAlign,
    cap: EvidenceStrokeCap,
    join: EvidenceStrokeJoin,
    miter_limit: f32,
    dash_array: Option<Vec<f32>>,
}

impl EvidenceStroke {
    fn visible_paints(&self) -> Vec<EvidenceSolid> {
        self.paints
            .iter()
            .copied()
            .filter(|paint| paint.visible())
            .collect()
    }

    fn model_with_paints(&self, paints: Vec<EvidenceSolid>) -> Stroke {
        Stroke {
            paints: model_paints(&paints),
            width: self.width.model(),
            align: match self.align {
                EvidenceStrokeAlign::Inside => StrokeAlign::Inside,
                EvidenceStrokeAlign::Center => StrokeAlign::Center,
                EvidenceStrokeAlign::Outside => StrokeAlign::Outside,
            },
            cap: match self.cap {
                EvidenceStrokeCap::Butt => StrokeCap::Butt,
                EvidenceStrokeCap::Round => StrokeCap::Round,
                EvidenceStrokeCap::Square => StrokeCap::Square,
            },
            join: match self.join {
                EvidenceStrokeJoin::Miter => StrokeJoin::Miter,
                EvidenceStrokeJoin::Round => StrokeJoin::Round,
                EvidenceStrokeJoin::Bevel => StrokeJoin::Bevel,
            },
            miter_limit: self.miter_limit,
            dash_array: self.dash_array.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct EvidenceCorners {
    tl: (f32, f32),
    tr: (f32, f32),
    br: (f32, f32),
    bl: (f32, f32),
}

impl EvidenceCorners {
    const fn circular(radius: f32) -> Self {
        Self {
            tl: (radius, radius),
            tr: (radius, radius),
            br: (radius, radius),
            bl: (radius, radius),
        }
    }

    fn model(self) -> RectangularCornerRadius {
        RectangularCornerRadius {
            tl: Radius {
                rx: self.tl.0,
                ry: self.tl.1,
            },
            tr: Radius {
                rx: self.tr.0,
                ry: self.tr.1,
            },
            br: Radius {
                rx: self.br.0,
                ry: self.br.1,
            },
            bl: Radius {
                rx: self.bl.0,
                ry: self.bl.1,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct EvidencePath {
    commands: Arc<[PathCommand]>,
    fill_rule: FillRule,
    local_bounds: RectF,
    all_contours_closed: bool,
}

impl EvidencePath {
    fn model(&self) -> Arc<ResolvedPathArtifact> {
        Arc::new(ResolvedPathArtifact {
            commands: Arc::clone(&self.commands),
            fill_rule: self.fill_rule,
            local_bounds: self.local_bounds,
            all_contours_closed: self.all_contours_closed,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
enum EvidenceGeometry {
    Rect {
        w: f32,
        h: f32,
        corners: EvidenceCorners,
    },
    Ellipse {
        w: f32,
        h: f32,
    },
    Path {
        w: f32,
        h: f32,
        path: EvidencePath,
    },
    Line {
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        paint_w: f32,
        paint_h: f32,
    },
}

#[derive(Debug, Clone, PartialEq)]
struct EvidenceVisual {
    world: Affine,
    world_bounds: RectF,
    geometry: EvidenceGeometry,
    fills: Vec<EvidenceSolid>,
    strokes: Vec<EvidenceStroke>,
}

/// Flat visual composition, not an authored hierarchy. A fill and its strokes
/// may occur on opposite sides of descendant scopes, which preserves painter
/// order without copying n0's payload tree into differently named records.
#[derive(Debug, Clone, Copy, PartialEq)]
enum EvidenceCommand {
    BeginOpacity { owner: VisualRef, opacity: f32 },
    Fill(VisualRef),
    BeginClip(VisualRef),
    EndClip(VisualRef),
    Strokes(VisualRef),
    EndOpacity(VisualRef),
}

#[derive(Debug, Clone, Default, PartialEq)]
struct EvidenceInput {
    visuals: BTreeMap<VisualRef, EvidenceVisual>,
    composition: Vec<EvidenceCommand>,
}

#[derive(Debug, Clone, PartialEq)]
enum EvidenceError {
    MissingNodeIdentity(NodeId),
    MissingVisualTarget(VisualRef),
    MissingVisualFacts(VisualRef),
    UnsupportedPaint(VisualRef),
    UnsupportedDerivedOpacity(NodeId),
    UnsupportedRootComposition,
    CornerSmoothingRequiresLowerJoin(VisualRef),
    LineCannotFill(VisualRef),
    ClipRequiresRect(VisualRef),
    InadmissibleStroke { visual: VisualRef, index: usize },
    InvalidScope,
    MissingTextAnchor(VisualRef),
}

#[derive(Debug)]
struct CompiledVectors {
    list: DrawList,
    /// One owner per item. Private text insertion resolves its location through
    /// this identity/provenance sequence rather than by incidental item index.
    owners: Vec<VisualRef>,
}

/// Resolved facts not already represented by compiled draw items.
///
/// Paint and stroke materiality belongs to the drawlist comparison. Keeping
/// source paint records out of this projection means inactive or transparent
/// paint edits cannot create damage without changing compiled output.
#[derive(Debug, Clone, PartialEq)]
enum EvidenceDamageFact {
    Vector {
        world: Affine,
        geometry: EvidenceGeometry,
    },
    PrivateText,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Scope {
    Opacity(VisualRef),
    Clip(VisualRef),
}

fn paint_ctx() -> PaintCtx {
    let typeface = FontMgr::new()
        .new_from_data(INTER, None)
        .expect("bundled Inter typeface");
    PaintCtx::new(Some(typeface))
}

fn options() -> ResolveOptions {
    ResolveOptions {
        viewport: (WIDTH as f32, HEIGHT as f32),
        ..Default::default()
    }
}

fn property_target(document: &Document, node: NodeId, key: PropertyKey) -> PropertyTarget {
    PropertyTarget::new(document.key_of(node).expect("live scene node"), key)
}

#[derive(Debug, Clone, Copy)]
struct SceneIds {
    frame: NodeId,
    rect: NodeId,
    ellipse: NodeId,
    text: NodeId,
    path: NodeId,
}

fn model_paints(paints: &[EvidenceSolid]) -> Paints {
    Paints::new(paints.iter().copied().map(EvidenceSolid::model))
}

fn solid_stroke(
    argb: u32,
    width: f32,
    align: StrokeAlign,
    cap: StrokeCap,
    join: StrokeJoin,
    dash_array: Option<Vec<f32>>,
) -> Stroke {
    Stroke {
        paints: Paints::solid(Color(argb)),
        width: StrokeWidth::Uniform(width),
        align,
        cap,
        join,
        miter_limit: 4.0,
        dash_array,
    }
}

fn scene() -> (Document, SceneIds, IdentityProjection) {
    let mut builder = DocBuilder::new();

    let mut frame_header = Header::new(SizeIntent::Fixed(160.0), SizeIntent::Fixed(90.0));
    frame_header.x = n0_model::model::AxisBinding::start(4.0);
    frame_header.y = n0_model::model::AxisBinding::start(4.0);
    frame_header.opacity = 0.75;
    let frame_id = builder.add(
        0,
        frame_header,
        Payload::Frame {
            layout: LayoutBehavior::default(),
            clips_content: true,
        },
    );

    let rect_id = builder.add(
        frame_id,
        positioned_header(6.0, 6.0, 28.0, 24.0),
        Payload::Shape {
            desc: ShapeDesc::Rect,
        },
    );
    let ellipse_id = builder.add(
        frame_id,
        positioned_header(42.0, 6.0, 26.0, 24.0),
        Payload::Shape {
            desc: ShapeDesc::Ellipse,
        },
    );
    let text_id = builder.add(
        frame_id,
        positioned_header(72.0, 6.0, 30.0, 24.0),
        Payload::Text {
            content: "A".into(),
            font_size: 18.0,
        },
    );
    let path_artifact = path::analyze(
        "M 0 0 H 1 V 1 H 0 Z M .25 .25 H .75 V .75 H .25 Z",
        FillRule::EvenOdd,
    )
    .expect("literal path is valid");
    let path_id = builder.add(
        frame_id,
        positioned_header(110.0, 6.0, 22.0, 24.0),
        Payload::Shape {
            desc: ShapeDesc::Path(path_artifact),
        },
    );
    let line_id = builder.add(
        frame_id,
        positioned_header(6.0, 50.0, 126.0, 0.0),
        Payload::Shape {
            desc: ShapeDesc::Line,
        },
    );

    builder.node_mut(frame_id).corner_radius = RectangularCornerRadius::circular(6.0);
    builder.node_mut(frame_id).fills = Paints::solid(Color(0xFFF3_F4F6));
    builder.node_mut(frame_id).strokes = vec![solid_stroke(
        0xFF25_63EB,
        2.0,
        StrokeAlign::Outside,
        StrokeCap::Butt,
        StrokeJoin::Miter,
        None,
    )];

    builder.node_mut(rect_id).corner_radius = RectangularCornerRadius::circular(4.0);
    let mut inactive = SolidPaint::new(Color(0xFFFF_FF00));
    inactive.active = false;
    builder.node_mut(rect_id).fills = Paints::new([
        Paint::Solid(inactive),
        Paint::Solid(SolidPaint::new(Color(0xFFDC_2626))),
    ]);
    builder.node_mut(rect_id).strokes = vec![solid_stroke(
        0xFF7F_1D1D,
        2.0,
        StrokeAlign::Inside,
        StrokeCap::Round,
        StrokeJoin::Round,
        Some(vec![4.0, 2.0]),
    )];

    builder.node_mut(ellipse_id).fills = Paints::solid(Color(0x8005_9669));
    builder.node_mut(ellipse_id).strokes = vec![solid_stroke(
        0xFF06_473E,
        2.0,
        StrokeAlign::Center,
        StrokeCap::Square,
        StrokeJoin::Bevel,
        None,
    )];

    builder.node_mut(text_id).fills = Paints::solid(Color::BLACK);
    builder.node_mut(text_id).strokes = vec![solid_stroke(
        0xFF25_63EB,
        1.0,
        StrokeAlign::Center,
        StrokeCap::Butt,
        StrokeJoin::Miter,
        None,
    )];

    builder.node_mut(path_id).fills = Paints::solid(Color(0xFF7C_3AED));
    builder.node_mut(path_id).strokes = vec![solid_stroke(
        0xFF4C_1D95,
        2.0,
        StrokeAlign::Center,
        StrokeCap::Butt,
        StrokeJoin::Miter,
        Some(vec![3.0, 1.0]),
    )];

    // The authored model can contain a compatibility fill on a line, but the
    // current compiler and the candidate both refuse to turn it into ink.
    builder.node_mut(line_id).fills = Paints::solid(Color(0xFFFF_0000));
    builder.node_mut(line_id).strokes = vec![solid_stroke(
        0xFF11_1827,
        3.0,
        StrokeAlign::Center,
        StrokeCap::Round,
        StrokeJoin::Miter,
        Some(vec![8.0, 4.0]),
    )];

    let document = builder.build();
    let ids = SceneIds {
        frame: frame_id,
        rect: rect_id,
        ellipse: ellipse_id,
        text: text_id,
        path: path_id,
    };
    let identities = IdentityProjection::new([
        (frame_id, FRAME_REF),
        (rect_id, RECT_REF),
        (ellipse_id, ELLIPSE_REF),
        (text_id, TEXT_REF),
        (path_id, PATH_REF),
        (line_id, LINE_REF),
    ]);
    (document, ids, identities)
}

fn positioned_header(x: f32, y: f32, w: f32, h: f32) -> Header {
    let mut header = Header::new(SizeIntent::Fixed(w), SizeIntent::Fixed(h));
    header.x = n0_model::model::AxisBinding::start(x);
    header.y = n0_model::model::AxisBinding::start(y);
    header
}

fn extract_paints(visual: VisualRef, paints: &Paints) -> Result<Vec<EvidenceSolid>, EvidenceError> {
    paints
        .iter()
        .map(|paint| match paint {
            Paint::Solid(solid) if solid.blend_mode == BlendMode::Normal => Ok(EvidenceSolid {
                active: solid.active,
                argb: solid.color.argb(),
            }),
            _ => Err(EvidenceError::UnsupportedPaint(visual)),
        })
        .collect()
}

fn extract_stroke(visual: VisualRef, stroke: &Stroke) -> Result<EvidenceStroke, EvidenceError> {
    Ok(EvidenceStroke {
        paints: extract_paints(visual, &stroke.paints)?,
        width: match stroke.width {
            StrokeWidth::None => EvidenceStrokeWidth::None,
            StrokeWidth::Uniform(width) => EvidenceStrokeWidth::Uniform(width),
            StrokeWidth::Rectangular(widths) => EvidenceStrokeWidth::Rectangular(widths.values()),
        },
        align: match stroke.align {
            StrokeAlign::Inside => EvidenceStrokeAlign::Inside,
            StrokeAlign::Center => EvidenceStrokeAlign::Center,
            StrokeAlign::Outside => EvidenceStrokeAlign::Outside,
        },
        cap: match stroke.cap {
            StrokeCap::Butt => EvidenceStrokeCap::Butt,
            StrokeCap::Round => EvidenceStrokeCap::Round,
            StrokeCap::Square => EvidenceStrokeCap::Square,
        },
        join: match stroke.join {
            StrokeJoin::Miter => EvidenceStrokeJoin::Miter,
            StrokeJoin::Round => EvidenceStrokeJoin::Round,
            StrokeJoin::Bevel => EvidenceStrokeJoin::Bevel,
        },
        miter_limit: stroke.miter_limit,
        dash_array: stroke.dash_array.clone(),
    })
}

fn evidence_corners(corners: RectangularCornerRadius) -> EvidenceCorners {
    EvidenceCorners {
        tl: (corners.tl.rx, corners.tl.ry),
        tr: (corners.tr.rx, corners.tr.ry),
        br: (corners.br.rx, corners.br.ry),
        bl: (corners.bl.rx, corners.bl.ry),
    }
}

fn extract_input<V: DrawValues + ?Sized>(
    values: &V,
    resolved: &Resolved,
    identities: &IdentityProjection,
) -> Result<EvidenceInput, EvidenceError> {
    let document = values.document();
    let root = document.get(document.root);
    if values.opacity(document.root) != 1.0 || values.clips_content(document.root) {
        return Err(EvidenceError::UnsupportedRootComposition);
    }

    let mut input = EvidenceInput::default();
    for &child in &root.children {
        extract_node(values, resolved, identities, child, &mut input)?;
    }
    Ok(input)
}

fn extract_node<V: DrawValues + ?Sized>(
    values: &V,
    resolved: &Resolved,
    identities: &IdentityProjection,
    node_id: NodeId,
    input: &mut EvidenceInput,
) -> Result<(), EvidenceError> {
    let document = values.document();
    let Some(world) = resolved.world_opt(node_id) else {
        return Ok(());
    };
    let node = document.get(node_id);

    if node.payload.as_text().is_some() {
        return Ok(());
    }
    if node.payload.box_is_derived() {
        if values.opacity(node_id) != 1.0 {
            return Err(EvidenceError::UnsupportedDerivedOpacity(node_id));
        }
        for &child in &node.children {
            extract_node(values, resolved, identities, child, input)?;
        }
        return Ok(());
    }

    let visual = identities.visual(node_id)?;
    let bounds = resolved.box_of(node_id);
    let (geometry, fills) = match &node.payload {
        Payload::Frame { .. } => {
            if !values.corner_smoothing(node_id).is_zero() {
                return Err(EvidenceError::CornerSmoothingRequiresLowerJoin(visual));
            }
            (
                EvidenceGeometry::Rect {
                    w: bounds.w,
                    h: bounds.h,
                    corners: evidence_corners(values.corner_radius(node_id)),
                },
                extract_paints(visual, values.fills(node_id))?,
            )
        }
        Payload::Shape {
            desc: ShapeDesc::Rect,
        } => {
            if !values.corner_smoothing(node_id).is_zero() {
                return Err(EvidenceError::CornerSmoothingRequiresLowerJoin(visual));
            }
            (
                EvidenceGeometry::Rect {
                    w: bounds.w,
                    h: bounds.h,
                    corners: evidence_corners(values.corner_radius(node_id)),
                },
                extract_paints(visual, values.fills(node_id))?,
            )
        }
        Payload::Shape {
            desc: ShapeDesc::Ellipse,
        } => (
            EvidenceGeometry::Ellipse {
                w: bounds.w,
                h: bounds.h,
            },
            extract_paints(visual, values.fills(node_id))?,
        ),
        Payload::Shape {
            desc: ShapeDesc::Path(_),
        } => {
            let path = resolved.resolved_path_of(node_id);
            (
                EvidenceGeometry::Path {
                    w: bounds.w,
                    h: bounds.h,
                    path: EvidencePath {
                        commands: Arc::clone(&path.commands),
                        fill_rule: path.fill_rule,
                        local_bounds: path.local_bounds,
                        all_contours_closed: path.all_contours_closed,
                    },
                },
                extract_paints(visual, values.fills(node_id))?,
            )
        }
        Payload::Shape {
            desc: ShapeDesc::Line,
        } => (
            EvidenceGeometry::Line {
                x1: 0.0,
                y1: 0.0,
                x2: bounds.w,
                y2: 0.0,
                paint_w: bounds.w,
                paint_h: bounds.h,
            },
            Vec::new(),
        ),
        Payload::Text { .. }
        | Payload::AttributedText { .. }
        | Payload::Group
        | Payload::Lens { .. } => unreachable!("handled above"),
    };
    let strokes = values
        .strokes(node_id)
        .iter()
        .map(|stroke| extract_stroke(visual, stroke))
        .collect::<Result<Vec<_>, _>>()?;
    assert!(
        input
            .visuals
            .insert(
                visual,
                EvidenceVisual {
                    world,
                    world_bounds: resolved.aabb_of(node_id),
                    geometry,
                    fills,
                    strokes,
                },
            )
            .is_none(),
        "one visual identity/provenance pair owns one fact record"
    );

    let opacity = values.opacity(node_id);
    if opacity != 1.0 {
        input.composition.push(EvidenceCommand::BeginOpacity {
            owner: visual,
            opacity,
        });
    }
    input.composition.push(EvidenceCommand::Fill(visual));

    let clips = matches!(node.payload, Payload::Frame { .. }) && values.clips_content(node_id);
    if clips {
        input.composition.push(EvidenceCommand::BeginClip(visual));
    }
    for &child in &node.children {
        extract_node(values, resolved, identities, child, input)?;
    }
    if clips {
        input.composition.push(EvidenceCommand::EndClip(visual));
    }
    input.composition.push(EvidenceCommand::Strokes(visual));
    if opacity != 1.0 {
        input.composition.push(EvidenceCommand::EndOpacity(visual));
    }
    Ok(())
}

fn literal_input() -> EvidenceInput {
    let path = EvidencePath {
        commands: Arc::from([
            PathCommand::MoveTo { x: 0.0, y: 0.0 },
            PathCommand::LineTo { x: 22.0, y: 0.0 },
            PathCommand::LineTo { x: 22.0, y: 24.0 },
            PathCommand::LineTo { x: 0.0, y: 24.0 },
            PathCommand::Close,
            PathCommand::MoveTo { x: 5.5, y: 6.0 },
            PathCommand::LineTo { x: 16.5, y: 6.0 },
            PathCommand::LineTo { x: 16.5, y: 18.0 },
            PathCommand::LineTo { x: 5.5, y: 18.0 },
            PathCommand::Close,
        ]),
        fill_rule: FillRule::EvenOdd,
        local_bounds: RectF {
            x: 0.0,
            y: 0.0,
            w: 22.0,
            h: 24.0,
        },
        all_contours_closed: true,
    };

    let visuals = BTreeMap::from([
        (
            FRAME_REF,
            EvidenceVisual {
                world: Affine::translate(4.0, 4.0),
                world_bounds: RectF {
                    x: 2.0,
                    y: 2.0,
                    w: 164.0,
                    h: 94.0,
                },
                geometry: EvidenceGeometry::Rect {
                    w: 160.0,
                    h: 90.0,
                    corners: EvidenceCorners::circular(6.0),
                },
                fills: vec![EvidenceSolid::opaque(0xFFF3_F4F6)],
                strokes: vec![evidence_stroke(
                    0xFF25_63EB,
                    2.0,
                    EvidenceStrokeAlign::Outside,
                    EvidenceStrokeCap::Butt,
                    EvidenceStrokeJoin::Miter,
                    None,
                )],
            },
        ),
        (
            RECT_REF,
            EvidenceVisual {
                world: Affine::translate(10.0, 10.0),
                world_bounds: RectF {
                    x: 10.0,
                    y: 10.0,
                    w: 28.0,
                    h: 24.0,
                },
                geometry: EvidenceGeometry::Rect {
                    w: 28.0,
                    h: 24.0,
                    corners: EvidenceCorners::circular(4.0),
                },
                fills: vec![
                    EvidenceSolid::inactive(0xFFFF_FF00),
                    EvidenceSolid::opaque(0xFFDC_2626),
                ],
                strokes: vec![evidence_stroke(
                    0xFF7F_1D1D,
                    2.0,
                    EvidenceStrokeAlign::Inside,
                    EvidenceStrokeCap::Round,
                    EvidenceStrokeJoin::Round,
                    Some(vec![4.0, 2.0]),
                )],
            },
        ),
        (
            ELLIPSE_REF,
            EvidenceVisual {
                world: Affine::translate(46.0, 10.0),
                world_bounds: RectF {
                    x: 45.0,
                    y: 9.0,
                    w: 28.0,
                    h: 26.0,
                },
                geometry: EvidenceGeometry::Ellipse { w: 26.0, h: 24.0 },
                fills: vec![EvidenceSolid::opaque(0x8005_9669)],
                strokes: vec![evidence_stroke(
                    0xFF06_473E,
                    2.0,
                    EvidenceStrokeAlign::Center,
                    EvidenceStrokeCap::Square,
                    EvidenceStrokeJoin::Bevel,
                    None,
                )],
            },
        ),
        (
            PATH_REF,
            EvidenceVisual {
                world: Affine::translate(114.0, 10.0),
                world_bounds: RectF {
                    x: 110.0,
                    y: 6.0,
                    w: 30.0,
                    h: 32.0,
                },
                geometry: EvidenceGeometry::Path {
                    w: 22.0,
                    h: 24.0,
                    path,
                },
                fills: vec![EvidenceSolid::opaque(0xFF7C_3AED)],
                strokes: vec![evidence_stroke(
                    0xFF4C_1D95,
                    2.0,
                    EvidenceStrokeAlign::Center,
                    EvidenceStrokeCap::Butt,
                    EvidenceStrokeJoin::Miter,
                    Some(vec![3.0, 1.0]),
                )],
            },
        ),
        (
            LINE_REF,
            EvidenceVisual {
                world: Affine::translate(10.0, 54.0),
                world_bounds: RectF {
                    x: 8.5,
                    y: 52.5,
                    w: 129.0,
                    h: 3.0,
                },
                geometry: EvidenceGeometry::Line {
                    x1: 0.0,
                    y1: 0.0,
                    x2: 126.0,
                    y2: 0.0,
                    paint_w: 126.0,
                    paint_h: 0.0,
                },
                fills: Vec::new(),
                strokes: vec![evidence_stroke(
                    0xFF11_1827,
                    3.0,
                    EvidenceStrokeAlign::Center,
                    EvidenceStrokeCap::Round,
                    EvidenceStrokeJoin::Miter,
                    Some(vec![8.0, 4.0]),
                )],
            },
        ),
    ]);
    EvidenceInput {
        visuals,
        composition: vec![
            EvidenceCommand::BeginOpacity {
                owner: FRAME_REF,
                opacity: 0.75,
            },
            EvidenceCommand::Fill(FRAME_REF),
            EvidenceCommand::BeginClip(FRAME_REF),
            EvidenceCommand::Fill(RECT_REF),
            EvidenceCommand::Strokes(RECT_REF),
            EvidenceCommand::Fill(ELLIPSE_REF),
            EvidenceCommand::Strokes(ELLIPSE_REF),
            EvidenceCommand::Fill(PATH_REF),
            EvidenceCommand::Strokes(PATH_REF),
            EvidenceCommand::Fill(LINE_REF),
            EvidenceCommand::Strokes(LINE_REF),
            EvidenceCommand::EndClip(FRAME_REF),
            EvidenceCommand::Strokes(FRAME_REF),
            EvidenceCommand::EndOpacity(FRAME_REF),
        ],
    }
}

fn literal_fill_transition_input() -> EvidenceInput {
    let mut input = literal_input();
    input
        .visuals
        .get_mut(&RECT_REF)
        .expect("literal rectangle exists")
        .fills = vec![EvidenceSolid::opaque(0xFF25_63EB)];
    input
}

fn literal_inactive_fill_transition_input() -> EvidenceInput {
    let mut input = literal_input();
    input
        .visuals
        .get_mut(&RECT_REF)
        .expect("literal rectangle exists")
        .fills[0] = EvidenceSolid::inactive(0xFF00_FFFF);
    input
}

fn evidence_stroke(
    argb: u32,
    width: f32,
    align: EvidenceStrokeAlign,
    cap: EvidenceStrokeCap,
    join: EvidenceStrokeJoin,
    dash_array: Option<Vec<f32>>,
) -> EvidenceStroke {
    EvidenceStroke {
        paints: vec![EvidenceSolid::opaque(argb)],
        width: EvidenceStrokeWidth::Uniform(width),
        align,
        cap,
        join,
        miter_limit: 4.0,
        dash_array,
    }
}

fn compile_vectors(
    input: &EvidenceInput,
    identities: &IdentityProjection,
) -> Result<CompiledVectors, EvidenceError> {
    let lowered_paths = input
        .visuals
        .iter()
        .filter_map(|(visual, facts)| match &facts.geometry {
            EvidenceGeometry::Path { path, .. } => Some((*visual, path.model())),
            _ => None,
        })
        .collect::<BTreeMap<_, _>>();
    let mut items = Vec::new();
    let mut owners = Vec::new();
    let mut scopes = Vec::new();

    for command in &input.composition {
        match *command {
            EvidenceCommand::BeginOpacity { owner, opacity } => {
                let facts = facts(input, owner)?;
                if !opacity.is_finite() {
                    return Err(EvidenceError::InvalidScope);
                }
                push_evidence_item(
                    &mut items,
                    &mut owners,
                    identities,
                    owner,
                    facts.world,
                    ItemKind::BeginOpacity { opacity },
                )?;
                scopes.push(Scope::Opacity(owner));
            }
            EvidenceCommand::Fill(visual) => {
                let facts = facts(input, visual)?;
                let paints = facts
                    .fills
                    .iter()
                    .copied()
                    .filter(|paint| paint.visible())
                    .collect::<Vec<_>>();
                if matches!(facts.geometry, EvidenceGeometry::Line { .. }) {
                    if !facts.fills.is_empty() {
                        return Err(EvidenceError::LineCannotFill(visual));
                    }
                    continue;
                }
                if paints.is_empty() {
                    continue;
                }
                let paints = model_paints(&paints);
                let kind = match &facts.geometry {
                    EvidenceGeometry::Rect { w, h, corners } => ItemKind::RectFill {
                        w: *w,
                        h: *h,
                        corner_radius: corners.model(),
                        corner_smoothing: CornerSmoothing::default(),
                        paints,
                    },
                    EvidenceGeometry::Ellipse { w, h } => ItemKind::OvalFill {
                        w: *w,
                        h: *h,
                        paints,
                    },
                    EvidenceGeometry::Path { w, h, .. } => ItemKind::PathFill {
                        w: *w,
                        h: *h,
                        path: Arc::clone(
                            lowered_paths
                                .get(&visual)
                                .expect("every candidate path was lowered once"),
                        ),
                        paints,
                    },
                    EvidenceGeometry::Line { .. } => unreachable!("handled above"),
                };
                push_evidence_item(
                    &mut items,
                    &mut owners,
                    identities,
                    visual,
                    facts.world,
                    kind,
                )?;
            }
            EvidenceCommand::BeginClip(visual) => {
                let facts = facts(input, visual)?;
                let EvidenceGeometry::Rect { w, h, corners } = facts.geometry else {
                    return Err(EvidenceError::ClipRequiresRect(visual));
                };
                push_evidence_item(
                    &mut items,
                    &mut owners,
                    identities,
                    visual,
                    facts.world,
                    ItemKind::BeginClipRect {
                        w,
                        h,
                        corner_radius: corners.model(),
                        corner_smoothing: CornerSmoothing::default(),
                    },
                )?;
                scopes.push(Scope::Clip(visual));
            }
            EvidenceCommand::EndClip(visual) => {
                if scopes.pop() != Some(Scope::Clip(visual)) {
                    return Err(EvidenceError::InvalidScope);
                }
                let facts = facts(input, visual)?;
                push_evidence_item(
                    &mut items,
                    &mut owners,
                    identities,
                    visual,
                    facts.world,
                    ItemKind::EndClip,
                )?;
            }
            EvidenceCommand::Strokes(visual) => {
                let facts = facts(input, visual)?;
                for (index, stroke) in facts.strokes.iter().enumerate() {
                    let paints = stroke.visible_paints();
                    if stroke.width.max() <= 0.0 || paints.is_empty() {
                        continue;
                    }
                    if !stroke_admitted(&facts.geometry, stroke) {
                        return Err(EvidenceError::InadmissibleStroke { visual, index });
                    }
                    let stroke = stroke.model_with_paints(paints);
                    let kind = match &facts.geometry {
                        EvidenceGeometry::Rect { w, h, corners } => ItemKind::RectStroke {
                            w: *w,
                            h: *h,
                            corner_radius: corners.model(),
                            corner_smoothing: CornerSmoothing::default(),
                            stroke,
                        },
                        EvidenceGeometry::Ellipse { w, h } => ItemKind::OvalStroke {
                            w: *w,
                            h: *h,
                            stroke,
                        },
                        EvidenceGeometry::Path { w, h, .. } => ItemKind::PathStroke {
                            w: *w,
                            h: *h,
                            path: Arc::clone(
                                lowered_paths
                                    .get(&visual)
                                    .expect("every candidate path was lowered once"),
                            ),
                            stroke,
                        },
                        EvidenceGeometry::Line {
                            x1,
                            y1,
                            x2,
                            y2,
                            paint_w,
                            paint_h,
                        } => ItemKind::LineStroke {
                            x1: *x1,
                            y1: *y1,
                            x2: *x2,
                            y2: *y2,
                            paint_w: *paint_w,
                            paint_h: *paint_h,
                            stroke,
                        },
                    };
                    push_evidence_item(
                        &mut items,
                        &mut owners,
                        identities,
                        visual,
                        facts.world,
                        kind,
                    )?;
                }
            }
            EvidenceCommand::EndOpacity(visual) => {
                if scopes.pop() != Some(Scope::Opacity(visual)) {
                    return Err(EvidenceError::InvalidScope);
                }
                let facts = facts(input, visual)?;
                push_evidence_item(
                    &mut items,
                    &mut owners,
                    identities,
                    visual,
                    facts.world,
                    ItemKind::EndOpacity,
                )?;
            }
        }
    }
    if !scopes.is_empty() {
        return Err(EvidenceError::InvalidScope);
    }
    Ok(CompiledVectors {
        list: DrawList {
            items,
            text_fonts: None,
        },
        owners,
    })
}

fn facts(input: &EvidenceInput, visual: VisualRef) -> Result<&EvidenceVisual, EvidenceError> {
    input
        .visuals
        .get(&visual)
        .ok_or(EvidenceError::MissingVisualFacts(visual))
}

fn push_evidence_item(
    items: &mut Vec<Item>,
    owners: &mut Vec<VisualRef>,
    identities: &IdentityProjection,
    visual: VisualRef,
    world: Affine,
    kind: ItemKind,
) -> Result<(), EvidenceError> {
    let node = identities.node(visual)?;
    items.push(Item { node, world, kind });
    owners.push(visual);
    Ok(())
}

fn stroke_admitted(geometry: &EvidenceGeometry, stroke: &EvidenceStroke) -> bool {
    if let EvidenceGeometry::Path { path, .. } = geometry {
        if !path.all_contours_closed && stroke.align != EvidenceStrokeAlign::Center {
            return false;
        }
    }
    match stroke.width.normalized() {
        EvidenceStrokeWidth::None => false,
        EvidenceStrokeWidth::Uniform(_) => true,
        EvidenceStrokeWidth::Rectangular(_) => {
            matches!(geometry, EvidenceGeometry::Rect { .. })
                && stroke.cap == EvidenceStrokeCap::Butt
                && stroke.join == EvidenceStrokeJoin::Miter
                && stroke.miter_limit == 4.0
        }
    }
}

fn without_node(list: &DrawList, node: NodeId) -> DrawList {
    DrawList {
        items: list
            .items
            .iter()
            .filter(|item| item.node != node)
            .cloned()
            .collect(),
        text_fonts: None,
    }
}

/// Interleave only n0-private text items. The candidate input never receives a
/// text artifact or font registry; the insertion anchor is the complete opaque
/// identity/provenance pair of the following vector fact.
fn interleave_private_text_before(
    mut vectors: CompiledVectors,
    anchor: VisualRef,
    text_node: NodeId,
    current: &DrawList,
) -> Result<CompiledVectors, EvidenceError> {
    let index = vectors
        .owners
        .iter()
        .position(|owner| *owner == anchor)
        .ok_or(EvidenceError::MissingTextAnchor(anchor))?;
    let text_items = current
        .items
        .iter()
        .filter(|item| item.node == text_node)
        .cloned()
        .collect::<Vec<_>>();
    vectors.owners.splice(
        index..index,
        std::iter::repeat_n(TEXT_REF, text_items.len()),
    );
    vectors.list.items.splice(index..index, text_items);
    vectors.list.text_fonts = current.text_fonts.clone();
    Ok(vectors)
}

fn candidate_damage_input<'a>(
    input: &EvidenceInput,
    compiled: &'a CompiledVectors,
    text_bounds: RectF,
    environment: PaintEnvironmentKey,
) -> FrameDamageInput<'a, VisualRef, EvidenceDamageFact> {
    let mut owners = input
        .visuals
        .iter()
        .map(|(&visual, facts)| {
            (
                visual,
                DamageOwner::new(
                    EvidenceDamageFact::Vector {
                        world: facts.world,
                        geometry: facts.geometry.clone(),
                    },
                    Some(facts.world_bounds),
                ),
            )
        })
        .collect::<BTreeMap<_, _>>();
    assert!(
        owners
            .insert(
                TEXT_REF,
                DamageOwner::new(EvidenceDamageFact::PrivateText, Some(text_bounds)),
            )
            .is_none(),
        "private text has one distinct stable owner"
    );
    FrameDamageInput::new(
        owners,
        compiled.owners.iter().copied(),
        &compiled.list,
        environment,
    )
}

#[test]
fn independent_vector_facts_reach_the_existing_list_painter_and_private_text() {
    let (document, ids, identities) = scene();
    let context = paint_ctx();
    let product =
        frame::resolve_and_build(&document, &options(), &context).expect("valid current frame");
    let shaped_text = product
        .drawlist()
        .items
        .iter()
        .find_map(|item| {
            (item.node == ids.text)
                .then_some(&item.kind)
                .and_then(|kind| match kind {
                    ItemKind::TextFill { layout, .. } => Some(layout),
                    _ => None,
                })
        })
        .expect("the current frame contains its private shaped-text artifact");
    assert!(
        !shaped_text.glyph_runs.is_empty(),
        "the mixed proof must exercise glyph replay, not fontless text metrics"
    );
    assert!(
        product.drawlist().text_fonts.is_some(),
        "the glyph-bearing current list owns its exact private font registry"
    );
    let extracted = extract_input(&document, product.resolved(), &identities)
        .expect("admitted n0 facts extract");
    let literal = literal_input();

    assert_eq!(
        extracted, literal,
        "the normalized input is independently constructible without a document, resolved tier, or drawlist"
    );

    let current_vectors = without_node(product.drawlist(), ids.text);
    let compiled = compile_vectors(&extracted, &identities).expect("candidate facts compile");
    assert_eq!(
        compiled.list, current_vectors,
        "candidate and current compilers produce one exact private drawlist"
    );
    let compiled_literal =
        compile_vectors(&literal, &identities).expect("literal candidate facts compile");
    assert_eq!(compiled_literal.list, current_vectors);

    let vector_bytes =
        raster_to_bytes_unchecked(&compiled.list, &Affine::IDENTITY, WIDTH, HEIGHT, &context);
    assert_eq!(
        vector_bytes,
        raster_to_bytes_unchecked(&current_vectors, &Affine::IDENTITY, WIDTH, HEIGHT, &context,)
    );

    let mixed = interleave_private_text_before(compiled, PATH_REF, ids.text, product.drawlist())
        .expect("private text has one identity-anchored insertion");
    assert_eq!(
        mixed.list,
        *product.drawlist(),
        "real n0 text artifacts and their exact font registry preserve full painter order"
    );
    let mixed_bytes =
        raster_to_bytes_unchecked(&mixed.list, &Affine::IDENTITY, WIDTH, HEIGHT, &context);
    assert_eq!(
        mixed_bytes,
        product
            .raster_to_bytes(&Affine::IDENTITY, WIDTH, HEIGHT, &context)
            .expect("current checked frame raster")
    );
    assert_eq!(
        mixed_bytes,
        raster_to_bytes_unchecked(&mixed.list, &Affine::IDENTITY, WIDTH, HEIGHT, &context),
        "mixed replay is byte-deterministic"
    );
    assert_ne!(
        mixed_bytes, vector_bytes,
        "the real private text must contribute pixels, not merely list metadata"
    );

    let path_items = mixed
        .list
        .items
        .iter()
        .filter_map(|item| match &item.kind {
            ItemKind::PathFill { path, .. } | ItemKind::PathStroke { path, .. } => Some(path),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(path_items.len() >= 2);
    assert!(path_items[1..]
        .iter()
        .all(|path| Arc::ptr_eq(path_items[0], path)));
}

#[test]
fn stable_candidate_identity_drives_the_existing_complete_damage_policy() {
    let (document, ids, identities) = scene();
    let context = paint_ctx();
    let before_product =
        frame::resolve_and_build(&document, &options(), &context).expect("valid before frame");
    let before_extracted =
        extract_input(&document, before_product.resolved(), &identities).unwrap();
    let before_literal = literal_input();
    assert_eq!(
        before_extracted, before_literal,
        "the before frame is independently normalized"
    );

    let values = PropertyValues::new(
        &document,
        [(
            property_target(&document, ids.rect, PropertyKey::Fills),
            PropertyValue::Paints(Paints::solid(Color(0xFF25_63EB))),
        )],
    )
    .expect("valid immutable fill transition");
    let value_view = ValueView::new(&document, &values).expect("validated effective view");
    let after_product = frame::resolve_and_build_view(&value_view, &options(), &context)
        .expect("valid after frame");
    let after_extracted =
        extract_input(&value_view, after_product.resolved(), &identities).unwrap();
    let after_literal = literal_fill_transition_input();
    assert_eq!(
        after_extracted, after_literal,
        "the after frame is independently normalized"
    );
    assert_eq!(
        before_literal.visuals.keys().collect::<Vec<_>>(),
        after_literal.visuals.keys().collect::<Vec<_>>(),
        "full identity/provenance keys are stable across the effective transition"
    );

    let before_compiled = interleave_private_text_before(
        compile_vectors(&before_literal, &identities).unwrap(),
        PATH_REF,
        ids.text,
        before_product.drawlist(),
    )
    .unwrap();
    let after_compiled = interleave_private_text_before(
        compile_vectors(&after_literal, &identities).unwrap(),
        PATH_REF,
        ids.text,
        after_product.drawlist(),
    )
    .unwrap();
    assert_eq!(before_compiled.list, *before_product.drawlist());
    assert_eq!(after_compiled.list, *after_product.drawlist());

    let before_bytes = raster_to_bytes_unchecked(
        &before_compiled.list,
        &Affine::IDENTITY,
        WIDTH,
        HEIGHT,
        &context,
    );
    let after_bytes = raster_to_bytes_unchecked(
        &after_compiled.list,
        &Affine::IDENTITY,
        WIDTH,
        HEIGHT,
        &context,
    );
    assert_eq!(
        before_bytes,
        before_product
            .raster_to_bytes(&Affine::IDENTITY, WIDTH, HEIGHT, &context)
            .unwrap()
    );
    assert_eq!(
        after_bytes,
        after_product
            .raster_to_bytes(&Affine::IDENTITY, WIDTH, HEIGHT, &context)
            .unwrap()
    );
    assert_ne!(
        before_bytes, after_bytes,
        "the fill transition contributes different pixels"
    );

    let before_input = candidate_damage_input(
        &before_literal,
        &before_compiled,
        before_product.resolved().aabb_of(ids.text),
        before_product.environment(),
    );
    let after_input = candidate_damage_input(
        &after_literal,
        &after_compiled,
        after_product.resolved().aabb_of(ids.text),
        after_product.environment(),
    );
    let candidate_damage = diff_inputs(&before_input, &after_input);
    let expected_bounds = RectF {
        x: 10.0,
        y: 10.0,
        w: 28.0,
        h: 24.0,
    };
    assert_eq!(
        candidate_damage,
        FrameDamage {
            changed: vec![RECT_REF],
            union_world: Some(expected_bounds),
        }
    );
    assert!(
        !candidate_damage.changed.contains(&TEXT_REF),
        "unchanged real private text remains interleaved and undamaged"
    );

    let public_damage = crate::damage::diff_frame(&before_product, &after_product);
    let projected_public = public_damage
        .changed
        .iter()
        .map(|&node| {
            identities
                .visual(node)
                .expect("damaged node has stable identity")
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        candidate_damage
            .changed
            .iter()
            .copied()
            .collect::<BTreeSet<_>>(),
        projected_public,
        "damage attribution is a set; candidate-key order is independent from NodeId order"
    );
    assert_eq!(candidate_damage.union_world, public_damage.union_world);
}

#[test]
fn inactive_paint_edits_do_not_become_material_damage() {
    let (document, ids, identities) = scene();
    let context = paint_ctx();
    let before_product =
        frame::resolve_and_build(&document, &options(), &context).expect("valid before frame");
    let before_literal = literal_input();

    let mut inactive = SolidPaint::new(Color(0xFF00_FFFF));
    inactive.active = false;
    let values = PropertyValues::new(
        &document,
        [(
            property_target(&document, ids.rect, PropertyKey::Fills),
            PropertyValue::Paints(Paints::new([
                Paint::Solid(inactive),
                Paint::Solid(SolidPaint::new(Color(0xFFDC_2626))),
            ])),
        )],
    )
    .expect("valid immutable inactive-paint transition");
    let value_view = ValueView::new(&document, &values).expect("validated effective view");
    let after_product = frame::resolve_and_build_view(&value_view, &options(), &context)
        .expect("valid after frame");
    let after_extracted =
        extract_input(&value_view, after_product.resolved(), &identities).unwrap();
    let after_literal = literal_inactive_fill_transition_input();
    assert_eq!(after_extracted, after_literal);
    assert_ne!(
        before_literal, after_literal,
        "the source-normalized inactive paint fact genuinely changed"
    );

    let before_compiled = interleave_private_text_before(
        compile_vectors(&before_literal, &identities).unwrap(),
        PATH_REF,
        ids.text,
        before_product.drawlist(),
    )
    .unwrap();
    let after_compiled = interleave_private_text_before(
        compile_vectors(&after_literal, &identities).unwrap(),
        PATH_REF,
        ids.text,
        after_product.drawlist(),
    )
    .unwrap();
    assert_eq!(before_compiled.list, after_compiled.list);
    assert_eq!(before_compiled.list, *before_product.drawlist());
    assert_eq!(after_compiled.list, *after_product.drawlist());

    let before_bytes = raster_to_bytes_unchecked(
        &before_compiled.list,
        &Affine::IDENTITY,
        WIDTH,
        HEIGHT,
        &context,
    );
    let after_bytes = raster_to_bytes_unchecked(
        &after_compiled.list,
        &Affine::IDENTITY,
        WIDTH,
        HEIGHT,
        &context,
    );
    assert_eq!(before_bytes, after_bytes);

    let before_input = candidate_damage_input(
        &before_literal,
        &before_compiled,
        before_product.resolved().aabb_of(ids.text),
        before_product.environment(),
    );
    let after_input = candidate_damage_input(
        &after_literal,
        &after_compiled,
        after_product.resolved().aabb_of(ids.text),
        after_product.environment(),
    );
    assert_eq!(
        diff_inputs(&before_input, &after_input),
        FrameDamage::default(),
        "inactive source paint is not compiled visual material"
    );
    assert_eq!(
        crate::damage::diff_frame(&before_product, &after_product),
        crate::damage::Damage::default(),
        "ordinary n0 applies the same materiality rule"
    );
}

#[test]
fn unresolved_corners_and_inadmissible_primitive_states_fail_explicitly() {
    let (mut document, ids, identities) = scene();
    document.get_mut(ids.rect).corner_smoothing = CornerSmoothing(0.5);
    let resolved = resolve(&document, &options());
    assert_eq!(
        extract_input(&document, &resolved, &identities),
        Err(EvidenceError::CornerSmoothingRequiresLowerJoin(RECT_REF)),
        "corner smoothing cannot enter the candidate as an authoring parameter"
    );

    let (_, _, identities) = scene();
    let mut line_fill = literal_input();
    line_fill
        .visuals
        .get_mut(&LINE_REF)
        .unwrap()
        .fills
        .push(EvidenceSolid::opaque(0xFFFF_0000));
    assert!(matches!(
        compile_vectors(&line_fill, &identities),
        Err(EvidenceError::LineCannotFill(LINE_REF))
    ));

    let mut ellipse_clip = literal_input();
    let clip = ellipse_clip
        .composition
        .iter_mut()
        .find(|command| matches!(command, EvidenceCommand::BeginClip(_)))
        .unwrap();
    *clip = EvidenceCommand::BeginClip(ELLIPSE_REF);
    assert!(matches!(
        compile_vectors(&ellipse_clip, &identities),
        Err(EvidenceError::ClipRequiresRect(ELLIPSE_REF))
    ));

    let mut rectangular_ellipse_stroke = literal_input();
    rectangular_ellipse_stroke
        .visuals
        .get_mut(&ELLIPSE_REF)
        .unwrap()
        .strokes[0]
        .width = EvidenceStrokeWidth::Rectangular([1.0, 2.0, 3.0, 4.0]);
    assert!(matches!(
        compile_vectors(&rectangular_ellipse_stroke, &identities),
        Err(EvidenceError::InadmissibleStroke {
            visual: ELLIPSE_REF,
            index: 0
        })
    ));
}

#[test]
fn equal_sided_rectangular_widths_match_current_non_rect_normalization() {
    let (mut document, ids, identities) = scene();
    for node in [ids.ellipse, ids.path] {
        document.get_mut(node).strokes[0].width =
            StrokeWidth::Rectangular(RectangularStrokeWidth::all(2.0));
    }

    let context = paint_ctx();
    let product =
        frame::resolve_and_build(&document, &options(), &context).expect("valid current frame");
    let extracted = extract_input(&document, product.resolved(), &identities).unwrap();
    let compiled = compile_vectors(&extracted, &identities).unwrap();
    assert_eq!(
        compiled.list,
        without_node(product.drawlist(), ids.text),
        "admissibility normalizes equal-sided rectangular widths exactly as the current compiler"
    );

    for node in [ids.ellipse, ids.path] {
        let stroke = compiled
            .list
            .items
            .iter()
            .find_map(|item| {
                (item.node == node)
                    .then_some(&item.kind)
                    .and_then(|kind| match kind {
                        ItemKind::OvalStroke { stroke, .. }
                        | ItemKind::PathStroke { stroke, .. } => Some(stroke),
                        _ => None,
                    })
            })
            .expect("non-rect primitive keeps its admitted stroke");
        assert!(matches!(
            stroke.width,
            StrokeWidth::Rectangular(widths) if widths == RectangularStrokeWidth::all(2.0)
        ));
    }
}

#[test]
fn effective_values_feed_the_same_candidate_and_locate_a_material_transition() {
    let (document, ids, identities) = scene();
    let context = paint_ctx();
    let before_product =
        frame::resolve_and_build(&document, &options(), &context).expect("valid before frame");
    let before = extract_input(&document, before_product.resolved(), &identities).unwrap();

    let values = PropertyValues::new(
        &document,
        [
            (
                property_target(&document, ids.frame, PropertyKey::Opacity),
                PropertyValue::Number(0.5),
            ),
            (
                property_target(&document, ids.frame, PropertyKey::ClipsContent),
                PropertyValue::Boolean(false),
            ),
            (
                property_target(&document, ids.rect, PropertyKey::CornerRadius),
                PropertyValue::CornerRadius(RectangularCornerRadius::circular(2.0)),
            ),
            (
                property_target(&document, ids.rect, PropertyKey::Fills),
                PropertyValue::Paints(Paints::solid(Color(0xFF25_63EB))),
            ),
            (
                property_target(&document, ids.rect, PropertyKey::Strokes),
                PropertyValue::Strokes(vec![solid_stroke(
                    0xFF1E_3A8A,
                    3.0,
                    StrokeAlign::Inside,
                    StrokeCap::Butt,
                    StrokeJoin::Miter,
                    None,
                )]),
            ),
        ],
    )
    .expect("valid immutable effective values");
    let view = ValueView::new(&document, &values).expect("validated effective view");
    let after_product = frame::resolve_and_build_view(&view, &options(), &context)
        .expect("valid effective-value frame");
    let after = extract_input(&view, after_product.resolved(), &identities).unwrap();

    assert_eq!(
        before.visuals.keys().collect::<Vec<_>>(),
        after.visuals.keys().collect::<Vec<_>>(),
        "opaque identities and provenance survive the transition"
    );
    assert_ne!(
        before.composition, after.composition,
        "effective opacity and clipping change normalized composition"
    );
    let changed = before
        .visuals
        .iter()
        .filter_map(|(visual, facts)| (after.visuals.get(visual) != Some(facts)).then_some(*visual))
        .collect::<Vec<_>>();
    assert_eq!(
        changed,
        [RECT_REF],
        "effective radius/fill/stroke changes stay on the rectangle fact"
    );

    let compiled_after = compile_vectors(&after, &identities).unwrap();
    let current_after = without_node(after_product.drawlist(), ids.text);
    assert_eq!(compiled_after.list, current_after);
    assert_eq!(
        raster_to_bytes_unchecked(
            &compiled_after.list,
            &Affine::IDENTITY,
            WIDTH,
            HEIGHT,
            &context,
        ),
        raster_to_bytes_unchecked(&current_after, &Affine::IDENTITY, WIDTH, HEIGHT, &context,),
        "effective candidate and current n0 paint are byte-identical"
    );
    assert_ne!(
        compile_vectors(&before, &identities).unwrap().list,
        compiled_after.list
    );

    let forged_provenance = VisualRef {
        identity: RECT_REF.identity,
        provenance: VisualProvenance(999),
    };
    assert_eq!(
        identities.node(forged_provenance),
        Err(EvidenceError::MissingVisualTarget(forged_provenance)),
        "provenance participates in projection; it is not side metadata"
    );
}
