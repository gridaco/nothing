//! Source-neutral glyphless frames entering the n0 chassis.
//!
//! [`rframe::Frame`] is the backend-free resolved contract. It carries no
//! authored n0 document, HTML/CSS/SVG syntax, parser binding, backend object,
//! I/O handle, or clock. This module admits its current solid- and
//! gradient-filled rectangle, ellipse, and path slice, compiles it into n0's
//! one private drawlist, and executes it through n0's one private painter.
//!
//! The resulting [`FrameProduct`] is intentionally separate from
//! [`crate::frame::FrameProduct`]. The latter owns an n0-model
//! [`n0_model::resolve::Resolved`] and its document-specific query tier; a
//! foreign resolved frame cannot honestly manufacture either.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use cg::Paint as CgPaint;
use n0_model::math::Affine;
use n0_model::model::{
    BlendMode, Color, CornerSmoothing, Paint, Paints, RectangularCornerRadius, SolidPaint, Stroke,
    StrokeAlign, StrokeCap, StrokeJoin, StrokeWidth,
};
use n0_model::path::ResolvedPathArtifact;
use rframe::{Frame, FrameItem, Geometry, PaintStack, ScopeEffect, VisualRef};

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
    /// A gradient in the admitted paint stack failed the engine's paint
    /// preflight for its resolved paint box, so executing it would reach a
    /// backend construction the painter treats as already proven.
    Paint {
        owner: VisualRef,
        reason: String,
    },
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
            BuildError::Paint { owner, reason } => {
                write!(
                    f,
                    "glyphless visual {owner:?} paint preflight failed: {reason}"
                )
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
/// rectangle) and paths, the contract's admitted `cg` paints (solids, linear
/// and radial gradients — every gradient preflighted against its resolved
/// paint box before the product exists), a centred stroke over the fill,
/// isolated opacity scopes, and the frame-bounds clip.
///
/// The contract's item stream is a checked type ([`rframe::FrameItems`]):
/// balance, non-emptiness, and bounded nesting were proven at construction,
/// so this compile trusts them the way it trusts [`rframe::PathData`] —
/// nothing is re-derived, and the scope walk uses plain `expect`s.
pub fn compile(resolved: Frame) -> Result<FrameProduct, BuildError> {
    validate_rect(resolved.bounds).map_err(|_| BuildError::InvalidFrameBounds)?;
    let owner_count = resolved
        .items
        .iter()
        .filter(|item| !matches!(item, FrameItem::ScopeEnd))
        .count()
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

    // Open scopes: each entry is the scope's owner slot and the union of
    // the coverage that has accumulated inside it so far. A scope's damage
    // coverage is that union — an opacity edit repaints everything the
    // scope composites — and a child scope's union folds into its parent's
    // when it closes.
    let mut open_scopes: Vec<(GlyphlessOwnerSlot, Option<n0_model::math::RectF>)> = Vec::new();

    for frame_item in resolved.items.iter() {
        let node = match frame_item {
            FrameItem::Node(node) => node,
            FrameItem::ScopeBegin(scope) => {
                if !unique.insert(scope.owner) {
                    return Err(BuildError::DuplicateOwner(scope.owner));
                }
                let slot = GlyphlessOwnerSlot::new(
                    u32::try_from(provenance.owners.len()).expect("owner count checked above"),
                );
                provenance.owners.push(scope.owner);
                // Placeholder until the scope closes and its union is known.
                provenance.coverage.push(to_rectf(resolved.bounds));
                let ScopeEffect::Opacity(opacity) = scope.effect;
                items.push(Item {
                    node: slot,
                    world: frame_world,
                    kind: ItemKind::BeginIsolatedOpacity {
                        opacity: opacity.get(),
                    },
                });
                open_scopes.push((slot, None));
                continue;
            }
            FrameItem::ScopeEnd => {
                let (slot, coverage) = open_scopes.pop().expect("checked stream is balanced");
                let coverage = coverage.expect("checked stream has no empty scope");
                provenance.coverage[slot.index()] = coverage;
                if let Some((_, parent)) = open_scopes.last_mut() {
                    *parent = Some(match parent {
                        None => coverage,
                        Some(parent) => union_rectf(*parent, coverage),
                    });
                }
                items.push(Item {
                    node: slot,
                    world: frame_world,
                    kind: ItemKind::EndOpacity,
                });
                continue;
            }
        };
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
        // The paint reference box is the geometry's own extent. A box primitive
        // draws at its item's origin, so its paint box already starts there. A
        // path's stream carries absolute local coordinates, so its box starts
        // at the tight-bounds origin instead — the painter's unit box does not,
        // and the difference is observable only by a non-solid paint. The
        // origin travels as a unit-space pre-translate on each gradient's
        // transform: box(x,y,w,h) × T = box(0,0,w,h) × translate(x/w, y/h) × T.
        // A degenerate axis skips the fold; the producer resolves or refuses
        // gradients on degenerate geometry before they reach this compile.
        let unit_offset = match &node.geometry {
            Geometry::Path(_) if rect.width > 0.0 && rect.height > 0.0 => {
                Some((rect.x / rect.width, rect.y / rect.height))
            }
            _ => None,
        };
        let paints = compile_paints(&node.paints, unit_offset);
        let owner = GlyphlessOwnerSlot::new(
            u32::try_from(provenance.owners.len()).expect("owner count checked above"),
        );
        provenance.owners.push(node.owner);
        // A stroke paints outside the geometry, so the covered area — what the
        // damage policy repaints — is the node's bounds inflated by the
        // stroke's own reach, mapped through the same transform.
        let coverage = to_rectf(match &node.stroke {
            None => node.bounds,
            Some(stroke) => {
                math2::rect_transform(math2::rect_inset(rect, -stroke.outset()), &node.transform)
            }
        });
        provenance.coverage.push(coverage);
        if let Some((_, accumulated)) = open_scopes.last_mut() {
            *accumulated = Some(match accumulated {
                None => coverage,
                Some(accumulated) => union_rectf(*accumulated, coverage),
            });
        }

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
            let stroke = compile_stroke(stroke, unit_offset);
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
    let drawlist = DrawList::from_items(items);
    // The painter treats gradient shader construction as already proven
    // (`execute_unchecked` expects), so the same preflight the native route
    // runs must pass here before the product exists. The slot maps back to
    // the contract's opaque owner; slot zero is the frame itself and carries
    // no paints, so a preflight failure always names a visual.
    if let Err(error) = crate::paint::preflight_gradients(&drawlist) {
        let owner = *provenance
            .owners
            .get(error.node.index())
            .expect("preflighted item slots are minted from the owner table");
        return Err(BuildError::Paint {
            owner,
            reason: error.reason.to_string(),
        });
    }
    Ok(FrameProduct {
        resolved,
        drawlist,
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
            // rframe checked the weight's positive finite domain at
            // construction; the model's own validation re-states the same
            // domain, so this projection cannot manufacture a refusal.
            rframe::PathCommand::ConicTo {
                x1,
                y1,
                x,
                y,
                weight,
            } => n0_model::path::PathCommand::ConicTo {
                x1,
                y1,
                x,
                y,
                weight,
            },
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
/// Width is uniform because a Web stroke has one width. A checked dash cycle
/// crosses unchanged: its intervals are already even, finite, non-negative,
/// positive-sum local-space distances, so the private painter has nothing to
/// parse or resolve again.
fn compile_stroke(stroke: &rframe::Stroke, unit_offset: Option<(f32, f32)>) -> Stroke {
    Stroke {
        paints: compile_paints(stroke.paints(), unit_offset),
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
        dash_array: stroke
            .dash_intervals()
            .map(|intervals| intervals.as_slice().to_vec()),
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

fn compile_color(color: cg::CGColor) -> Color {
    Color(
        (u32::from(color.a()) << 24)
            | (u32::from(color.r()) << 16)
            | (u32::from(color.g()) << 8)
            | u32::from(color.b()),
    )
}

fn compile_gradient_stops(stops: &[cg::GradientStop]) -> Vec<n0_model::model::GradientStop> {
    stops
        .iter()
        .map(|stop| n0_model::model::GradientStop {
            offset: stop.offset,
            color: compile_color(stop.color),
        })
        .collect()
}

fn compile_tile_mode(mode: cg::TileMode) -> n0_model::model::TileMode {
    match mode {
        cg::TileMode::Clamp => n0_model::model::TileMode::Clamp,
        cg::TileMode::Repeated => n0_model::model::TileMode::Repeated,
        cg::TileMode::Mirror => n0_model::model::TileMode::Mirror,
        cg::TileMode::Decal => n0_model::model::TileMode::Decal,
    }
}

/// Project a contract gradient transform into the engine's unit space,
/// pre-translating by the item's paint-box origin when the geometry does not
/// start at the item origin (paths). Both transforms compose in the unit
/// square, so the origin enters as a plain offset on the translation column.
fn compile_gradient_transform(
    transform: &math2::transform::AffineTransform,
    unit_offset: Option<(f32, f32)>,
) -> Affine {
    let mut affine = to_affine(*transform);
    if let Some((u, v)) = unit_offset {
        affine.e += u;
        affine.f += v;
    }
    affine
}

fn compile_paints(paints: &PaintStack, unit_offset: Option<(f32, f32)>) -> Paints {
    let mut compiled = Vec::with_capacity(paints.len());
    for paint in paints.iter() {
        let paint = match paint {
            CgPaint::Solid(solid) => Paint::Solid(SolidPaint {
                active: solid.active,
                color: compile_color(solid.color),
                blend_mode: BlendMode::Normal,
            }),
            CgPaint::LinearGradient(gradient) => {
                Paint::LinearGradient(n0_model::model::LinearGradientPaint {
                    active: gradient.active,
                    xy1: n0_model::model::Alignment(gradient.xy1.0, gradient.xy1.1),
                    xy2: n0_model::model::Alignment(gradient.xy2.0, gradient.xy2.1),
                    tile_mode: compile_tile_mode(gradient.tile_mode),
                    transform: compile_gradient_transform(&gradient.transform, unit_offset),
                    stops: compile_gradient_stops(&gradient.stops),
                    opacity: gradient.opacity,
                    blend_mode: BlendMode::Normal,
                })
            }
            CgPaint::RadialGradient(gradient) => {
                Paint::RadialGradient(n0_model::model::RadialGradientPaint {
                    active: gradient.active,
                    transform: compile_gradient_transform(&gradient.transform, unit_offset),
                    stops: compile_gradient_stops(&gradient.stops),
                    opacity: gradient.opacity,
                    blend_mode: BlendMode::Normal,
                    tile_mode: compile_tile_mode(gradient.tile_mode),
                })
            }
            _ => unreachable!("PaintStack construction closes the variant set"),
        };
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

fn union_rectf(a: n0_model::math::RectF, b: n0_model::math::RectF) -> n0_model::math::RectF {
    let x = a.x.min(b.x);
    let y = a.y.min(b.y);
    n0_model::math::RectF {
        x,
        y,
        w: (a.x + a.w).max(b.x + b.w) - x,
        h: (a.y + a.h).max(b.y + b.h) - y,
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
    use rframe::{FrameItems, FrameNode, Identity, Provenance, Scope, ScopeOpacity};
    use skia_safe::surfaces;

    use super::*;

    const FRAME_OWNER: VisualRef = VisualRef::new(Identity::new(10), Provenance::new(100));
    const RECT_OWNER: VisualRef = VisualRef::new(Identity::new(20), Provenance::new(200));
    const SCOPE_OWNER: VisualRef = VisualRef::new(Identity::new(30), Provenance::new(300));
    const OTHER_OWNER: VisualRef = VisualRef::new(Identity::new(40), Provenance::new(400));

    fn cg_solid(argb: u32) -> CgPaint {
        CgPaint::Solid(CgSolidPaint::new_color(CGColor::from_u32_argb(argb)))
    }

    fn solid_stack<const N: usize>(paints: [CgPaint; N]) -> PaintStack {
        PaintStack::try_from_paints(CgPaints::new(paints))
            .expect("test paints are visible ordinary solids")
    }

    fn dashed_stroke(intervals: Vec<f32>, cap: rframe::StrokeCap) -> rframe::Stroke {
        let intervals = rframe::StrokeDashIntervals::new(intervals)
            .expect("test dash intervals are valid")
            .expect("test dash cycle is present");
        rframe::Stroke::new_with_dash_intervals(
            PaintStack::solid(CGColor::BLACK),
            8.0,
            cap,
            rframe::StrokeJoin::Miter,
            4.0,
            Some(intervals),
        )
        .expect("test stroke is valid")
        .expect("test stroke paints")
    }

    fn dashed_node(owner: VisualRef, geometry: Geometry, intervals: Vec<f32>) -> FrameNode {
        let bounds = geometry.local_box();
        FrameNode {
            owner,
            transform: AffineTransform::identity(),
            geometry,
            bounds,
            paints: PaintStack::empty(),
            stroke: Some(dashed_stroke(intervals, rframe::StrokeCap::Round)),
        }
    }

    fn rgba_at(pixels: &[u8], width: i32, x: i32, y: i32) -> [u8; 4] {
        let offset = ((y * width + x) * 4) as usize;
        pixels[offset..offset + 4].try_into().expect("RGBA pixel")
    }

    fn base_node(paints: PaintStack) -> FrameNode {
        let rect = Rectangle::from_xywh(8.0, 6.0, 20.0, 16.0);
        FrameNode {
            owner: RECT_OWNER,
            transform: AffineTransform::identity(),
            geometry: Geometry::Rect(rect),
            bounds: rect,
            paints,
            stroke: None,
        }
    }

    fn frame_of(items: FrameItems) -> Frame {
        Frame {
            owner: FRAME_OWNER,
            bounds: Rectangle::from_xywh(0.0, 0.0, 64.0, 48.0),
            items,
        }
    }

    fn resolved_frame(paints: PaintStack) -> Frame {
        frame_of(FrameItems::from_nodes(vec![base_node(paints)]))
    }

    fn scope_begin(owner: VisualRef, opacity: f32) -> FrameItem {
        FrameItem::ScopeBegin(Scope {
            owner,
            effect: ScopeEffect::Opacity(
                ScopeOpacity::new(opacity).expect("test opacity is a scope fact"),
            ),
        })
    }

    fn rect_node(owner: VisualRef, rect: Rectangle, argb: u32) -> FrameNode {
        FrameNode {
            owner,
            transform: AffineTransform::identity(),
            geometry: Geometry::Rect(rect),
            bounds: rect,
            paints: PaintStack::solid(CGColor::from_u32_argb(argb)),
            stroke: None,
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

    /// The source-neutral seam copies a checked local-space cycle exactly; it
    /// does not drop, repeat, scale, or otherwise reinterpret producer facts.
    #[test]
    fn resolved_dash_intervals_project_exactly_into_private_stroke_material() {
        let mut node = base_node(PaintStack::empty());
        node.stroke = Some(dashed_stroke(
            vec![0.0, 8.0, 3.0, 0.0],
            rframe::StrokeCap::Round,
        ));
        let product =
            compile(frame_of(FrameItems::from_nodes(vec![node]))).expect("admitted dashed frame");
        let ItemKind::RectStroke { stroke, .. } = &product.drawlist.items[1].kind else {
            panic!("second item is the rectangle stroke");
        };

        assert_eq!(
            stroke.dash_array.as_deref(),
            Some(&[0.0, 8.0, 3.0, 0.0][..])
        );
        assert_eq!(stroke.cap, StrokeCap::Round);
        assert_eq!(stroke.width, StrokeWidth::Uniform(8.0));
    }

    /// Changing only the resolved dash cycle changes the private draw item and
    /// therefore participates in the complete-frame damage policy.
    #[test]
    fn resolved_dash_intervals_participate_in_frame_damage() {
        let scene = |intervals| {
            let node = dashed_node(
                RECT_OWNER,
                Geometry::Rect(Rectangle::from_xywh(8.0, 6.0, 20.0, 16.0)),
                intervals,
            );
            compile(frame_of(FrameItems::from_nodes(vec![node]))).expect("admitted dashed frame")
        };
        let before = scene(vec![4.0, 4.0]);
        let after = scene(vec![8.0, 8.0]);

        assert_eq!(diff_frame(&before, &after).changed, vec![RECT_OWNER]);
        assert!(diff_frame(&before, &before).is_empty());
    }

    /// A closed ellipse has no ends only while its stroke is solid. The zero
    /// painted slots in this cycle are visible round dots, so the solid-only
    /// cap normalization must leave the authored cap intact.
    #[test]
    fn round_zero_length_dashes_survive_closed_ellipse_cap_normalization() {
        let node = dashed_node(
            RECT_OWNER,
            Geometry::Ellipse(Rectangle::from_xywh(16.0, 8.0, 32.0, 32.0)),
            vec![0.0, 16.0],
        );
        let product = compile(frame_of(FrameItems::from_nodes(vec![node])))
            .expect("admitted dashed ellipse frame");
        let context = PaintCtx::new(None);
        let neutral_view = AffineTransform::identity();
        let pixels = product
            .raster_to_bytes(&neutral_view, 64, 48, &context)
            .expect("resource-free dashed ellipse raster");

        assert_eq!(
            rgba_at(&pixels, 64, 48, 24),
            [0, 0, 0, 255],
            "the zero-length dash at the oval origin is a round dot"
        );
        assert_eq!(
            rgba_at(&pixels, 64, 32, 24),
            [255, 255, 255, 255],
            "the unfilled oval center stays at the clear color"
        );
        assert_eq!(
            pixels,
            product
                .raster_to_bytes(&neutral_view, 64, 48, &context)
                .expect("deterministic repeat")
        );
    }

    /// The path arm shares the same solid-only normalization but is a distinct
    /// painter route. Pin its closed-contour zero-length dash independently.
    #[test]
    fn round_zero_length_dashes_survive_closed_path_cap_normalization() {
        let path = rframe::PathData::new(
            vec![
                rframe::PathCommand::MoveTo { x: 12.0, y: 8.0 },
                rframe::PathCommand::LineTo { x: 44.0, y: 8.0 },
                rframe::PathCommand::LineTo { x: 44.0, y: 40.0 },
                rframe::PathCommand::LineTo { x: 12.0, y: 40.0 },
                rframe::PathCommand::Close,
            ],
            rframe::FillRule::NonZero,
        )
        .expect("test path is valid");
        let node = dashed_node(RECT_OWNER, Geometry::Path(Arc::new(path)), vec![0.0, 16.0]);
        let product = compile(frame_of(FrameItems::from_nodes(vec![node])))
            .expect("admitted dashed path frame");
        let context = PaintCtx::new(None);
        let pixels = product
            .raster_to_bytes(&AffineTransform::identity(), 64, 48, &context)
            .expect("resource-free dashed path raster");

        assert_eq!(
            rgba_at(&pixels, 64, 12, 8),
            [0, 0, 0, 255],
            "the zero-length dash at the path origin is a round dot"
        );
        assert_eq!(
            rgba_at(&pixels, 64, 20, 8),
            [255, 255, 255, 255],
            "the midpoint of the following gap stays clear"
        );
    }

    #[test]
    fn transformed_geometry_requires_exact_contract_bounds() {
        let mut node = base_node(PaintStack::solid(CGColor::RED));
        node.transform = AffineTransform::new(3.0, 4.0, 0.0);
        let expected = math2::rect_transform(node.geometry.local_box(), &node.transform);
        node.bounds = Rectangle {
            x: f32::from_bits(expected.x.to_bits() + 1),
            ..expected
        };
        assert!(matches!(
            compile(frame_of(FrameItems::from_nodes(vec![node.clone()]))),
            Err(BuildError::VisualBoundsMismatch(RECT_OWNER))
        ));

        node.bounds = expected;
        compile(frame_of(FrameItems::from_nodes(vec![node])))
            .expect("exact transformed bounds are admitted");
    }

    /// An ellipse node admits only bounds that exactly equal its transformed
    /// local-space rectangle; a bit-nudged contract bound fails loudly.
    #[test]
    fn transformed_ellipse_requires_exact_contract_bounds() {
        let bbox = Rectangle::from_xywh(8.0, 6.0, 20.0, 16.0);
        let mut node = base_node(PaintStack::solid(CGColor::RED));
        node.geometry = Geometry::Ellipse(bbox);
        node.transform = AffineTransform::new(3.0, 4.0, 0.0);
        let expected = math2::rect_transform(bbox, &node.transform);
        node.bounds = Rectangle {
            x: f32::from_bits(expected.x.to_bits() + 1),
            ..expected
        };
        assert!(matches!(
            compile(frame_of(FrameItems::from_nodes(vec![node.clone()]))),
            Err(BuildError::VisualBoundsMismatch(RECT_OWNER))
        ));

        node.bounds = expected;
        compile(frame_of(FrameItems::from_nodes(vec![node])))
            .expect("exact transformed ellipse bounds are admitted");
    }

    /// An ellipse compiles to the oval fill inscribed in its local-space
    /// rectangle: the box center rasters solid paint, the box corners stay at
    /// the surface clear color, and a repeat raster is byte-identical.
    #[test]
    fn ellipse_geometry_rasters_the_inscribed_oval() {
        let context = PaintCtx::new(None);
        let mut node = base_node(PaintStack::solid(CGColor::from_rgb(0x16, 0xa3, 0x4a)));
        node.geometry = Geometry::Ellipse(Rectangle::from_xywh(8.0, 6.0, 20.0, 16.0));
        let frame = frame_of(FrameItems::from_nodes(vec![node]));
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
        let duplicate = frame_of(FrameItems::from_nodes(vec![rect_node(
            FRAME_OWNER,
            Rectangle::from_xywh(0.0, 0.0, 8.0, 8.0),
            0xFFFF_0000,
        )]));
        assert!(matches!(
            compile(duplicate),
            Err(BuildError::DuplicateOwner(FRAME_OWNER))
        ));
    }

    /// A scope owner shares the one owner namespace: a scope repeating a
    /// node's owner fails exactly as a repeated node does.
    #[test]
    fn duplicate_scope_owner_fails_explicitly() {
        let items = FrameItems::try_new(vec![
            scope_begin(RECT_OWNER, 0.5),
            FrameItem::Node(base_node(PaintStack::solid(CGColor::RED))),
            FrameItem::ScopeEnd,
        ])
        .expect("balanced scope stream");
        assert!(matches!(
            compile(frame_of(items)),
            Err(BuildError::DuplicateOwner(RECT_OWNER))
        ));
    }

    /// The scope lowers onto the one private opacity layer: Begin/End
    /// items owned by the scope, enclosing its span, with the frame clip
    /// outside.
    #[test]
    fn scope_lowers_onto_the_private_opacity_items() {
        let items = FrameItems::try_new(vec![
            scope_begin(SCOPE_OWNER, 0.5),
            FrameItem::Node(base_node(PaintStack::solid(CGColor::RED))),
            FrameItem::ScopeEnd,
        ])
        .expect("balanced scope stream");
        let product = compile(frame_of(items)).expect("admitted scoped frame");
        let kinds = product
            .drawlist
            .items
            .iter()
            .map(|item| std::mem::discriminant(&item.kind))
            .collect::<Vec<_>>();
        assert_eq!(
            kinds,
            [
                std::mem::discriminant(&ItemKind::BeginClipRect {
                    w: 0.0,
                    h: 0.0,
                    corner_radius: RectangularCornerRadius::default(),
                    corner_smoothing: CornerSmoothing::default(),
                }),
                std::mem::discriminant(&ItemKind::BeginIsolatedOpacity { opacity: 0.5 }),
                std::mem::discriminant(&ItemKind::RectFill {
                    w: 0.0,
                    h: 0.0,
                    corner_radius: RectangularCornerRadius::default(),
                    corner_smoothing: CornerSmoothing::default(),
                    paints: Paints::default(),
                }),
                std::mem::discriminant(&ItemKind::EndOpacity),
                std::mem::discriminant(&ItemKind::EndClip),
            ],
            "frame clip, then the scope enclosing its span"
        );
        let ItemKind::BeginIsolatedOpacity { opacity } = product.drawlist.items[1].kind else {
            panic!("second item begins the opacity scope");
        };
        assert_eq!(opacity, 0.5);
        assert_eq!(
            product.drawlist.items[1].node, product.drawlist.items[3].node,
            "begin and end are owned by the one scope"
        );
    }

    /// A scope opacity edit damages exactly the union of what the scope
    /// composites.
    #[test]
    fn scope_opacity_edit_damages_the_scope_union() {
        let scene = |opacity: f32| {
            let items = FrameItems::try_new(vec![
                scope_begin(SCOPE_OWNER, opacity),
                FrameItem::Node(rect_node(
                    RECT_OWNER,
                    Rectangle::from_xywh(8.0, 6.0, 20.0, 16.0),
                    0xFF16_A34A,
                )),
                FrameItem::Node(rect_node(
                    OTHER_OWNER,
                    Rectangle::from_xywh(24.0, 18.0, 20.0, 16.0),
                    0xFF25_63EB,
                )),
                FrameItem::ScopeEnd,
            ])
            .expect("balanced scope stream");
            compile(frame_of(items)).expect("admitted scoped frame")
        };
        let damage = diff_frame(&scene(0.5), &scene(0.25));
        assert_eq!(damage.changed, vec![SCOPE_OWNER]);
        assert_eq!(
            damage.union_frame,
            Some(Rectangle::from_xywh(8.0, 6.0, 36.0, 28.0)),
            "the scope's coverage is the union of its span"
        );
    }

    /// The layer meaning, pinned against Chromium 149.0.7827.55 (probe
    /// p4a of the group-scope rung): two opaque children under one 0.5
    /// scope over white — every covered pixel is the topmost child at the
    /// scope's alpha over the backdrop, at the oracle's own layer
    /// quantization.
    #[test]
    fn scope_raster_matches_the_chromium_layer_composite() {
        let context = PaintCtx::new(None);
        let items = FrameItems::try_new(vec![
            scope_begin(SCOPE_OWNER, 0.5),
            FrameItem::Node(rect_node(
                RECT_OWNER,
                Rectangle::from_xywh(8.0, 8.0, 32.0, 32.0),
                0xFF16_A34A,
            )),
            FrameItem::Node(rect_node(
                OTHER_OWNER,
                Rectangle::from_xywh(24.0, 24.0, 32.0, 32.0),
                0xFF25_63EB,
            )),
            FrameItem::ScopeEnd,
        ])
        .expect("balanced scope stream");
        let frame = Frame {
            owner: FRAME_OWNER,
            bounds: Rectangle::from_xywh(0.0, 0.0, 64.0, 64.0),
            items,
        };
        let product = compile(frame).expect("admitted scoped frame");
        let pixels = product
            .raster_to_bytes(&AffineTransform::identity(), 64, 64, &context)
            .expect("resource-free scoped raster");
        let at = |x: i32, y: i32| -> [u8; 4] {
            let offset = ((y * 64 + x) * 4) as usize;
            pixels[offset..offset + 4].try_into().expect("RGBA pixel")
        };
        assert_eq!(at(16, 16), [137, 208, 163, 255], "green-only region");
        assert_eq!(
            at(32, 32),
            [145, 176, 244, 255],
            "overlap is the topmost child at the scope alpha — composited once"
        );
        assert_eq!(at(48, 48), [145, 176, 244, 255], "blue-only region");
    }

    /// The nested meaning, pinned against Chromium (probe p5a): an outer
    /// 0.5 layer over an inner *folded* translucent draw (alpha 128) — the
    /// per-layer quantization Chromium shows, one code value below the
    /// flat 0.25 fold.
    #[test]
    fn nested_scope_raster_matches_the_chromium_per_layer_quantization() {
        let context = PaintCtx::new(None);
        let items = FrameItems::try_new(vec![
            scope_begin(SCOPE_OWNER, 0.5),
            FrameItem::Node(rect_node(
                RECT_OWNER,
                Rectangle::from_xywh(8.0, 8.0, 48.0, 48.0),
                0x8016_A34A,
            )),
            FrameItem::ScopeEnd,
        ])
        .expect("balanced scope stream");
        let frame = Frame {
            owner: FRAME_OWNER,
            bounds: Rectangle::from_xywh(0.0, 0.0, 64.0, 64.0),
            items,
        };
        let product = compile(frame).expect("admitted scoped frame");
        let pixels = product
            .raster_to_bytes(&AffineTransform::identity(), 64, 64, &context)
            .expect("resource-free scoped raster");
        let offset = ((32 * 64 + 32) * 4) as usize;
        assert_eq!(
            &pixels[offset..offset + 4],
            [196, 232, 209, 255],
            "layer-over-folded-draw quantization matches the oracle"
        );
    }

    #[test]
    fn frame_diff_reuses_the_complete_generic_damage_policy() {
        let before = compile(resolved_frame(PaintStack::solid(CGColor::RED))).expect("before");
        let after = compile(resolved_frame(PaintStack::solid(CGColor::BLUE))).expect("after");

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
