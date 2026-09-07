//! Cross-seam execution laws. The external pixel oracle remains the separate
//! Chromium corpus; these tests make the compiler's backdrop boundary explicit.
use math2::transform::AffineTransform;
use n0::paint::{PaintCtx, read_pixels};
use rframe::{Frame, FrameItems};
use skia_safe::{Color, surfaces};
use websem::{InitialViewport, compile_standalone_svg};

fn compile(body: &str, style: &str) -> Frame {
    let source = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" style="{style}">{body}</svg>"#
    );
    compile_standalone_svg(&source, InitialViewport::new(64.0, 64.0)).unwrap()
}

fn paint(frame: Frame, backdrop: Color) -> Vec<u8> {
    let product = n0::glyphless::compile(frame).unwrap();
    let mut surface = surfaces::raster_n32_premul((64, 64)).unwrap();
    surface.canvas().clear(backdrop);
    product
        .execute(
            surface.canvas(),
            &AffineTransform::identity(),
            &PaintCtx::new(None),
        )
        .unwrap();
    read_pixels(&mut surface, 64, 64)
}

#[test]
fn standalone_root_blend_does_not_consume_the_callers_backdrop() {
    let body = r##"<rect x="8" y="8" width="40" height="40" fill="#cd6843"/>"##;
    for color in [
        Color::from_argb(255, 51, 102, 153),
        Color::from_argb(170, 0, 255, 0),
    ] {
        let reference = paint(compile(body, "mix-blend-mode:normal"), color);
        for mode in ["multiply", "screen"] {
            let frame = compile(body, &format!("mix-blend-mode:{mode}"));
            assert_eq!(paint(frame.clone(), color), reference, "{mode}");
            // The colored destination makes removal of the initial transparent
            // boundary observable; a transparent-only oracle cannot do that.
            let mut without_initial_boundary = frame;
            let items: Vec<_> = without_initial_boundary.items.iter().cloned().collect();
            without_initial_boundary.items =
                FrameItems::try_new(items[1..items.len() - 1].to_vec()).unwrap();
            assert_ne!(paint(without_initial_boundary, color), reference, "{mode}");
        }
    }
}

#[test]
fn redundant_isolation_preserves_partial_backdrop_opacity_pixels() {
    let backdrop = r##"<rect width="64" height="64" fill="#00ff00aa"/>"##;
    let plain = "<rect x='8' y='8' width='40' height='40' fill='red' opacity='.6'/>";
    let reference = paint(
        compile(&format!("{backdrop}{plain}"), ""),
        Color::TRANSPARENT,
    );
    let offset = (16 * 64 + 16) * 4;
    assert_eq!(&reference[offset..offset + 4], &[153, 68, 0, 221]);
    for body in [
        "<rect x='8' y='8' width='40' height='40' fill='red' opacity='.6' style='isolation:isolate'/>",
        "<g opacity='.6' style='isolation:isolate'><rect x='8' y='8' width='40' height='40' fill='red'/></g>",
        "<g opacity='.6'><rect x='8' y='8' width='40' height='40' fill='red' style='isolation:isolate'/></g>",
    ] {
        assert_eq!(
            paint(
                compile(&format!("{backdrop}{body}"), ""),
                Color::TRANSPARENT
            ),
            reference,
            "{body}"
        );
    }
}
