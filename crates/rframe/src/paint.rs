//! The painter — the **one** Skia module in this crate, and the whole of its
//! backend surface. It replays a [`DrawList`] directly onto a caller-supplied
//! `skia_safe::Canvas`; it records **no** `SkPicture` and hands nothing opaque
//! past the kernel (no escape hatch — see the Web-First Amendment).
//!
//! PROVISIONAL: for the first slice this duplicates the replay discipline that
//! `n0::paint` already implements. It is a candidate to collapse once the
//! owner's evidence spike decides where producers join the shared downstream.

use skia_safe::image::CachingHint;
use skia_safe::{
    AlphaType, Canvas, Color as SkColor, ColorType, IPoint, ImageInfo, Matrix, Paint as SkPaint,
    Rect as SkRect, surfaces,
};

use crate::drawlist::{self, DrawItem, DrawList};
use crate::frame::{Color, Frame};
use math2::Rectangle;
use math2::transform::AffineTransform;

fn sk_color(c: Color) -> SkColor {
    SkColor::from_argb(c.a, c.r, c.g, c.b)
}

fn sk_rect(r: Rectangle) -> SkRect {
    SkRect::from_xywh(r.x, r.y, r.width, r.height)
}

/// math2 stores `[[a, c, e], [b, d, f]]`; Skia's affine array is
/// `[scaleX(a), skewY(b), skewX(c), scaleY(d), transX(e), transY(f)]`.
fn sk_matrix(t: AffineTransform) -> Matrix {
    let m = t.matrix;
    Matrix::from_affine(&[m[0][0], m[1][0], m[0][1], m[1][1], m[0][2], m[1][2]])
}

/// Replay a drawlist onto `canvas`. Deterministic: anti-aliasing is disabled so
/// solid fills produce byte-stable interiors across runs and machines.
pub(crate) fn paint(canvas: &Canvas, list: &DrawList) {
    for item in &list.items {
        match item {
            DrawItem::ClipRect(rect) => {
                canvas.save();
                canvas.clip_rect(sk_rect(*rect), None, false);
            }
            DrawItem::Restore => {
                canvas.restore();
            }
            DrawItem::FillRect {
                rect,
                transform,
                color,
            } => {
                let mut paint = SkPaint::default();
                paint.set_color(sk_color(*color));
                paint.set_anti_alias(false);
                canvas.save();
                canvas.concat(&sk_matrix(*transform));
                canvas.draw_rect(sk_rect(*rect), &paint);
                canvas.restore();
            }
        }
    }
}

/// An in-memory RGBA8888 (straight-alpha) raster, for headless rendering and
/// probe assertions.
pub struct Raster {
    pub width: i32,
    pub height: i32,
    /// Row-major RGBA8888, unpremultiplied.
    pub pixels: Vec<u8>,
}

impl Raster {
    /// The `[r, g, b, a]` at `(x, y)`.
    pub fn at(&self, x: i32, y: i32) -> [u8; 4] {
        assert!(
            (0..self.width).contains(&x) && (0..self.height).contains(&y),
            "probe ({x}, {y}) outside {}x{}",
            self.width,
            self.height
        );
        let o = ((y * self.width + x) * 4) as usize;
        self.pixels[o..o + 4].try_into().expect("rgba slice")
    }
}

/// Lower a resolved [`Frame`] to the private drawlist and rasterize it — the
/// whole downstream in one call, for producers that just want pixels.
pub fn render(frame: &Frame, width: i32, height: i32) -> Raster {
    raster(&drawlist::build(frame), width, height)
}

/// Render a [`Frame`] and encode it as PNG bytes — the downstream plus
/// encoding, for a host that just wants a file to write.
pub fn render_png(frame: &Frame, width: i32, height: i32) -> Vec<u8> {
    let mut surface = surfaces::raster_n32_premul((width, height)).expect("raster surface");
    surface.canvas().clear(SkColor::TRANSPARENT);
    paint(surface.canvas(), &drawlist::build(frame));
    let image = surface.image_snapshot();
    skia_safe::encode::image(None, &image, skia_safe::EncodedImageFormat::PNG, None)
        .expect("encode PNG")
        .as_bytes()
        .to_vec()
}

/// Decode PNG bytes into an RGBA8888 [`Raster`] (for comparing against a
/// committed oracle). Returns `None` if the bytes are not a decodable image.
pub fn decode_png(bytes: &[u8]) -> Option<Raster> {
    let data = skia_safe::Data::new_copy(bytes);
    let image = skia_safe::Image::from_encoded(data)?;
    let width = image.width();
    let height = image.height();
    let info = ImageInfo::new(
        (width, height),
        ColorType::RGBA8888,
        AlphaType::Unpremul,
        None,
    );
    let row_bytes = width as usize * 4;
    let mut pixels = vec![0u8; row_bytes * height as usize];
    if !image.read_pixels(
        &info,
        &mut pixels,
        row_bytes,
        IPoint::new(0, 0),
        CachingHint::Disallow,
    ) {
        return None;
    }
    Some(Raster {
        width,
        height,
        pixels,
    })
}

/// Rasterize a drawlist onto a fresh transparent CPU surface and read the
/// pixels back. Deterministic (CPU raster, AA disabled): two calls with the
/// same drawlist and dimensions produce byte-identical `pixels`.
pub(crate) fn raster(list: &DrawList, width: i32, height: i32) -> Raster {
    let mut surface = surfaces::raster_n32_premul((width, height)).expect("raster surface");
    surface.canvas().clear(SkColor::TRANSPARENT);
    paint(surface.canvas(), list);
    let image = surface.image_snapshot();
    let info = ImageInfo::new(
        (width, height),
        ColorType::RGBA8888,
        AlphaType::Unpremul,
        None,
    );
    let row_bytes = width as usize * 4;
    let mut pixels = vec![0u8; row_bytes * height as usize];
    assert!(
        image.read_pixels(
            &info,
            &mut pixels,
            row_bytes,
            IPoint::new(0, 0),
            CachingHint::Disallow
        ),
        "read RGBA raster"
    );
    Raster {
        width,
        height,
        pixels,
    }
}
