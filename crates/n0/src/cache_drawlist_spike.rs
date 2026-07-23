use super::*;
use crate::drawlist::{Item, ItemKind};
use n0_model::model::{
    AxisBinding, Color as ModelColor, CornerSmoothing, DocBuilder, GradientStop, Header,
    ImagePaint, LinearGradientPaint, Paint as ModelPaint, Paints, Payload, RectangularCornerRadius,
    ShapeDesc, SizeIntent,
};
use skia_safe::{surfaces, Color as SkColor};

const W: i32 = 96;
const H: i32 = 72;

fn options() -> ResolveOptions {
    ResolveOptions {
        viewport: (W as f32, H as f32),
        rotation_in_flow: RotationInFlow::VisualOnly,
    }
}

fn rect_list(node: n0_model::model::NodeId, paints: Paints) -> DrawList {
    let mut list = DrawList::default();
    list.items.push(Item {
        node,
        world: Affine::translate(8.0, 7.0),
        kind: ItemKind::RectFill {
            w: 30.0,
            h: 24.0,
            corner_radius: RectangularCornerRadius::default(),
            corner_smoothing: CornerSmoothing::default(),
            paints,
        },
    });
    list
}

fn solid_list(node: n0_model::model::NodeId, color: ModelColor) -> DrawList {
    rect_list(node, Paints::solid(color))
}

fn document(color: ModelColor) -> Document {
    let mut builder = DocBuilder::new();
    let mut header = Header::new(SizeIntent::Fixed(30.0), SizeIntent::Fixed(24.0));
    header.x = AxisBinding::start(8.0);
    header.y = AxisBinding::start(7.0);
    let node = builder.add(
        0,
        header,
        Payload::Shape {
            desc: ShapeDesc::Rect,
        },
    );
    builder.node_mut(node).fills = Paints::solid(color);
    builder.build()
}

fn drawlist_bytes(
    cache: &mut SceneCache,
    list: DrawList,
    environment: PaintEnvironmentKey,
    view: &Affine,
    ctx: &PaintCtx,
) -> (Vec<u8>, bool) {
    let mut surface = surfaces::raster_n32_premul((W, H)).unwrap();
    surface.canvas().clear(SkColor::WHITE);
    let rerastered = cache
        .frame_drawlist(surface.canvas(), list, environment, view, ctx)
        .unwrap();
    (crate::paint::read_pixels(&mut surface, W, H), rerastered)
}

fn ordinary_bytes(
    cache: &mut SceneCache,
    document: &Document,
    view: &Affine,
    ctx: &PaintCtx,
) -> (Vec<u8>, bool) {
    let mut surface = surfaces::raster_n32_premul((W, H)).unwrap();
    surface.canvas().clear(SkColor::WHITE);
    let rerastered = cache
        .frame(surface.canvas(), document, &options(), view, ctx, false)
        .unwrap();
    (crate::paint::read_pixels(&mut surface, W, H), rerastered)
}

fn assert_source_entry_reuses(
    cache: &mut SceneCache,
    document: &Document,
    expected: &[u8],
    ctx: &PaintCtx,
) {
    let (pixels, rerastered) = ordinary_bytes(cache, document, &Affine::IDENTITY, ctx);
    assert!(!rerastered, "the prior source entry was not retained");
    assert_eq!(pixels, expected);
}

#[test]
fn drawlist_cold_reuse_change_reuse_uses_raster_identity() {
    let ctx = PaintCtx::default();
    let environment = ctx.environment_key();
    let before = solid_list(7, ModelColor(0xFF11_2233));
    let same_raster_different_diagnostic_node = solid_list(91, ModelColor(0xFF11_2233));
    let after = solid_list(91, ModelColor(0xFFAA_5500));
    let mut cache = SceneCache::new(W, H);

    let (before_pixels, cold) =
        drawlist_bytes(&mut cache, before, environment, &Affine::IDENTITY, &ctx);
    let (same_pixels, reused) = drawlist_bytes(
        &mut cache,
        same_raster_different_diagnostic_node,
        environment,
        &Affine::IDENTITY,
        &ctx,
    );
    assert_eq!(
        cache.list.as_ref().unwrap().items[0].node,
        7,
        "raster-equal reuse retains the cached diagnostic node slot"
    );
    let (after_pixels, changed) = drawlist_bytes(
        &mut cache,
        after.clone(),
        environment,
        &Affine::IDENTITY,
        &ctx,
    );
    let (same_after_pixels, reused_after) =
        drawlist_bytes(&mut cache, after, environment, &Affine::IDENTITY, &ctx);

    assert_eq!(
        [cold, reused, changed, reused_after],
        [true, false, true, false]
    );
    assert_eq!(before_pixels, same_pixels);
    assert_ne!(before_pixels, after_pixels);
    assert_eq!(after_pixels, same_after_pixels);
}

#[test]
fn drawlist_input_uses_the_same_preview_view_policy() {
    let ctx = PaintCtx::default();
    let environment = ctx.environment_key();
    let list = solid_list(7, ModelColor::BLACK);
    let mut cache = SceneCache::new(W, H);
    let identity = Affine::IDENTITY;
    let panned = Affine::translate(16.0, 12.0);
    let zoomed = Affine {
        a: 1.5,
        b: 0.0,
        c: 0.0,
        d: 1.5,
        e: 16.0,
        f: 12.0,
    };
    let outside_margin = Affine { e: 400.0, ..zoomed };

    let (_, cold) = drawlist_bytes(&mut cache, list.clone(), environment, &identity, &ctx);
    let (_, exact) = drawlist_bytes(&mut cache, list.clone(), environment, &identity, &ctx);
    let (_, pan) = drawlist_bytes(&mut cache, list.clone(), environment, &panned, &ctx);
    let (_, zoom) = drawlist_bytes(&mut cache, list.clone(), environment, &zoomed, &ctx);
    let (_, far_pan) = drawlist_bytes(&mut cache, list, environment, &outside_margin, &ctx);

    // This locks policy decisions, not universal pixel identity with an
    // immediate frame raster. The margin-shifted preview path has its own
    // fixture-scoped exact gates.
    assert_eq!(
        [cold, exact, pan, zoom, far_pan],
        [true, false, false, true, true]
    );
}

#[test]
fn clean_source_input_selects_the_retained_drawlist() {
    let document = document(ModelColor::BLACK);
    let ctx = PaintCtx::default();
    let mut cache = SceneCache::new(W, H);
    let (_, cold) = ordinary_bytes(&mut cache, &document, &Affine::IDENTITY, &ctx);
    assert!(cold);

    let values = PropertyValues::default();
    let source = SourceCacheRequest {
        options: (&options()).into(),
        scene: document.key_of(document.root).unwrap(),
        values: &values,
        environment: ctx.environment_key(),
    };
    assert_eq!(
        cache.source_drawlist_decision(&source, false),
        SourceDrawListDecision::Retain
    );
    assert_eq!(
        cache.source_drawlist_decision(&source, true),
        SourceDrawListDecision::Rebuild
    );

    let (_, clean) = ordinary_bytes(&mut cache, &document, &Affine::IDENTITY, &ctx);
    assert!(!clean);
}

#[test]
fn drawlist_environment_mismatch_is_transactional() {
    let document = document(ModelColor::BLACK);
    let ctx = PaintCtx::default();
    let other_ctx = PaintCtx::default();
    let mut cache = SceneCache::new(W, H);
    let (prior_pixels, cold) = ordinary_bytes(&mut cache, &document, &Affine::IDENTITY, &ctx);
    assert!(cold);
    let prior_source_key = cache.source_key.clone();

    let mut destination = surfaces::raster_n32_premul((W, H)).unwrap();
    destination.canvas().clear(SkColor::MAGENTA);
    let before = crate::paint::read_pixels(&mut destination, W, H);
    let error = cache
        .frame_drawlist(
            destination.canvas(),
            solid_list(9, ModelColor(0xFF22_66AA)),
            other_ctx.environment_key(),
            &Affine::IDENTITY,
            &ctx,
        )
        .expect_err("mismatched drawlist environment must fail");
    assert!(matches!(
        error,
        SceneCacheError::FrameExecution(FrameExecutionError::Environment(_))
    ));
    assert_eq!(crate::paint::read_pixels(&mut destination, W, H), before);
    assert_eq!(cache.source_key, prior_source_key);
    assert_source_entry_reuses(&mut cache, &document, &prior_pixels, &ctx);
}

#[test]
fn drawlist_gradient_failure_is_transactional() {
    let document = document(ModelColor::BLACK);
    let ctx = PaintCtx::default();
    let mut cache = SceneCache::new(W, H);
    let (prior_pixels, cold) = ordinary_bytes(&mut cache, &document, &Affine::IDENTITY, &ctx);
    assert!(cold);
    let prior_source_key = cache.source_key.clone();
    let invalid = rect_list(
        9,
        Paints::new([ModelPaint::LinearGradient(LinearGradientPaint {
            transform: Affine {
                a: 1e-20,
                b: 1e-20,
                c: 0.0,
                d: 1e-20,
                e: 0.0,
                f: 0.0,
            },
            stops: vec![
                GradientStop {
                    offset: 0.0,
                    color: ModelColor::BLACK,
                },
                GradientStop {
                    offset: 1.0,
                    color: ModelColor(0xFFFF_FFFF),
                },
            ],
            ..Default::default()
        })]),
    );

    let mut destination = surfaces::raster_n32_premul((W, H)).unwrap();
    destination.canvas().clear(SkColor::MAGENTA);
    let before = crate::paint::read_pixels(&mut destination, W, H);
    let error = cache
        .frame_drawlist(
            destination.canvas(),
            invalid,
            ctx.environment_key(),
            &Affine::IDENTITY,
            &ctx,
        )
        .expect_err("invalid drawlist gradient must fail");
    assert!(matches!(
        error,
        SceneCacheError::FrameBuild(FrameBuildError::Gradient(_))
    ));
    assert_eq!(crate::paint::read_pixels(&mut destination, W, H), before);
    assert_eq!(cache.source_key, prior_source_key);
    assert_source_entry_reuses(&mut cache, &document, &prior_pixels, &ctx);
}

#[test]
fn drawlist_missing_image_failure_is_transactional() {
    let document = document(ModelColor::BLACK);
    let ctx = PaintCtx::default();
    let mut cache = SceneCache::new(W, H);
    let (prior_pixels, cold) = ordinary_bytes(&mut cache, &document, &Affine::IDENTITY, &ctx);
    assert!(cold);
    let prior_source_key = cache.source_key.clone();
    let missing = rect_list(
        9,
        Paints::new([ModelPaint::Image(ImagePaint::from_rid("missing"))]),
    );

    let mut destination = surfaces::raster_n32_premul((W, H)).unwrap();
    destination.canvas().clear(SkColor::CYAN);
    let before = crate::paint::read_pixels(&mut destination, W, H);
    let error = cache
        .frame_drawlist(
            destination.canvas(),
            missing,
            ctx.environment_key(),
            &Affine::IDENTITY,
            &ctx,
        )
        .expect_err("missing drawlist image must fail");
    assert!(matches!(
        error,
        SceneCacheError::FrameExecution(FrameExecutionError::Image(_))
    ));
    assert_eq!(crate::paint::read_pixels(&mut destination, W, H), before);
    assert_eq!(cache.source_key, prior_source_key);
    assert_source_entry_reuses(&mut cache, &document, &prior_pixels, &ctx);
}

#[test]
fn changed_drawlist_clears_stale_source_validity() {
    let ordinary = document(ModelColor::BLACK);
    let candidate = document(ModelColor(0xFF22_66AA));
    let ctx = PaintCtx::default();
    let mut cache = SceneCache::new(W, H);

    let (ordinary_pixels, cold) = ordinary_bytes(&mut cache, &ordinary, &Affine::IDENTITY, &ctx);
    assert!(cold);
    let ordinary_source_key = cache.source_key.clone();
    let ordinary_product =
        resolve_and_build_view(&ValueView::base(&ordinary), &options(), &ctx).unwrap();
    let (_, equal_list, equal_environment) = ordinary_product.into_parts();
    let (_, equal_reraster) = drawlist_bytes(
        &mut cache,
        equal_list,
        equal_environment,
        &Affine::IDENTITY,
        &ctx,
    );
    assert!(!equal_reraster);
    assert_eq!(
        cache.source_key, ordinary_source_key,
        "raster-equal drawlist reuse must preserve source validity"
    );

    let product = resolve_and_build_view(&ValueView::base(&candidate), &options(), &ctx).unwrap();
    let (_, candidate_list, environment) = product.into_parts();
    let (candidate_pixels, candidate_reraster) = drawlist_bytes(
        &mut cache,
        candidate_list,
        environment,
        &Affine::IDENTITY,
        &ctx,
    );
    assert!(candidate_reraster);
    assert_ne!(candidate_pixels, ordinary_pixels);
    assert!(
        cache.source_key.is_none(),
        "drawlist replacement must clear source validity atomically"
    );

    let (restored_pixels, restored) =
        ordinary_bytes(&mut cache, &ordinary, &Affine::IDENTITY, &ctx);
    assert!(restored, "ordinary source must rebuild after replacement");
    assert_eq!(restored_pixels, ordinary_pixels);
    let (_, reused) = ordinary_bytes(&mut cache, &ordinary, &Affine::IDENTITY, &ctx);
    assert!(!reused);
}
