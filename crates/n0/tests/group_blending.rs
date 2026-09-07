//! Independent hand-built rframe consumer probes, without Web lowering.
//! Interior RGBA-premultiplied pixels use exact, integer-valued source-over
//! blend equations, with explicitly named native layer quantization below.
//! Reuse checks are equivalence laws, not Chromium reftests;
//! Chromium's layer quantization is independently probed by the Web rung.

use std::sync::Arc;

use cg::CGColor;
use math2::{transform::AffineTransform, Rectangle};
use n0::glyphless::{compile, diff_frame, BuildError, FrameProduct};
use n0::paint::{read_pixels, PaintCtx};
use rframe::{
    ClipGeometry, ClipLayer, ClipPath, Frame, FrameItem, FrameItems, FrameNode, Geometry, Identity,
    Mask, MaskMode, PaintStack, PatternPaint, Provenance, Scope, ScopeBlend, ScopeBlendMode,
    ScopeEffect, ScopeOpacity, Stroke, StrokeCap, StrokeJoin, VisualRef,
};

const SIZE: i32 = 48;
const MODES: [ScopeBlendMode; 3] = [
    ScopeBlendMode::Normal,
    ScopeBlendMode::Multiply,
    ScopeBlendMode::Screen,
];
const BACKDROP: CGColor = CGColor::from_rgb(51, 102, 153);
const FIRST: CGColor = CGColor::from_rgb(85, 170, 255);
const SECOND: CGColor = CGColor::from_rgb(170, 85, 0);

fn owner(id: u64) -> VisualRef {
    VisualRef::new(Identity::new(id), Provenance::new(id + 1000))
}

fn rect(x: f32, y: f32, w: f32, h: f32) -> Rectangle {
    Rectangle::from_xywh(x, y, w, h)
}

fn node(id: u64, bounds: Rectangle, paints: PaintStack) -> FrameNode {
    FrameNode {
        owner: owner(id),
        transform: AffineTransform::identity(),
        geometry: Geometry::Rect(bounds),
        bounds,
        paints,
        stroke: None,
    }
}

fn solid(id: u64, bounds: Rectangle, color: CGColor) -> FrameItem {
    FrameItem::Node(node(id, bounds, PaintStack::solid(color)))
}

fn begin(id: u64, effect: ScopeEffect) -> FrameItem {
    FrameItem::ScopeBegin(Scope {
        owner: owner(id),
        effect,
    })
}

fn blend(id: u64, mode: ScopeBlendMode, opacity: Option<f32>) -> FrameItem {
    begin(
        id,
        ScopeEffect::Blend(ScopeBlend::new(
            mode,
            opacity.map(|opacity| ScopeOpacity::new(opacity).unwrap()),
        )),
    )
}

fn frame(items: Vec<FrameItem>) -> Frame {
    Frame {
        owner: owner(900),
        bounds: rect(0.0, 0.0, SIZE as f32, SIZE as f32),
        items: FrameItems::try_new(items).unwrap(),
    }
}

fn raster(product: &FrameProduct, backdrop: CGColor) -> Vec<u8> {
    let mut surface = skia_safe::surfaces::raster_n32_premul((SIZE, SIZE)).unwrap();
    surface.canvas().clear(skia_safe::Color::from_argb(
        backdrop.a, backdrop.r, backdrop.g, backdrop.b,
    ));
    let saves = surface.canvas().save_count();
    product
        .execute(
            surface.canvas(),
            &AffineTransform::identity(),
            &PaintCtx::new(None),
        )
        .unwrap();
    assert_eq!(
        surface.canvas().save_count(),
        saves,
        "all group scopes restore"
    );
    read_pixels(&mut surface, SIZE, SIZE)
}

fn at(pixels: &[u8], x: usize, y: usize) -> [u8; 4] {
    let offset = (y * SIZE as usize + x) * 4;
    pixels[offset..offset + 4].try_into().unwrap()
}

#[cfg(feature = "trace")]
mod layer_metrics {
    use super::*;
    use n0::trace::{sink::drain_blend_layers, BlendLayerMetrics};

    fn single() -> FrameProduct {
        compile(frame(vec![
            blend(10, ScopeBlendMode::Multiply, Some(0.5)),
            solid(1, rect(8.0, 8.0, 24.0, 24.0), FIRST),
            FrameItem::ScopeEnd,
        ]))
        .unwrap()
    }

    #[test]
    fn combined_blend_and_opacity_observe_one_real_raster_layer() {
        drain_blend_layers();
        let pixels = raster(&single(), BACKDROP);
        let metrics = drain_blend_layers();
        assert_eq!(
            metrics,
            [BlendLayerMetrics {
                save_layer_calls: 1,
                observed_raster_layers: 1,
                observed_raster_bytes: (SIZE * SIZE * 4) as u128,
                observed_raster_pixels: (SIZE * SIZE) as u128,
                peak_live_blend_bytes: (SIZE * SIZE * 4) as u128,
                ..BlendLayerMetrics::default()
            }]
        );
        assert_eq!(at(&pixels, 0, 0), [51, 102, 153, 255]);
        assert!(drain_blend_layers().is_empty());
    }

    #[test]
    fn byte_255_opacity_is_observed_as_one_blend_operation_without_erasing_its_layer() {
        for (value, count) in [(0.998, 0), (0.999, 1), (1.0_f32.next_down(), 1)] {
            drain_blend_layers();
            let product = compile(frame(vec![
                begin(10, ScopeEffect::Opacity(ScopeOpacity::new(value).unwrap())),
                solid(1, rect(8.0, 8.0, 24.0, 24.0), FIRST),
                FrameItem::ScopeEnd,
            ]))
            .unwrap();
            assert!(
                drain_blend_layers().is_empty(),
                "build issues no raster commands"
            );
            raster(&product, BACKDROP);
            let metrics = drain_blend_layers();
            assert_eq!(metrics.len(), 1);
            assert_eq!(metrics[0].save_layer_calls, count, "opacity {value}");
            assert_eq!(metrics[0].observed_raster_layers, count);
            assert_eq!(
                metrics[0].observed_raster_bytes,
                u128::from(count) * (SIZE * SIZE * 4) as u128
            );
            assert_eq!(metrics[0].missing_observations, 0);
            // Byte 254 still has a native opacity layer; it is deliberately
            // outside these execute-seam blend-operation counters.
        }
    }

    #[test]
    fn nested_and_sequential_layers_have_distinct_live_peaks() {
        let scene = |nested| {
            let first = solid(1, rect(8.0, 8.0, 24.0, 24.0), FIRST);
            let second = solid(2, rect(16.0, 16.0, 24.0, 24.0), SECOND);
            compile(frame(if nested {
                vec![
                    blend(10, ScopeBlendMode::Normal, None),
                    first,
                    blend(11, ScopeBlendMode::Multiply, None),
                    second,
                    FrameItem::ScopeEnd,
                    FrameItem::ScopeEnd,
                ]
            } else {
                vec![
                    blend(10, ScopeBlendMode::Normal, None),
                    first,
                    FrameItem::ScopeEnd,
                    blend(11, ScopeBlendMode::Multiply, None),
                    second,
                    FrameItem::ScopeEnd,
                ]
            }))
            .unwrap()
        };
        for nested in [false, true] {
            drain_blend_layers();
            raster(&scene(nested), BACKDROP);
            let metrics = drain_blend_layers();
            assert_eq!(metrics.len(), 1);
            assert_eq!(metrics[0].save_layer_calls, 2);
            assert_eq!(metrics[0].observed_raster_layers, 2);
            let bytes = (SIZE * SIZE * 4) as u128;
            assert_eq!(metrics[0].observed_raster_bytes, 2 * bytes);
            assert_eq!(
                metrics[0].peak_live_blend_bytes,
                if nested { 2 * bytes } else { bytes }
            );
        }
    }

    #[test]
    fn an_empty_clip_never_counts_the_parent_surface_as_a_blend_layer() {
        drain_blend_layers();
        let mut surface = skia_safe::surfaces::raster_n32_premul((SIZE, SIZE)).unwrap();
        surface
            .canvas()
            .clip_rect(skia_safe::Rect::new_empty(), None, false);
        single()
            .execute(
                surface.canvas(),
                &AffineTransform::identity(),
                &PaintCtx::new(None),
            )
            .unwrap();
        assert_eq!(
            drain_blend_layers(),
            [BlendLayerMetrics {
                save_layer_calls: 1,
                empty_clip_saves: 1,
                ..BlendLayerMetrics::default()
            }]
        );
    }

    #[test]
    fn observed_storage_follows_the_real_clipped_layer_not_the_frame_bounds() {
        drain_blend_layers();
        let mut surface = skia_safe::surfaces::raster_n32_premul((SIZE, SIZE)).unwrap();
        surface.canvas().clip_rect(
            skia_safe::Rect::from_xywh(3.0, 5.0, 12.0, 10.0),
            None,
            false,
        );
        single()
            .execute(
                surface.canvas(),
                &AffineTransform::identity(),
                &PaintCtx::new(None),
            )
            .unwrap();
        assert_eq!(
            drain_blend_layers(),
            [BlendLayerMetrics {
                save_layer_calls: 1,
                observed_raster_layers: 1,
                observed_raster_bytes: 12 * 10 * 4,
                observed_raster_pixels: 12 * 10,
                peak_live_blend_bytes: 12 * 10 * 4,
                ..BlendLayerMetrics::default()
            }]
        );
    }

    #[test]
    fn a_recording_canvas_reports_missing_storage_without_inventing_bytes() {
        drain_blend_layers();
        let mut recorder = skia_safe::PictureRecorder::new();
        let canvas =
            recorder.begin_recording(skia_safe::Rect::from_wh(SIZE as f32, SIZE as f32), false);
        single()
            .execute(canvas, &AffineTransform::identity(), &PaintCtx::new(None))
            .unwrap();
        assert_eq!(
            drain_blend_layers(),
            [BlendLayerMetrics {
                save_layer_calls: 1,
                missing_observations: 1,
                ..BlendLayerMetrics::default()
            }]
        );
        assert!(recorder.finish_recording_as_picture(None).is_some());
    }

    #[test]
    fn nested_resource_recording_contributes_missing_observations_to_execute() {
        let tile = FrameItems::try_new(vec![
            blend(11, ScopeBlendMode::Screen, None),
            solid(1, rect(0.0, 0.0, 16.0, 16.0), FIRST),
            FrameItem::ScopeEnd,
        ])
        .unwrap();
        let pattern =
            PatternPaint::new(16.0, 16.0, AffineTransform::identity(), Arc::new(tile), 1.0)
                .unwrap();
        let product = compile(frame(vec![
            blend(10, ScopeBlendMode::Normal, None),
            FrameItem::Node(node(
                20,
                rect(0.0, 0.0, 48.0, 48.0),
                PaintStack::from_pattern(pattern),
            )),
            FrameItem::ScopeEnd,
        ]))
        .unwrap();
        // Compiling/preflighting a repeating program can itself record a
        // picture. Discard that completed execute observation before replay.
        drain_blend_layers();
        let pixels = raster(&product, BACKDROP);
        assert_eq!(at(&pixels, 8, 8), [85, 170, 255, 255]);
        let mut metrics = drain_blend_layers();
        // FrameProduct::execute preflights the pattern before its outer
        // drawlist replay; that recording is a separate completed execution.
        assert_eq!(
            metrics.remove(0),
            BlendLayerMetrics {
                save_layer_calls: 1,
                missing_observations: 1,
                ..BlendLayerMetrics::default()
            }
        );
        assert_eq!(
            metrics,
            [BlendLayerMetrics {
                save_layer_calls: 2,
                observed_raster_layers: 1,
                observed_raster_bytes: (SIZE * SIZE * 4) as u128,
                observed_raster_pixels: (SIZE * SIZE) as u128,
                peak_live_blend_bytes: (SIZE * SIZE * 4) as u128,
                missing_observations: 1,
                ..BlendLayerMetrics::default()
            }]
        );
    }

    #[test]
    fn instrumented_execute_matches_equivalent_operations_without_observation() {
        // Instrumentation equivalence, not an independent Chromium oracle.
        // The control independently spells the ordered byte operations for
        // Multiply/partial Screen; native lowp restoration is not portable.
        // Partial Normal retains native opacity. No control operation calls
        // access_top_layer_pixels or a production blend helper.
        drain_blend_layers();
        let source = CGColor::from_rgba(205, 104, 67, 153);
        let bounds = rect(8.25, 8.125, 24.5, 24.75);
        for mode in MODES {
            let product = compile(frame(vec![
                blend(10, mode, Some(0.5)),
                solid(1, bounds, source),
                FrameItem::ScopeEnd,
            ]))
            .unwrap();
            let instrumented = raster(&product, BACKDROP);
            assert_eq!(drain_blend_layers().len(), 1, "{mode:?} execute observed");
            let mut control = skia_safe::surfaces::raster_n32_premul((SIZE, SIZE)).unwrap();
            let canvas = control.canvas();
            canvas.clear(skia_safe::Color::from_rgb(
                BACKDROP.r, BACKDROP.g, BACKDROP.b,
            ));
            let mut restore = skia_safe::Paint::default();
            match mode {
                ScopeBlendMode::Normal => {
                    restore.set_alpha_f(0.5);
                    restore.set_blend_mode(skia_safe::BlendMode::SrcOver);
                }
                ScopeBlendMode::Multiply | ScopeBlendMode::Screen => {
                    let expression = match mode {
                        ScopeBlendMode::Multiply => {
                            "q(s * (255.0 - d.a) + d * (255.0 - s.a) + s * d)"
                        }
                        ScopeBlendMode::Screen => "s + d - q(s * d)",
                        ScopeBlendMode::Normal => unreachable!(),
                    };
                    let shader = format!(
                        r#"
uniform float opacity_byte;
float4 q(float4 value) {{
    return floor((value + 127.0) / 255.0);
}}
half4 main(half4 src, half4 dst) {{
    float4 s = floor(float4(src) * 255.0 + 0.5);
    float4 d = floor(float4(dst) * 255.0 + 0.5);
    s = q(s * opacity_byte);
    float4 result = {expression};
    return half4(clamp(result, 0.0, 255.0) / 255.0);
}}
"#
                    );
                    let effect = skia_safe::RuntimeEffect::make_for_blender(shader, None)
                        .expect("test control byte blender compiles");
                    // round(0.5 * 255) = 128. Scale inside the blender once;
                    // restore paint alpha remains one, avoiding float-first scaling.
                    restore.set_blender(
                        effect
                            .make_blender(skia_safe::Data::new_copy(&128.0_f32.to_ne_bytes()), None)
                            .expect("test control opacity binding is valid"),
                    );
                }
            }
            canvas.save_layer(&skia_safe::canvas::SaveLayerRec::default().paint(&restore));
            let mut paint = skia_safe::Paint::default();
            paint.set_anti_alias(true);
            paint.set_color(skia_safe::Color::from_argb(
                source.a, source.r, source.g, source.b,
            ));
            canvas.draw_rect(
                skia_safe::Rect::from_xywh(bounds.x, bounds.y, bounds.width, bounds.height),
                &paint,
            );
            canvas.restore();
            assert_eq!(
                instrumented,
                read_pixels(&mut control, SIZE, SIZE),
                "{mode:?}"
            );
            assert!(
                drain_blend_layers().is_empty(),
                "{mode:?} control unobserved"
            );
        }
    }
}

#[test]
fn opaque_group_blends_once_after_overlapping_children_complete() {
    // b*s and b+s-b*s are integer code values for this palette. Probes:
    // (8,8) first only, (20,20) overlap, (36,36) second only, (2,2) untouched.
    let first = [
        [85, 170, 255, 255],
        [17, 68, 153, 255],
        [119, 204, 255, 255],
    ];
    let second = [[170, 85, 0, 255], [34, 34, 0, 255], [187, 153, 153, 255]];
    for (index, mode) in MODES.into_iter().enumerate() {
        let product = compile(frame(vec![
            blend(10, mode, None),
            solid(1, rect(4.0, 4.0, 24.0, 24.0), FIRST),
            solid(2, rect(16.0, 16.0, 24.0, 24.0), SECOND),
            FrameItem::ScopeEnd,
        ]))
        .unwrap();
        let pixels = raster(&product, BACKDROP);
        assert_eq!(at(&pixels, 8, 8), first[index], "{mode:?} first");
        assert_eq!(at(&pixels, 20, 20), second[index], "{mode:?} overlap");
        assert_eq!(at(&pixels, 36, 36), second[index], "{mode:?} second");
        assert_eq!(at(&pixels, 2, 2), [51, 102, 153, 255]);
    }
}

#[test]
fn fill_and_stroke_complete_before_the_group_blends() {
    // An opaque centred stroke overlaps the fill at (10,20). Applying the
    // blend independently to fill and stroke would blend against the fill.
    for (mode, expected) in [
        (ScopeBlendMode::Multiply, [34, 34, 0, 255]),
        (ScopeBlendMode::Screen, [187, 153, 153, 255]),
    ] {
        let mut shape = node(1, rect(8.0, 8.0, 24.0, 24.0), PaintStack::solid(FIRST));
        shape.stroke = Stroke::new(
            PaintStack::solid(SECOND),
            8.0,
            StrokeCap::Butt,
            StrokeJoin::Round,
            4.0,
        )
        .unwrap();
        let product = compile(frame(vec![
            blend(10, mode, None),
            FrameItem::Node(shape),
            FrameItem::ScopeEnd,
        ]))
        .unwrap();
        let pixels = raster(&product, BACKDROP);
        assert_eq!(
            at(&pixels, 10, 20),
            expected,
            "fill/stroke overlap {mode:?}"
        );
        assert_eq!(at(&pixels, 6, 20), expected, "stroke-only {mode:?}");
    }
}

#[test]
fn partially_transparent_children_finish_source_over_before_group_blending() {
    // Red alpha 153 followed by green alpha 170 yields premul [51,170,0,221].
    // Over opaque blue, multiply keeps only the uncovered blue (255-221);
    // screen keeps the red/green source channels and all of the blue backdrop.
    for (mode, expected) in [
        (ScopeBlendMode::Multiply, [0, 0, 34, 255]),
        (ScopeBlendMode::Screen, [51, 170, 255, 255]),
    ] {
        let product = compile(frame(vec![
            blend(10, mode, None),
            solid(
                1,
                rect(4.0, 4.0, 24.0, 24.0),
                CGColor::from_rgba(255, 0, 0, 153),
            ),
            solid(
                2,
                rect(16.0, 16.0, 24.0, 24.0),
                CGColor::from_rgba(0, 255, 0, 170),
            ),
            FrameItem::ScopeEnd,
        ]))
        .unwrap();
        assert_eq!(
            at(&raster(&product, CGColor::BLUE), 20, 20),
            expected,
            "{mode:?}"
        );
    }
}

#[test]
fn partial_source_and_backdrop_use_source_over_alpha_for_every_mode() {
    // as=153/255, ab=170/255: as*ab=102/255, ao=221/255.
    // Green backdrop, red source make each blend channel exactly calculable.
    let backdrop = CGColor::from_rgba(0, 255, 0, 170);
    for (mode, expected) in [
        (ScopeBlendMode::Normal, [153, 68, 0, 221]),
        (ScopeBlendMode::Multiply, [51, 68, 0, 221]),
        (ScopeBlendMode::Screen, [153, 170, 0, 221]),
    ] {
        let product = compile(frame(vec![
            blend(10, mode, None),
            solid(
                1,
                rect(8.0, 8.0, 24.0, 24.0),
                CGColor::from_rgba(255, 0, 0, 153),
            ),
            FrameItem::ScopeEnd,
        ]))
        .unwrap();
        assert_eq!(
            at(&raster(&product, backdrop), 16, 16),
            expected,
            "{mode:?}"
        );
        assert_eq!(
            at(&raster(&product, CGColor::TRANSPARENT), 16, 16),
            [153, 0, 0, 153],
            "transparent backdrop {mode:?}"
        );
    }
}

#[test]
fn transparent_group_leaves_a_partial_backdrop_unchanged() {
    for mode in MODES {
        let product = compile(frame(vec![
            blend(10, mode, Some(0.6)),
            solid(1, rect(8.0, 8.0, 24.0, 24.0), CGColor::TRANSPARENT),
            FrameItem::ScopeEnd,
        ]))
        .unwrap();
        let pixels = raster(&product, CGColor::from_rgba(0, 255, 0, 170));
        assert_eq!(at(&pixels, 16, 16), [0, 170, 0, 170], "{mode:?}");
    }
}

#[test]
fn group_opacity_attenuates_the_completed_source_once() {
    // Two opaque overlapping red children become one opaque red source.
    // Final opacity 0.6 produces as=153/255. The pinned native Normal layer
    // restore gives green=67, one below the exact green=68 from an alpha-153
    // paint above. This is a native quantization regression assertion, not a
    // claim of Chromium parity and not a tolerance over that difference.
    for (mode, expected) in [
        (ScopeBlendMode::Normal, [153, 67, 0, 221]),
        (ScopeBlendMode::Multiply, [51, 68, 0, 221]),
        (ScopeBlendMode::Screen, [153, 170, 0, 221]),
    ] {
        let product = compile(frame(vec![
            blend(10, mode, Some(0.6)),
            solid(1, rect(4.0, 4.0, 24.0, 24.0), CGColor::RED),
            solid(2, rect(16.0, 16.0, 24.0, 24.0), CGColor::RED),
            FrameItem::ScopeEnd,
        ]))
        .unwrap();
        let pixels = raster(&product, CGColor::from_rgba(0, 255, 0, 170));
        for (x, y) in [(8, 8), (20, 20), (36, 36)] {
            assert_eq!(at(&pixels, x, y), expected, "{mode:?} at ({x},{y})");
        }
    }
}

#[test]
fn neutral_outer_group_isolates_a_nested_blend_from_the_external_backdrop() {
    for mode in [ScopeBlendMode::Multiply, ScopeBlendMode::Screen] {
        let child = vec![
            blend(11, mode, None),
            solid(1, rect(8.0, 8.0, 24.0, 24.0), FIRST),
            FrameItem::ScopeEnd,
        ];
        let direct = compile(frame(child.clone())).unwrap();
        let mut isolated = vec![blend(10, ScopeBlendMode::Normal, None)];
        isolated.extend(child);
        isolated.push(FrameItem::ScopeEnd);
        let isolated = compile(frame(isolated)).unwrap();
        let pixels = raster(&isolated, BACKDROP);
        assert_eq!(
            at(&pixels, 16, 16),
            [85, 170, 255, 255],
            "{mode:?} sees transparent"
        );
        assert_ne!(
            pixels,
            raster(&direct, BACKDROP),
            "unit normal must not be erased"
        );
    }
}

#[test]
fn combined_opacity_and_blend_differ_from_an_outer_opacity_scope() {
    for (mode, combined_expected) in [
        (ScopeBlendMode::Multiply, [51, 68, 0, 221]),
        (ScopeBlendMode::Screen, [153, 170, 0, 221]),
    ] {
        let shape = solid(1, rect(8.0, 8.0, 24.0, 24.0), CGColor::RED);
        let combined = compile(frame(vec![
            blend(10, mode, Some(0.6)),
            shape.clone(),
            FrameItem::ScopeEnd,
        ]))
        .unwrap();
        let nested = compile(frame(vec![
            begin(12, ScopeEffect::Opacity(ScopeOpacity::new(0.6).unwrap())),
            blend(10, mode, None),
            shape,
            FrameItem::ScopeEnd,
            FrameItem::ScopeEnd,
        ]))
        .unwrap();
        let backdrop = CGColor::from_rgba(0, 255, 0, 170);
        assert_eq!(at(&raster(&combined, backdrop), 16, 16), combined_expected);
        assert_eq!(
            at(&raster(&nested, backdrop), 16, 16),
            [153, 67, 0, 221],
            "nested blend sees transparent black, then normal opacity sees green"
        );
    }
}

#[test]
fn normal_blend_with_opacity_matches_existing_isolated_opacity() {
    let children = vec![
        solid(1, rect(4.0, 4.0, 24.0, 24.0), FIRST),
        solid(2, rect(16.0, 16.0, 24.0, 24.0), SECOND),
    ];
    let values = (0..=255_u32)
        .map(|alpha| match alpha {
            0 => 0.001,
            255 => 1.0_f32.next_down(),
            _ => alpha as f32 / 255.0,
        })
        .chain([0.125, 0.375, 0.5, 0.6, 0.998, 0.999]);
    for opacity in values {
        let scene = |effect| {
            let mut items = vec![begin(10, effect)];
            items.extend(children.clone());
            items.push(FrameItem::ScopeEnd);
            compile(frame(items)).unwrap()
        };
        let opacity = ScopeOpacity::new(opacity).unwrap();
        let old = scene(ScopeEffect::Opacity(opacity));
        let new = scene(ScopeEffect::Blend(ScopeBlend::new(
            ScopeBlendMode::Normal,
            Some(opacity),
        )));
        for backdrop in [
            BACKDROP,
            CGColor::TRANSPARENT,
            CGColor::from_rgba(0, 255, 0, 170),
        ] {
            let pixels = raster(&old, backdrop);
            assert_eq!(pixels, raster(&new, backdrop), "opacity {opacity:?}");
            let mut restore = skia_safe::Paint::default();
            restore.set_alpha_f(opacity.get());
            if restore.alpha() < 255 {
                // Lower byte factors must remain exactly the old native
                // saveLayer operation, not the ordered byte-blender formula.
                let mut surface = skia_safe::surfaces::raster_n32_premul((SIZE, SIZE)).unwrap();
                surface.canvas().clear(skia_safe::Color::from_argb(
                    backdrop.a, backdrop.r, backdrop.g, backdrop.b,
                ));
                surface
                    .canvas()
                    .save_layer(&skia_safe::canvas::SaveLayerRec::default().paint(&restore));
                for (x, y, color) in [(4.0, 4.0, FIRST), (16.0, 16.0, SECOND)] {
                    let mut paint = skia_safe::Paint::default();
                    paint.set_anti_alias(true);
                    paint.set_color(skia_safe::Color::from_argb(
                        color.a, color.r, color.g, color.b,
                    ));
                    surface
                        .canvas()
                        .draw_rect(skia_safe::Rect::from_xywh(x, y, 24.0, 24.0), &paint);
                }
                surface.canvas().restore();
                assert_eq!(
                    pixels,
                    read_pixels(&mut surface, SIZE, SIZE),
                    "native opacity {opacity:?}"
                );
            }
        }
    }
}

#[test]
fn near_unit_normal_spellings_share_exact_partial_alpha_restore_and_retain_the_frame() {
    // Both alphas are partial. Unlike an opaque axis-aligned child, this
    // palette distinguishes exact /255 from x86's native unit sprite /256.
    let source_color = CGColor::from_rgba(215, 104, 67, 8);
    let backdrop = CGColor::from_rgba(66, 101, 137, 170);
    let source = [7_u32, 3, 2, 8];
    let destination = [44_u32, 67, 91, 170];
    let expected: [u8; 4] = std::array::from_fn(|i| {
        (source[i] + (destination[i] * (255 - source[3]) + 127) / 255) as u8
    });
    let native_x86: [u8; 4] =
        std::array::from_fn(|i| (source[i] + destination[i] * (256 - source[3]) / 256) as u8);
    assert_ne!(
        expected, native_x86,
        "the witness must discriminate the rounding routes"
    );
    let scene = |effect| {
        frame(vec![
            begin(10, effect),
            solid(1, rect(8.0, 8.0, 24.0, 24.0), source_color),
            FrameItem::ScopeEnd,
        ])
    };
    let unit = compile(scene(ScopeEffect::Blend(ScopeBlend::new(
        ScopeBlendMode::Normal,
        None,
    ))))
    .unwrap();
    let unit_pixels = raster(&unit, backdrop);
    assert_eq!(at(&unit_pixels, 16, 16), expected);
    for value in [0.999, 1.0_f32.next_down()] {
        let opacity = ScopeOpacity::new(value).unwrap();
        for effect in [
            ScopeEffect::Opacity(opacity),
            ScopeEffect::Blend(ScopeBlend::new(ScopeBlendMode::Normal, Some(opacity))),
        ] {
            let resolved = scene(effect);
            let product = compile(resolved.clone()).unwrap();
            assert_eq!(product.resolved(), &resolved);
            let retained = product.clone();
            let pixels = raster(&retained, backdrop);
            assert_eq!(at(&pixels, 16, 16), expected, "opacity {value}");
            assert_eq!(
                pixels, unit_pixels,
                "same retained isolation topology at {value}"
            );
            assert_eq!(pixels, raster(&compile(resolved).unwrap(), backdrop));
        }
    }
}

fn clip(bounds: Rectangle) -> ClipPath {
    ClipPath::new(vec![ClipLayer::new(vec![ClipGeometry::new(
        AffineTransform::identity(),
        Geometry::Rect(bounds),
    )
    .unwrap()])
    .unwrap()])
    .unwrap()
}

#[test]
fn changing_blend_or_opacity_damages_only_the_scope_and_its_clipped_child_union() {
    let scene = |mode, opacity| {
        compile(frame(vec![
            blend(10, mode, opacity),
            solid(1, rect(4.0, 8.0, 12.0, 12.0), FIRST),
            begin(11, ScopeEffect::Clip(clip(rect(16.0, 12.0, 8.0, 8.0)))),
            solid(2, rect(0.0, 0.0, 48.0, 48.0), SECOND),
            FrameItem::ScopeEnd,
            solid(3, rect(60.0, 60.0, 12.0, 12.0), FIRST),
            FrameItem::ScopeEnd,
        ]))
        .unwrap()
    };
    let before = scene(ScopeBlendMode::Multiply, None);
    assert!(diff_frame(&before, &scene(ScopeBlendMode::Multiply, None)).is_empty());
    for after in [
        scene(ScopeBlendMode::Screen, None),
        scene(ScopeBlendMode::Multiply, Some(0.6)),
    ] {
        let damage = diff_frame(&before, &after);
        assert_eq!(damage.changed, [owner(10)]);
        assert_eq!(damage.union_frame, Some(rect(4.0, 8.0, 20.0, 12.0)));
    }
}

#[test]
fn opacity_byte_254_to_255_keeps_scope_damage_coverage_and_retained_matches_fresh() {
    for blend_spelling in [false, true] {
        let scene = |value| {
            let opacity = ScopeOpacity::new(value).unwrap();
            let effect = if blend_spelling {
                ScopeEffect::Blend(ScopeBlend::new(ScopeBlendMode::Normal, Some(opacity)))
            } else {
                ScopeEffect::Opacity(opacity)
            };
            frame(vec![
                begin(10, effect),
                solid(1, rect(4.0, 8.0, 12.0, 12.0), FIRST),
                begin(11, ScopeEffect::Clip(clip(rect(16.0, 12.0, 8.0, 8.0)))),
                solid(2, rect(0.0, 0.0, 48.0, 48.0), SECOND),
                FrameItem::ScopeEnd,
                FrameItem::ScopeEnd,
            ])
        };
        for (old, new) in [(0.998, 0.999), (0.999, 0.998)] {
            let before = compile(scene(old)).unwrap();
            let resolved = scene(new);
            let retained = compile(resolved.clone()).unwrap().clone();
            assert_eq!(retained.resolved(), &resolved);
            let fresh = compile(scene(new)).unwrap();
            assert!(diff_frame(&retained, &fresh).is_empty());
            let damage = diff_frame(&before, &retained);
            assert_eq!(damage.changed, [owner(10)]);
            assert_eq!(damage.union_frame, Some(rect(4.0, 8.0, 20.0, 12.0)));
            let before_pixels = raster(&before, BACKDROP);
            let after_pixels = raster(&retained, BACKDROP);
            assert_eq!(after_pixels, raster(&fresh, BACKDROP));
            let union = damage.union_frame.unwrap();
            let mut changed_pixels = 0;
            for y in 0..SIZE as usize {
                for x in 0..SIZE as usize {
                    if at(&before_pixels, x, y) != at(&after_pixels, x, y) {
                        changed_pixels += 1;
                        assert!(
                            x as f32 >= union.x
                                && y as f32 >= union.y
                                && (x + 1) as f32 <= union.x + union.width
                                && (y + 1) as f32 <= union.y + union.height,
                            "{old} -> {new}: changed pixel ({x},{y}) escaped {union:?}"
                        );
                    }
                }
            }
            assert!(changed_pixels > 0, "byte-route crossing must change pixels");
        }
        // Distinct resolved floats in byte 255 remain distinct damage facts,
        // even though their restoration pixels alias on this backend.
        let near = compile(scene(1.0_f32.next_down())).unwrap();
        let alias = compile(scene(0.999)).unwrap();
        assert_eq!(diff_frame(&near, &alias).changed, [owner(10)]);
        assert_eq!(raster(&near, BACKDROP), raster(&alias, BACKDROP));
    }
}

#[test]
fn fully_clipped_blend_edit_keeps_owner_but_has_no_coverage() {
    let scene = |mode| {
        compile(frame(vec![
            blend(10, mode, None),
            solid(1, rect(60.0, 60.0, 12.0, 12.0), FIRST),
            FrameItem::ScopeEnd,
        ]))
        .unwrap()
    };
    let damage = diff_frame(
        &scene(ScopeBlendMode::Multiply),
        &scene(ScopeBlendMode::Screen),
    );
    assert_eq!(damage.changed, [owner(10)]);
    assert_eq!(damage.union_frame, None);
}

#[test]
fn blend_scope_coverage_includes_its_childs_stroke_outset() {
    let scene = |mode| {
        let mut shape = node(1, rect(8.0, 8.0, 24.0, 24.0), PaintStack::solid(FIRST));
        shape.stroke = Stroke::new(
            PaintStack::solid(SECOND),
            8.0,
            StrokeCap::Butt,
            StrokeJoin::Round,
            4.0,
        )
        .unwrap();
        compile(frame(vec![
            blend(10, mode, None),
            FrameItem::Node(shape),
            FrameItem::ScopeEnd,
        ]))
        .unwrap()
    };
    let damage = diff_frame(
        &scene(ScopeBlendMode::Multiply),
        &scene(ScopeBlendMode::Screen),
    );
    assert_eq!(damage.changed, [owner(10)]);
    // Stroke coverage deliberately rounds outwards. Its law is containment,
    // not equality to the unrounded mathematical stroke box (4,4)-(36,36).
    let coverage = damage.union_frame.unwrap();
    assert!(coverage.x <= 4.0 && coverage.y <= 4.0);
    assert!(coverage.x + coverage.width >= 36.0);
    assert!(coverage.y + coverage.height >= 36.0);
    assert!(coverage.x >= 0.0 && coverage.y >= 0.0);
    assert!(coverage.x + coverage.width <= SIZE as f32);
    assert!(coverage.y + coverage.height <= SIZE as f32);
}

#[test]
fn a_blend_owner_cannot_alias_a_child_owner() {
    assert!(
        matches!(compile(frame(vec![blend(1, ScopeBlendMode::Normal, None),
        solid(1, rect(8.0, 8.0, 24.0, 24.0), FIRST), FrameItem::ScopeEnd])),
        Err(BuildError::DuplicateOwner(value)) if value == owner(1))
    );
}

#[test]
fn changed_earlier_backdrop_with_retained_group_matches_fresh() {
    // Glyphless reuse is immutable compiled replay; there is no raster cache.
    // Reuse the group under a changed host backdrop, then compare a complete
    // fresh frame. An unchanged group's previous output is NOT reusable.
    for mode in [ScopeBlendMode::Multiply, ScopeBlendMode::Screen] {
        let children = vec![
            blend(10, mode, Some(0.6)),
            solid(1, rect(8.0, 8.0, 24.0, 24.0), FIRST),
            FrameItem::ScopeEnd,
        ];
        let retained = compile(frame(children.clone())).unwrap();
        let scene = |color| {
            let mut items = vec![solid(3, rect(0.0, 0.0, 48.0, 48.0), color)];
            items.extend(children.clone());
            compile(frame(items)).unwrap()
        };
        let before = scene(BACKDROP);
        let after = scene(SECOND);
        let damage = diff_frame(&before, &after);
        assert_eq!(
            damage.changed,
            [owner(3)],
            "damage attributes changed facts"
        );
        assert_eq!(damage.union_frame, Some(rect(0.0, 0.0, 48.0, 48.0)));
        let old_pixels = raster(&retained, BACKDROP);
        let reused = raster(&retained, SECOND);
        assert_ne!(
            old_pixels, reused,
            "unchanged group still depends on backdrop"
        );
        assert_eq!(reused, raster(&after, CGColor::TRANSPARENT));
        assert_eq!(raster(&retained.clone(), SECOND), reused);
        assert_eq!(raster(&before, CGColor::TRANSPARENT), old_pixels);
    }
}

#[test]
fn rect_group_edit_matrix_retained_matches_fresh_and_damage_contains_changed_pixels() {
    #[derive(Clone, Copy, Debug)]
    enum Edit {
        Base,
        SourceColor,
        SourcePosition,
        Reorder,
        Removal,
        IsolationToggle,
    }

    let scene = |edit| {
        let isolated = !matches!(edit, Edit::IsolationToggle);
        let mut items = vec![solid(101, rect(0.0, 0.0, 48.0, 48.0), BACKDROP)];
        if isolated {
            items.push(blend(10, ScopeBlendMode::Normal, None));
        }
        items.push(solid(
            102,
            if matches!(edit, Edit::SourcePosition) {
                rect(8.0, 4.0, 24.0, 24.0)
            } else {
                rect(4.0, 8.0, 24.0, 24.0)
            },
            FIRST,
        ));
        items.push(blend(11, ScopeBlendMode::Multiply, Some(0.6)));
        let lower = solid(
            103,
            rect(16.0, 12.0, 24.0, 24.0),
            if matches!(edit, Edit::SourceColor) {
                CGColor::from_rgb(255, 0, 0)
            } else {
                SECOND
            },
        );
        let upper = solid(
            104,
            rect(12.0, 20.0, 24.0, 24.0),
            CGColor::from_rgba(0, 255, 0, 153),
        );
        match edit {
            Edit::Reorder => items.extend([upper, lower]),
            Edit::Removal => items.push(lower),
            _ => items.extend([lower, upper]),
        }
        items.push(FrameItem::ScopeEnd);
        if isolated {
            items.push(FrameItem::ScopeEnd);
        }
        frame(items)
    };

    let before = compile(scene(Edit::Base)).unwrap();
    let before_pixels = raster(&before, CGColor::TRANSPARENT);
    for edit in [
        Edit::SourceColor,
        Edit::SourcePosition,
        Edit::Reorder,
        Edit::Removal,
        Edit::IsolationToggle,
    ] {
        let retained = compile(scene(edit)).unwrap().clone();
        // Interleave another replay; no earlier raster result may be reused
        // as the completed source of this retained command product.
        assert_eq!(raster(&before, CGColor::TRANSPARENT), before_pixels);
        let reused = raster(&retained, CGColor::TRANSPARENT);
        let fresh = compile(scene(edit)).unwrap();
        assert_eq!(reused, raster(&fresh, CGColor::TRANSPARENT), "{edit:?}");
        let union = diff_frame(&before, &retained)
            .union_frame
            .expect("every edit has a damage union");
        let mut changed_pixels = 0;
        for y in 0..SIZE as usize {
            for x in 0..SIZE as usize {
                if at(&before_pixels, x, y) == at(&reused, x, y) {
                    continue;
                }
                changed_pixels += 1;
                // Integer Rect-only sources: require the whole changed pixel,
                // not merely its centre, to fit the declared damage union.
                assert!(
                    x as f32 >= union.x
                        && y as f32 >= union.y
                        && (x + 1) as f32 <= union.x + union.width
                        && (y + 1) as f32 <= union.y + union.height,
                    "{edit:?}: changed pixel ({x},{y}) escaped {union:?}"
                );
            }
        }
        assert!(changed_pixels > 0, "{edit:?} must not be a vacuous probe");
    }
}

#[test]
fn blend_scopes_compile_inside_repeating_programs_and_damage_the_outer_client() {
    let scene = |mode| {
        let tile = FrameItems::try_new(vec![
            solid(1, rect(0.0, 0.0, 16.0, 16.0), BACKDROP),
            blend(10, mode, None),
            solid(2, rect(4.0, 4.0, 8.0, 8.0), FIRST),
            FrameItem::ScopeEnd,
        ])
        .unwrap();
        let pattern =
            PatternPaint::new(16.0, 16.0, AffineTransform::identity(), Arc::new(tile), 1.0)
                .unwrap();
        compile(frame(vec![FrameItem::Node(node(
            20,
            rect(0.0, 0.0, 48.0, 48.0),
            PaintStack::from_pattern(pattern),
        ))]))
        .unwrap()
    };
    let before = scene(ScopeBlendMode::Multiply);
    let after = scene(ScopeBlendMode::Screen);
    for (product, expected) in [
        (&before, [17, 68, 153, 255]),
        (&after, [119, 204, 255, 255]),
    ] {
        let pixels = raster(product, CGColor::TRANSPARENT);
        for (x, y) in [(8, 8), (24, 24), (40, 40)] {
            assert_eq!(at(&pixels, x, y), expected);
        }
    }
    assert_eq!(diff_frame(&before, &after).changed, [owner(20)]);
    assert_eq!(
        raster(&after.clone(), BACKDROP),
        raster(&scene(ScopeBlendMode::Screen), BACKDROP)
    );
}

#[test]
fn blend_scopes_enclose_masks_and_replay_in_both_mask_phases() {
    let product = compile(frame(vec![
        blend(10, ScopeBlendMode::Multiply, None),
        FrameItem::MaskBegin(Mask::new(
            owner(11),
            MaskMode::Alpha,
            clip(rect(0.0, 0.0, 48.0, 48.0)),
        )),
        blend(12, ScopeBlendMode::Screen, None),
        solid(1, rect(8.0, 8.0, 24.0, 24.0), FIRST),
        FrameItem::ScopeEnd,
        FrameItem::MaskSource,
        blend(13, ScopeBlendMode::Normal, None),
        solid(2, rect(0.0, 0.0, 24.0, 48.0), CGColor::WHITE),
        FrameItem::ScopeEnd,
        FrameItem::MaskEnd,
        FrameItem::ScopeEnd,
    ]))
    .unwrap();
    let pixels = raster(&product, BACKDROP);
    assert_eq!(at(&pixels, 16, 16), [17, 68, 153, 255]);
    assert_eq!(at(&pixels, 28, 16), [51, 102, 153, 255]);
}

#[test]
fn a_source_generating_filter_supplies_blend_pixels_and_scope_coverage() {
    use rframe::{Filter, FilterColorSpace, FilterNode, FilterPrimitive, FilterProgram};
    let region = rect(8.0, 8.0, 24.0, 24.0);
    let scene = |mode| {
        let program = FilterProgram::new(Arc::from([FilterNode::new(
            Arc::from([]),
            region,
            FilterColorSpace::Srgb,
            FilterPrimitive::SolidColor {
                color: FIRST.into(),
            },
        )]))
        .unwrap();
        let filter = Filter::new(AffineTransform::identity(), region, program)
            .unwrap()
            .with_transparent_source();
        compile(frame(vec![
            blend(10, mode, None),
            begin(11, ScopeEffect::Filter(filter)),
            FrameItem::ScopeEnd,
            FrameItem::ScopeEnd,
        ]))
        .unwrap()
    };
    let before = scene(ScopeBlendMode::Multiply);
    let after = scene(ScopeBlendMode::Screen);
    assert_eq!(at(&raster(&before, BACKDROP), 16, 16), [17, 68, 153, 255]);
    assert_eq!(at(&raster(&after, BACKDROP), 16, 16), [119, 204, 255, 255]);
    let damage = diff_frame(&before, &after);
    assert_eq!(damage.changed, [owner(10)]);
    assert_eq!(damage.union_frame, Some(region));
}
