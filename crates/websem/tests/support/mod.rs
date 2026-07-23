//! Shared pixel-gate plumbing for the websem integration tests.
//!
//! Every rendered pixel here is produced by the one n0 downstream; the tests
//! own decoding of the committed Chromium oracle PNGs. There is no second
//! painter to compare against — rframe's temporary proving downstream retired
//! when the D-M vector join was taken
//! (docs/wg/consolidation/n0-join-point.md).

use n0::paint::PaintCtx;
use skia_safe::image::CachingHint;
use skia_safe::{AlphaType, Color, ColorType, IPoint, ImageInfo, surfaces};

/// An in-memory raster: row-major RGBA8888, unpremultiplied.
pub(crate) struct Raster {
    pub(crate) width: i32,
    pub(crate) height: i32,
    pub(crate) pixels: Vec<u8>,
}

/// Decode PNG bytes (a committed Chromium oracle) into a straight-alpha RGBA
/// [`Raster`]. Returns `None` if the bytes are not a decodable image.
pub(crate) fn decode_png(bytes: &[u8]) -> Option<Raster> {
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

/// Compile a resolved Web frame and rasterize it through n0's one private
/// drawlist and painter, reading back straight-alpha RGBA bytes.
pub(crate) fn render_through_n0(frame: &rframe::Frame, width: i32, height: i32) -> Vec<u8> {
    let context = PaintCtx::new(None);
    let product = n0::glyphless::compile(frame.clone()).expect("compile admitted Web frame");
    let mut surface = surfaces::raster_n32_premul((width, height)).expect("CPU raster surface");
    surface.canvas().clear(Color::TRANSPARENT);
    product
        .execute(
            surface.canvas(),
            &math2::transform::AffineTransform::identity(),
            &context,
        )
        .expect("execute admitted Web frame through n0");

    let image = surface.image_snapshot();
    let info = ImageInfo::new(
        (width, height),
        ColorType::RGBA8888,
        AlphaType::Unpremul,
        None,
    );
    let row_bytes = width as usize * 4;
    let mut pixels = vec![0; row_bytes * height as usize];
    assert!(image.read_pixels(
        &info,
        &mut pixels,
        row_bytes,
        IPoint::new(0, 0),
        CachingHint::Disallow,
    ));
    pixels
}
