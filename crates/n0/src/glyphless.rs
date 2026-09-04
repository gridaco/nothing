//! Source-neutral glyphless frames entering the n0 chassis.
//!
//! [`rframe::Frame`] is the backend-free resolved contract. It carries no
//! authored n0 document, HTML/CSS/SVG syntax, parser binding, backend object,
//! I/O handle, or clock. This module admits its current solid-, gradient-, and
//! resolved-pattern-painted rectangle, ellipse, and path slice plus checked
//! opacity, clip, mask, and image-filter effects, compiles them into n0's one
//! private drawlist, and executes them through n0's one private painter.
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
use rframe::{
    ClipPath, FilterBlend, FilterColorSpace, FilterComposite, FilterConvolveEdgeMode,
    FilterDisplacementChannel, FilterInput, FilterLightSource, FilterMorphology, FilterPrimitive,
    FilterTurbulenceKind, Frame, FrameItem, Geometry, MaskMode, PaintStack, ScopeEffect, VisualRef,
};

use crate::damage::{diff_inputs, DamageOwner, FrameDamageInput};
use crate::drawlist::{
    DrawList, GlyphlessOwnerSlot, Item, ItemKind, PostPaintOpacity, ResolvedClipGeometry,
    ResolvedClipGeometryKind, ResolvedClipLayer, ResolvedClipPath, ResolvedFilter,
    ResolvedFilterBlend, ResolvedFilterColorSpace, ResolvedFilterComposite,
    ResolvedFilterConvolveEdgeMode, ResolvedFilterDisplacementChannel, ResolvedFilterInput,
    ResolvedFilterLightSource, ResolvedFilterMorphology, ResolvedFilterNode,
    ResolvedFilterPrimitive, ResolvedFilterTurbulenceKind, ResolvedMaskMode, ResolvedPattern,
    ResolvedPatternGeometry, StrokeDashPhase, StrokeSpace,
};
use crate::frame::FrameExecutionError;
use crate::paint::PaintCtx;

/// Private projection from draw-item owner slots back to the contract's opaque
/// identity and provenance.
#[derive(Debug, Clone)]
struct ProvenanceProjection {
    owners: Vec<VisualRef>,
    coverage: Vec<Option<n0_model::math::RectF>>,
}

impl ProvenanceProjection {
    fn get(&self, slot: GlyphlessOwnerSlot) -> Option<(VisualRef, Option<n0_model::math::RectF>)> {
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
    /// A resolved geometric clip failed the backend's deterministic path-op
    /// preflight. Replaying it could not be trusted to preserve the contract.
    Clip {
        owner: VisualRef,
        reason: String,
    },
    /// A resolved image mask's geometric region failed deterministic backend
    /// preflight. Executing it could otherwise turn a mask into no mask.
    Mask {
        owner: VisualRef,
        reason: String,
    },
    /// A resolved image-filter program failed deterministic backend preflight.
    /// Executing it could otherwise silently restore the group unfiltered.
    Filter {
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
            BuildError::Clip { owner, reason } => {
                write!(
                    f,
                    "glyphless visual {owner:?} clip preflight failed: {reason}"
                )
            }
            BuildError::Mask { owner, reason } => {
                write!(
                    f,
                    "glyphless visual {owner:?} mask preflight failed: {reason}"
                )
            }
            BuildError::Filter { owner, reason } => {
                write!(
                    f,
                    "glyphless visual {owner:?} filter preflight failed: {reason}"
                )
            }
        }
    }
}

impl std::error::Error for BuildError {}

/// One immutable source-neutral frame, its private compiled material, and its
/// opaque provenance projection.
///
/// Every admitted paint is already resolved and carries no resource handle, so
/// this product neither captures nor checks a
/// [`crate::paint::PaintEnvironmentKey`]. A repeating vector program is nested
/// immutable draw material, not a late resource lookup.
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
        crate::paint::preflight_patterns(&self.drawlist, ctx)?;
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
        crate::paint::preflight_patterns(&self.drawlist, ctx)?;
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

/// Exact material/order attribution between two source-neutral frame products,
/// plus the optional frame-space envelope of pixels that can be affected.
/// A fully clipped change remains attributable in `changed` while its envelope
/// is `None`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Damage {
    pub changed: Vec<VisualRef>,
    pub union_frame: Option<math2::Rectangle>,
}

#[derive(Debug, Clone)]
enum OpenScopeKind {
    Opacity,
    Clip {
        bounds: Option<n0_model::math::RectF>,
    },
    Mask {
        mode: ResolvedMaskMode,
        region: Arc<ResolvedClipPath>,
        bounds: Option<n0_model::math::RectF>,
        target_coverage: Option<n0_model::math::RectF>,
    },
    Filter {
        bounds: Option<n0_model::math::RectF>,
    },
}

#[derive(Debug, Clone)]
struct OpenScope {
    slot: GlyphlessOwnerSlot,
    coverage: Option<n0_model::math::RectF>,
    kind: OpenScopeKind,
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
                .or_insert_with(|| DamageOwner::new((), coverage));
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
/// paint box before the product exists), checked repeating vector programs, a
/// centred stroke over the fill, isolated opacity scopes, resolved geometric
/// clip scopes, and the frame-bounds clip.
///
/// The contract's item stream is a checked type ([`rframe::FrameItems`]):
/// balance, non-emptiness, and bounded nesting were proven at construction,
/// so this compile trusts them the way it trusts [`rframe::PathData`] —
/// nothing is re-derived, and the scope walk uses plain `expect`s.
pub fn compile(resolved: Frame) -> Result<FrameProduct, BuildError> {
    validate_frame_bounds(resolved.bounds).map_err(|_| BuildError::InvalidFrameBounds)?;
    let owner_count = resolved
        .items
        .iter()
        .filter(|item| {
            matches!(
                item,
                FrameItem::Node(_) | FrameItem::ScopeBegin(_) | FrameItem::MaskBegin(_)
            )
        })
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
    provenance
        .coverage
        .push(bounded_geometry_coverage(resolved.bounds, resolved.bounds));

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
    let mut open_scopes: Vec<OpenScope> = Vec::new();

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
                provenance.coverage.push(None);
                let (kind, initial_coverage) = match &scope.effect {
                    ScopeEffect::Opacity(opacity) => {
                        items.push(Item {
                            node: slot,
                            world: frame_world,
                            kind: ItemKind::BeginIsolatedOpacity {
                                opacity: opacity.get(),
                            },
                        });
                        (OpenScopeKind::Opacity, None)
                    }
                    ScopeEffect::Clip(clip) => {
                        let compiled = Arc::new(compile_clip_path(clip));
                        if !crate::paint::preflight_clip_path(&compiled) {
                            return Err(BuildError::Clip {
                                owner: scope.owner,
                                reason: "geometric union/intersection operation failed".to_string(),
                            });
                        }
                        let bounds = clip
                            .bounds()
                            .and_then(|bounds| bounded_geometry_coverage(bounds, resolved.bounds));
                        items.push(Item {
                            node: slot,
                            world: Affine::IDENTITY,
                            kind: ItemKind::BeginClipPath { clip: compiled },
                        });
                        (OpenScopeKind::Clip { bounds }, None)
                    }
                    ScopeEffect::Filter(filter) => {
                        let compiled = Arc::new(compile_filter(filter));
                        if let Err(reason) = crate::paint::preflight_filter(&compiled) {
                            return Err(BuildError::Filter {
                                owner: scope.owner,
                                reason,
                            });
                        }
                        let bounds = bounded_geometry_coverage(
                            math2::rect_transform(filter.region(), &filter.transform()),
                            resolved.bounds,
                        );
                        items.push(Item {
                            node: slot,
                            world: to_affine(filter.transform()),
                            kind: ItemKind::BeginFilter { filter: compiled },
                        });
                        let initial_coverage = if filter.source_is_transparent()
                            && filter.program().may_paint_transparent_input()
                        {
                            bounds
                        } else {
                            None
                        };
                        (OpenScopeKind::Filter { bounds }, initial_coverage)
                    }
                };
                open_scopes.push(OpenScope {
                    slot,
                    coverage: initial_coverage,
                    kind,
                });
                continue;
            }
            FrameItem::ScopeEnd => {
                let scope = open_scopes.pop().expect("checked stream is balanced");
                let (coverage, world, end) = match scope.kind {
                    OpenScopeKind::Opacity => (scope.coverage, frame_world, ItemKind::EndOpacity),
                    OpenScopeKind::Clip { bounds } => match (scope.coverage, bounds) {
                        (Some(coverage), Some(bounds)) => (
                            bounded_intersection_rectf(coverage, bounds, resolved.bounds),
                            Affine::IDENTITY,
                            ItemKind::EndClip,
                        ),
                        _ => (None, Affine::IDENTITY, ItemKind::EndClip),
                    },
                    OpenScopeKind::Mask { .. } => {
                        unreachable!("checked mask scopes close with MaskEnd")
                    }
                    OpenScopeKind::Filter { bounds } => (
                        scope.coverage.and(bounds),
                        Affine::IDENTITY,
                        ItemKind::EndFilter,
                    ),
                };
                let slot = scope.slot;
                provenance.coverage[slot.index()] = coverage;
                if let (Some(coverage), Some(parent)) = (coverage, open_scopes.last_mut()) {
                    parent.coverage = Some(match parent.coverage {
                        None => coverage,
                        Some(parent) => bounded_union_rectf(parent, coverage, resolved.bounds),
                    });
                }
                items.push(Item {
                    node: slot,
                    world,
                    kind: end,
                });
                continue;
            }
            FrameItem::MaskBegin(mask) => {
                if !unique.insert(mask.owner) {
                    return Err(BuildError::DuplicateOwner(mask.owner));
                }
                let slot = GlyphlessOwnerSlot::new(
                    u32::try_from(provenance.owners.len()).expect("owner count checked above"),
                );
                provenance.owners.push(mask.owner);
                provenance.coverage.push(None);
                let region = Arc::new(compile_clip_path(mask.region()));
                if !crate::paint::preflight_clip_path(&region) {
                    return Err(BuildError::Mask {
                        owner: mask.owner,
                        reason: "geometric mask-region operation failed".to_string(),
                    });
                }
                let bounds = mask
                    .region()
                    .bounds()
                    .and_then(|bounds| bounded_geometry_coverage(bounds, resolved.bounds));
                let mode = match mask.mode() {
                    MaskMode::Alpha => ResolvedMaskMode::Alpha,
                    MaskMode::Luminance => ResolvedMaskMode::Luminance,
                };
                items.push(Item {
                    node: slot,
                    world: frame_world,
                    kind: ItemKind::BeginMaskContent,
                });
                open_scopes.push(OpenScope {
                    slot,
                    coverage: None,
                    kind: OpenScopeKind::Mask {
                        mode,
                        region,
                        bounds,
                        target_coverage: None,
                    },
                });
                continue;
            }
            FrameItem::MaskSource => {
                let scope = open_scopes
                    .last_mut()
                    .expect("checked mask source has an open target");
                let OpenScopeKind::Mask {
                    mode,
                    region,
                    target_coverage,
                    ..
                } = &mut scope.kind
                else {
                    unreachable!("checked mask source directly follows its target")
                };
                *target_coverage = scope.coverage.take();
                items.push(Item {
                    node: scope.slot,
                    world: Affine::IDENTITY,
                    kind: ItemKind::BeginMaskSource {
                        mode: *mode,
                        region: Arc::clone(region),
                    },
                });
                continue;
            }
            FrameItem::MaskEnd => {
                let scope = open_scopes
                    .pop()
                    .expect("checked mask end has an open source");
                let OpenScopeKind::Mask {
                    bounds,
                    target_coverage,
                    ..
                } = scope.kind
                else {
                    unreachable!("checked mask end closes a mask source")
                };
                let coverage = match (target_coverage, scope.coverage, bounds) {
                    (Some(target), Some(source), Some(bounds)) => {
                        bounded_intersection_rectf(target, source, resolved.bounds).and_then(
                            |intersection| {
                                bounded_intersection_rectf(intersection, bounds, resolved.bounds)
                            },
                        )
                    }
                    _ => None,
                };
                provenance.coverage[scope.slot.index()] = coverage;
                if let (Some(coverage), Some(parent)) = (coverage, open_scopes.last_mut()) {
                    parent.coverage = Some(match parent.coverage {
                        None => coverage,
                        Some(parent) => bounded_union_rectf(parent, coverage, resolved.bounds),
                    });
                }
                items.push(Item {
                    node: scope.slot,
                    world: Affine::IDENTITY,
                    kind: ItemKind::EndMaskSource,
                });
                items.push(Item {
                    node: scope.slot,
                    world: frame_world,
                    kind: ItemKind::EndMaskContent,
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
        // The paint reference box is the geometry's own extent. Ordinary box
        // routes draw at their item origin, so their paint box already starts
        // there. A path's stream carries absolute local coordinates, so its box
        // starts at the tight-bounds origin instead — the painter's unit box
        // does not, and the difference is observable only by a non-solid paint.
        // The origin travels as a unit-space pre-translate on each gradient's
        // transform: box(x,y,w,h) × T = box(0,0,w,h) × translate(x/w, y/h) × T.
        // A degenerate axis skips the fold; the producer resolves or refuses
        // gradients on degenerate geometry before they reach this compile. The
        // dashed-ellipse exception below instead gives the painter its exact
        // positioned paint box and therefore needs no compensating arithmetic.
        let unit_offset = match &node.geometry {
            Geometry::Path(_) if rect.width > 0.0 && rect.height > 0.0 => {
                Some((rect.x / rect.width, rect.y / rect.height))
            }
            _ => None,
        };
        let paints = compile_paints(&node.paints, unit_offset);
        let fill_pattern = node
            .paints
            .pattern()
            .map(|pattern| compile_pattern(pattern, node.owner))
            .transpose()?;
        let fill_post_paint_opacity =
            PostPaintOpacity::from_resolved(node.paints.alpha_factor().get());
        let owner = GlyphlessOwnerSlot::new(
            u32::try_from(provenance.owners.len()).expect("owner count checked above"),
        );
        provenance.owners.push(node.owner);

        // An ordinary box route draws at its item's origin, so its own local
        // offset enters the world transform. A path carries absolute local
        // coordinates instead: its stream is the geometry, and translating it
        // would be a second coordinate mapping over values the contract has
        // already resolved. The dashed-ellipse stroke below bypasses this box
        // transform for the same exact-coordinate reason; its independent fill
        // still uses this ordinary route.
        let box_world = match &node.geometry {
            Geometry::Rect(_) | Geometry::Ellipse(_) => {
                to_affine(node.transform).then(&Affine::translate(rect.x, rect.y))
            }
            Geometry::Path(_) => to_affine(node.transform),
        };
        // A stroke paints outside the geometry, so coverage — what the damage
        // policy repaints — includes its checked reach. Local-space reach is
        // inflated before projection; frame-space reach is inflated around
        // the already-projected centerline bounds. Both routes intersect with
        // the frame clip before encoding the finite RectF used by damage.
        // Paint reference boxes above deliberately remain the geometry's box.
        let coverage = match (&node.stroke, node.paints.is_empty()) {
            (None, true) => None,
            (None, false) => bounded_geometry_coverage(node.bounds, resolved.bounds),
            (Some(stroke), _)
                if stroke.space() == rframe::StrokeSpace::Frame
                    && !transform_has_identity_linear_part(&node.transform) =>
            {
                if frame_stroke_transform_is_unusable(&node.transform) {
                    if node.paints.is_empty() {
                        None
                    } else {
                        bounded_geometry_coverage(node.bounds, resolved.bounds)
                    }
                } else {
                    bounded_frame_stroke_coverage(node.bounds, stroke.outset(), resolved.bounds)
                }
            }
            (Some(stroke), _) => {
                let box_world = matches!(&node.geometry, Geometry::Rect(_) | Geometry::Ellipse(_))
                    .then_some(&box_world);
                bounded_stroke_coverage(
                    rect,
                    &node.transform,
                    box_world,
                    stroke.outset(),
                    resolved.bounds,
                )
            }
        };
        provenance.coverage.push(coverage);
        if let (Some(coverage), Some(accumulated)) = (coverage, open_scopes.last_mut()) {
            accumulated.coverage = Some(match accumulated.coverage {
                None => coverage,
                Some(accumulated) => bounded_union_rectf(accumulated, coverage, resolved.bounds),
            });
        }

        let (w, h) = (rect.width, rect.height);
        let path = match &node.geometry {
            Geometry::Path(path) => Some(compile_path(path)),
            _ => None,
        };
        if let Some(pattern) = fill_pattern {
            items.push(Item {
                node: owner,
                world: to_affine(node.transform),
                kind: ItemKind::PatternFill {
                    geometry: compile_pattern_geometry(&node.geometry),
                    pattern,
                    post_paint_opacity: fill_post_paint_opacity,
                },
            });
        } else if !paints.is_empty() {
            let kind = match &node.geometry {
                Geometry::Rect(_) => ItemKind::RectFill {
                    w,
                    h,
                    corner_radius: RectangularCornerRadius::default(),
                    corner_smoothing: CornerSmoothing::default(),
                    paints,
                    post_paint_opacity: fill_post_paint_opacity,
                },
                Geometry::Ellipse(_) => ItemKind::OvalFill {
                    w,
                    h,
                    paints,
                    post_paint_opacity: fill_post_paint_opacity,
                },
                Geometry::Path(_) => ItemKind::PathFill {
                    w,
                    h,
                    path: Arc::clone(path.as_ref().expect("path geometry compiled its stream")),
                    paints,
                    post_paint_opacity: fill_post_paint_opacity,
                },
            };
            items.push(Item {
                node: owner,
                world: box_world,
                kind,
            });
        }
        // SVG's default paint order is fill, then stroke — one item after the
        // other in the same private drawlist, which is why a stroke needs no
        // group scope.
        if let Some(stroke) = &node.stroke {
            // Blink cannot project a frame-space stroke outline back through a
            // singular local-to-frame map. The stroke is therefore the exact
            // nothing; an independent fill, if any, already emitted above.
            if stroke.space() == rframe::StrokeSpace::Frame
                && frame_stroke_transform_is_unusable(&node.transform)
            {
                continue;
            }
            let stroke_pattern = stroke
                .paints()
                .pattern()
                .map(|pattern| compile_pattern(pattern, node.owner))
                .transpose()?;
            // A resolved dashed oval must preserve the exact local conic
            // stream over which its producer resolved the dash facts. Skia's
            // path measurement and dash traversal are f32
            // translation-sensitive, so moving the ellipse's box origin into
            // `world` first is not equivalent: the interval and phase facts
            // are unchanged, but their antialiased endpoints drift. Preserve
            // the contract's absolute local coordinates for every live dashed
            // ellipse. Solid ellipses remain the private oval primitive, and
            // fills keep their existing independent box route.
            let dashed_ellipse = matches!(&node.geometry, Geometry::Ellipse(_))
                && rect.width > 0.0
                && rect.height > 0.0
                && stroke.dash().is_some();
            let (stroke, space, dash_phase, post_paint_opacity) =
                compile_stroke(stroke, unit_offset);
            if let Some(pattern) = stroke_pattern {
                items.push(Item {
                    node: owner,
                    world: to_affine(node.transform),
                    kind: ItemKind::PatternStroke {
                        geometry: compile_pattern_geometry(&node.geometry),
                        pattern,
                        stroke,
                        space,
                        dash_phase,
                        post_paint_opacity,
                    },
                });
                continue;
            }
            let kind = match &node.geometry {
                Geometry::Rect(_) => ItemKind::RectStroke {
                    w,
                    h,
                    corner_radius: RectangularCornerRadius::default(),
                    corner_smoothing: CornerSmoothing::default(),
                    stroke,
                    space,
                    dash_phase,
                    post_paint_opacity,
                },
                Geometry::Ellipse(_) if dashed_ellipse => ItemKind::AbsoluteDashedOvalStroke {
                    x: rect.x,
                    y: rect.y,
                    w,
                    h,
                    stroke,
                    space,
                    dash_phase,
                    post_paint_opacity,
                },
                Geometry::Ellipse(_) => ItemKind::OvalStroke {
                    w,
                    h,
                    stroke,
                    space,
                    dash_phase,
                    post_paint_opacity,
                },
                Geometry::Path(_) => ItemKind::PathStroke {
                    w,
                    h,
                    path: Arc::clone(path.as_ref().expect("path geometry compiled its stream")),
                    stroke,
                    space,
                    dash_phase,
                    post_paint_opacity,
                },
            };
            items.push(Item {
                node: owner,
                world: if dashed_ellipse {
                    to_affine(node.transform)
                } else {
                    box_world
                },
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

fn compile_clip_path(clip: &ClipPath) -> ResolvedClipPath {
    let layers = clip
        .layers()
        .iter()
        .map(|layer| ResolvedClipLayer {
            geometries: layer
                .geometries()
                .iter()
                .map(|geometry| {
                    let local = geometry.geometry().local_box();
                    let kind = match geometry.geometry() {
                        Geometry::Rect(_) => ResolvedClipGeometryKind::Rect {
                            x: local.x,
                            y: local.y,
                            w: local.width,
                            h: local.height,
                        },
                        Geometry::Ellipse(_) => ResolvedClipGeometryKind::Oval {
                            x: local.x,
                            y: local.y,
                            w: local.width,
                            h: local.height,
                        },
                        Geometry::Path(path) => ResolvedClipGeometryKind::Path(compile_path(path)),
                    };
                    ResolvedClipGeometry {
                        world: to_affine(geometry.transform()),
                        kind,
                    }
                })
                .collect::<Vec<_>>()
                .into(),
        })
        .collect::<Vec<_>>()
        .into();
    ResolvedClipPath {
        layers,
        anti_alias: clip.edge_mode() == rframe::ClipEdgeMode::AntiAliased,
    }
}

/// Project one checked, source-neutral filter program into private painter
/// material. Every index and scalar has already been validated by `rframe`;
/// this is a vocabulary translation, not a second resolver.
fn compile_filter(filter: &rframe::Filter) -> ResolvedFilter {
    let program = filter.program();
    let nodes = program
        .iter()
        .map(|node| ResolvedFilterNode {
            inputs: node
                .inputs()
                .iter()
                .map(|input| match input {
                    FilterInput::Source => ResolvedFilterInput::Source,
                    FilterInput::SourceAlpha => ResolvedFilterInput::SourceAlpha,
                    FilterInput::Node(index) => ResolvedFilterInput::Node(*index),
                })
                .collect::<Vec<_>>()
                .into(),
            region: to_rectf(node.region()),
            color_space: match node.color_space() {
                FilterColorSpace::Srgb => ResolvedFilterColorSpace::Srgb,
                FilterColorSpace::LinearRgb => ResolvedFilterColorSpace::LinearRgb,
            },
            primitive: match node.primitive() {
                FilterPrimitive::GaussianBlur { sigma_x, sigma_y } => {
                    ResolvedFilterPrimitive::GaussianBlur { sigma_x, sigma_y }
                }
                FilterPrimitive::Offset { dx, dy } => ResolvedFilterPrimitive::Offset { dx, dy },
                FilterPrimitive::SolidColor { color } => ResolvedFilterPrimitive::SolidColor {
                    color: compile_color32f(color),
                },
                FilterPrimitive::Composite { operator } => ResolvedFilterPrimitive::Composite {
                    operator: match operator {
                        FilterComposite::Over => ResolvedFilterComposite::Over,
                        FilterComposite::In => ResolvedFilterComposite::In,
                        FilterComposite::Out => ResolvedFilterComposite::Out,
                        FilterComposite::Atop => ResolvedFilterComposite::Atop,
                        FilterComposite::Xor => ResolvedFilterComposite::Xor,
                        FilterComposite::Lighter => ResolvedFilterComposite::Lighter,
                        FilterComposite::Arithmetic { k1, k2, k3, k4 } => {
                            ResolvedFilterComposite::Arithmetic { k1, k2, k3, k4 }
                        }
                    },
                },
                FilterPrimitive::Blend { mode } => ResolvedFilterPrimitive::Blend {
                    mode: match mode {
                        FilterBlend::Normal => ResolvedFilterBlend::Normal,
                        FilterBlend::Multiply => ResolvedFilterBlend::Multiply,
                        FilterBlend::Screen => ResolvedFilterBlend::Screen,
                        FilterBlend::Overlay => ResolvedFilterBlend::Overlay,
                        FilterBlend::Darken => ResolvedFilterBlend::Darken,
                        FilterBlend::Lighten => ResolvedFilterBlend::Lighten,
                        FilterBlend::ColorDodge => ResolvedFilterBlend::ColorDodge,
                        FilterBlend::ColorBurn => ResolvedFilterBlend::ColorBurn,
                        FilterBlend::HardLight => ResolvedFilterBlend::HardLight,
                        FilterBlend::SoftLight => ResolvedFilterBlend::SoftLight,
                        FilterBlend::Difference => ResolvedFilterBlend::Difference,
                        FilterBlend::Exclusion => ResolvedFilterBlend::Exclusion,
                        FilterBlend::Hue => ResolvedFilterBlend::Hue,
                        FilterBlend::Saturation => ResolvedFilterBlend::Saturation,
                        FilterBlend::Color => ResolvedFilterBlend::Color,
                        FilterBlend::Luminosity => ResolvedFilterBlend::Luminosity,
                    },
                },
                FilterPrimitive::DropShadow {
                    dx,
                    dy,
                    sigma_x,
                    sigma_y,
                    color,
                } => ResolvedFilterPrimitive::DropShadow {
                    dx,
                    dy,
                    sigma_x,
                    sigma_y,
                    color: compile_color32f(color),
                },
                FilterPrimitive::ColorMatrix { matrix } => {
                    ResolvedFilterPrimitive::ColorMatrix { matrix }
                }
                FilterPrimitive::ComponentTransfer { tables } => {
                    ResolvedFilterPrimitive::ComponentTransfer {
                        tables: Arc::new([
                            *tables.red(),
                            *tables.green(),
                            *tables.blue(),
                            *tables.alpha(),
                        ]),
                    }
                }
                FilterPrimitive::Morphology {
                    operator,
                    radius_x,
                    radius_y,
                } => ResolvedFilterPrimitive::Morphology {
                    operator: match operator {
                        FilterMorphology::Erode => ResolvedFilterMorphology::Erode,
                        FilterMorphology::Dilate => ResolvedFilterMorphology::Dilate,
                    },
                    radius_x,
                    radius_y,
                },
                FilterPrimitive::Turbulence {
                    kind,
                    base_frequency_x,
                    base_frequency_y,
                    num_octaves,
                    seed,
                    stitch_tiles,
                } => ResolvedFilterPrimitive::Turbulence {
                    kind: match kind {
                        FilterTurbulenceKind::Turbulence => {
                            ResolvedFilterTurbulenceKind::Turbulence
                        }
                        FilterTurbulenceKind::FractalNoise => {
                            ResolvedFilterTurbulenceKind::FractalNoise
                        }
                    },
                    base_frequency_x,
                    base_frequency_y,
                    num_octaves,
                    seed,
                    stitch_tiles,
                },
                FilterPrimitive::DisplacementMap {
                    scale,
                    x_channel,
                    y_channel,
                } => ResolvedFilterPrimitive::DisplacementMap {
                    scale,
                    x_channel: match x_channel {
                        FilterDisplacementChannel::Red => ResolvedFilterDisplacementChannel::Red,
                        FilterDisplacementChannel::Green => {
                            ResolvedFilterDisplacementChannel::Green
                        }
                        FilterDisplacementChannel::Blue => ResolvedFilterDisplacementChannel::Blue,
                        FilterDisplacementChannel::Alpha => {
                            ResolvedFilterDisplacementChannel::Alpha
                        }
                    },
                    y_channel: match y_channel {
                        FilterDisplacementChannel::Red => ResolvedFilterDisplacementChannel::Red,
                        FilterDisplacementChannel::Green => {
                            ResolvedFilterDisplacementChannel::Green
                        }
                        FilterDisplacementChannel::Blue => ResolvedFilterDisplacementChannel::Blue,
                        FilterDisplacementChannel::Alpha => {
                            ResolvedFilterDisplacementChannel::Alpha
                        }
                    },
                },
                FilterPrimitive::ConvolveMatrix {
                    order_x,
                    order_y,
                    kernel,
                    gain,
                    bias,
                    target_x,
                    target_y,
                    edge_mode,
                    preserve_alpha,
                } => ResolvedFilterPrimitive::ConvolveMatrix {
                    order_x,
                    order_y,
                    kernel,
                    gain,
                    bias,
                    target_x,
                    target_y,
                    edge_mode: match edge_mode {
                        FilterConvolveEdgeMode::Duplicate => {
                            ResolvedFilterConvolveEdgeMode::Duplicate
                        }
                        FilterConvolveEdgeMode::Wrap => ResolvedFilterConvolveEdgeMode::Wrap,
                        FilterConvolveEdgeMode::None => ResolvedFilterConvolveEdgeMode::None,
                    },
                    preserve_alpha,
                },
                FilterPrimitive::DiffuseLighting {
                    surface_scale,
                    diffuse_constant,
                    color,
                    light,
                } => ResolvedFilterPrimitive::DiffuseLighting {
                    surface_scale,
                    diffuse_constant,
                    color: compile_color(color),
                    light: match light {
                        FilterLightSource::Distant { direction } => {
                            ResolvedFilterLightSource::Distant { direction }
                        }
                        FilterLightSource::Point { location } => {
                            ResolvedFilterLightSource::Point { location }
                        }
                        FilterLightSource::Spot {
                            location,
                            target,
                            falloff_exponent,
                            cutoff_angle,
                        } => ResolvedFilterLightSource::Spot {
                            location,
                            target,
                            falloff_exponent,
                            cutoff_angle,
                        },
                    },
                },
                FilterPrimitive::Merge => ResolvedFilterPrimitive::Merge,
            },
        })
        .collect::<Vec<_>>()
        .into();
    ResolvedFilter {
        region: to_rectf(filter.region()),
        nodes,
        may_paint_transparent_input: program.may_paint_transparent_input(),
        source_is_transparent: filter.source_is_transparent(),
    }
}

/// Project the contract's resolved stroke into the engine's private stroke.
///
/// The contract's stroke is centred on the geometry, which is the only
/// alignment a Web source can express; the engine's own vocabulary carries an
/// alignment, so the projection names it rather than relying on a default.
/// Width is uniform because a Web stroke has one width. A checked dash cycle
/// crosses unchanged: its intervals are already even, finite, non-negative,
/// positive-sum distances in the stroke's declared construction space, so the
/// private painter has nothing to parse or resolve again. Its paired canonical
/// phase crosses as the same scalar; normalization remains the contract
/// producer's responsibility.
fn compile_stroke(
    stroke: &rframe::Stroke,
    unit_offset: Option<(f32, f32)>,
) -> (Stroke, StrokeSpace, StrokeDashPhase, PostPaintOpacity) {
    let dash = stroke.dash();
    let material = Stroke {
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
        dash_array: dash.map(|dash| dash.intervals().as_slice().to_vec()),
    };
    let phase = dash.map_or(StrokeDashPhase::ZERO, |dash| {
        StrokeDashPhase::from_canonical(dash.phase())
    });
    let space = match stroke.space() {
        rframe::StrokeSpace::Local => StrokeSpace::Local,
        rframe::StrokeSpace::Frame => StrokeSpace::Frame,
    };
    let post_paint_opacity = PostPaintOpacity::from_resolved(stroke.paints().alpha_factor().get());
    (material, space, phase, post_paint_opacity)
}

fn transform_has_identity_linear_part(transform: &math2::transform::AffineTransform) -> bool {
    let [[a, c, _], [b, d, _]] = transform.matrix;
    a == 1.0 && b == 0.0 && c == 0.0 && d == 1.0
}

/// Blink and Skia suppress non-scaling-stroke when the f32 affine cannot
/// supply a finite nonzero determinant to the backend path projection. This
/// intentionally includes mathematical transforms whose determinant
/// underflows or overflows f32; Chromium paints no stroke for both measured
/// classes, while widening the centerline would paint a silent extra dot.
fn frame_stroke_transform_is_unusable(transform: &math2::transform::AffineTransform) -> bool {
    let [[a, c, _], [b, d, _]] = transform.matrix;
    let determinant = a * d - b * c;
    determinant == 0.0 || !determinant.is_finite()
}

fn validate_rect(rect: math2::Rectangle) -> Result<(), ()> {
    [rect.x, rect.y, rect.width, rect.height]
        .into_iter()
        .all(f32::is_finite)
        .then_some(())
        .filter(|_| rect.width >= 0.0 && rect.height >= 0.0)
        .ok_or(())
}

/// The frame clip is also the finite coordinate envelope for glyphless damage.
/// Its carried components are not enough: an individually finite origin and
/// extent can still produce an infinite far edge in the `RectF` arithmetic
/// used by the damage policy.
fn validate_frame_bounds(rect: math2::Rectangle) -> Result<(), ()> {
    validate_rect(rect)?;
    [rect.x + rect.width, rect.y + rect.height]
        .into_iter()
        .all(f32::is_finite)
        .then_some(())
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
            // Component-for-component: both leaves are checked unit sRGB, so
            // the resolved stop crosses into the model without a quantization
            // step that would substitute a neighbouring alpha.
            color: compile_color32f(stop.color),
        })
        .collect()
}

fn compile_color32f(color: cg::CGColor32F) -> n0_model::model::Color32F {
    n0_model::model::Color32F::new(color.r(), color.g(), color.b(), color.a())
        .expect("a checked cg colour is inside the model's checked unit domain")
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

fn compile_pattern_geometry(geometry: &Geometry) -> ResolvedPatternGeometry {
    match geometry {
        Geometry::Rect(rect) => ResolvedPatternGeometry::Rect {
            x: rect.x,
            y: rect.y,
            w: rect.width,
            h: rect.height,
        },
        Geometry::Ellipse(rect) => ResolvedPatternGeometry::Oval {
            x: rect.x,
            y: rect.y,
            w: rect.width,
            h: rect.height,
        },
        Geometry::Path(path) => ResolvedPatternGeometry::Path(compile_path(path)),
    }
}

/// Compile a checked nested frame program without issuing raster commands.
/// Recursive programs re-enter this same proving shell; `rframe` already
/// bounds their depth, and every nested gradient/effect receives the same
/// deterministic preflight as a top-level frame.
fn compile_pattern(
    pattern: &rframe::PatternPaint,
    owner: VisualRef,
) -> Result<Arc<ResolvedPattern>, BuildError> {
    let nested = Frame {
        owner: VisualRef::new(rframe::Identity::new(0), rframe::Provenance::new(0)),
        bounds: math2::Rectangle::from_xywh(0.0, 0.0, pattern.width(), pattern.height()),
        items: pattern.items().as_ref().clone(),
    };
    let product = compile(nested).map_err(|error| BuildError::Paint {
        owner,
        reason: format!("nested pattern program failed projection: {error}"),
    })?;
    Ok(Arc::new(ResolvedPattern {
        width: pattern.width(),
        height: pattern.height(),
        transform: to_affine(pattern.transform()),
        program: Arc::new(product.drawlist),
        opacity: pattern.opacity(),
    }))
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

/// An internal LTRB envelope wide enough for transforming the contract's
/// finite `f32` geometry by its current `f32`-derived wide stroke reach.
#[derive(Clone, Copy, Debug)]
struct WideRect {
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
}

impl WideRect {
    fn from_rectangle(rect: math2::Rectangle) -> Self {
        Self {
            left: f64::from(rect.x),
            top: f64::from(rect.y),
            right: f64::from(rect.x) + f64::from(rect.width),
            bottom: f64::from(rect.y) + f64::from(rect.height),
        }
    }

    fn from_rectf(rect: n0_model::math::RectF) -> Self {
        Self {
            left: f64::from(rect.x),
            top: f64::from(rect.y),
            right: f64::from(rect.x) + f64::from(rect.w),
            bottom: f64::from(rect.y) + f64::from(rect.h),
        }
    }

    fn inflated(self, outset: f64) -> Self {
        Self {
            left: (self.left - outset).next_down(),
            top: (self.top - outset).next_down(),
            right: (self.right + outset).next_up(),
            bottom: (self.bottom + outset).next_up(),
        }
    }

    fn transformed(self, transform: &math2::transform::AffineTransform) -> Option<Self> {
        let [[a_f32, c_f32, e_f32], [b_f32, d_f32, f_f32]] = transform.matrix;
        let [[a, c, e], [b, d, f]] = transform.matrix.map(|row| row.map(f64::from));
        let corners = [
            (self.left, self.top),
            (self.right, self.top),
            (self.right, self.bottom),
            (self.left, self.bottom),
        ];
        let mut left = f64::INFINITY;
        let mut top = f64::INFINITY;
        let mut right = f64::NEG_INFINITY;
        let mut bottom = f64::NEG_INFINITY;
        for (local_x, local_y) in corners {
            let (x_low, x_high) = affine_component_bounds(a, local_x, c, local_y, e)?;
            let (y_low, y_high) = affine_component_bounds(b, local_x, d, local_y, f)?;
            left = left.min(x_low);
            top = top.min(y_low);
            right = right.max(x_high);
            bottom = bottom.max(y_high);
        }

        // The contract's carried node bounds and the painter both use
        // sequential-f32 affine arithmetic, while the wide stroke reach above
        // deliberately uses f64. Cancellation can put an f32 result beyond the
        // real-arithmetic interval by one or more f32 values. Map an outward
        // f32 cover of the inflated local box through that same operation order
        // and union both envelopes: one covers the derived wide reach, the
        // other every finite SkScalar/local-f32 point the consumer can map.
        let f32_corners = [
            (
                floor_f32_saturated(self.left)?,
                floor_f32_saturated(self.top)?,
            ),
            (
                ceil_f32_saturated(self.right)?,
                floor_f32_saturated(self.top)?,
            ),
            (
                ceil_f32_saturated(self.right)?,
                ceil_f32_saturated(self.bottom)?,
            ),
            (
                floor_f32_saturated(self.left)?,
                ceil_f32_saturated(self.bottom)?,
            ),
        ];
        for (local_x, local_y) in f32_corners {
            let x = sequential_f32_affine_component(a_f32, local_x, c_f32, local_y, e_f32)?;
            let y = sequential_f32_affine_component(b_f32, local_x, d_f32, local_y, f_f32)?;
            left = left.min(x);
            top = top.min(y);
            right = right.max(x);
            bottom = bottom.max(y);
        }
        Some(Self {
            left,
            top,
            right,
            bottom,
        })
    }

    fn intersection(self, other: Self) -> Option<Self> {
        let intersection = Self {
            left: self.left.max(other.left),
            top: self.top.max(other.top),
            right: self.right.min(other.right),
            bottom: self.bottom.min(other.bottom),
        };
        (intersection.left < intersection.right && intersection.top < intersection.bottom)
            .then_some(intersection)
    }

    fn union(self, other: Self) -> Self {
        Self {
            left: self.left.min(other.left),
            top: self.top.min(other.top),
            right: self.right.max(other.right),
            bottom: self.bottom.max(other.bottom),
        }
    }
}

/// Outward bounds for one affine component `p*x + q*y + translation`.
/// Advancing each rounded operation in both directions makes the f64 stage a
/// conservative envelope rather than assuming round-to-nearest chose the
/// harmless side of an exact geometric boundary.
fn affine_component_bounds(p: f64, x: f64, q: f64, y: f64, translation: f64) -> Option<(f64, f64)> {
    let px = p * x;
    let qy = q * y;
    if !px.is_finite() || !qy.is_finite() {
        return None;
    }
    let low = (px.next_down() + qy.next_down()).next_down();
    let high = (px.next_up() + qy.next_up()).next_up();
    if !low.is_finite() || !high.is_finite() {
        return None;
    }
    let low = (low + translation).next_down();
    let high = (high + translation).next_up();
    (low.is_finite() && high.is_finite()).then_some((low, high))
}

/// The operation order used by `math2::vector2::transform` and by the carried
/// node bounds. Named intermediates keep the two products and two additions as
/// four f32 rounding steps; the wide real-arithmetic interval is unioned by the
/// caller rather than substituted for these semantics.
fn sequential_f32_affine_component(
    p: f32,
    x: f32,
    q: f32,
    y: f32,
    translation: f32,
) -> Option<f64> {
    let px = p * x;
    let qy = q * y;
    let sum = px + qy;
    let value = sum + translation;
    value.is_finite().then(|| f64::from(value))
}

fn bounded_geometry_coverage(
    geometry_bounds: math2::Rectangle,
    frame_bounds: math2::Rectangle,
) -> Option<n0_model::math::RectF> {
    let clipped = WideRect::from_rectangle(geometry_bounds)
        .intersection(WideRect::from_rectangle(frame_bounds))?;
    Some(rectf_covering_bounded(clipped, frame_bounds))
}

fn bounded_stroke_coverage(
    local_bounds: math2::Rectangle,
    transform: &math2::transform::AffineTransform,
    box_world: Option<&Affine>,
    outset: f64,
    frame_bounds: math2::Rectangle,
) -> Option<n0_model::math::RectF> {
    let clip = WideRect::from_rectangle(frame_bounds);
    let direct = WideRect::from_rectangle(local_bounds)
        .inflated(outset)
        .transformed(transform);
    let projected = direct.and_then(|direct| match box_world {
        None => Some(direct),
        Some(world) => {
            // Rects and ordinary origin-relative ellipses fold their local
            // origin into `world` before painting. A live dashed ellipse takes
            // the absolute-coordinate route instead; retaining this box
            // projection for it is a deliberate conservative union, not a
            // second claim about the pixels it paints. The two projections are
            // mathematically equivalent but not rounding-equivalent, so damage
            // keeps both until generic envelope work can treat every route
            // uniformly. Paths keep the direct absolute-coordinate route only.
            let box_transform = math2::transform::AffineTransform::from_acebdf(
                world.a, world.c, world.e, world.b, world.d, world.f,
            );
            WideRect::from_rectangle(math2::Rectangle::from_xywh(
                0.0,
                0.0,
                local_bounds.width,
                local_bounds.height,
            ))
            .inflated(outset)
            .transformed(&box_transform)
            .map(|box_projection| direct.union(box_projection))
        }
    });
    // The current contract derives `outset` only from finite f32 stroke facts,
    // so this branch is unreachable today. Keeping it conservative makes a
    // future widening fall back to full-frame damage rather than mint NaN/inf
    // or silently under-report coverage.
    let projected = match projected {
        Some(projected) => projected,
        None => return bounded_geometry_coverage(frame_bounds, frame_bounds),
    };
    let clipped = projected.intersection(clip)?;
    Some(rectf_covering_bounded(clipped, frame_bounds))
}

fn bounded_frame_stroke_coverage(
    geometry_bounds: math2::Rectangle,
    outset: f64,
    frame_bounds: math2::Rectangle,
) -> Option<n0_model::math::RectF> {
    let clipped = WideRect::from_rectangle(geometry_bounds)
        .inflated(outset)
        .intersection(WideRect::from_rectangle(frame_bounds))?;
    Some(rectf_covering_bounded(clipped, frame_bounds))
}

fn bounded_union_rectf(
    a: n0_model::math::RectF,
    b: n0_model::math::RectF,
    frame_bounds: math2::Rectangle,
) -> n0_model::math::RectF {
    let frame = WideRect::from_rectangle(frame_bounds);
    let union = WideRect::from_rectf(a).union(WideRect::from_rectf(b));
    let clipped = union
        .intersection(frame)
        .expect("non-empty bounded coverages have a non-empty bounded union");
    rectf_covering_bounded(clipped, frame_bounds)
}

fn bounded_intersection_rectf(
    a: n0_model::math::RectF,
    b: n0_model::math::RectF,
    frame_bounds: math2::Rectangle,
) -> Option<n0_model::math::RectF> {
    let frame = WideRect::from_rectangle(frame_bounds);
    let intersection = WideRect::from_rectf(a)
        .intersection(WideRect::from_rectf(b))?
        .intersection(frame)?;
    Some(rectf_covering_bounded(intersection, frame_bounds))
}

/// Encode a wide non-empty rectangle as an outward-covering `RectF`, without
/// ever crossing the frame envelope. If one axis cannot be rounded outward and
/// remain inside that envelope, the frame's full axis is the smallest stable
/// fallback available in the target representation.
fn rectf_covering_bounded(
    bounds: WideRect,
    frame_bounds: math2::Rectangle,
) -> n0_model::math::RectF {
    let (x, w) = covering_axis_bounded(
        bounds.left,
        bounds.right,
        frame_bounds.x,
        frame_bounds.width,
    );
    let (y, h) = covering_axis_bounded(
        bounds.top,
        bounds.bottom,
        frame_bounds.y,
        frame_bounds.height,
    );
    n0_model::math::RectF { x, y, w, h }
}

fn covering_axis_bounded(min: f64, max: f64, frame_start: f32, frame_extent: f32) -> (f32, f32) {
    let frame_end = f64::from(frame_start) + f64::from(frame_extent);
    if let Some((start, extent)) = covering_axis(min, max) {
        let rounded_end = start + extent;
        let exact_start = f64::from(start);
        let exact_end = exact_start + f64::from(extent);
        if exact_start >= f64::from(frame_start)
            && rounded_end.is_finite()
            && exact_end <= frame_end
        {
            return (start, extent);
        }
    }
    (frame_start, frame_extent)
}

fn covering_axis(min: f64, max: f64) -> Option<(f32, f32)> {
    let start = floor_f32(min)?;
    let target_end = ceil_f32(max)?;
    let mut extent = ceil_f32(f64::from(target_end) - f64::from(start))?;
    loop {
        let end = start + extent;
        if end.is_finite() && end >= target_end {
            return Some((start, extent));
        }
        extent = next_up_f32(extent);
        if !extent.is_finite() {
            return None;
        }
    }
}

fn floor_f32(value: f64) -> Option<f32> {
    let rounded = finite_f32(value)?;
    Some(if f64::from(rounded) > value {
        next_down_f32(rounded)
    } else {
        rounded
    })
}

fn ceil_f32(value: f64) -> Option<f32> {
    let rounded = finite_f32(value)?;
    Some(if f64::from(rounded) < value {
        next_up_f32(rounded)
    } else {
        rounded
    })
}

/// Outward f32 domain endpoints for the painter's finite scalar space. A wide
/// stroke reach may exceed that space; saturating only this companion domain
/// keeps the separate f64 projection intact instead of broadening every such
/// stroke to the frame.
fn floor_f32_saturated(value: f64) -> Option<f32> {
    if value.is_nan() {
        None
    } else if value <= -f64::from(f32::MAX) {
        Some(-f32::MAX)
    } else if value >= f64::from(f32::MAX) {
        Some(f32::MAX)
    } else {
        floor_f32(value)
    }
}

fn ceil_f32_saturated(value: f64) -> Option<f32> {
    if value.is_nan() {
        None
    } else if value <= -f64::from(f32::MAX) {
        Some(-f32::MAX)
    } else if value >= f64::from(f32::MAX) {
        Some(f32::MAX)
    } else {
        ceil_f32(value)
    }
}

fn finite_f32(value: f64) -> Option<f32> {
    let rounded = value as f32;
    rounded.is_finite().then_some(rounded)
}

fn next_up_f32(value: f32) -> f32 {
    if value == f32::INFINITY {
        return value;
    }
    if value == 0.0 {
        return f32::from_bits(1);
    }
    let bits = value.to_bits();
    f32::from_bits(if value > 0.0 { bits + 1 } else { bits - 1 })
}

fn next_down_f32(value: f32) -> f32 {
    if value == f32::NEG_INFINITY {
        return value;
    }
    if value == 0.0 {
        return -f32::from_bits(1);
    }
    let bits = value.to_bits();
    f32::from_bits(if value > 0.0 { bits - 1 } else { bits + 1 })
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
    use rframe::{
        Filter, FilterNode, FilterProgram, FrameItems, FrameNode, Identity, PaintAlphaFactor,
        Provenance, Scope, ScopeOpacity,
    };
    use skia_safe::surfaces;

    use super::*;

    const FRAME_OWNER: VisualRef = VisualRef::new(Identity::new(10), Provenance::new(100));
    const RECT_OWNER: VisualRef = VisualRef::new(Identity::new(20), Provenance::new(200));
    const SCOPE_OWNER: VisualRef = VisualRef::new(Identity::new(30), Provenance::new(300));
    const OTHER_OWNER: VisualRef = VisualRef::new(Identity::new(40), Provenance::new(400));
    const INNER_SCOPE_OWNER: VisualRef = VisualRef::new(Identity::new(50), Provenance::new(500));
    const CLIPPED_OWNER: VisualRef = VisualRef::new(Identity::new(60), Provenance::new(600));

    fn cg_solid(argb: u32) -> CgPaint {
        CgPaint::Solid(CgSolidPaint::new_color(CGColor::from_u32_argb(argb)))
    }

    fn solid_stack<const N: usize>(paints: [CgPaint; N]) -> PaintStack {
        PaintStack::try_from_paints(CgPaints::new(paints))
            .expect("test paints are visible ordinary solids")
    }

    fn alpha_factor(value: f32) -> PaintAlphaFactor {
        PaintAlphaFactor::new(value).expect("test factor is in the closed unit interval")
    }

    fn post_paint_opacity(kind: &ItemKind) -> Option<PostPaintOpacity> {
        match kind {
            ItemKind::PatternFill {
                post_paint_opacity, ..
            }
            | ItemKind::PatternStroke {
                post_paint_opacity, ..
            }
            | ItemKind::RectFill {
                post_paint_opacity, ..
            }
            | ItemKind::OvalFill {
                post_paint_opacity, ..
            }
            | ItemKind::PathFill {
                post_paint_opacity, ..
            }
            | ItemKind::TextFill {
                post_paint_opacity, ..
            }
            | ItemKind::RectStroke {
                post_paint_opacity, ..
            }
            | ItemKind::OvalStroke {
                post_paint_opacity, ..
            }
            | ItemKind::AbsoluteDashedOvalStroke {
                post_paint_opacity, ..
            }
            | ItemKind::LineStroke {
                post_paint_opacity, ..
            }
            | ItemKind::PathStroke {
                post_paint_opacity, ..
            }
            | ItemKind::TextStroke {
                post_paint_opacity, ..
            } => Some(*post_paint_opacity),
            ItemKind::BeginOpacity { .. }
            | ItemKind::BeginIsolatedOpacity { .. }
            | ItemKind::EndOpacity
            | ItemKind::BeginClipRect { .. }
            | ItemKind::BeginClipPath { .. }
            | ItemKind::EndClip
            | ItemKind::BeginMaskContent
            | ItemKind::BeginMaskSource { .. }
            | ItemKind::EndMaskSource
            | ItemKind::EndMaskContent
            | ItemKind::BeginFilter { .. }
            | ItemKind::EndFilter => None,
        }
    }

    fn dashed_stroke(intervals: Vec<f32>, cap: rframe::StrokeCap) -> rframe::Stroke {
        phased_stroke(intervals, cap, 0.0)
    }

    fn phased_stroke(intervals: Vec<f32>, cap: rframe::StrokeCap, phase: f32) -> rframe::Stroke {
        let intervals = rframe::StrokeDashIntervals::new(intervals)
            .expect("test dash intervals are valid")
            .expect("test dash cycle is present");
        let dash = rframe::StrokeDash::new(intervals, phase).expect("test dash phase is finite");
        rframe::Stroke::new_with_dash(
            PaintStack::solid(CGColor::BLACK),
            8.0,
            cap,
            rframe::StrokeJoin::Miter,
            4.0,
            Some(dash),
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

    fn checked_stroke(
        width: f32,
        cap: rframe::StrokeCap,
        join: rframe::StrokeJoin,
        miter_limit: f32,
        dash: Option<Vec<f32>>,
    ) -> rframe::Stroke {
        let dash = dash.map(|intervals| {
            rframe::StrokeDashIntervals::new(intervals)
                .expect("test dash intervals are valid")
                .expect("test dash cycle is present")
        });
        rframe::Stroke::new_with_dash_intervals(
            PaintStack::solid(CGColor::BLACK),
            width,
            cap,
            join,
            miter_limit,
            dash,
        )
        .expect("test stroke is valid")
        .expect("test stroke paints")
    }

    fn stroked_node(
        owner: VisualRef,
        geometry: Geometry,
        transform: AffineTransform,
        stroke: rframe::Stroke,
    ) -> FrameNode {
        let bounds = math2::rect_transform(geometry.local_box(), &transform);
        FrameNode {
            owner,
            transform,
            geometry,
            bounds,
            paints: PaintStack::empty(),
            stroke: Some(stroke),
        }
    }

    fn coverage_for(product: &FrameProduct, owner: VisualRef) -> Option<n0_model::math::RectF> {
        let slot = product
            .provenance
            .owners
            .iter()
            .position(|candidate| *candidate == owner)
            .expect("test owner has provenance");
        product.provenance.coverage[slot]
    }

    fn private_stroke(product: &FrameProduct) -> &Stroke {
        product
            .drawlist
            .items
            .iter()
            .find_map(|item| match &item.kind {
                ItemKind::RectStroke { stroke, .. }
                | ItemKind::OvalStroke { stroke, .. }
                | ItemKind::AbsoluteDashedOvalStroke { stroke, .. }
                | ItemKind::PathStroke { stroke, .. } => Some(stroke),
                _ => None,
            })
            .expect("test product has one stroke item")
    }

    fn private_dash_phase(product: &FrameProduct) -> StrokeDashPhase {
        product
            .drawlist
            .items
            .iter()
            .find_map(|item| match &item.kind {
                ItemKind::RectStroke { dash_phase, .. }
                | ItemKind::OvalStroke { dash_phase, .. }
                | ItemKind::AbsoluteDashedOvalStroke { dash_phase, .. }
                | ItemKind::PathStroke { dash_phase, .. } => Some(*dash_phase),
                _ => None,
            })
            .expect("test product has one stroke item")
    }

    fn private_stroke_space(product: &FrameProduct) -> StrokeSpace {
        product
            .drawlist
            .items
            .iter()
            .find_map(|item| match &item.kind {
                ItemKind::PatternStroke { space, .. }
                | ItemKind::RectStroke { space, .. }
                | ItemKind::OvalStroke { space, .. }
                | ItemKind::AbsoluteDashedOvalStroke { space, .. }
                | ItemKind::LineStroke { space, .. }
                | ItemKind::PathStroke { space, .. }
                | ItemKind::TextStroke { space, .. } => Some(*space),
                _ => None,
            })
            .expect("test product has one stroke item")
    }

    fn assert_finite_bounded_coverage(coverage: n0_model::math::RectF, frame: Rectangle) {
        assert!(
            [coverage.x, coverage.y, coverage.w, coverage.h]
                .into_iter()
                .all(f32::is_finite),
            "coverage components stay finite: {coverage:?}"
        );
        assert!(coverage.w > 0.0 && coverage.h > 0.0);
        assert!((coverage.x + coverage.w).is_finite());
        assert!((coverage.y + coverage.h).is_finite());
        let coverage = WideRect::from_rectf(coverage);
        let frame = WideRect::from_rectangle(frame);
        assert!(coverage.left >= frame.left);
        assert!(coverage.top >= frame.top);
        assert!(coverage.right <= frame.right);
        assert!(coverage.bottom <= frame.bottom);
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

    fn blur_scope_begin(owner: VisualRef, input: FilterInput) -> FrameItem {
        let region = Rectangle::from_xywh(0.0, 0.0, 64.0, 48.0);
        let program = FilterProgram::new(Arc::from([FilterNode::new(
            Arc::from([input]),
            region,
            FilterColorSpace::LinearRgb,
            FilterPrimitive::GaussianBlur {
                sigma_x: 3.0,
                sigma_y: 3.0,
            },
        )]))
        .expect("test blur is a checked program");
        FrameItem::ScopeBegin(Scope {
            owner,
            effect: ScopeEffect::Filter(
                Filter::new(AffineTransform::identity(), region, program)
                    .expect("test filter is a checked effect"),
            ),
        })
    }

    fn empty_turbulence_scope_begin(owner: VisualRef, seed: f32) -> FrameItem {
        let region = Rectangle::from_xywh(4.0, 5.0, 10.0, 12.0);
        let program = FilterProgram::new(Arc::from([FilterNode::new(
            Arc::from([]),
            region,
            FilterColorSpace::LinearRgb,
            FilterPrimitive::Turbulence {
                kind: FilterTurbulenceKind::Turbulence,
                base_frequency_x: 0.08,
                base_frequency_y: 0.11,
                num_octaves: 2,
                seed,
                stitch_tiles: false,
            },
        )]))
        .expect("test turbulence is a checked generated program");
        let transform = AffineTransform::from_acebdf(1.0, 0.0, 7.0, 0.0, 1.0, 8.0);
        FrameItem::ScopeBegin(Scope {
            owner,
            effect: ScopeEffect::Filter(
                Filter::new(transform, region, program)
                    .expect("test filter is a checked effect")
                    .with_transparent_source(),
            ),
        })
    }

    fn clip_begin(owner: VisualRef, layers: Vec<Vec<(Rectangle, AffineTransform)>>) -> FrameItem {
        clip_begin_with_edge(owner, layers, rframe::ClipEdgeMode::AntiAliased)
    }

    fn clip_begin_with_edge(
        owner: VisualRef,
        layers: Vec<Vec<(Rectangle, AffineTransform)>>,
        edge_mode: rframe::ClipEdgeMode,
    ) -> FrameItem {
        let layers = layers
            .into_iter()
            .map(|geometries| {
                let geometries = geometries
                    .into_iter()
                    .map(|(rect, transform)| {
                        rframe::ClipGeometry::new(transform, Geometry::Rect(rect))
                            .expect("test clip geometry is resolved")
                    })
                    .collect::<Vec<_>>();
                rframe::ClipLayer::new(geometries).expect("test clip layer is bounded")
            })
            .collect::<Vec<_>>();
        FrameItem::ScopeBegin(Scope {
            owner,
            effect: ScopeEffect::Clip(
                rframe::ClipPath::new_with_edge_mode(layers, edge_mode)
                    .expect("test clip has at least one layer"),
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
        let ItemKind::RectFill {
            paints,
            post_paint_opacity: resolved_factor,
            ..
        } = &product.drawlist.items[1].kind
        else {
            panic!("second item is the rectangle fill");
        };
        assert_eq!(*resolved_factor, PostPaintOpacity::IDENTITY);
        assert_eq!(
            ordinary
                .drawlist()
                .items
                .iter()
                .find_map(|item| post_paint_opacity(&item.kind)),
            Some(PostPaintOpacity::IDENTITY),
            "an ordinary native producer projects the identity factor"
        );
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
    fn paint_alpha_factor_crosses_every_glyphless_geometry_route_bit_exactly() {
        let fill_value = f32::from_bits(0x3eaa_aaab);
        let stroke_value = f32::from_bits(0x3f20_0001);
        let fill_factor = alpha_factor(fill_value);
        let stroke_factor = alpha_factor(stroke_value);
        let path = rframe::PathData::new(
            vec![
                rframe::PathCommand::MoveTo { x: 8.0, y: 6.0 },
                rframe::PathCommand::LineTo { x: 28.0, y: 6.0 },
                rframe::PathCommand::LineTo { x: 28.0, y: 22.0 },
                rframe::PathCommand::LineTo { x: 8.0, y: 22.0 },
                rframe::PathCommand::Close,
            ],
            rframe::FillRule::NonZero,
        )
        .expect("test path is valid");
        let routes = [
            (
                "rect",
                Geometry::Rect(Rectangle::from_xywh(8.0, 6.0, 20.0, 16.0)),
                false,
            ),
            (
                "ellipse",
                Geometry::Ellipse(Rectangle::from_xywh(8.0, 6.0, 20.0, 16.0)),
                false,
            ),
            (
                "dashed ellipse",
                Geometry::Ellipse(Rectangle::from_xywh(8.0, 6.0, 20.0, 16.0)),
                true,
            ),
            ("path", Geometry::Path(Arc::new(path)), false),
        ];

        for (route, geometry, dashed) in routes {
            let stroke = if dashed {
                dashed_stroke(vec![4.0, 2.0], rframe::StrokeCap::Round)
            } else {
                checked_stroke(
                    4.0,
                    rframe::StrokeCap::Butt,
                    rframe::StrokeJoin::Miter,
                    4.0,
                    None,
                )
            }
            .with_paint_alpha_factor(stroke_factor)
            .expect("nonzero factor keeps the stroke");
            let bounds = geometry.local_box();
            let node = FrameNode {
                owner: RECT_OWNER,
                transform: AffineTransform::identity(),
                geometry,
                bounds,
                paints: PaintStack::solid(CGColor::RED).with_alpha_factor(fill_factor),
                stroke: Some(stroke),
            };
            let product = compile(frame_of(FrameItems::from_nodes(vec![node])))
                .unwrap_or_else(|error| panic!("{route} factor frame failed: {error}"));
            let factors = product
                .drawlist
                .items
                .iter()
                .filter_map(|item| post_paint_opacity(&item.kind))
                .map(PostPaintOpacity::value)
                .collect::<Vec<_>>();
            assert_eq!(factors.len(), 2, "{route} has one fill and one stroke");
            assert_eq!(factors[0].to_bits(), fill_value.to_bits(), "{route} fill");
            assert_eq!(
                factors[1].to_bits(),
                stroke_value.to_bits(),
                "{route} stroke"
            );
            assert!(
                matches!(
                    (
                        route,
                        &product.drawlist.items[1].kind,
                        &product.drawlist.items[2].kind
                    ),
                    (
                        "rect",
                        ItemKind::RectFill { .. },
                        ItemKind::RectStroke { .. }
                    ) | (
                        "ellipse",
                        ItemKind::OvalFill { .. },
                        ItemKind::OvalStroke { .. }
                    ) | (
                        "dashed ellipse",
                        ItemKind::OvalFill { .. },
                        ItemKind::AbsoluteDashedOvalStroke { .. },
                    ) | (
                        "path",
                        ItemKind::PathFill { .. },
                        ItemKind::PathStroke { .. }
                    )
                ),
                "{route} retained its geometry-specific fill and stroke routes"
            );
        }
    }

    #[test]
    fn factor_only_change_affects_raster_identity_and_bounded_damage() {
        let scene = |factor: f32| {
            let paints = PaintStack::solid(CGColor::BLACK).with_alpha_factor(alpha_factor(factor));
            compile(resolved_frame(paints)).expect("factored solid frame")
        };
        let identity = scene(1.0);
        let half = scene(0.5);

        assert_ne!(identity.drawlist, half.drawlist);
        assert!(!identity.drawlist.raster_eq(&half.drawlist));
        assert_eq!(
            diff_frame(&identity, &half),
            Damage {
                changed: vec![RECT_OWNER],
                union_frame: Some(Rectangle::from_xywh(8.0, 6.0, 20.0, 16.0)),
            }
        );

        let context = PaintCtx::new(None);
        let view = AffineTransform::identity();
        let identity_pixels = identity
            .raster_to_bytes(&view, 64, 48, &context)
            .expect("identity raster");
        let half_pixels = half
            .raster_to_bytes(&view, 64, 48, &context)
            .expect("factored raster");
        assert_eq!(rgba_at(&identity_pixels, 64, 16, 12), [0, 0, 0, 255]);
        assert_eq!(rgba_at(&half_pixels, 64, 16, 12), [127, 127, 127, 255]);
        assert_eq!(
            half_pixels,
            half.raster_to_bytes(&view, 64, 48, &context)
                .expect("repeat factored raster"),
            "the factor is retained raster material"
        );
    }

    #[test]
    fn factor_does_not_bypass_or_change_gradient_preflight() {
        let factor = alpha_factor(0.37);
        let colors = vec![CGColor::BLACK, CGColor::WHITE];
        let valid = [
            CgPaint::LinearGradient(cg::LinearGradientPaint::from_colors(colors.clone())),
            CgPaint::RadialGradient(cg::RadialGradientPaint::from_colors(colors.clone())),
        ];
        for paint in valid {
            let paints = PaintStack::try_from_paints(CgPaints::new([paint]))
                .expect("test gradient stack")
                .with_alpha_factor(factor);
            compile(resolved_frame(paints)).expect("factored valid gradient passes preflight");
        }

        let mut singular = cg::LinearGradientPaint::from_colors(colors);
        singular.transform = AffineTransform::from_acebdf(0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        let paints =
            PaintStack::try_from_paints(CgPaints::new([CgPaint::LinearGradient(singular)]))
                .expect("finite singular gradient remains a preflight concern")
                .with_alpha_factor(factor);
        let error = compile(resolved_frame(paints)).expect_err("factor cannot bypass preflight");
        let BuildError::Paint { owner, reason } = error else {
            panic!("singular gradient returned the wrong error: {error:?}");
        };
        assert_eq!(owner, RECT_OWNER);
        assert!(reason.contains("invertible"), "unexpected reason: {reason}");
    }

    /// The source-neutral seam copies one checked local-space pattern exactly;
    /// it does not drop, repeat, scale, renormalize, or otherwise reinterpret
    /// producer facts.
    #[test]
    fn resolved_dash_pattern_projects_exactly_into_private_stroke_material() {
        let mut node = base_node(PaintStack::empty());
        node.stroke = Some(phased_stroke(
            vec![0.0, 8.0, 3.0, 0.0],
            rframe::StrokeCap::Round,
            9.25,
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
        assert_eq!(
            private_dash_phase(&product).value().to_bits(),
            9.25f32.to_bits()
        );
    }

    /// The frame contract states the construction space directly. The private
    /// drawlist preserves it, coverage expands after centerline projection,
    /// and rasterization is identical to an explicitly transformed centerline
    /// stroked at the same nominal width.
    #[test]
    fn frame_space_stroke_projects_without_recovering_meaning_from_transform() {
        let geometry = Geometry::Rect(Rectangle::from_xywh(8.0, 8.0, 8.0, 8.0));
        let transform = AffineTransform::from_acebdf(2.0, 0.0, 0.0, 0.0, 0.5, 0.0);
        let stroke = checked_stroke(
            4.0,
            rframe::StrokeCap::Butt,
            rframe::StrokeJoin::Bevel,
            1.0,
            None,
        )
        .with_space(rframe::StrokeSpace::Frame);
        let projected = compile(frame_of(FrameItems::from_nodes(vec![stroked_node(
            RECT_OWNER,
            geometry,
            transform,
            stroke.clone(),
        )])))
        .expect("frame-space stroke compiles");

        assert_eq!(private_stroke_space(&projected), StrokeSpace::Frame);
        let coverage = coverage_for(&projected, RECT_OWNER).expect("stroke has coverage");
        let wide = WideRect::from_rectf(coverage);
        assert!(wide.left <= 14.0 && wide.left > 13.9);
        assert!(wide.top <= 2.0 && wide.top > 1.9);
        assert!(wide.right >= 34.0 && wide.right < 34.1);
        assert!(wide.bottom >= 10.0 && wide.bottom < 10.1);
        assert_finite_bounded_coverage(coverage, projected.resolved.bounds);

        let manual = compile(frame_of(FrameItems::from_nodes(vec![stroked_node(
            RECT_OWNER,
            Geometry::Rect(Rectangle::from_xywh(16.0, 4.0, 16.0, 4.0)),
            AffineTransform::identity(),
            stroke.with_space(rframe::StrokeSpace::Local),
        )])))
        .expect("manual transformed-centerline control compiles");
        let ctx = PaintCtx::new(None);
        assert_eq!(
            projected
                .raster_to_bytes(&AffineTransform::identity(), 64, 48, &ctx)
                .expect("frame-space raster"),
            manual
                .raster_to_bytes(&AffineTransform::identity(), 64, 48, &ctx)
                .expect("manual raster")
        );
    }

    #[test]
    fn translation_only_frame_space_stroke_keeps_the_local_f32_draw_order() {
        let path = rframe::PathData::new(
            vec![
                rframe::PathCommand::MoveTo {
                    x: 100_000_008.0,
                    y: 100_000_016.0,
                },
                rframe::PathCommand::LineTo {
                    x: 100_000_048.0,
                    y: 100_000_016.0,
                },
                rframe::PathCommand::LineTo {
                    x: 100_000_048.0,
                    y: 100_000_040.0,
                },
            ],
            rframe::FillRule::NonZero,
        )
        .expect("large translated path is valid");
        let transform =
            AffineTransform::from_acebdf(1.0, 0.0, -100_000_000.0, 0.0, 1.0, -100_000_000.0);
        let local = checked_stroke(
            4.0,
            rframe::StrokeCap::Round,
            rframe::StrokeJoin::Round,
            1.0,
            None,
        );
        let scene = |space| {
            compile(frame_of(FrameItems::from_nodes(vec![stroked_node(
                RECT_OWNER,
                Geometry::Path(Arc::new(path.clone())),
                transform,
                local.clone().with_space(space),
            )])))
            .expect("translated stroke compiles")
        };
        let ordinary = scene(rframe::StrokeSpace::Local);
        let non_scaling = scene(rframe::StrokeSpace::Frame);
        assert_eq!(private_stroke_space(&non_scaling), StrokeSpace::Frame);
        let ctx = PaintCtx::new(None);
        assert_eq!(
            non_scaling
                .raster_to_bytes(&AffineTransform::identity(), 64, 48, &ctx)
                .expect("frame-space translation raster"),
            ordinary
                .raster_to_bytes(&AffineTransform::identity(), 64, 48, &ctx)
                .expect("ordinary translation raster"),
            "translation is an equivalent local execution, including f32 cancellation order"
        );
    }

    #[test]
    fn backend_unusable_frame_space_affines_suppress_the_stroke() {
        let path = rframe::PathData::new(
            vec![
                rframe::PathCommand::MoveTo { x: 0.0, y: 0.0 },
                rframe::PathCommand::LineTo { x: 0.0, y: 0.0 },
            ],
            rframe::FillRule::NonZero,
        )
        .expect("zero-length path is valid");
        for scale in [1.0e-30, 1.0e30] {
            let transform = AffineTransform::from_acebdf(scale, 0.0, 32.0, 0.0, scale, 24.0);
            let stroke = checked_stroke(
                8.0,
                rframe::StrokeCap::Round,
                rframe::StrokeJoin::Round,
                1.0,
                None,
            )
            .with_space(rframe::StrokeSpace::Frame);
            let product = compile(frame_of(FrameItems::from_nodes(vec![stroked_node(
                RECT_OWNER,
                Geometry::Path(Arc::new(path.clone())),
                transform,
                stroke,
            )])))
            .expect("backend-unusable frame affine resolves to no stroke");
            assert_eq!(coverage_for(&product, RECT_OWNER), None, "scale={scale}");
            assert!(
                product.drawlist.items.iter().all(|item| !matches!(
                    item.kind,
                    ItemKind::PathStroke { .. } | ItemKind::PatternStroke { .. }
                )),
                "scale={scale} must emit no stroke item"
            );
        }
    }

    #[test]
    fn singular_frame_space_stroke_is_exact_nothing_but_fill_survives() {
        let geometry = Geometry::Rect(Rectangle::from_xywh(8.0, 8.0, 8.0, 8.0));
        let transform = AffineTransform::from_acebdf(0.0, 0.0, 24.0, 0.0, 1.0, 0.0);
        let stroke = checked_stroke(
            4.0,
            rframe::StrokeCap::Round,
            rframe::StrokeJoin::Round,
            1.0,
            None,
        )
        .with_space(rframe::StrokeSpace::Frame);
        let mut node = stroked_node(RECT_OWNER, geometry, transform, stroke);
        node.paints = PaintStack::solid(CGColor::BLACK);
        let product = compile(frame_of(FrameItems::from_nodes(vec![node])))
            .expect("singular frame-space stroke does not poison its fill");

        assert!(
            product.drawlist.items.iter().all(|item| !matches!(
                item.kind,
                ItemKind::PatternStroke { .. }
                    | ItemKind::RectStroke { .. }
                    | ItemKind::OvalStroke { .. }
                    | ItemKind::AbsoluteDashedOvalStroke { .. }
                    | ItemKind::LineStroke { .. }
                    | ItemKind::PathStroke { .. }
                    | ItemKind::TextStroke { .. }
            )),
            "the singular stroke emits no private paint item"
        );
        assert!(
            product
                .drawlist
                .items
                .iter()
                .any(|item| matches!(item.kind, ItemKind::RectFill { .. })),
            "the independent fill remains"
        );
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

    /// Phase is paint-consumed private material: changing only the canonical
    /// phase damages the owner and changes pixels, while phases that the
    /// contract has already reduced to the same cycle position remain one
    /// structural and raster identity.
    #[test]
    fn resolved_dash_phase_participates_in_damage_and_raster_identity() {
        let scene = |phase| {
            let rect = Rectangle::from_xywh(8.0, 6.0, 40.0, 28.0);
            let node = stroked_node(
                RECT_OWNER,
                Geometry::Rect(rect),
                AffineTransform::identity(),
                phased_stroke(vec![8.0, 4.0], rframe::StrokeCap::Round, phase),
            );
            compile(frame_of(FrameItems::from_nodes(vec![node]))).expect("admitted phased frame")
        };
        let zero = scene(0.0);
        let shifted = scene(3.0);
        let shifted_by_a_cycle = scene(15.0);

        assert_ne!(zero.drawlist, shifted.drawlist);
        assert!(!zero.drawlist.raster_eq(&shifted.drawlist));
        assert_eq!(shifted.drawlist, shifted_by_a_cycle.drawlist);
        assert!(shifted.drawlist.raster_eq(&shifted_by_a_cycle.drawlist));
        assert_eq!(diff_frame(&zero, &shifted).changed, vec![RECT_OWNER]);
        assert!(diff_frame(&shifted, &shifted_by_a_cycle).is_empty());
        assert_eq!(private_dash_phase(&shifted).value(), 3.0);
        assert_eq!(private_dash_phase(&shifted_by_a_cycle).value(), 3.0);

        let context = PaintCtx::new(None);
        let raster = |product: &FrameProduct| {
            product
                .raster_to_bytes(&AffineTransform::identity(), 64, 48, &context)
                .expect("resource-free dash raster")
        };
        assert_ne!(raster(&zero), raster(&shifted));
        assert_eq!(raster(&shifted), raster(&shifted_by_a_cycle));
    }

    /// The widest carried stroke remains an exact painter fact. Only its
    /// derived damage envelope widens, and every cap and box route projects
    /// that envelope back into the finite frame clip.
    #[test]
    fn maximum_finite_width_projects_exactly_across_caps_and_box_routes() {
        let frame_bounds = Rectangle::from_xywh(0.0, 0.0, 64.0, 48.0);
        for geometry in [
            Geometry::Rect(Rectangle::from_xywh(8.0, 6.0, 20.0, 16.0)),
            Geometry::Ellipse(Rectangle::from_xywh(8.0, 6.0, 20.0, 16.0)),
        ] {
            let is_rect = matches!(&geometry, Geometry::Rect(_));
            for (contract_cap, private_cap) in [
                (rframe::StrokeCap::Butt, StrokeCap::Butt),
                (rframe::StrokeCap::Round, StrokeCap::Round),
                (rframe::StrokeCap::Square, StrokeCap::Square),
            ] {
                let stroke =
                    checked_stroke(f32::MAX, contract_cap, rframe::StrokeJoin::Miter, 4.0, None);
                let node = stroked_node(
                    RECT_OWNER,
                    geometry.clone(),
                    AffineTransform::identity(),
                    stroke,
                );
                let product = compile(frame_of(FrameItems::from_nodes(vec![node])))
                    .expect("maximum finite stroke is admitted");
                let projected = private_stroke(&product);

                assert_eq!(projected.width, StrokeWidth::Uniform(f32::MAX));
                assert_eq!(projected.cap, private_cap);
                assert_eq!(projected.join, StrokeJoin::Miter);
                assert_eq!(projected.miter_limit, 4.0);
                assert_eq!(projected.dash_array, None);
                assert!(if is_rect {
                    matches!(product.drawlist.items[1].kind, ItemKind::RectStroke { .. })
                } else {
                    matches!(product.drawlist.items[1].kind, ItemKind::OvalStroke { .. })
                });

                let coverage = coverage_for(&product, RECT_OWNER)
                    .expect("the extreme stroke crosses the frame clip");
                assert_eq!(coverage, to_rectf(frame_bounds));
                assert_finite_bounded_coverage(coverage, frame_bounds);
            }
        }
    }

    /// A multi-contour path exercises the path painter route, a checked dash
    /// cycle, a non-axis-aligned transform, and the largest miter reach at the
    /// same seam. Every carried field remains bit-exact.
    #[test]
    fn maximum_miter_dashed_multi_contour_path_stays_exact_and_bounded() {
        let path = rframe::PathData::new(
            vec![
                rframe::PathCommand::MoveTo { x: 4.0, y: 4.0 },
                rframe::PathCommand::LineTo { x: 24.0, y: 6.0 },
                rframe::PathCommand::MoveTo { x: 10.0, y: 12.0 },
                rframe::PathCommand::LineTo { x: 30.0, y: 16.0 },
                rframe::PathCommand::LineTo { x: 18.0, y: 30.0 },
                rframe::PathCommand::Close,
            ],
            rframe::FillRule::NonZero,
        )
        .expect("test multi-contour path is valid");
        let transform = AffineTransform::from_acebdf(0.75, -0.25, 12.0, 0.5, 1.25, 3.0);
        let node = stroked_node(
            RECT_OWNER,
            Geometry::Path(Arc::new(path)),
            transform,
            checked_stroke(
                f32::MAX,
                rframe::StrokeCap::Square,
                rframe::StrokeJoin::Miter,
                f32::MAX,
                Some(vec![3.0, 5.0, 0.0, 2.0]),
            ),
        );
        let product = compile(frame_of(FrameItems::from_nodes(vec![node])))
            .expect("wide transformed path stroke is admitted");
        let projected = private_stroke(&product);

        assert!(matches!(
            product.drawlist.items[1].kind,
            ItemKind::PathStroke { .. }
        ));
        assert_eq!(projected.width, StrokeWidth::Uniform(f32::MAX));
        assert_eq!(projected.cap, StrokeCap::Square);
        assert_eq!(projected.join, StrokeJoin::Miter);
        assert_eq!(projected.miter_limit, f32::MAX);
        assert_eq!(
            projected.dash_array.as_deref(),
            Some(&[3.0, 5.0, 0.0, 2.0][..])
        );
        let coverage = coverage_for(&product, RECT_OWNER).expect("stroke crosses the frame");
        assert_eq!(coverage, to_rectf(product.resolved.bounds));
        assert_finite_bounded_coverage(coverage, product.resolved.bounds);
    }

    /// This affine has both cross terms, so using an already-transformed x to
    /// compute y produces a different rectangle. Pin the independently worked
    /// f64 projection, including the stroke's local-space reach.
    #[test]
    fn stroke_coverage_uses_each_original_coordinate_for_affine_projection() {
        let transform = AffineTransform::from_acebdf(2.0, 0.5, 10.0, -0.25, 1.5, 20.0);
        let node = stroked_node(
            RECT_OWNER,
            Geometry::Rect(Rectangle::from_xywh(2.0, 3.0, 4.0, 5.0)),
            transform,
            checked_stroke(
                2.0,
                rframe::StrokeCap::Butt,
                rframe::StrokeJoin::Bevel,
                1.0,
                None,
            ),
        );
        let product = compile(frame_of(FrameItems::from_nodes(vec![node])))
            .expect("finite transformed stroke is admitted");

        let coverage = coverage_for(&product, RECT_OWNER).expect("stroke has coverage");
        let coverage = WideRect::from_rectf(coverage);
        assert!(coverage.left <= 13.0 && coverage.left > 12.99);
        assert!(coverage.top <= 21.25 && coverage.top > 21.0);
        assert!(coverage.right >= 28.5 && coverage.right < 28.51);
        assert!(coverage.bottom >= 33.25 && coverage.bottom < 33.5);
    }

    /// The wide stroke envelope is real-arithmetic, but both carried geometry
    /// and SkScalar stroke points traverse the matrix through sequential f32
    /// operations. With cancellation and a cross term, that result can land
    /// beyond the f64 interval. This pins an actual nonzero-width outer stroke
    /// point from the reviewer-found bit pattern, not merely the base geometry.
    #[test]
    fn inflated_stroke_projection_contains_sequential_f32_cancellation_cross_term() {
        let p = f32::from_bits(0xb40d_dda9);
        let local_x = f32::from_bits(0x4aad_0dd4);
        let q = f32::from_bits(0x14d6_9197);
        let local_y = f32::from_bits(0x6b1d_9171);
        let translation = f32::from_bits(0xbdae_da06);
        let outset = 0.5;
        let local_width = next_up_f32(local_x) - local_x;
        let local_height = next_up_f32(local_y) - local_y;
        let local_bounds = Rectangle::from_xywh(local_x, local_y, local_width, local_height);
        let inflated = WideRect::from_rectangle(local_bounds).inflated(outset);

        // `p` is negative and `q` positive, so this is the outer corner that
        // maximizes x after a nondegenerate rect's one-unit stroke is inflated
        // in local f32 space.
        let stroke_x = local_x - outset as f32;
        let stroke_y = (local_y + local_height) + outset as f32;
        assert_ne!(stroke_x, local_x, "the test must exercise stroke reach");
        assert_ne!(stroke_y, local_y, "the rect must have a finite height");
        let px = p * stroke_x;
        let qy = q * stroke_y;
        let sum = px + qy;
        let sequential_f32 = sum + translation;
        assert_eq!(sequential_f32.to_bits(), 0x4052_b85a);

        // Prove the regression discriminates: even an operation-by-operation
        // outward f64 interval around the wide corners stops below the actual
        // sequential-f32 stroke endpoint.
        let mut wide_only_right = f64::NEG_INFINITY;
        for (x, y) in [
            (inflated.left, inflated.top),
            (inflated.right, inflated.top),
            (inflated.right, inflated.bottom),
            (inflated.left, inflated.bottom),
        ] {
            let (_, high) =
                affine_component_bounds(f64::from(p), x, f64::from(q), y, f64::from(translation))
                    .expect("the wide cancellation case stays finite");
            wide_only_right = wide_only_right.max(high);
        }
        assert!(wide_only_right < f64::from(sequential_f32));

        let transform = AffineTransform::from_acebdf(p, q, translation, 0.0, 0.0, 1.0);
        let projected = inflated
            .transformed(&transform)
            .expect("both affine semantics stay finite");
        assert!(projected.left <= f64::from(sequential_f32));
        assert!(projected.right >= f64::from(sequential_f32));

        let frame_bounds = Rectangle::from_xywh(0.0, 0.0, 8.0, 4.0);
        let encoded = bounded_stroke_coverage(local_bounds, &transform, None, outset, frame_bounds)
            .expect("the cancellation case intersects the frame");
        let encoded = WideRect::from_rectf(encoded);
        assert!(encoded.left <= f64::from(sequential_f32));
        assert!(encoded.right >= f64::from(sequential_f32));
    }

    /// Rects and ellipses fold their local-box origin into the private world
    /// matrix before the painter maps 0..w / 0..h. Under cancellation, that
    /// f32 composition can put an ordinary stroke endpoint outside both the
    /// wide real-arithmetic and direct-coordinate f32 envelopes. Pin the exact
    /// reviewer witness through the compiled box world and final bounded RectF.
    #[test]
    fn box_stroke_coverage_contains_composed_f32_painter_endpoint() {
        let a = f32::from_bits(0x9479_82c2);
        let rect_x = f32::from_bits(0xee47_e90e);
        let rect_width = f32::from_bits(0x6e12_45fa);
        let translation = f32::from_bits(0x214a_fd2a);
        let rect = Rectangle::from_xywh(rect_x, 0.0, rect_width, 1.0);
        let transform = AffineTransform::from_acebdf(a, 0.0, translation, 0.0, 1.0, 0.0);
        let frame_bounds = Rectangle::from_xywh(0.0, 0.0, 64.0, 64.0);

        // Prove the case discriminates the old direct-coordinate projection
        // after its final RectF encoding, not just an internal f64 interval.
        let old_projected = WideRect::from_rectangle(rect)
            .inflated(0.5)
            .transformed(&transform)
            .expect("direct projection stays finite")
            .intersection(WideRect::from_rectangle(frame_bounds))
            .expect("direct projection intersects the frame");
        let old_encoded = rectf_covering_bounded(old_projected, frame_bounds);
        assert_eq!(old_encoded.x.to_bits(), 0x4251_1c0f);

        let node = stroked_node(
            RECT_OWNER,
            Geometry::Rect(rect),
            transform,
            checked_stroke(
                1.0,
                rframe::StrokeCap::Butt,
                rframe::StrokeJoin::Bevel,
                1.0,
                None,
            ),
        );
        let frame = Frame {
            owner: FRAME_OWNER,
            bounds: frame_bounds,
            items: FrameItems::from_nodes(vec![node]),
        };
        let product = compile(frame).expect("finite transformed box stroke is admitted");
        let (box_world, width) = product
            .drawlist
            .items
            .iter()
            .find_map(|item| match &item.kind {
                ItemKind::RectStroke { w, .. } => Some((item.world, *w)),
                _ => None,
            })
            .expect("compiled product has its private rect stroke");
        let painter_end = box_world.apply((width, 0.0)).0;
        assert_eq!(painter_end.to_bits(), 0x4251_1c0c);
        assert!(painter_end < old_encoded.x);

        let coverage = coverage_for(&product, RECT_OWNER).expect("box stroke crosses the frame");
        assert!(coverage.x <= painter_end);
        assert!(coverage.x + coverage.w >= painter_end);
        assert_finite_bounded_coverage(coverage, frame_bounds);
    }

    /// A nonzero tiny scale makes the widest stroke only locally visible. The
    /// wide projection must retain that finite reach instead of saturating the
    /// intermediate rectangle or broadening every extreme stroke to the clip.
    #[test]
    fn tiny_nonzero_scale_keeps_maximum_width_coverage_tighter_than_the_frame() {
        let scale = 1.0e-38;
        let transform = AffineTransform::from_acebdf(scale, 0.0, 32.0, 0.0, scale, 24.0);
        let node = stroked_node(
            RECT_OWNER,
            Geometry::Rect(Rectangle::from_xywh(8.0, 6.0, 20.0, 16.0)),
            transform,
            checked_stroke(
                f32::MAX,
                rframe::StrokeCap::Butt,
                rframe::StrokeJoin::Miter,
                4.0,
                None,
            ),
        );
        let product = compile(frame_of(FrameItems::from_nodes(vec![node])))
            .expect("compressed maximum stroke is admitted");
        let coverage = coverage_for(&product, RECT_OWNER).expect("compressed stroke remains ink");

        assert_finite_bounded_coverage(coverage, product.resolved.bounds);
        assert!(coverage.x > 0.0 && coverage.y > 0.0);
        assert!(coverage.x + coverage.w < product.resolved.bounds.width);
        assert!(coverage.y + coverage.h < product.resolved.bounds.height);
        assert!(coverage.w > 13.0 && coverage.w < 14.0);
        assert!(coverage.h > 13.0 && coverage.h < 14.0);
    }

    /// A contract scope is structurally non-empty even when its transformed
    /// child is fully clipped. Such a material edit remains attributable, but
    /// it has no pixel envelope and must never fabricate an invalid rectangle.
    #[test]
    fn all_clipped_extreme_stroke_and_scope_have_no_damage_envelope() {
        let scene = |opacity| {
            let collapsed_offscreen = AffineTransform::from_acebdf(
                0.0, 0.0, 100.0, // x = 100
                0.0, 0.0, 100.0, // y = 100
            );
            let node = stroked_node(
                RECT_OWNER,
                Geometry::Rect(Rectangle::from_xywh(8.0, 6.0, 20.0, 16.0)),
                collapsed_offscreen,
                checked_stroke(
                    f32::MAX,
                    rframe::StrokeCap::Square,
                    rframe::StrokeJoin::Miter,
                    f32::MAX,
                    None,
                ),
            );
            let items = FrameItems::try_new(vec![
                scope_begin(SCOPE_OWNER, opacity),
                FrameItem::Node(node),
                FrameItem::ScopeEnd,
            ])
            .expect("scope stream is structurally non-empty");
            compile(frame_of(items)).expect("fully clipped scope still compiles")
        };
        let before = scene(0.5);
        let after = scene(0.25);

        assert_eq!(coverage_for(&before, RECT_OWNER), None);
        assert_eq!(coverage_for(&before, SCOPE_OWNER), None);
        let damage = diff_frame(&before, &after);
        assert_eq!(damage.changed, vec![SCOPE_OWNER]);
        assert_eq!(damage.union_frame, None);
        assert!(
            !damage.is_empty(),
            "changed reports exact material attribution even without covered pixels"
        );
    }

    /// A nested all-clipped scope contributes nothing to its parent; the
    /// parent's visible sibling alone determines the bounded opacity envelope.
    #[test]
    fn scope_union_ignores_clipped_children_and_stays_frame_bounded() {
        let scene = |opacity| {
            let collapsed_offscreen = AffineTransform::from_acebdf(
                0.0, 0.0, 100.0, // x = 100
                0.0, 0.0, 100.0, // y = 100
            );
            let clipped = stroked_node(
                CLIPPED_OWNER,
                Geometry::Ellipse(Rectangle::from_xywh(8.0, 6.0, 20.0, 16.0)),
                collapsed_offscreen,
                checked_stroke(
                    f32::MAX,
                    rframe::StrokeCap::Round,
                    rframe::StrokeJoin::Round,
                    4.0,
                    Some(vec![2.0, 2.0]),
                ),
            );
            let visible = rect_node(
                RECT_OWNER,
                Rectangle::from_xywh(8.0, 6.0, 20.0, 16.0),
                0xFF16_A34A,
            );
            let items = FrameItems::try_new(vec![
                scope_begin(SCOPE_OWNER, opacity),
                FrameItem::Node(visible),
                scope_begin(INNER_SCOPE_OWNER, 0.5),
                FrameItem::Node(clipped),
                FrameItem::ScopeEnd,
                FrameItem::ScopeEnd,
            ])
            .expect("nested scope stream is valid");
            compile(frame_of(items)).expect("mixed clipped scope compiles")
        };
        let before = scene(0.5);
        let after = scene(0.25);
        let visible = n0_model::math::RectF {
            x: 8.0,
            y: 6.0,
            w: 20.0,
            h: 16.0,
        };

        assert_eq!(coverage_for(&before, CLIPPED_OWNER), None);
        assert_eq!(coverage_for(&before, INNER_SCOPE_OWNER), None);
        assert_eq!(coverage_for(&before, SCOPE_OWNER), Some(visible));
        let damage = diff_frame(&before, &after);
        assert_eq!(damage.changed, vec![SCOPE_OWNER]);
        assert_eq!(damage.union_frame, Some(from_rectf(visible)));
        assert_finite_bounded_coverage(visible, before.resolved.bounds);
    }

    #[test]
    fn extreme_stroke_diff_union_is_finite_and_frame_bounded() {
        let scene = |cap| {
            let node = stroked_node(
                RECT_OWNER,
                Geometry::Rect(Rectangle::from_xywh(8.0, 6.0, 20.0, 16.0)),
                AffineTransform::identity(),
                checked_stroke(f32::MAX, cap, rframe::StrokeJoin::Miter, 4.0, None),
            );
            compile(frame_of(FrameItems::from_nodes(vec![node])))
                .expect("maximum finite stroke compiles")
        };
        let before = scene(rframe::StrokeCap::Butt);
        let after = scene(rframe::StrokeCap::Square);
        let damage = diff_frame(&before, &after);

        assert_eq!(damage.changed, vec![RECT_OWNER]);
        let coverage = damage.union_frame.expect("changed stroke has damage");
        assert_eq!(coverage, before.resolved.bounds);
        assert_finite_bounded_coverage(to_rectf(coverage), before.resolved.bounds);
    }

    #[test]
    fn frame_clip_with_unrepresentable_far_edge_refuses_before_damage_projection() {
        let frame = Frame {
            owner: FRAME_OWNER,
            bounds: Rectangle::from_xywh(f32::MAX, 0.0, f32::MAX, 48.0),
            items: FrameItems::default(),
        };
        assert!(matches!(
            compile(frame),
            Err(BuildError::InvalidFrameBounds)
        ));
    }

    #[test]
    fn unencodable_wide_projection_falls_back_to_finite_frame_coverage() {
        let frame = Rectangle::from_xywh(0.0, 0.0, 64.0, 48.0);
        let transform = AffineTransform::from_acebdf(f32::MAX, 0.0, 0.0, 0.0, f32::MAX, 0.0);
        let coverage = bounded_stroke_coverage(
            Rectangle::from_xywh(8.0, 6.0, 20.0, 16.0),
            &transform,
            None,
            f64::MAX,
            frame,
        )
        .expect("non-empty frame is the conservative fallback");

        assert_eq!(coverage, to_rectf(frame));
        assert_finite_bounded_coverage(coverage, frame);
    }

    #[test]
    fn outward_rounding_never_crosses_the_exact_frame_endpoint() {
        let frame = Rectangle::from_xywh(
            f32::from_bits(0xfcbc_6019),
            0.0,
            f32::from_bits(0x7d73_111a),
            1.0,
        );
        let coverage = rectf_covering_bounded(WideRect::from_rectangle(frame), frame);

        assert_eq!(coverage, to_rectf(frame));
        assert_finite_bounded_coverage(coverage, frame);
    }

    #[test]
    fn zero_area_frame_has_no_coverage_envelope() {
        let frame = Frame {
            owner: FRAME_OWNER,
            bounds: Rectangle::from_xywh(0.0, 0.0, 0.0, 48.0),
            items: FrameItems::default(),
        };
        let product = compile(frame).expect("finite zero-area clip is admitted");

        assert_eq!(coverage_for(&product, FRAME_OWNER), None);
        assert!(diff_frame(&product, &product).is_empty());
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

    /// A live dashed ellipse is the one box route whose local origin must not
    /// be folded into the private world: Skia measures and slices conics in
    /// f32, so translating first changes dash endpoints. The private item
    /// retains the absolute local oval while a solid ellipse remains the
    /// ordinary origin-relative primitive.
    #[test]
    fn dashed_ellipse_preserves_absolute_local_coordinates_until_paint() {
        let rect = Rectangle::from_xywh(16.0, 8.0, 32.0, 24.0);
        let intervals = rframe::StrokeDashIntervals::new(vec![6.0, 3.0])
            .expect("test dash intervals are valid")
            .expect("test dash cycle is present");
        let dash = rframe::StrokeDash::new(intervals, 0.0).expect("test dash phase is finite");
        let gradient = CgPaint::LinearGradient(cg::LinearGradientPaint::from_colors(vec![
            CGColor::BLACK,
            CGColor::WHITE,
        ]));
        let paints = PaintStack::try_from_paints(CgPaints::new([gradient]))
            .expect("test gradient is admitted");
        let stroke = rframe::Stroke::new_with_dash(
            paints,
            8.0,
            rframe::StrokeCap::Round,
            rframe::StrokeJoin::Miter,
            4.0,
            Some(dash),
        )
        .expect("test stroke is valid")
        .expect("test stroke paints");
        let dashed = compile(frame_of(FrameItems::from_nodes(vec![stroked_node(
            RECT_OWNER,
            Geometry::Ellipse(rect),
            AffineTransform::identity(),
            stroke,
        )])))
        .expect("admitted dashed ellipse frame");
        let item = &dashed.drawlist.items[1];
        assert_eq!(item.world, to_affine(AffineTransform::identity()));
        match &item.kind {
            ItemKind::AbsoluteDashedOvalStroke {
                x,
                y,
                w,
                h,
                stroke,
                dash_phase,
                ..
            } => {
                assert_eq!((*x, *y, *w, *h), (16.0, 8.0, 32.0, 24.0));
                assert_eq!(stroke.dash_array.as_deref(), Some(&[6.0, 3.0][..]));
                assert_eq!(*dash_phase, StrokeDashPhase::ZERO);
                match stroke.paints.as_slice() {
                    [Paint::LinearGradient(gradient)] => {
                        assert_eq!(gradient.transform, Affine::IDENTITY)
                    }
                    other => panic!("dashed ellipse lost its gradient material: {other:?}"),
                }
            }
            other => panic!("dashed ellipse lost its absolute oval route: {other:?}"),
        }

        let solid = checked_stroke(
            8.0,
            rframe::StrokeCap::Round,
            rframe::StrokeJoin::Miter,
            4.0,
            None,
        );
        let solid = compile(frame_of(FrameItems::from_nodes(vec![stroked_node(
            RECT_OWNER,
            Geometry::Ellipse(rect),
            AffineTransform::identity(),
            solid,
        )])))
        .expect("admitted solid ellipse frame");
        assert!(matches!(
            solid.drawlist.items[1].kind,
            ItemKind::OvalStroke { .. }
        ));
    }

    #[test]
    fn absolute_dashed_ellipse_coordinates_participate_in_damage_and_raster_identity() {
        let scene = |x| {
            let node = dashed_node(
                RECT_OWNER,
                Geometry::Ellipse(Rectangle::from_xywh(x, 8.0, 32.0, 24.0)),
                vec![6.0, 3.0],
            );
            compile(frame_of(FrameItems::from_nodes(vec![node])))
                .expect("admitted dashed ellipse frame")
        };
        let before = scene(16.0);
        let after = scene(17.0);

        assert_ne!(before.drawlist, after.drawlist);
        assert!(!before.drawlist.raster_eq(&after.drawlist));
        assert_eq!(diff_frame(&before, &after).changed, vec![RECT_OWNER]);
        assert!(diff_frame(&before, &before).is_empty());
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
                    post_paint_opacity: PostPaintOpacity::IDENTITY,
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

    /// A checked filter scope lowers to one private graph layer. The painter
    /// evaluates Source and SourceAlpha distinctly and never treats a valid
    /// graph as an unfiltered fallback.
    #[test]
    fn gaussian_filter_scope_lowers_and_rasters_both_source_inputs() {
        let make = |input| {
            let items = FrameItems::try_new(vec![
                blur_scope_begin(SCOPE_OWNER, input),
                FrameItem::Node(base_node(PaintStack::solid(CGColor::RED))),
                FrameItem::ScopeEnd,
            ])
            .expect("balanced filter scope");
            compile(frame_of(items)).expect("admitted filtered frame")
        };
        let source = make(FilterInput::Source);
        assert!(matches!(
            source.drawlist.items[1].kind,
            ItemKind::BeginFilter { .. }
        ));
        assert!(matches!(source.drawlist.items[3].kind, ItemKind::EndFilter));
        assert_eq!(
            source.drawlist.items[1].node, source.drawlist.items[3].node,
            "begin and end retain the filter scope owner"
        );

        let context = PaintCtx::new(None);
        let neutral_view = AffineTransform::identity();
        let source_pixels = source
            .raster_to_bytes(&neutral_view, 64, 48, &context)
            .expect("source blur raster");
        assert_ne!(
            rgba_at(&source_pixels, 64, 6, 14),
            [0xff, 0xff, 0xff, 0xff],
            "blur spreads source pixels beyond the unfiltered rectangle"
        );

        let alpha_pixels = make(FilterInput::SourceAlpha)
            .raster_to_bytes(&neutral_view, 64, 48, &context)
            .expect("source-alpha blur raster");
        assert_ne!(
            rgba_at(&source_pixels, 64, 18, 14),
            rgba_at(&alpha_pixels, 64, 18, 14),
            "SourceAlpha clears RGB before the same blur"
        );
    }

    #[test]
    fn an_empty_generated_filter_edit_damages_its_transformed_region() {
        let scene = |seed| {
            let items = FrameItems::try_new(vec![
                empty_turbulence_scope_begin(SCOPE_OWNER, seed),
                FrameItem::ScopeEnd,
            ])
            .expect("a declared generated filter is meaningful without source draws");
            compile(frame_of(items)).expect("admitted generated-filter frame")
        };
        let before = scene(2.0);
        let after = scene(3.0);
        let coverage = Rectangle::from_xywh(11.0, 13.0, 10.0, 12.0);

        assert_eq!(coverage_for(&before, SCOPE_OWNER), Some(to_rectf(coverage)));
        assert_eq!(
            diff_frame(&before, &after),
            Damage {
                changed: vec![SCOPE_OWNER],
                union_frame: Some(coverage),
            }
        );
        assert!(diff_frame(&before, &before).is_empty());
    }

    /// The contract's union/intersection normal form lowers to one private
    /// clip item and clips the enclosed paint without inventing a resource or
    /// an isolated layer.
    #[test]
    fn geometric_clip_lowers_and_rasters_its_union_intersection() {
        let items = FrameItems::try_new(vec![
            clip_begin(
                SCOPE_OWNER,
                vec![
                    vec![
                        (
                            Rectangle::from_xywh(8.0, 6.0, 16.0, 20.0),
                            AffineTransform::identity(),
                        ),
                        (
                            Rectangle::from_xywh(36.0, 6.0, 16.0, 20.0),
                            AffineTransform::identity(),
                        ),
                    ],
                    vec![(
                        Rectangle::from_xywh(0.0, 10.0, 64.0, 10.0),
                        AffineTransform::identity(),
                    )],
                ],
            ),
            FrameItem::Node(rect_node(
                RECT_OWNER,
                Rectangle::from_xywh(0.0, 0.0, 64.0, 48.0),
                0xFF16_A34A,
            )),
            FrameItem::ScopeEnd,
        ])
        .expect("balanced clip scope");
        let product = compile(frame_of(items)).expect("admitted geometric clip");
        assert!(matches!(
            product.drawlist.items[1].kind,
            ItemKind::BeginClipPath { .. }
        ));
        assert!(matches!(product.drawlist.items[3].kind, ItemKind::EndClip));

        let pixels = product
            .raster_to_bytes(&AffineTransform::identity(), 64, 48, &PaintCtx::new(None))
            .expect("resource-free clip raster");
        assert_eq!(rgba_at(&pixels, 64, 12, 14), [0x16, 0xa3, 0x4a, 0xff]);
        assert_eq!(rgba_at(&pixels, 64, 40, 14), [0x16, 0xa3, 0x4a, 0xff]);
        assert_eq!(rgba_at(&pixels, 64, 30, 14), [0xff, 0xff, 0xff, 0xff]);
        assert_eq!(rgba_at(&pixels, 64, 12, 24), [0xff, 0xff, 0xff, 0xff]);
    }

    #[test]
    fn hard_clip_edge_policy_reaches_the_painter() {
        let scene = |edge_mode| {
            let items = FrameItems::try_new(vec![
                clip_begin_with_edge(
                    SCOPE_OWNER,
                    vec![vec![(
                        Rectangle::from_xywh(8.25, 6.25, 20.0, 16.0),
                        AffineTransform::identity(),
                    )]],
                    edge_mode,
                ),
                FrameItem::Node(rect_node(
                    RECT_OWNER,
                    Rectangle::from_xywh(0.0, 0.0, 64.0, 48.0),
                    0xFF16_A34A,
                )),
                FrameItem::ScopeEnd,
            ])
            .expect("balanced clip scope");
            compile(frame_of(items)).expect("admitted clip scene")
        };

        let ordinary = scene(rframe::ClipEdgeMode::AntiAliased);
        let hard = scene(rframe::ClipEdgeMode::Hard);
        let ItemKind::BeginClipPath { clip } = &hard.drawlist.items[1].kind else {
            panic!("resolved clip item")
        };
        assert!(!clip.anti_alias, "hard policy survives contract lowering");

        let raster = |product: &FrameProduct| {
            product
                .raster_to_bytes(&AffineTransform::identity(), 64, 48, &PaintCtx::new(None))
                .expect("resource-free clip raster")
        };
        assert_ne!(
            raster(&ordinary),
            raster(&hard),
            "the backend edge flag changes fractional clip coverage"
        );
    }

    #[test]
    fn an_empty_clip_layer_is_the_checked_clip_all_fact() {
        let items = FrameItems::try_new(vec![
            clip_begin(SCOPE_OWNER, vec![Vec::new()]),
            FrameItem::Node(rect_node(
                RECT_OWNER,
                Rectangle::from_xywh(0.0, 0.0, 64.0, 48.0),
                0xFF16_A34A,
            )),
            FrameItem::ScopeEnd,
        ])
        .expect("balanced empty clip scope");
        let product = compile(frame_of(items)).expect("empty clip is admitted");
        assert_eq!(coverage_for(&product, SCOPE_OWNER), None);
        let pixels = product
            .raster_to_bytes(&AffineTransform::identity(), 64, 48, &PaintCtx::new(None))
            .expect("empty clip raster");
        assert!(
            pixels
                .chunks_exact(4)
                .all(|pixel| pixel == [255, 255, 255, 255]),
            "a valid empty clip admits no target pixel"
        );
    }

    #[test]
    fn a_clip_edit_damages_the_old_and_new_conservative_coverage() {
        let scene = |x| {
            let items = FrameItems::try_new(vec![
                clip_begin(
                    SCOPE_OWNER,
                    vec![vec![(
                        Rectangle::from_xywh(x, 6.0, 20.0, 16.0),
                        AffineTransform::identity(),
                    )]],
                ),
                FrameItem::Node(rect_node(
                    RECT_OWNER,
                    Rectangle::from_xywh(0.0, 0.0, 64.0, 48.0),
                    0xFF16_A34A,
                )),
                FrameItem::ScopeEnd,
            ])
            .expect("balanced clip scope");
            compile(frame_of(items)).expect("admitted clip scene")
        };
        let damage = diff_frame(&scene(8.0), &scene(12.0));
        assert_eq!(damage.changed, vec![SCOPE_OWNER]);
        assert_eq!(
            damage.union_frame,
            Some(Rectangle::from_xywh(8.0, 6.0, 24.0, 16.0))
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
