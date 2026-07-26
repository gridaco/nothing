//! Source-neutral glyphless frames entering the n0 chassis.
//!
//! [`rframe::Frame`] is the backend-free resolved contract. It carries no
//! authored n0 document, HTML/CSS/SVG syntax, parser binding, backend object,
//! I/O handle, or clock. This module admits its current solid-fill
//! rectangle, ellipse, and path slice, compiles it into n0's one private
//! drawlist, and executes it through n0's one private painter.
//!
//! The resulting [`FrameProduct`] is intentionally separate from
//! [`crate::frame::FrameProduct`]. The latter owns an n0-model
//! [`n0_model::resolve::Resolved`] and its document-specific query tier; a
//! foreign resolved frame cannot honestly manufacture either.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use n0_model::math::Affine;
use n0_model::model::{
    BlendMode, Color, CornerSmoothing, Paint, Paints, RectangularCornerRadius, SolidPaint, Stroke,
    StrokeAlign, StrokeCap, StrokeJoin, StrokeWidth,
};
use n0_model::path::ResolvedPathArtifact;
use rframe::{Frame, Geometry, SolidPaintStack, VisualRef};

use crate::damage::{diff_inputs, DamageOwner, FrameDamageInput};
use crate::drawlist::{DrawList, GlyphlessOwnerSlot, Item, ItemKind};
use crate::frame::FrameExecutionError;
use crate::paint::PaintCtx;

/// Private projection from draw-item owner slots back to the contract's opaque
/// identity and provenance.
#[derive(Debug, Clone)]
struct ProvenanceProjection {
    owners: Vec<VisualRef>,
    coverage: Vec<n0_model::math::RectF>,
}

impl ProvenanceProjection {
    fn get(&self, slot: GlyphlessOwnerSlot) -> Option<(VisualRef, n0_model::math::RectF)> {
        Some((
            *self.owners.get(slot.index())?,
            *self.coverage.get(slot.index())?,
        ))
    }
}

/// A source-neutral frame failed validation or exceeded the admitted slice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildError {
    InvalidFrameBounds,
    InvalidVisualBounds(VisualRef),
    InvalidTransform(VisualRef),
    InvalidRectangle(VisualRef),
    VisualBoundsMismatch(VisualRef),
    DuplicateOwner(VisualRef),
    TooManyVisuals,
}

impl std::fmt::Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildError::InvalidFrameBounds => f.write_str("glyphless frame bounds are invalid"),
            BuildError::InvalidVisualBounds(owner) => {
                write!(f, "glyphless visual {owner:?} has invalid frame bounds")
            }
            BuildError::InvalidTransform(owner) => {
                write!(f, "glyphless visual {owner:?} has a non-finite transform")
            }
            BuildError::InvalidRectangle(owner) => {
                write!(
                    f,
                    "glyphless visual {owner:?} has invalid rectangle geometry"
                )
            }
            BuildError::VisualBoundsMismatch(owner) => write!(
                f,
                "glyphless visual {owner:?} bounds do not exactly equal its transformed geometry"
            ),
            BuildError::DuplicateOwner(owner) => {
                write!(f, "glyphless frame repeats visual owner {owner:?}")
            }
            BuildError::TooManyVisuals => {
                f.write_str("glyphless frame exceeds the private owner-slot space")
            }
        }
    }
}

impl std::error::Error for BuildError {}

/// One immutable source-neutral frame, its private compiled material, and its
/// opaque provenance projection.
///
/// The admitted solid/geometry slice is resource-free, so this product neither
/// captures nor checks a [`crate::paint::PaintEnvironmentKey`].
#[derive(Debug, Clone)]
pub struct FrameProduct {
    resolved: Frame,
    drawlist: DrawList<GlyphlessOwnerSlot>,
    provenance: ProvenanceProjection,
}

impl FrameProduct {
    pub fn resolved(&self) -> &Frame {
        &self.resolved
    }

    /// Replay through n0's one private painter. The current material is
    /// resource-free, so any paint context is valid.
    pub fn execute(
        &self,
        canvas: &skia_safe::Canvas,
        view: &math2::transform::AffineTransform,
        ctx: &PaintCtx,
    ) -> Result<(), FrameExecutionError> {
        self.assert_provenance_complete();
        crate::paint::execute_unchecked(canvas, &self.drawlist, &to_affine(*view), ctx);
        Ok(())
    }

    /// Produce CPU-raster bytes through n0's one private painter. The neutral
    /// view transform is converted only inside this engine boundary.
    pub fn raster_to_bytes(
        &self,
        view: &math2::transform::AffineTransform,
        w: i32,
        h: i32,
        ctx: &PaintCtx,
    ) -> Result<Vec<u8>, FrameExecutionError> {
        self.assert_provenance_complete();
        Ok(crate::paint::raster_to_bytes_unchecked(
            &self.drawlist,
            &to_affine(*view),
            w,
            h,
            ctx,
        ))
    }

    fn assert_provenance_complete(&self) {
        debug_assert!(self
            .drawlist
            .items
            .iter()
            .all(|item| self.provenance.get(item.node).is_some()));
    }
}

/// Pixel-affecting change between two source-neutral frame products.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Damage {
    pub changed: Vec<VisualRef>,
    pub union_frame: Option<math2::Rectangle>,
}

impl Damage {
    pub fn is_empty(&self) -> bool {
        self.changed.is_empty()
    }
}

/// Diff two compiled glyphless products through n0's existing exact
/// complete-frame damage policy.
pub fn diff_frame(prev: &FrameProduct, next: &FrameProduct) -> Damage {
    let prev = damage_input(prev);
    let next = damage_input(next);
    let damage = diff_inputs(&prev, &next);
    Damage {
        changed: damage.changed,
        union_frame: damage.union_world.map(from_rectf),
    }
}

fn damage_input(product: &FrameProduct) -> FrameDamageInput<'_, VisualRef, (), GlyphlessOwnerSlot> {
    let mut owners = BTreeMap::new();
    let item_owners = product
        .drawlist
        .items
        .iter()
        .map(|item| {
            let (owner, coverage) = product
                .provenance
                .get(item.node)
                .expect("compiled item has complete provenance");
            owners
                .entry(owner)
                .or_insert_with(|| DamageOwner::new((), Some(coverage)));
            owner
        })
        .collect::<Vec<_>>();
    FrameDamageInput::resource_free(owners, item_owners, &product.drawlist)
}

/// Compile one already-resolved glyphless frame into the n0 chassis.
///
/// Validation and private drawlist construction complete before the immutable
/// product is returned. No raster command is issued here. The current admitted
/// slice is rectangles, ellipses (each carried as its local-space bounding
/// rectangle) and paths, ordinary solid `cg` paints, a centred stroke over the
/// fill, and the frame-bounds clip.
pub fn compile(resolved: Frame) -> Result<FrameProduct, BuildError> {
    validate_rect(resolved.bounds).map_err(|_| BuildError::InvalidFrameBounds)?;
    let owner_count = resolved
        .nodes
        .len()
        .checked_add(1)
        .ok_or(BuildError::TooManyVisuals)?;
    u32::try_from(owner_count).map_err(|_| BuildError::TooManyVisuals)?;

    let mut unique = BTreeSet::new();
    if !unique.insert(resolved.owner) {
        return Err(BuildError::DuplicateOwner(resolved.owner));
    }
    let mut provenance = ProvenanceProjection {
        owners: Vec::with_capacity(owner_count),
        coverage: Vec::with_capacity(owner_count),
    };
    provenance.owners.push(resolved.owner);
    provenance.coverage.push(to_rectf(resolved.bounds));

    let frame_owner = GlyphlessOwnerSlot::new(0);
    let frame_world = Affine::translate(resolved.bounds.x, resolved.bounds.y);
    let mut items = vec![Item {
        node: frame_owner,
        world: frame_world,
        kind: ItemKind::BeginClipRect {
            w: resolved.bounds.width,
            h: resolved.bounds.height,
            corner_radius: RectangularCornerRadius::default(),
            corner_smoothing: CornerSmoothing::default(),
        },
    }];

    for node in &resolved.nodes {
        if !unique.insert(node.owner) {
            return Err(BuildError::DuplicateOwner(node.owner));
        }
        validate_rect(node.bounds).map_err(|_| BuildError::InvalidVisualBounds(node.owner))?;
        validate_transform(node.transform).map_err(|_| BuildError::InvalidTransform(node.owner))?;
        let rect = node.geometry.local_box();
        validate_rect(rect).map_err(|_| BuildError::InvalidRectangle(node.owner))?;
        if node.bounds != math2::rect_transform(rect, &node.transform) {
            return Err(BuildError::VisualBoundsMismatch(node.owner));
        }
        let paints = compile_paints(&node.paints);
        let owner = GlyphlessOwnerSlot::new(
            u32::try_from(provenance.owners.len()).expect("owner count checked above"),
        );
        provenance.owners.push(node.owner);
        // A stroke paints outside the geometry, so the covered area — what the
        // damage policy repaints — is the node's bounds inflated by the
        // stroke's own reach, mapped through the same transform.
        provenance.coverage.push(to_rectf(match &node.stroke {
            None => node.bounds,
            Some(stroke) => {
                math2::rect_transform(math2::rect_inset(rect, -stroke.outset()), &node.transform)
            }
        }));

        // A box primitive draws at its item's origin, so its own local offset
        // enters the world transform. A path carries absolute local
        // coordinates instead: its stream is the geometry, and translating it
        // would be a second coordinate mapping over values the contract has
        // already resolved.
        let world = match &node.geometry {
            Geometry::Rect(_) | Geometry::Ellipse(_) => {
                to_affine(node.transform).then(&Affine::translate(rect.x, rect.y))
            }
            Geometry::Path(_) => to_affine(node.transform),
        };
        // The paint reference box is the geometry's own extent. For a path its
        // origin coincides with local space, which only a non-solid paint could
        // observe — and the admitted slice has none, so the rung that admits
        // one decides how the box travels.
        let (w, h) = (rect.width, rect.height);
        let path = match &node.geometry {
            Geometry::Path(path) => Some(compile_path(path)),
            _ => None,
        };
        if !paints.is_empty() {
            let kind = match &node.geometry {
                Geometry::Rect(_) => ItemKind::RectFill {
                    w,
                    h,
                    corner_radius: RectangularCornerRadius::default(),
                    corner_smoothing: CornerSmoothing::default(),
                    paints,
                },
                Geometry::Ellipse(_) => ItemKind::OvalFill { w, h, paints },
                Geometry::Path(_) => ItemKind::PathFill {
                    w,
                    h,
                    path: Arc::clone(path.as_ref().expect("path geometry compiled its stream")),
                    paints,
                },
            };
            items.push(Item {
                node: owner,
                world,
                kind,
            });
        }
        // SVG's default paint order is fill, then stroke — one item after the
        // other in the same private drawlist, which is why a stroke needs no
        // group scope.
        if let Some(stroke) = &node.stroke {
            let stroke = compile_stroke(stroke);
            let kind = match &node.geometry {
                Geometry::Rect(_) => ItemKind::RectStroke {
                    w,
                    h,
                    corner_radius: RectangularCornerRadius::default(),
                    corner_smoothing: CornerSmoothing::default(),
                    stroke,
                },
                Geometry::Ellipse(_) => ItemKind::OvalStroke { w, h, stroke },
                Geometry::Path(_) => ItemKind::PathStroke {
                    w,
                    h,
                    path: Arc::clone(path.as_ref().expect("path geometry compiled its stream")),
                    stroke,
                },
            };
            items.push(Item {
                node: owner,
                world,
                kind,
            });
        }
    }

    items.push(Item {
        node: frame_owner,
        world: frame_world,
        kind: ItemKind::EndClip,
    });
    Ok(FrameProduct {
        resolved,
        drawlist: DrawList::from_items(items),
        provenance,
    })
}

/// Project the contract's checked command stream into the engine's resolved
/// path material.
///
/// The engine model's *authored* path type keeps its geometry normalized into
/// a unit reference box and multiplies out at resolve time. A resolved
/// contract has no authored box and no such indirection: its coordinates are
/// already final in the node's local space, so they enter the resolved
/// artifact unchanged. Normalizing them into a unit box and multiplying back
/// would put a divide and a multiply between the producer's numbers and the
/// rasterizer's — different pixels for no gain.
///
/// Nothing is re-derived here: [`rframe::PathData`] is a checked type whose
/// construction resolved the bounds and the closed-contour fact from these
/// same commands.
fn compile_path(data: &rframe::PathData) -> Arc<ResolvedPathArtifact> {
    let commands = data
        .commands()
        .iter()
        .map(|command| match *command {
            rframe::PathCommand::MoveTo { x, y } => n0_model::path::PathCommand::MoveTo { x, y },
            rframe::PathCommand::LineTo { x, y } => n0_model::path::PathCommand::LineTo { x, y },
            rframe::PathCommand::QuadTo { x1, y1, x, y } => {
                n0_model::path::PathCommand::QuadTo { x1, y1, x, y }
            }
            rframe::PathCommand::CubicTo {
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => n0_model::path::PathCommand::CubicTo {
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            },
            rframe::PathCommand::Close => n0_model::path::PathCommand::Close,
        })
        .collect::<Vec<_>>();
    let fill_rule = match data.fill_rule() {
        rframe::FillRule::NonZero => n0_model::path::FillRule::NonZero,
        rframe::FillRule::EvenOdd => n0_model::path::FillRule::EvenOdd,
    };
    Arc::new(ResolvedPathArtifact {
        commands: commands.into(),
        fill_rule,
        local_bounds: to_rectf(data.local_bounds()),
        all_contours_closed: data.all_contours_closed(),
    })
}

/// Project the contract's resolved stroke into the engine's private stroke.
///
/// The contract's stroke is centred on the geometry, which is the only
/// alignment a Web source can express; the engine's own vocabulary carries an
/// alignment, so the projection names it rather than relying on a default.
/// Width is uniform because a Web stroke has one width, and the dash array is
/// absent because the producer refuses a dashed stroke.
fn compile_stroke(stroke: &rframe::Stroke) -> Stroke {
    Stroke {
        paints: compile_paints(stroke.paints()),
        width: StrokeWidth::Uniform(stroke.width()),
        align: StrokeAlign::Center,
        cap: match stroke.cap() {
            rframe::StrokeCap::Butt => StrokeCap::Butt,
            rframe::StrokeCap::Round => StrokeCap::Round,
            rframe::StrokeCap::Square => StrokeCap::Square,
        },
        join: match stroke.join() {
            rframe::StrokeJoin::Miter => StrokeJoin::Miter,
            rframe::StrokeJoin::Round => StrokeJoin::Round,
            rframe::StrokeJoin::Bevel => StrokeJoin::Bevel,
        },
        miter_limit: stroke.miter_limit(),
        dash_array: None,
    }
}

fn validate_rect(rect: math2::Rectangle) -> Result<(), ()> {
    [rect.x, rect.y, rect.width, rect.height]
        .into_iter()
        .all(f32::is_finite)
        .then_some(())
        .filter(|_| rect.width >= 0.0 && rect.height >= 0.0)
        .ok_or(())
}

fn validate_transform(transform: math2::transform::AffineTransform) -> Result<(), ()> {
    transform
        .matrix
        .into_iter()
        .flatten()
        .all(f32::is_finite)
        .then_some(())
        .ok_or(())
}

fn to_affine(transform: math2::transform::AffineTransform) -> Affine {
    let [[a, c, e], [b, d, f]] = transform.matrix;
    Affine { a, b, c, d, e, f }
}

fn compile_paints(paints: &SolidPaintStack) -> Paints {
    let mut compiled = Vec::with_capacity(paints.len());
    for solid in paints.iter() {
        let color = solid.color;
        let argb = (u32::from(color.a()) << 24)
            | (u32::from(color.r()) << 16)
            | (u32::from(color.g()) << 8)
            | u32::from(color.b());
        let paint = Paint::Solid(SolidPaint {
            active: solid.active,
            color: Color(argb),
            blend_mode: BlendMode::Normal,
        });
        compiled.push(paint);
    }
    Paints::new(compiled)
}

fn to_rectf(rect: math2::Rectangle) -> n0_model::math::RectF {
    n0_model::math::RectF {
        x: rect.x,
        y: rect.y,
        w: rect.width,
        h: rect.height,
    }
}

fn from_rectf(rect: n0_model::math::RectF) -> math2::Rectangle {
    math2::Rectangle::from_xywh(rect.x, rect.y, rect.w, rect.h)
}

#[cfg(test)]
mod tests {
    use cg::{CGColor, Paint as CgPaint, Paints as CgPaints, SolidPaint as CgSolidPaint};
    use math2::transform::AffineTransform;
    use math2::Rectangle;
    use n0_model::model::{
        AxisBinding, DocBuilder, Header, LayoutBehavior, Paints as ModelPaints, Payload, ShapeDesc,
        SizeIntent,
    };
    use n0_model::resolve::ResolveOptions;
    use rframe::{FrameNode, Identity, Provenance};
    use skia_safe::surfaces;

    use super::*;

    const FRAME_OWNER: VisualRef = VisualRef::new(Identity::new(10), Provenance::new(100));
    const RECT_OWNER: VisualRef = VisualRef::new(Identity::new(20), Provenance::new(200));

    fn cg_solid(argb: u32) -> CgPaint {
        CgPaint::Solid(CgSolidPaint::new_color(CGColor::from_u32_argb(argb)))
    }

    fn solid_stack<const N: usize>(paints: [CgPaint; N]) -> SolidPaintStack {
        SolidPaintStack::try_from_paints(CgPaints::new(paints))
            .expect("test paints are visible ordinary solids")
    }

    fn resolved_frame(paints: SolidPaintStack) -> Frame {
        Frame {
            owner: FRAME_OWNER,
            bounds: Rectangle::from_xywh(0.0, 0.0, 64.0, 48.0),
            nodes: vec![FrameNode {
                owner: RECT_OWNER,
                transform: AffineTransform::identity(),
                geometry: Geometry::Rect(Rectangle::from_xywh(8.0, 6.0, 20.0, 16.0)),
                bounds: Rectangle::from_xywh(8.0, 6.0, 20.0, 16.0),
                paints,
                stroke: None,
            }],
        }
    }

    fn ordinary_frame(paints: ModelPaints) -> n0_model::model::Document {
        let mut builder = DocBuilder::new();
        let frame = builder.add(
            0,
            Header::new(SizeIntent::Fixed(64.0), SizeIntent::Fixed(48.0)),
            Payload::Frame {
                layout: LayoutBehavior::default(),
                clips_content: true,
            },
        );
        let mut header = Header::new(SizeIntent::Fixed(20.0), SizeIntent::Fixed(16.0));
        header.x = AxisBinding::start(8.0);
        header.y = AxisBinding::start(6.0);
        let rect = builder.add(
            frame,
            header,
            Payload::Shape {
                desc: ShapeDesc::Rect,
            },
        );
        builder.node_mut(rect).fills = paints;
        builder.build()
    }

    #[test]
    fn resolved_rect_uses_the_ordinary_private_material_without_an_environment_key() {
        let lower = 0x8012_3456;
        let upper = 0xFFA1_B2C3;
        let context = PaintCtx::new(None);
        let resolved = resolved_frame(solid_stack([cg_solid(lower), cg_solid(upper)]));
        let product = compile(resolved.clone()).expect("admitted glyphless frame");
        let ordinary = ordinary_frame(ModelPaints::new([
            Paint::Solid(n0_model::model::SolidPaint::new(Color(lower))),
            Paint::Solid(n0_model::model::SolidPaint::new(Color(upper))),
        ]));
        let ordinary = crate::frame::resolve_and_build(
            &ordinary,
            &ResolveOptions {
                viewport: (64.0, 48.0),
                ..Default::default()
            },
            &context,
        )
        .expect("equivalent ordinary frame");

        assert_eq!(product.resolved(), &resolved);
        assert!(
            product.drawlist.raster_eq(ordinary.drawlist()),
            "owner domains differ, but every paint-consumed drawlist field is exact"
        );
        let ItemKind::RectFill { paints, .. } = &product.drawlist.items[1].kind else {
            panic!("second item is the rectangle fill");
        };
        assert_eq!(
            paints
                .iter()
                .map(|paint| match paint {
                    Paint::Solid(solid) => solid.color.0,
                    _ => panic!("the admitted slice is solid-only"),
                })
                .collect::<Vec<_>>(),
            [lower, upper],
            "cg stack order and canonical AARRGGBB words survive compilation"
        );

        let neutral_view = AffineTransform::identity();
        let pixels = product
            .raster_to_bytes(&neutral_view, 64, 48, &context)
            .expect("resource-free glyphless raster");
        assert_eq!(
            pixels,
            ordinary
                .raster_to_bytes(&Affine::IDENTITY, 64, 48, &context)
                .expect("checked ordinary raster")
        );
        assert_eq!(
            pixels,
            product
                .raster_to_bytes(&neutral_view, 64, 48, &context)
                .expect("deterministic repeat")
        );

        let other = PaintCtx::new(None);
        let mut surface = surfaces::raster_n32_premul((64, 48)).unwrap();
        product
            .execute(surface.canvas(), &neutral_view, &other)
            .expect("resource-free product accepts any context");
        assert_eq!(
            pixels,
            product
                .raster_to_bytes(&neutral_view, 64, 48, &other)
                .expect("another context has no effect on solid geometry")
        );

        let error = ordinary
            .execute(surface.canvas(), &Affine::IDENTITY, &other)
            .expect_err("ordinary product retains its exact environment check");
        let FrameExecutionError::Environment(error) = error else {
            panic!("expected environment mismatch");
        };
        assert_eq!(error.expected, ordinary.environment());
        assert_eq!(error.actual, other.environment_key());
    }

    #[test]
    fn transformed_geometry_requires_exact_contract_bounds() {
        let mut frame = resolved_frame(SolidPaintStack::solid(CGColor::RED));
        frame.nodes[0].transform = AffineTransform::new(3.0, 4.0, 0.0);
        let expected = math2::rect_transform(
            frame.nodes[0].geometry.local_box(),
            &frame.nodes[0].transform,
        );
        frame.nodes[0].bounds = Rectangle {
            x: f32::from_bits(expected.x.to_bits() + 1),
            ..expected
        };
        assert!(matches!(
            compile(frame.clone()),
            Err(BuildError::VisualBoundsMismatch(RECT_OWNER))
        ));

        frame.nodes[0].bounds = expected;
        compile(frame).expect("exact transformed bounds are admitted");
    }

    /// An ellipse node admits only bounds that exactly equal its transformed
    /// local-space rectangle; a bit-nudged contract bound fails loudly.
    #[test]
    fn transformed_ellipse_requires_exact_contract_bounds() {
        let bbox = Rectangle::from_xywh(8.0, 6.0, 20.0, 16.0);
        let mut frame = resolved_frame(SolidPaintStack::solid(CGColor::RED));
        frame.nodes[0].geometry = Geometry::Ellipse(bbox);
        frame.nodes[0].transform = AffineTransform::new(3.0, 4.0, 0.0);
        let expected = math2::rect_transform(bbox, &frame.nodes[0].transform);
        frame.nodes[0].bounds = Rectangle {
            x: f32::from_bits(expected.x.to_bits() + 1),
            ..expected
        };
        assert!(matches!(
            compile(frame.clone()),
            Err(BuildError::VisualBoundsMismatch(RECT_OWNER))
        ));

        frame.nodes[0].bounds = expected;
        compile(frame).expect("exact transformed ellipse bounds are admitted");
    }

    /// An ellipse compiles to the oval fill inscribed in its local-space
    /// rectangle: the box center rasters solid paint, the box corners stay at
    /// the surface clear color, and a repeat raster is byte-identical.
    #[test]
    fn ellipse_geometry_rasters_the_inscribed_oval() {
        let context = PaintCtx::new(None);
        let mut frame = resolved_frame(SolidPaintStack::solid(CGColor::from_rgb(0x16, 0xa3, 0x4a)));
        frame.nodes[0].geometry = Geometry::Ellipse(Rectangle::from_xywh(8.0, 6.0, 20.0, 16.0));
        let product = compile(frame).expect("admitted glyphless ellipse frame");
        let ItemKind::OvalFill { w, h, .. } = &product.drawlist.items[1].kind else {
            panic!("second item is the oval fill");
        };
        assert_eq!((*w, *h), (20.0, 16.0));

        let neutral_view = AffineTransform::identity();
        let pixels = product
            .raster_to_bytes(&neutral_view, 64, 48, &context)
            .expect("resource-free glyphless raster");
        let at = |x: i32, y: i32| -> [u8; 4] {
            let offset = ((y * 64 + x) * 4) as usize;
            pixels[offset..offset + 4].try_into().expect("RGBA pixel")
        };
        assert_eq!(
            at(18, 14),
            [0x16, 0xa3, 0x4a, 0xff],
            "the oval covers its bounding-box center"
        );
        assert_eq!(
            at(9, 7),
            [0xff, 0xff, 0xff, 0xff],
            "the oval leaves its bounding-box corner at the surface clear color"
        );
        assert_eq!(
            pixels,
            product
                .raster_to_bytes(&neutral_view, 64, 48, &context)
                .expect("deterministic repeat")
        );
    }

    #[test]
    fn duplicate_owners_fail_explicitly() {
        let duplicate = Frame {
            owner: FRAME_OWNER,
            bounds: Rectangle::from_xywh(0.0, 0.0, 64.0, 48.0),
            nodes: vec![FrameNode {
                owner: FRAME_OWNER,
                transform: AffineTransform::identity(),
                geometry: Geometry::Rect(Rectangle::from_xywh(0.0, 0.0, 8.0, 8.0)),
                bounds: Rectangle::from_xywh(0.0, 0.0, 8.0, 8.0),
                paints: SolidPaintStack::solid(CGColor::RED),
                stroke: None,
            }],
        };
        assert!(matches!(
            compile(duplicate),
            Err(BuildError::DuplicateOwner(FRAME_OWNER))
        ));
    }

    #[test]
    fn frame_diff_reuses_the_complete_generic_damage_policy() {
        let before = compile(resolved_frame(SolidPaintStack::solid(CGColor::RED))).expect("before");
        let after = compile(resolved_frame(SolidPaintStack::solid(CGColor::BLUE))).expect("after");

        assert_eq!(
            diff_frame(&before, &after),
            Damage {
                changed: vec![RECT_OWNER],
                union_frame: Some(Rectangle::from_xywh(8.0, 6.0, 20.0, 16.0)),
            }
        );
        assert!(diff_frame(&before, &before).is_empty());
    }
}
