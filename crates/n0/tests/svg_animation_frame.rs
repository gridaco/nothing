//! Producer contract for the bounded retained SVG-animation frame adapter.

use n0::paint::{read_pixels, PaintCtx};
use n0::svg_animation_frame;
use n0_model::animation::SampleTime;
use n0_model::model::NodeId;
use skia_safe::{surfaces, Color};

const WIDTH: i32 = 32;
const HEIGHT: i32 = 16;

fn pixel(bytes: &[u8], x: i32, y: i32) -> &[u8] {
    let offset = ((y * WIDTH + x) * 4) as usize;
    &bytes[offset..offset + 4]
}

fn named(compiled: &n0_model::svg_animation::CompiledSvgAnimation, name: &str) -> NodeId {
    (0..compiled.document().capacity() as NodeId)
        .find(|id| {
            compiled
                .document()
                .get_opt(*id)
                .and_then(|node| node.header.name.as_deref())
                == Some(name)
        })
        .unwrap()
}

#[test]
fn sample_entry_renders_one_explicit_time_into_caller_owned_storage() {
    let compiled = svg_animation_frame::compile_latest(
        "exact-time.svg",
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="32" height="16">
  <rect id="moving" x="0" y="4" width="8" height="8" fill="#ff0000">
    <animate attributeName="x" from="0" to="16" dur="1s" fill="freeze"/>
  </rect>
</svg>"##,
    )
    .unwrap();
    assert_eq!(
        compiled.animation().compiler_id(),
        n0_model::svg_animation::LATEST_COMPILER_ID
    );

    let moving = named(&compiled, "moving");
    let context = PaintCtx::new(None);
    let mut surface = surfaces::raster_n32_premul((WIDTH, HEIGHT)).unwrap();
    surface.canvas().clear(Color::TRANSPARENT);

    let product = svg_animation_frame::render_sample(
        surface.canvas(),
        &compiled,
        SampleTime::from_nanoseconds(1_000_000_000),
        &context,
    )
    .unwrap();

    assert_ne!(product.query().hit_point(20.0, 8.0), None);
    assert_eq!(product.query().hit_point(20.0, 8.0), Some(moving));
    assert_ne!(product.query().hit_point(4.0, 8.0), Some(moving));
    let bytes = read_pixels(&mut surface, WIDTH, HEIGHT);
    assert_eq!(pixel(&bytes, 4, 8), &[0, 0, 0, 0]);
    assert_ne!(pixel(&bytes, 20, 8), &[0, 0, 0, 0]);
}

#[test]
fn base_entry_renders_a_static_shell_without_a_sample_time() {
    let compiled = svg_animation_frame::compile_latest(
        "static-base.svg",
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="32" height="16">
  <rect id="static" x="8" y="4" width="8" height="8" fill="#00ff00"/>
</svg>"##,
    )
    .unwrap();
    let static_rect = named(&compiled, "static");
    let context = PaintCtx::new(None);
    let mut surface = surfaces::raster_n32_premul((WIDTH, HEIGHT)).unwrap();
    surface.canvas().clear(Color::TRANSPARENT);

    let product = svg_animation_frame::render_base(surface.canvas(), &compiled, &context).unwrap();

    assert_eq!(product.query().hit_point(12.0, 8.0), Some(static_rect));
    let bytes = read_pixels(&mut surface, WIDTH, HEIGHT);
    assert_ne!(pixel(&bytes, 12, 8), &[0, 0, 0, 0]);
}

#[test]
fn base_and_sample_at_zero_remain_distinct_requests() {
    let compiled = svg_animation_frame::compile_latest(
        "base-vs-sample.svg",
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="32" height="16">
  <rect id="moving" x="4" y="4" width="8" height="8" fill="#ff0000">
    <animate attributeName="x" from="20" to="0" dur="1s" fill="freeze"/>
  </rect>
</svg>"##,
    )
    .unwrap();
    let moving = named(&compiled, "moving");
    let context = PaintCtx::new(None);
    let mut base_surface = surfaces::raster_n32_premul((WIDTH, HEIGHT)).unwrap();
    let mut sample_surface = surfaces::raster_n32_premul((WIDTH, HEIGHT)).unwrap();
    base_surface.canvas().clear(Color::TRANSPARENT);
    sample_surface.canvas().clear(Color::TRANSPARENT);

    let base =
        svg_animation_frame::render_base(base_surface.canvas(), &compiled, &context).unwrap();
    let sample = svg_animation_frame::render_sample(
        sample_surface.canvas(),
        &compiled,
        SampleTime::ZERO,
        &context,
    )
    .unwrap();

    assert_eq!(base.query().hit_point(8.0, 8.0), Some(moving));
    assert_ne!(sample.query().hit_point(8.0, 8.0), Some(moving));
    assert_ne!(base.query().hit_point(24.0, 8.0), Some(moving));
    assert_eq!(sample.query().hit_point(24.0, 8.0), Some(moving));
    assert_ne!(
        read_pixels(&mut base_surface, WIDTH, HEIGHT),
        read_pixels(&mut sample_surface, WIDTH, HEIGHT)
    );
}

#[test]
fn unsupported_static_svg_is_rejected_before_a_frame_contract_exists() {
    let error = svg_animation_frame::compile_latest(
        "unsupported.svg",
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="32" height="16">
  <circle cx="8" cy="8" r="4"/>
</svg>"##,
    )
    .unwrap_err();

    assert_eq!(error.source_identity(), "unsupported.svg");
    assert!(
        error
            .message
            .contains("supports only direct <rect> or <path> children"),
        "{error}"
    );
}
