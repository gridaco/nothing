//! Source-neutral glyphless frames entering the n0 chassis.
//!
//! [`rframe::Frame`] is the backend-free resolved contract. It carries no
//! authored n0 document, HTML/CSS/SVG syntax, parser binding, backend object,
//! I/O handle, or clock. This module admits its current solid-rectangle slice,
//! compiles it into n0's one private drawlist, and executes it through n0's one
//! private painter.
//!
//! The resulting [`FrameProduct`] is intentionally separate from
//! [`crate::frame::FrameProduct`]. The latter owns an n0-model
//! [`n0_model::resolve::Resolved`] and its document-specific query tier; a
//! foreign resolved frame cannot honestly manufacture either.

use std::collections::{BTreeMap, BTreeSet};

use n0_model::math::Affine;
use n0_model::model::{
    BlendMode, Color, CornerSmoothing, Paint, Paints, RectangularCornerRadius, SolidPaint,
};
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
/// slice is rectangles, ordinary solid `cg` paints, and the frame-bounds clip.
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
        let Geometry::Rect(rect) = node.geometry;
        validate_rect(rect).map_err(|_| BuildError::InvalidRectangle(node.owner))?;
        if node.bounds != math2::rect_transform(rect, &node.transform) {
            return Err(BuildError::VisualBoundsMismatch(node.owner));
        }
        let paints = compile_paints(&node.paints);
        let owner = GlyphlessOwnerSlot::new(
            u32::try_from(provenance.owners.len()).expect("owner count checked above"),
        );
        provenance.owners.push(node.owner);
        provenance.coverage.push(to_rectf(node.bounds));
        if paints.is_empty() {
            continue;
        }

        let world = to_affine(node.transform).then(&Affine::translate(rect.x, rect.y));
        items.push(Item {
            node: owner,
            world,
            kind: ItemKind::RectFill {
                w: rect.width,
                h: rect.height,
                corner_radius: RectangularCornerRadius::default(),
                corner_smoothing: CornerSmoothing::default(),
                paints,
            },
        });
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
            match frame.nodes[0].geometry {
                Geometry::Rect(rect) => rect,
            },
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
