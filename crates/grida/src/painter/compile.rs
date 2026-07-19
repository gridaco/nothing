//! The model→display-list compiler — the painter's single model seam.
//!
//! This module is the **only** place in `painter/` that reads the v1 node
//! model (`node::schema`, `SceneGraph`). It compiles the scene graph into
//! the painter's display list — [`LayerList`] holding
//! [`PainterPictureLayer`]s and [`PainterRenderCommand`]s whose payload is
//! `cg`-typed values, Skia geometry, and `NodeId` (the sole model type).
//! The draw loop (`painter.rs`) consumes that display list and never reads
//! the model. `tests/painter_architecture.rs` locks the boundary; the
//! `painter::debug` overlay is the one other permitted model consumer.
//!
//! Seam: gridaco/nothing#31 (painter narrowing), part of the legacy seam
//! program gridaco/nothing#27.
//!
//! ## Pipeline Guarantees
//!
//! This module guarantees:
//! - build_shape() requires resolved bounds from GeometryCache for all nodes
//! - V2 nodes (with auto-sizing) ALWAYS use provided bounds
//! - V1 nodes (fixed schema) use schema values (bounds parameter for future migration)
//! - Missing bounds when accessed is a PANIC (pipeline bug)

use super::geometry::{merge_shapes, PainterShape};
use super::layer::{
    LayerEntry, LayerList, PainterMaskGroup, PainterPictureHtmlEmbedLayer, PainterPictureLayer,
    PainterPictureLayerBase, PainterPictureMarkdownEmbedLayer, PainterPictureShapeLayer,
    PainterPictureTextLayer, PainterPictureVectorLayer, PainterRenderCommand, PainterRenderSurface,
};
use crate::backends::skia as sk;
use crate::cache::geometry::GeometryCache;
use crate::cache::scene::SceneCache;
use crate::cg::prelude::*;
use crate::node::scene_graph::SceneGraph;
use crate::node::schema::*;
use crate::shape::*;
use math2::{rect::Rectangle, transform::AffineTransform};
use skia_safe::{Path, Rect};

/// Build shape from node + resolved geometry
///
/// All dimensions come from bounds (resolved by GeometryCache).
/// This ensures V2 auto-sized nodes and future migrations render correctly.
pub fn build_shape(node: &Node, bounds: &Rectangle) -> PainterShape {
    match node {
        Node::Polygon(n) => {
            let shape = n.to_shape();
            PainterShape::from_shape(&shape)
        }
        Node::RegularPolygon(n) => {
            let shape = n.to_shape();
            PainterShape::from_shape(&shape)
        }
        Node::RegularStarPolygon(n) => {
            let shape = n.to_shape();
            PainterShape::from_shape(&shape)
        }
        Node::Line(n) => PainterShape::from_path(Path::line((0.0, 0.0), (n.size.width, 0.0))),
        Node::Path(n) => {
            if let Some(path) = Path::from_svg(&n.data) {
                PainterShape::from_path(path)
            } else {
                // Fallback to empty rect if path is invalid
                PainterShape::from_rect(Rect::new(0.0, 0.0, 0.0, 0.0))
            }
        }
        Node::Vector(n) => {
            let path = n.to_path();
            PainterShape::from_path(path)
        }
        Node::Ellipse(n) => {
            let shape = n.to_shape();
            PainterShape::from_shape(&shape)
        }
        Node::Rectangle(n) => {
            let shape = n.to_shape();
            PainterShape::from_shape(&shape)
        }
        Node::Container(n) => {
            // ALWAYS use resolved bounds from GeometryCache
            let width = bounds.width;
            let height = bounds.height;

            let r = n.corner_radius;
            if !r.is_zero() {
                // Check if corner smoothing is enabled
                if n.corner_smoothing.value() > 0.0 {
                    let smooth = OrthogonalSmoothRRectShape {
                        width,
                        height,
                        corner_radius: n.corner_radius,
                        corner_smoothing: n.corner_smoothing,
                    };
                    PainterShape::from_path(build_orthogonal_smooth_rrect_path(&smooth))
                } else {
                    let rrect = build_rrect(&RRectShape {
                        width,
                        height,
                        corner_radius: n.corner_radius,
                    });
                    PainterShape::from_rrect(rrect)
                }
            } else {
                let rect = Rect::from_xywh(0.0, 0.0, width, height);
                PainterShape::from_rect(rect)
            }
        }
        Node::Image(n) => {
            let r = n.corner_radius;
            if !r.is_zero() {
                // Check if corner smoothing is enabled
                if n.corner_smoothing.value() > 0.0 {
                    let smooth = OrthogonalSmoothRRectShape {
                        width: n.size.width,
                        height: n.size.height,
                        corner_radius: r,
                        corner_smoothing: n.corner_smoothing,
                    };
                    PainterShape::from_path(build_orthogonal_smooth_rrect_path(&smooth))
                } else {
                    let rrect = build_rrect(&n.to_own_shape());
                    PainterShape::from_rrect(rrect)
                }
            } else {
                let rect = Rect::from_xywh(0.0, 0.0, n.size.width, n.size.height);
                PainterShape::from_rect(rect)
            }
        }
        Node::Tray(n) => {
            // Tray uses resolved bounds (like Container) with optional corner radius
            let width = bounds.width;
            let height = bounds.height;

            let r = n.corner_radius;
            if !r.is_zero() {
                if n.corner_smoothing.value() > 0.0 {
                    let smooth = OrthogonalSmoothRRectShape {
                        width,
                        height,
                        corner_radius: n.corner_radius,
                        corner_smoothing: n.corner_smoothing,
                    };
                    PainterShape::from_path(build_orthogonal_smooth_rrect_path(&smooth))
                } else {
                    let rrect = build_rrect(&RRectShape {
                        width,
                        height,
                        corner_radius: n.corner_radius,
                    });
                    PainterShape::from_rrect(rrect)
                }
            } else {
                let rect = Rect::from_xywh(0.0, 0.0, width, height);
                PainterShape::from_rect(rect)
            }
        }
        Node::Error(n) => {
            let rect = Rect::from_xywh(0.0, 0.0, n.size.width, n.size.height);
            PainterShape::from_rect(rect)
        }
        Node::MarkdownEmbed(n) => {
            // Use resolved bounds for auto-sized markdown embeds
            let width = bounds.width;
            let height = bounds.height;
            let r = n.corner_radius;
            if !r.is_zero() {
                let rrect = build_rrect(&RRectShape {
                    width,
                    height,
                    corner_radius: n.corner_radius,
                });
                PainterShape::from_rrect(rrect)
            } else {
                let rect = Rect::from_xywh(0.0, 0.0, width, height);
                PainterShape::from_rect(rect)
            }
        }
        Node::HTMLEmbed(n) => {
            let r = n.corner_radius;
            if !r.is_zero() {
                let rrect = build_rrect(&RRectShape {
                    width: n.size.width,
                    height: n.size.height,
                    corner_radius: n.corner_radius,
                });
                PainterShape::from_rrect(rrect)
            } else {
                let rect = Rect::from_xywh(0.0, 0.0, n.size.width, n.size.height);
                PainterShape::from_rect(rect)
            }
        }
        // Non-shape nodes (Group, BooleanOperation, InitialContainer, TextSpan)
        _ => PainterShape::from_rect(Rect::new(0.0, 0.0, 0.0, 0.0)),
    }
}

/// Compute the resulting path for a [`BooleanPathOperationNode`] in its local coordinate space.
pub fn boolean_operation_path(
    id: &NodeId,
    node: &BooleanPathOperationNodeRec,
    graph: &SceneGraph,
    cache: &GeometryCache,
) -> Option<Path> {
    let world = cache
        .get_world_transform(id)
        .unwrap_or_else(AffineTransform::identity);
    let inv = world.inverse().unwrap_or_else(AffineTransform::identity);

    let mut shapes_with_ops = Vec::new();

    let children = graph.get_children(id)?;
    for (i, child_id) in children.iter().enumerate() {
        if let Ok(child_node) = graph.get_node(child_id) {
            let mut path = match child_node {
                Node::BooleanOperation(child_bool) => {
                    boolean_operation_path(child_id, child_bool, graph, cache)?
                }
                _ => {
                    // Get bounds from geometry cache - guaranteed to exist
                    let bounds = cache
                        .get_world_bounds(child_id)
                        .expect("Geometry must exist for all nodes");
                    let intrinsic = match child_node {
                        Node::Rectangle(n) => Node::Rectangle(n.clone()),
                        Node::Ellipse(n) => Node::Ellipse(n.clone()),
                        Node::Polygon(n) => Node::Polygon(n.clone()),
                        Node::RegularPolygon(n) => Node::RegularPolygon(n.clone()),
                        Node::RegularStarPolygon(n) => Node::RegularStarPolygon(n.clone()),
                        Node::Line(n) => Node::Line(n.clone()),
                        Node::Path(n) => Node::Path(n.clone()),
                        Node::Vector(n) => Node::Vector(n.clone()),
                        Node::Image(n) => Node::Image(n.clone()),
                        Node::Container(n) => Node::Container(n.clone()),
                        Node::Error(n) => Node::Error(n.clone()),
                        // TODO: Support Group nodes as children of boolean operations.
                        // Currently, boolean operations like `boolop(group(a, b), c)` are not supported.
                        // When a Group is encountered, this function returns None, causing the boolean
                        // operation to fail rendering.
                        _ => return None, // Non-shape nodes (including Group)
                    };
                    build_shape(&intrinsic, &bounds).to_path()
                }
            };

            let child_world = cache
                .get_world_transform(child_id)
                .unwrap_or_else(AffineTransform::identity);
            let relative = inv.compose(&child_world);
            path = path.make_transform(&sk::sk_matrix(relative.matrix));

            let op = if i == 0 {
                BooleanPathOperation::Union
            } else {
                node.op
            };
            shapes_with_ops.push((PainterShape::from_path(path), op));
        }
    }

    if shapes_with_ops.is_empty() {
        return None;
    }

    let path = merge_shapes(&shapes_with_ops);
    let path = if let Some(r) = node.corner_radius {
        if r > 0.0 {
            build_corner_radius_path(&path, r)
        } else {
            path
        }
    } else {
        path
    };

    Some(path)
}

/// Convenience wrapper around [`boolean_operation_path`] returning a [`PainterShape`].
pub fn boolean_operation_shape(
    id: &NodeId,
    node: &BooleanPathOperationNodeRec,
    graph: &SceneGraph,
    cache: &GeometryCache,
) -> Option<PainterShape> {
    boolean_operation_path(id, node, graph, cache).map(PainterShape::from_path)
}

#[derive(Debug, Default)]
struct FlattenResult {
    commands: Vec<PainterRenderCommand>,
    mask: Option<LayerMaskType>,
}

impl LayerList {
    /// Filter paints to only include visible ones for performance optimization.
    ///
    /// This removes paints that are inactive or have zero opacity, which have no visual effect
    /// regardless of blend mode and can be safely skipped during rendering.
    fn filter_visible_paints(paints: &Paints) -> Paints {
        Paints::new(
            paints
                .iter()
                .filter(|paint| paint.visible())
                .cloned()
                .collect::<Vec<_>>(),
        )
    }

    fn filter_active_noises(noises: &[FeNoiseEffect]) -> Vec<FeNoiseEffect> {
        noises.iter().filter(|n| n.active).cloned().collect()
    }

    fn filter_active_shadows(shadows: &[FilterShadowEffect]) -> Vec<FilterShadowEffect> {
        shadows.iter().filter(|s| s.active()).cloned().collect()
    }

    fn filter_active_layer_blur(blur: &Option<FeLayerBlur>) -> Option<FeLayerBlur> {
        blur.as_ref()
            .and_then(|b| if b.active { Some(b.clone()) } else { None })
    }

    fn filter_active_backdrop_blur(blur: &Option<FeBackdropBlur>) -> Option<FeBackdropBlur> {
        blur.as_ref()
            .and_then(|b| if b.active { Some(b.clone()) } else { None })
    }

    fn filter_active_glass(glass: &Option<FeLiquidGlass>) -> Option<FeLiquidGlass> {
        glass
            .as_ref()
            .and_then(|g| if g.active { Some(*g) } else { None })
    }

    fn filter_active_effects(effects: &LayerEffects) -> LayerEffects {
        LayerEffects {
            shadows: Self::filter_active_shadows(&effects.shadows),
            blur: Self::filter_active_layer_blur(&effects.blur),
            backdrop_blur: Self::filter_active_backdrop_blur(&effects.backdrop_blur),
            glass: Self::filter_active_glass(&effects.glass),
            noises: Self::filter_active_noises(&effects.noises),
        }
    }

    /// Split effects into surface-level effects and per-node effects.
    ///
    /// Surface-level effects (shadows, layer blur) are applied to the entire
    /// composited subtree via a render surface. Per-node effects (backdrop blur,
    /// glass, noise) remain on the container's own draw layer.
    ///
    /// Returns `(surface_effects, own_effects)`.
    fn split_surface_effects(effects: &LayerEffects) -> (LayerEffects, LayerEffects) {
        let surface = LayerEffects {
            shadows: effects.shadows.clone(),
            blur: effects.blur.clone(),
            // Backdrop blur is context-dependent — stays per-node
            backdrop_blur: None,
            // Glass is context-dependent — stays per-node
            glass: None,
            // Noise is composited onto fills — stays per-node
            noises: Vec::new(),
        };
        let own = LayerEffects {
            // Shadows and blur moved to surface level
            shadows: Vec::new(),
            blur: None,
            backdrop_blur: effects.backdrop_blur.clone(),
            glass: effects.glass,
            noises: effects.noises.clone(),
        };
        (surface, own)
    }

    /// Computes stroke geometry for rectangular shapes with support for per-side widths.
    ///
    /// This handles both uniform and per-side stroke widths for rectangular shapes.
    /// Per-side strokes are rendered as filled ring geometry (outer - inner rectangles).
    /// Falls back to uniform stroke rendering when corners are rounded.
    ///
    /// # Parameters
    ///
    /// - `stroke_width`: The resolved stroke width (uniform, rectangular, or none)
    /// - `corner_radius`: Corner radius configuration (per-side strokes need zero radius)
    /// - `stroke_style`: Stroke style (alignment, dash pattern, etc.)
    /// - `size`: The size of the rectangular shape
    /// - `shape`: The painter shape (used for uniform stroke fallback)
    ///
    /// # Returns
    ///
    /// A `Path` representing the stroke geometry, or `None` if there's no stroke.
    fn compute_rectangular_stroke_path(
        stroke_width: &StrokeWidth,
        corner_radius: &RectangularCornerRadius,
        stroke_style: &StrokeStyle,
        size: &Size,
        shape: &PainterShape,
    ) -> Option<Path> {
        match stroke_width {
            StrokeWidth::None => None,
            StrokeWidth::Uniform(width) => {
                if *width > 0.0 {
                    Some(stroke_geometry(
                        &shape.to_path(),
                        *width,
                        stroke_style.stroke_align,
                        stroke_style.stroke_cap,
                        stroke_style.stroke_join,
                        stroke_style.stroke_miter_limit,
                        stroke_style.stroke_dash_array.as_ref(),
                    ))
                } else {
                    None
                }
            }
            StrokeWidth::Rectangular(rect_stroke) => {
                // Per-side strokes support all alignments and corner radii
                // Use local-space rect (0, 0) since transform is already applied by painter
                let rect = skia_safe::Rect::from_xywh(0.0, 0.0, size.width, size.height);
                Some(stroke_geometry_rectangular(
                    rect,
                    rect_stroke,
                    corner_radius,
                    stroke_style.stroke_align,
                    stroke_style.stroke_miter_limit,
                    stroke_style.stroke_dash_array.as_ref(),
                ))
            }
        }
    }

    /// Flatten an entire scene into a layer list using the provided scene cache.
    pub fn from_scene(scene: &Scene, scene_cache: &SceneCache) -> Self {
        let mut list = LayerList::default();
        for id in scene.graph.roots() {
            let result = Self::flatten_node(id, &scene.graph, scene_cache, 1.0, &mut list.layers);
            list.commands.extend(result.commands);
        }
        // Build a LUT (id -> index) for picture caching and quick lookup
        // by ensuring `layers` order and `commands` reference the same layer instances.
        list
    }

    /// Build a layer list starting from a node subtree using a scene cache.
    ///
    /// `opacity` is the starting parent opacity for the subtree root.
    /// For export, pass `1.0` — ancestor opacity is not propagated, so the
    /// node renders in isolation with only its own opacity. This matches
    /// standard design tool export semantics.
    /// For compositing within a live scene, pass the accumulated ancestor
    /// opacity so the subtree blends correctly in context.
    pub fn from_node(
        id: &NodeId,
        graph: &SceneGraph,
        scene_cache: &SceneCache,
        opacity: f32,
    ) -> Self {
        let mut list = LayerList::default();
        let result = Self::flatten_node(id, graph, scene_cache, opacity, &mut list.layers);
        list.commands = result.commands;
        list
    }

    /// Compute a fill path with the stroke region subtracted (PathOp::Difference).
    ///
    /// Returns `Some(path)` when stroke overlaps fill and both are present.
    /// The resulting path, drawn with per-paint-alpha opacity, produces output
    /// identical to `save_layer_alpha` group isolation — zero overlap, zero
    /// GPU surface allocations.
    ///
    /// Returns `None` if no overlap, fills/strokes are empty, or PathOp fails
    /// (degenerate geometry). The painter falls back to `save_layer_alpha`.
    fn compute_non_overlapping_fill_path(
        shape: &PainterShape,
        stroke_path: Option<&skia_safe::Path>,
        stroke_overlaps_fill: bool,
        fills: &Paints,
        strokes: &Paints,
        node_opacity: f32,
    ) -> Option<skia_safe::Path> {
        // The non-overlapping fill path is only needed when the node's own
        // opacity < 1.0. At full opacity, stroke/fill overlap is invisible —
        // no compositing artifact, no need for the expensive PathOp::Difference.
        // Parent opacity is handled at the parent level via save_layer_alpha.
        if node_opacity >= 1.0 || !stroke_overlaps_fill || fills.is_empty() || strokes.is_empty() {
            return None;
        }
        stroke_path
            .and_then(|sp| skia_safe::op(&shape.to_path(), sp, skia_safe::PathOp::Difference))
    }

    fn flatten_node(
        id: &NodeId,
        graph: &SceneGraph,
        scene_cache: &SceneCache,
        parent_opacity: f32,
        out: &mut Vec<LayerEntry>,
    ) -> FlattenResult {
        // Fast-path: check active from compact layer_core (~16 bytes)
        // before touching the full Node enum (~500+ bytes).
        if let Some(lc) = graph.get_layer_core(id) {
            if !lc.active {
                return FlattenResult::default();
            }
        }

        let Ok(node) = graph.get_node(id) else {
            return FlattenResult::default();
        };

        let transform = scene_cache
            .geometry()
            .get_world_transform(id)
            .unwrap_or_else(AffineTransform::identity);

        match node {
            Node::Group(n) => {
                let opacity = parent_opacity * n.opacity;
                let children = graph.get_children(id).map(|c| c.as_slice()).unwrap_or(&[]);
                FlattenResult {
                    commands: Self::build_render_commands(
                        children,
                        graph,
                        scene_cache,
                        opacity,
                        out,
                    ),
                    mask: n.mask,
                }
            }
            Node::Tray(n) => {
                // Tray renders like a simplified Container — has fills, strokes, corner_radius,
                // explicit dimensions. No effects, no clipping, no render surface.
                let opacity = parent_opacity * n.opacity;
                let local_bounds = scene_cache
                    .geometry()
                    .get_entry(id)
                    .expect("Geometry must exist")
                    .bounding_box;
                let shape = build_shape(node, &local_bounds);
                let size = Size {
                    width: local_bounds.width,
                    height: local_bounds.height,
                };
                let stroke_path = Self::compute_rectangular_stroke_path(
                    &n.stroke_width,
                    &n.corner_radius,
                    &n.stroke_style,
                    &size,
                    &shape,
                );

                let fills = Self::filter_visible_paints(&n.fills);
                let strokes = Self::filter_visible_paints(&n.strokes);
                let stroke_overlaps_fill =
                    !matches!(n.stroke_style.stroke_align, StrokeAlign::Outside);
                let non_overlapping_fill_path = Self::compute_non_overlapping_fill_path(
                    &shape,
                    stroke_path.as_ref(),
                    stroke_overlaps_fill,
                    &fills,
                    &strokes,
                    n.opacity,
                );

                let layer = PainterPictureLayer::Shape(PainterPictureShapeLayer {
                    base: PainterPictureLayerBase {
                        id: *id,
                        z_index: out.len(),
                        opacity,
                        blend_mode: n.blend_mode,
                        transform,
                        clip_path: None, // Tray never clips
                    },
                    shape,
                    effects: LayerEffects::default(), // Tray has no effects
                    strokes,
                    fills,
                    stroke_path,
                    marker_start_shape: StrokeMarkerPreset::None,
                    marker_end_shape: StrokeMarkerPreset::None,
                    stroke_width: 0.0,
                    stroke_overlaps_fill,
                    non_overlapping_fill_path,
                });
                out.push(LayerEntry {
                    id: *id,
                    layer: layer.clone(),
                });

                // Children (no clipping — Tray never clips)
                let children = graph.get_children(id).map(|c| c.as_slice()).unwrap_or(&[]);
                let child_commands =
                    Self::build_render_commands(children, graph, scene_cache, opacity, out);

                let mut commands = vec![PainterRenderCommand::Draw(layer)];
                commands.extend(child_commands);
                FlattenResult {
                    commands,
                    mask: n.mask,
                }
            }
            Node::Container(n) => {
                let opacity = parent_opacity * n.opacity;
                let geo_entry = scene_cache
                    .geometry()
                    .get_entry(id)
                    .expect("Geometry must exist");
                // Use LOCAL bounds for shape building (not the world AABB).
                // The world AABB has swapped/inflated dimensions when rotated,
                // producing the wrong shape. The local `bounding_box` gives
                // the actual container dimensions in its own coordinate space.
                let local_bounds = geo_entry.bounding_box;
                let shape = build_shape(node, &local_bounds);
                let size = Size {
                    width: local_bounds.width,
                    height: local_bounds.height,
                };
                let stroke_path = Self::compute_rectangular_stroke_path(
                    &n.stroke_width,
                    &n.corner_radius,
                    &n.stroke_style,
                    &size,
                    &shape,
                );

                // Check if the effect tree marks this container for a render surface.
                let effect_node = scene_cache.effect_tree.get(id);
                let use_render_surface = effect_node
                    .map(|en| {
                        en.has_reason(crate::runtime::effect_tree::RenderSurfaceReason::Shadow)
                            || en.has_reason(
                                crate::runtime::effect_tree::RenderSurfaceReason::LayerBlur,
                            )
                    })
                    .unwrap_or(false);

                let all_effects = Self::filter_active_effects(&n.effects);

                // Split effects: surface-level effects go to the RenderSurface,
                // remaining effects stay on the container's own layer.
                let (surface_effects, own_effects) = if use_render_surface {
                    Self::split_surface_effects(&all_effects)
                } else {
                    (LayerEffects::default(), all_effects)
                };

                let clip_path = Self::compute_clip_path(id, graph, scene_cache);

                // When using a render surface, the container's own layer and
                // children are rendered at identity compositing (opacity=1.0,
                // blend=PassThrough) so the RenderSurface applies opacity and
                // blend mode exactly once during compositing.
                let (inner_opacity, inner_blend_mode) = if use_render_surface {
                    (parent_opacity, LayerBlendMode::PassThrough)
                } else {
                    (opacity, n.blend_mode)
                };

                let fills = Self::filter_visible_paints(&n.fills);
                let strokes = Self::filter_visible_paints(&n.strokes);
                let stroke_overlaps_fill =
                    !matches!(n.stroke_style.stroke_align, StrokeAlign::Outside);
                let non_overlapping_fill_path = Self::compute_non_overlapping_fill_path(
                    &shape,
                    stroke_path.as_ref(),
                    stroke_overlaps_fill,
                    &fills,
                    &strokes,
                    n.opacity,
                );

                let layer = PainterPictureLayer::Shape(PainterPictureShapeLayer {
                    base: PainterPictureLayerBase {
                        id: *id,
                        z_index: out.len(),
                        opacity: inner_opacity,
                        blend_mode: inner_blend_mode,
                        transform,
                        clip_path: clip_path.clone(),
                    },
                    shape,
                    effects: own_effects,
                    strokes,
                    fills,
                    stroke_path,
                    marker_start_shape: StrokeMarkerPreset::None,
                    marker_end_shape: StrokeMarkerPreset::None,
                    stroke_width: 0.0,
                    stroke_overlaps_fill,
                    non_overlapping_fill_path,
                });
                out.push(LayerEntry {
                    id: *id,
                    layer: layer.clone(),
                });

                let children = graph.get_children(id).map(|c| c.as_slice()).unwrap_or(&[]);
                let child_opacity = if use_render_surface {
                    parent_opacity
                } else {
                    opacity
                };
                let child_commands =
                    Self::build_render_commands(children, graph, scene_cache, child_opacity, out);

                if use_render_surface {
                    // Wrap the container's own layer + children in a RenderSurface.
                    // The surface-level effects (shadows, blur) are applied to the
                    // composited result instead of per-child.
                    let render_bounds = scene_cache
                        .geometry()
                        .get_render_bounds(id)
                        .unwrap_or(geo_entry.absolute_bounding_box);

                    // The clip_path from compute_clip_path is in the node's LOCAL
                    // coordinate space. However, draw_render_surface applies it
                    // directly to the canvas in WORLD space (before any per-node
                    // transform). Transform the clip path to world space so it
                    // clips correctly.
                    let surface_clip_path =
                        clip_path.map(|path| path.make_transform(&sk::sk_matrix(transform.matrix)));

                    let surface = PainterRenderSurface {
                        id: *id,
                        bounds: render_bounds,
                        transform,
                        opacity,
                        blend_mode: n.blend_mode,
                        effects: surface_effects,
                        clip_path: surface_clip_path,
                        own_layer: Some(layer),
                        children: child_commands,
                    };
                    FlattenResult {
                        commands: vec![PainterRenderCommand::RenderSurface(surface)],
                        mask: n.mask,
                    }
                } else {
                    let mut commands = vec![PainterRenderCommand::Draw(layer)];
                    commands.extend(child_commands);
                    FlattenResult {
                        commands,
                        mask: n.mask,
                    }
                }
            }
            Node::InitialContainer(_) => {
                // ICB is invisible - only render children
                let children = graph.get_children(id).map(|c| c.as_slice()).unwrap_or(&[]);
                FlattenResult {
                    commands: Self::build_render_commands(
                        children,
                        graph,
                        scene_cache,
                        parent_opacity,
                        out,
                    ),
                    mask: None,
                }
            }
            Node::BooleanOperation(n) => {
                let opacity = parent_opacity * n.opacity;
                if let Some(shape) = boolean_operation_shape(id, n, graph, scene_cache.geometry()) {
                    let stroke_width = n.stroke_width.value_or_zero();
                    let stroke_path = if !n.strokes.is_empty() && stroke_width > 0.0 {
                        Some(stroke_geometry(
                            &shape.to_path(),
                            stroke_width,
                            n.stroke_style.stroke_align,
                            n.stroke_style.stroke_cap,
                            n.stroke_style.stroke_join,
                            n.stroke_style.stroke_miter_limit,
                            n.stroke_style.stroke_dash_array.as_ref(),
                        ))
                    } else {
                        None
                    };
                    let fills = Self::filter_visible_paints(&n.fills);
                    let strokes = Self::filter_visible_paints(&n.strokes);
                    let stroke_overlaps_fill =
                        !matches!(n.stroke_style.stroke_align, StrokeAlign::Outside);
                    let non_overlapping_fill_path = Self::compute_non_overlapping_fill_path(
                        &shape,
                        stroke_path.as_ref(),
                        stroke_overlaps_fill,
                        &fills,
                        &strokes,
                        n.opacity,
                    );

                    let layer = PainterPictureLayer::Shape(PainterPictureShapeLayer {
                        base: PainterPictureLayerBase {
                            id: *id,
                            z_index: out.len(),
                            opacity,
                            blend_mode: n.blend_mode,
                            transform,
                            clip_path: Self::compute_clip_path(id, graph, scene_cache),
                        },
                        shape,
                        effects: Self::filter_active_effects(&n.effects),
                        strokes,
                        fills,
                        stroke_path,
                        marker_start_shape: StrokeMarkerPreset::None,
                        marker_end_shape: StrokeMarkerPreset::None,
                        stroke_width: 0.0,
                        stroke_overlaps_fill,
                        non_overlapping_fill_path,
                    });
                    out.push(LayerEntry {
                        id: *id,
                        layer: layer.clone(),
                    });
                    FlattenResult {
                        commands: vec![PainterRenderCommand::Draw(layer)],
                        mask: n.mask,
                    }
                } else {
                    let children = graph.get_children(id).map(|c| c.as_slice()).unwrap_or(&[]);
                    FlattenResult {
                        commands: Self::build_render_commands(
                            children,
                            graph,
                            scene_cache,
                            opacity,
                            out,
                        ),
                        mask: n.mask,
                    }
                }
            }
            Node::Rectangle(n) => {
                let bounds = scene_cache
                    .geometry()
                    .get_world_bounds(id)
                    .expect("Geometry must exist");
                let shape = build_shape(node, &bounds);
                let stroke_path = Self::compute_rectangular_stroke_path(
                    &n.stroke_width,
                    &n.corner_radius,
                    &n.stroke_style,
                    &n.size,
                    &shape,
                );
                let fills = Self::filter_visible_paints(&n.fills);
                let strokes = Self::filter_visible_paints(&n.strokes);
                let stroke_overlaps_fill =
                    !matches!(n.stroke_style.stroke_align, StrokeAlign::Outside);
                let non_overlapping_fill_path = Self::compute_non_overlapping_fill_path(
                    &shape,
                    stroke_path.as_ref(),
                    stroke_overlaps_fill,
                    &fills,
                    &strokes,
                    n.opacity,
                );

                let layer = PainterPictureLayer::Shape(PainterPictureShapeLayer {
                    base: PainterPictureLayerBase {
                        id: *id,
                        z_index: out.len(),
                        opacity: parent_opacity * n.opacity,
                        blend_mode: n.blend_mode,
                        transform,
                        clip_path: Self::compute_clip_path(id, graph, scene_cache),
                    },
                    shape,
                    effects: Self::filter_active_effects(&n.effects),
                    strokes,
                    fills,
                    stroke_path,
                    marker_start_shape: StrokeMarkerPreset::None,
                    marker_end_shape: StrokeMarkerPreset::None,
                    stroke_width: 0.0,
                    stroke_overlaps_fill,
                    non_overlapping_fill_path,
                });
                out.push(LayerEntry {
                    id: *id,
                    layer: layer.clone(),
                });
                FlattenResult {
                    commands: vec![PainterRenderCommand::Draw(layer)],
                    mask: n.mask,
                }
            }
            Node::Ellipse(n) => {
                let bounds = scene_cache
                    .geometry()
                    .get_world_bounds(id)
                    .expect("Geometry must exist");
                let shape = build_shape(node, &bounds);
                let stroke_width = n.render_bounds_stroke_width();
                let stroke_path = if stroke_width > 0.0 {
                    Some(stroke_geometry(
                        &shape.to_path(),
                        stroke_width,
                        n.stroke_style.stroke_align,
                        n.stroke_style.stroke_cap,
                        n.stroke_style.stroke_join,
                        n.stroke_style.stroke_miter_limit,
                        n.stroke_style.stroke_dash_array.as_ref(),
                    ))
                } else {
                    None
                };
                let fills = Self::filter_visible_paints(&n.fills);
                let strokes = Self::filter_visible_paints(&n.strokes);
                let stroke_overlaps_fill =
                    !matches!(n.stroke_style.stroke_align, StrokeAlign::Outside);
                let non_overlapping_fill_path = Self::compute_non_overlapping_fill_path(
                    &shape,
                    stroke_path.as_ref(),
                    stroke_overlaps_fill,
                    &fills,
                    &strokes,
                    n.opacity,
                );

                let layer = PainterPictureLayer::Shape(PainterPictureShapeLayer {
                    base: PainterPictureLayerBase {
                        id: *id,
                        z_index: out.len(),
                        opacity: parent_opacity * n.opacity,
                        blend_mode: n.blend_mode,
                        transform,
                        clip_path: Self::compute_clip_path(id, graph, scene_cache),
                    },
                    shape,
                    effects: Self::filter_active_effects(&n.effects),
                    strokes,
                    fills,
                    stroke_path,
                    marker_start_shape: StrokeMarkerPreset::None,
                    marker_end_shape: StrokeMarkerPreset::None,
                    stroke_width: 0.0,
                    stroke_overlaps_fill,
                    non_overlapping_fill_path,
                });
                out.push(LayerEntry {
                    id: *id,
                    layer: layer.clone(),
                });
                FlattenResult {
                    commands: vec![PainterRenderCommand::Draw(layer)],
                    mask: n.mask,
                }
            }
            Node::Polygon(n) => {
                let bounds = scene_cache
                    .geometry()
                    .get_world_bounds(id)
                    .expect("Geometry must exist");
                let shape = build_shape(node, &bounds);
                let stroke_width = n.render_bounds_stroke_width();
                let stroke_path = if stroke_width > 0.0 {
                    Some(stroke_geometry(
                        &shape.to_path(),
                        stroke_width,
                        n.stroke_style.stroke_align,
                        n.stroke_style.stroke_cap,
                        n.stroke_style.stroke_join,
                        n.stroke_style.stroke_miter_limit,
                        n.stroke_style.stroke_dash_array.as_ref(),
                    ))
                } else {
                    None
                };
                let fills = Self::filter_visible_paints(&n.fills);
                let strokes = Self::filter_visible_paints(&n.strokes);
                let stroke_overlaps_fill =
                    !matches!(n.stroke_style.stroke_align, StrokeAlign::Outside);
                let non_overlapping_fill_path = Self::compute_non_overlapping_fill_path(
                    &shape,
                    stroke_path.as_ref(),
                    stroke_overlaps_fill,
                    &fills,
                    &strokes,
                    n.opacity,
                );

                let layer = PainterPictureLayer::Shape(PainterPictureShapeLayer {
                    base: PainterPictureLayerBase {
                        id: *id,
                        z_index: out.len(),
                        opacity: parent_opacity * n.opacity,
                        blend_mode: n.blend_mode,
                        transform,
                        clip_path: Self::compute_clip_path(id, graph, scene_cache),
                    },
                    shape,
                    effects: Self::filter_active_effects(&n.effects),
                    strokes,
                    fills,
                    stroke_path,
                    marker_start_shape: StrokeMarkerPreset::None,
                    marker_end_shape: StrokeMarkerPreset::None,
                    stroke_width: 0.0,
                    stroke_overlaps_fill,
                    non_overlapping_fill_path,
                });
                out.push(LayerEntry {
                    id: *id,
                    layer: layer.clone(),
                });
                FlattenResult {
                    commands: vec![PainterRenderCommand::Draw(layer)],
                    mask: n.mask,
                }
            }
            Node::RegularPolygon(n) => {
                let bounds = scene_cache
                    .geometry()
                    .get_world_bounds(id)
                    .expect("Geometry must exist");
                let shape = build_shape(node, &bounds);
                let stroke_width = n.render_bounds_stroke_width();
                let stroke_path = if stroke_width > 0.0 {
                    Some(stroke_geometry(
                        &shape.to_path(),
                        stroke_width,
                        n.stroke_style.stroke_align,
                        n.stroke_style.stroke_cap,
                        n.stroke_style.stroke_join,
                        n.stroke_style.stroke_miter_limit,
                        n.stroke_style.stroke_dash_array.as_ref(),
                    ))
                } else {
                    None
                };
                let fills = Self::filter_visible_paints(&n.fills);
                let strokes = Self::filter_visible_paints(&n.strokes);
                let stroke_overlaps_fill =
                    !matches!(n.stroke_style.stroke_align, StrokeAlign::Outside);
                let non_overlapping_fill_path = Self::compute_non_overlapping_fill_path(
                    &shape,
                    stroke_path.as_ref(),
                    stroke_overlaps_fill,
                    &fills,
                    &strokes,
                    n.opacity,
                );

                let layer = PainterPictureLayer::Shape(PainterPictureShapeLayer {
                    base: PainterPictureLayerBase {
                        id: *id,
                        z_index: out.len(),
                        opacity: parent_opacity * n.opacity,
                        blend_mode: n.blend_mode,
                        transform,
                        clip_path: Self::compute_clip_path(id, graph, scene_cache),
                    },
                    shape,
                    effects: Self::filter_active_effects(&n.effects),
                    strokes,
                    fills,
                    stroke_path,
                    marker_start_shape: StrokeMarkerPreset::None,
                    marker_end_shape: StrokeMarkerPreset::None,
                    stroke_width: 0.0,
                    stroke_overlaps_fill,
                    non_overlapping_fill_path,
                });
                out.push(LayerEntry {
                    id: *id,
                    layer: layer.clone(),
                });
                FlattenResult {
                    commands: vec![PainterRenderCommand::Draw(layer)],
                    mask: n.mask,
                }
            }
            Node::RegularStarPolygon(n) => {
                let bounds = scene_cache
                    .geometry()
                    .get_world_bounds(id)
                    .expect("Geometry must exist");
                let shape = build_shape(node, &bounds);
                let stroke_width = n.render_bounds_stroke_width();
                let stroke_path = if stroke_width > 0.0 {
                    Some(stroke_geometry(
                        &shape.to_path(),
                        stroke_width,
                        n.stroke_style.stroke_align,
                        n.stroke_style.stroke_cap,
                        n.stroke_style.stroke_join,
                        n.stroke_style.stroke_miter_limit,
                        n.stroke_style.stroke_dash_array.as_ref(),
                    ))
                } else {
                    None
                };
                let fills = Self::filter_visible_paints(&n.fills);
                let strokes = Self::filter_visible_paints(&n.strokes);
                let stroke_overlaps_fill =
                    !matches!(n.stroke_style.stroke_align, StrokeAlign::Outside);
                let non_overlapping_fill_path = Self::compute_non_overlapping_fill_path(
                    &shape,
                    stroke_path.as_ref(),
                    stroke_overlaps_fill,
                    &fills,
                    &strokes,
                    n.opacity,
                );

                let layer = PainterPictureLayer::Shape(PainterPictureShapeLayer {
                    base: PainterPictureLayerBase {
                        id: *id,
                        z_index: out.len(),
                        opacity: parent_opacity * n.opacity,
                        blend_mode: n.blend_mode,
                        transform,
                        clip_path: Self::compute_clip_path(id, graph, scene_cache),
                    },
                    shape,
                    effects: Self::filter_active_effects(&n.effects),
                    strokes,
                    fills,
                    stroke_path,
                    marker_start_shape: StrokeMarkerPreset::None,
                    marker_end_shape: StrokeMarkerPreset::None,
                    stroke_width: 0.0,
                    stroke_overlaps_fill,
                    non_overlapping_fill_path,
                });
                out.push(LayerEntry {
                    id: *id,
                    layer: layer.clone(),
                });
                FlattenResult {
                    commands: vec![PainterRenderCommand::Draw(layer)],
                    mask: n.mask,
                }
            }
            Node::Line(n) => {
                let bounds = scene_cache
                    .geometry()
                    .get_world_bounds(id)
                    .expect("Geometry must exist");
                let shape = build_shape(node, &bounds);

                // Compute cutback distances for endpoint decorations
                let start_cutback =
                    crate::shape::marker::cutback_depth(n.marker_start_shape, n.stroke_width);
                let end_cutback =
                    crate::shape::marker::cutback_depth(n.marker_end_shape, n.stroke_width);

                // Force Butt cap when decorations are present so the native
                // cap geometry doesn't leak out from under the marker.
                let has_any_decoration =
                    n.marker_start_shape.has_marker() || n.marker_end_shape.has_marker();
                let effective_cap = if has_any_decoration {
                    StrokeCap::Butt
                } else {
                    n.stroke_cap
                };

                let stroke_path = if n.stroke_width > 0.0 {
                    // Trim the source path by cutback before computing stroke geometry
                    let source_path = shape.to_path();
                    let trimmed = if start_cutback > 0.0 || end_cutback > 0.0 {
                        crate::shape::marker::trim_path(&source_path, start_cutback, end_cutback)
                    } else {
                        source_path
                    };
                    Some(stroke_geometry(
                        &trimmed,
                        n.stroke_width,
                        n.get_stroke_align(),
                        effective_cap,
                        StrokeJoin::default(), // Join not applicable for single line
                        n.stroke_miter_limit,
                        n.stroke_dash_array.as_ref(),
                    ))
                } else {
                    None
                };
                let layer = PainterPictureLayer::Shape(PainterPictureShapeLayer {
                    base: PainterPictureLayerBase {
                        id: *id,
                        z_index: out.len(),
                        opacity: parent_opacity * n.opacity,
                        blend_mode: n.blend_mode,
                        transform,
                        clip_path: Self::compute_clip_path(id, graph, scene_cache),
                    },
                    shape,
                    effects: Self::filter_active_effects(&n.effects),
                    strokes: n.strokes.clone(),
                    fills: Paints::default(),
                    stroke_path,
                    marker_start_shape: n.marker_start_shape,
                    marker_end_shape: n.marker_end_shape,
                    stroke_width: n.stroke_width,
                    stroke_overlaps_fill: true,
                    non_overlapping_fill_path: None,
                });
                out.push(LayerEntry {
                    id: *id,
                    layer: layer.clone(),
                });
                FlattenResult {
                    commands: vec![PainterRenderCommand::Draw(layer)],
                    mask: n.mask,
                }
            }
            Node::TextSpan(n) => {
                let text_bounds =
                    scene_cache
                        .geometry()
                        .get_world_bounds(id)
                        .unwrap_or_else(|| Rectangle {
                            x: n.x(),
                            y: n.y(),
                            width: n.width.unwrap_or(100.0),
                            height: (n.text_style.font_size
                                * match n.text_style.line_height {
                                    TextLineHeight::Fixed(height) => {
                                        height / n.text_style.font_size
                                    }
                                    TextLineHeight::Factor(factor) => factor,
                                    TextLineHeight::Normal => 1.2,
                                }
                                * 2.0)
                                .max(0.0),
                        });

                let rect_height = n.height.unwrap_or(text_bounds.height);
                let shape = PainterShape::from_rect(skia_safe::Rect::from_xywh(
                    0.0,
                    0.0,
                    text_bounds.width,
                    rect_height,
                ));

                let layer = PainterPictureLayer::Text(PainterPictureTextLayer {
                    base: PainterPictureLayerBase {
                        id: *id,
                        z_index: out.len(),
                        opacity: parent_opacity * n.opacity,
                        blend_mode: n.blend_mode,
                        transform,
                        clip_path: Self::compute_clip_path(id, graph, scene_cache),
                    },
                    width: n.width,
                    height: n.height,
                    max_lines: n.max_lines,
                    ellipsis: n.ellipsis.clone(),
                    effects: Self::filter_active_effects(&n.effects),
                    strokes: Self::filter_visible_paints(&n.strokes),
                    fills: Self::filter_visible_paints(&n.fills),
                    stroke_width: n.stroke_width,
                    stroke_align: n.stroke_align,
                    stroke_path: None,
                    shape,
                    text: n.text.clone(),
                    text_style: n.text_style.clone(),
                    text_align: n.text_align,
                    text_align_vertical: n.text_align_vertical,
                    id: *id,
                    attributed_string: None,
                });
                out.push(LayerEntry {
                    id: *id,
                    layer: layer.clone(),
                });
                FlattenResult {
                    commands: vec![PainterRenderCommand::Draw(layer)],
                    mask: n.mask,
                }
            }
            Node::Path(n) => {
                let bounds = scene_cache
                    .geometry()
                    .get_world_bounds(id)
                    .expect("Geometry must exist");
                let shape = build_shape(node, &bounds);
                let stroke_width = n.stroke_width.value_or_zero();
                let stroke_path = if stroke_width > 0.0 {
                    Some(stroke_geometry(
                        &shape.to_path(),
                        stroke_width,
                        n.stroke_style.stroke_align,
                        n.stroke_style.stroke_cap,
                        n.stroke_style.stroke_join,
                        n.stroke_style.stroke_miter_limit,
                        n.stroke_style.stroke_dash_array.as_ref(),
                    ))
                } else {
                    None
                };
                let fills = Self::filter_visible_paints(&n.fills);
                let strokes = Self::filter_visible_paints(&n.strokes);
                let stroke_overlaps_fill =
                    !matches!(n.stroke_style.stroke_align, StrokeAlign::Outside);
                let non_overlapping_fill_path = Self::compute_non_overlapping_fill_path(
                    &shape,
                    stroke_path.as_ref(),
                    stroke_overlaps_fill,
                    &fills,
                    &strokes,
                    n.opacity,
                );

                let layer = PainterPictureLayer::Shape(PainterPictureShapeLayer {
                    base: PainterPictureLayerBase {
                        id: *id,
                        z_index: out.len(),
                        opacity: parent_opacity * n.opacity,
                        blend_mode: n.blend_mode,
                        transform,
                        clip_path: Self::compute_clip_path(id, graph, scene_cache),
                    },
                    shape,
                    effects: Self::filter_active_effects(&n.effects),
                    strokes,
                    fills,
                    stroke_path,
                    marker_start_shape: StrokeMarkerPreset::None,
                    marker_end_shape: StrokeMarkerPreset::None,
                    stroke_width: 0.0,
                    stroke_overlaps_fill,
                    non_overlapping_fill_path,
                });
                out.push(LayerEntry {
                    id: *id,
                    layer: layer.clone(),
                });
                FlattenResult {
                    commands: vec![PainterRenderCommand::Draw(layer)],
                    mask: n.mask,
                }
            }
            Node::Vector(n) => {
                let bounds = scene_cache
                    .geometry()
                    .get_world_bounds(id)
                    .expect("Geometry must exist");
                let shape = build_shape(node, &bounds);
                let layer = PainterPictureLayer::Vector(PainterPictureVectorLayer {
                    base: PainterPictureLayerBase {
                        id: *id,
                        z_index: out.len(),
                        opacity: parent_opacity * n.opacity,
                        blend_mode: n.blend_mode,
                        transform,
                        clip_path: Self::compute_clip_path(id, graph, scene_cache),
                    },
                    shape,
                    effects: Self::filter_active_effects(&n.effects),
                    strokes: Self::filter_visible_paints(&n.strokes),
                    fills: Self::filter_visible_paints(&n.fills),
                    vector: n.network.clone(),
                    stroke_width: n.stroke_width,
                    stroke_align: n.get_stroke_align(),
                    stroke_cap: n.stroke_cap,
                    stroke_join: n.stroke_join,
                    stroke_miter_limit: n.stroke_miter_limit,
                    stroke_width_profile: n.stroke_width_profile.clone(),
                    stroke_dash_array: n.stroke_dash_array.clone(),
                    corner_radius: n.corner_radius,
                    marker_start_shape: n.marker_start_shape,
                    marker_end_shape: n.marker_end_shape,
                });
                out.push(LayerEntry {
                    id: *id,
                    layer: layer.clone(),
                });
                FlattenResult {
                    commands: vec![PainterRenderCommand::Draw(layer)],
                    mask: n.mask,
                }
            }
            Node::Image(n) => {
                let bounds = scene_cache
                    .geometry()
                    .get_world_bounds(id)
                    .expect("Geometry must exist");
                let shape = build_shape(node, &bounds);
                let stroke_width = n.render_bounds_stroke_width();
                let stroke_path = if stroke_width > 0.0 {
                    Some(stroke_geometry(
                        &shape.to_path(),
                        stroke_width,
                        n.stroke_style.stroke_align,
                        n.stroke_style.stroke_cap,
                        n.stroke_style.stroke_join,
                        n.stroke_style.stroke_miter_limit,
                        n.stroke_style.stroke_dash_array.as_ref(),
                    ))
                } else {
                    None
                };
                let fills =
                    Self::filter_visible_paints(&Paints::new([Paint::Image(n.fill.clone())]));
                let strokes = Self::filter_visible_paints(&n.strokes);
                let stroke_overlaps_fill =
                    !matches!(n.stroke_style.stroke_align, StrokeAlign::Outside);
                let non_overlapping_fill_path = Self::compute_non_overlapping_fill_path(
                    &shape,
                    stroke_path.as_ref(),
                    stroke_overlaps_fill,
                    &fills,
                    &strokes,
                    n.opacity,
                );

                let layer = PainterPictureLayer::Shape(PainterPictureShapeLayer {
                    base: PainterPictureLayerBase {
                        id: *id,
                        z_index: out.len(),
                        opacity: parent_opacity * n.opacity,
                        blend_mode: n.blend_mode,
                        transform,
                        clip_path: Self::compute_clip_path(id, graph, scene_cache),
                    },
                    shape,
                    effects: Self::filter_active_effects(&n.effects),
                    strokes,
                    fills,
                    stroke_path,
                    marker_start_shape: StrokeMarkerPreset::None,
                    marker_end_shape: StrokeMarkerPreset::None,
                    stroke_width: 0.0,
                    stroke_overlaps_fill,
                    non_overlapping_fill_path,
                });
                out.push(LayerEntry {
                    id: *id,
                    layer: layer.clone(),
                });
                FlattenResult {
                    commands: vec![PainterRenderCommand::Draw(layer)],
                    mask: n.mask,
                }
            }
            Node::AttributedText(n) => {
                let text_bounds =
                    scene_cache
                        .geometry()
                        .get_world_bounds(id)
                        .unwrap_or_else(|| Rectangle {
                            x: n.x(),
                            y: n.y(),
                            width: n.width.unwrap_or(100.0),
                            height: (n.default_style.font_size
                                * match n.default_style.line_height {
                                    TextLineHeight::Fixed(height) => {
                                        height / n.default_style.font_size
                                    }
                                    TextLineHeight::Factor(factor) => factor,
                                    TextLineHeight::Normal => 1.2,
                                }
                                * 2.0)
                                .max(0.0),
                        });

                let rect_height = n.height.unwrap_or(text_bounds.height);
                let shape = PainterShape::from_rect(skia_safe::Rect::from_xywh(
                    0.0,
                    0.0,
                    text_bounds.width,
                    rect_height,
                ));

                let layer = PainterPictureLayer::Text(PainterPictureTextLayer {
                    base: PainterPictureLayerBase {
                        id: *id,
                        z_index: out.len(),
                        opacity: parent_opacity * n.opacity,
                        blend_mode: n.blend_mode,
                        transform,
                        clip_path: Self::compute_clip_path(id, graph, scene_cache),
                    },
                    width: n.width,
                    height: n.height,
                    max_lines: n.max_lines,
                    ellipsis: n.ellipsis.clone(),
                    effects: Self::filter_active_effects(&n.effects),
                    strokes: Self::filter_visible_paints(&n.strokes),
                    fills: Self::filter_visible_paints(&n.fills),
                    stroke_width: n.stroke_width,
                    stroke_align: n.stroke_align,
                    stroke_path: None,
                    shape,
                    text: n.attributed_string.text.clone(),
                    text_style: n.default_style.clone(),
                    text_align: n.text_align,
                    text_align_vertical: n.text_align_vertical,
                    id: *id,
                    attributed_string: Some(n.attributed_string.clone()),
                });
                out.push(LayerEntry {
                    id: *id,
                    layer: layer.clone(),
                });
                FlattenResult {
                    commands: vec![PainterRenderCommand::Draw(layer)],
                    mask: n.mask,
                }
            }
            Node::Error(n) => {
                let bounds = scene_cache
                    .geometry()
                    .get_world_bounds(id)
                    .expect("Geometry must exist");
                let shape = build_shape(node, &bounds);
                let layer = PainterPictureLayer::Shape(PainterPictureShapeLayer {
                    base: PainterPictureLayerBase {
                        id: *id,
                        z_index: out.len(),
                        opacity: parent_opacity * n.opacity,
                        blend_mode: LayerBlendMode::PassThrough,
                        transform,
                        clip_path: Self::compute_clip_path(id, graph, scene_cache),
                    },
                    shape,
                    effects: LayerEffects::default(),
                    strokes: Paints::default(),
                    fills: Paints::default(),
                    stroke_path: None,
                    marker_start_shape: StrokeMarkerPreset::None,
                    marker_end_shape: StrokeMarkerPreset::None,
                    stroke_width: 0.0,
                    stroke_overlaps_fill: false,
                    non_overlapping_fill_path: None,
                });
                out.push(LayerEntry {
                    id: *id,
                    layer: layer.clone(),
                });
                FlattenResult {
                    commands: vec![PainterRenderCommand::Draw(layer)],
                    mask: None,
                }
            }
            Node::MarkdownEmbed(n) => {
                let bounds = scene_cache
                    .geometry()
                    .get_world_bounds(id)
                    .expect("Geometry must exist");
                let shape = build_shape(node, &bounds);
                let fills = Self::filter_visible_paints(&n.fills);

                // Use resolved bounds for width/height so auto-sizing is
                // reflected correctly (the geometry cache already ran
                // markdown measurement when schema height is None).
                let layer = PainterPictureLayer::MarkdownEmbed(PainterPictureMarkdownEmbedLayer {
                    base: PainterPictureLayerBase {
                        id: *id,
                        z_index: out.len(),
                        opacity: parent_opacity * n.opacity,
                        blend_mode: n.blend_mode,
                        transform,
                        clip_path: Self::compute_clip_path(id, graph, scene_cache),
                    },
                    shape,
                    effects: Self::filter_active_effects(&n.effects),
                    fills,
                    markdown: n.markdown.clone(),
                    width: bounds.width,
                    height: bounds.height,
                });
                out.push(LayerEntry {
                    id: *id,
                    layer: layer.clone(),
                });
                FlattenResult {
                    commands: vec![PainterRenderCommand::Draw(layer)],
                    mask: n.mask,
                }
            }
            Node::HTMLEmbed(n) => {
                let bounds = scene_cache
                    .geometry()
                    .get_world_bounds(id)
                    .expect("Geometry must exist");
                let shape = build_shape(node, &bounds);
                let fills = Self::filter_visible_paints(&n.fills);

                let layer = PainterPictureLayer::HtmlEmbed(PainterPictureHtmlEmbedLayer {
                    base: PainterPictureLayerBase {
                        id: *id,
                        z_index: out.len(),
                        opacity: parent_opacity * n.opacity,
                        blend_mode: n.blend_mode,
                        transform,
                        clip_path: Self::compute_clip_path(id, graph, scene_cache),
                    },
                    shape,
                    effects: Self::filter_active_effects(&n.effects),
                    fills,
                    html: n.html.clone(),
                    width: n.size.width,
                    height: n.size.height,
                });
                out.push(LayerEntry {
                    id: *id,
                    layer: layer.clone(),
                });
                FlattenResult {
                    commands: vec![PainterRenderCommand::Draw(layer)],
                    mask: n.mask,
                }
            }
        }
    }

    fn build_render_commands(
        children: &[NodeId],
        graph: &SceneGraph,
        scene_cache: &SceneCache,
        parent_opacity: f32,
        out: &mut Vec<LayerEntry>,
    ) -> Vec<PainterRenderCommand> {
        // Build render commands from child nodes using flat list model.
        // Iterate children in z-order (bottom→top). Accumulate non-mask commands in `run`.
        // When a mask node is encountered, emit MaskGroup { mask, items: run } and clear run.
        let mut out_commands = Vec::new();
        let mut run: Vec<PainterRenderCommand> = Vec::new();
        for child_id in children {
            let result = Self::flatten_node(child_id, graph, scene_cache, parent_opacity, out);
            if let Some(mask_type) = result.mask {
                let mask_commands = result.commands;
                // Emit a scope with the accumulated run as content under this mask
                out_commands.push(PainterRenderCommand::MaskGroup(PainterMaskGroup {
                    mask_type,
                    mask_commands,
                    content_commands: std::mem::take(&mut run),
                }));
            } else {
                // Not a mask — accumulate into the current run
                run.extend(result.commands);
            }
        }
        // Flush remaining run (no mask above it)
        out_commands.extend(run);
        out_commands
    }

    /// Computes the clip path for a node by traversing up the hierarchy
    /// and collecting all clip shapes from parent nodes.
    ///
    /// This function walks up the node tree starting from the given node ID,
    /// collecting shapes from parent nodes that have `clip = true`.
    /// The shapes are merged using boolean operations to create a single clip path.
    ///
    /// # Parameters
    ///
    /// - `node_id`: The ID of the node to compute the clip path for
    /// - `repo`: The node repository containing all nodes
    /// - `scene_cache`: The scene cache for transforms
    ///
    /// # Returns
    ///
    /// An `Option<Path>` representing the merged clip path, or `None` if no clipping is needed.
    pub fn compute_clip_path(
        node_id: &NodeId,
        graph: &SceneGraph,
        scene_cache: &SceneCache,
    ) -> Option<Path> {
        let mut clip_shapes = Vec::new();
        // Start from the parent of the current node so that a node's own
        // `clip` property only affects its descendants and not itself.
        let mut current_id = scene_cache.geometry().get_parent(node_id);

        let current_world = scene_cache
            .geometry()
            .get_world_transform(node_id)
            .unwrap_or_else(AffineTransform::identity);
        let current_inv = current_world
            .inverse()
            .unwrap_or_else(AffineTransform::identity);

        // Walk up the hierarchy to collect clip shapes
        while let Some(id) = current_id {
            if let Ok(node) = graph.get_node(&id) {
                match node {
                    Node::Container(n) => {
                        if n.clip {
                            // Get the world transform for this node
                            let world_transform = scene_cache
                                .geometry()
                                .get_world_transform(&id)
                                .unwrap_or_else(AffineTransform::identity);

                            // Build the shape and transform it relative to the current node
                            let bounds = scene_cache
                                .geometry()
                                .get_world_bounds(&id)
                                .expect("Geometry must exist");
                            let shape = build_shape(node, &bounds);
                            let mut path = shape.to_path();
                            let relative_transform = current_inv.compose(&world_transform);
                            path = path.make_transform(&sk::sk_matrix(relative_transform.matrix));

                            clip_shapes.push((
                                PainterShape::from_path(path),
                                BooleanPathOperation::Intersection,
                            ));
                        }
                    }
                    Node::BooleanOperation(n) => {
                        if let Some(mut path) =
                            boolean_operation_path(&id, n, graph, scene_cache.geometry())
                        {
                            let world_transform = scene_cache
                                .geometry()
                                .get_world_transform(&id)
                                .unwrap_or_else(AffineTransform::identity);
                            let relative_transform = current_inv.compose(&world_transform);
                            path = path.make_transform(&sk::sk_matrix(relative_transform.matrix));

                            clip_shapes.push((
                                PainterShape::from_path(path),
                                BooleanPathOperation::Intersection,
                            ));
                        }
                    }
                    _ => {} // Skip other node types
                }

                // Move up to parent
                current_id = scene_cache.geometry().get_parent(&id);
            } else {
                break;
            }
        }

        // If we have clip shapes, merge them
        if !clip_shapes.is_empty() {
            Some(merge_shapes(&clip_shapes))
        } else {
            None
        }
    }
}
