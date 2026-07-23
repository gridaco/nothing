//! The thin `n0` CLI host for the mature static Web renderer: render an SVG or
//! HTML document (including inline SVG) to a PNG.
//!
//! This host is the executable adoption seam for the mature renderer. It does
//! not convert Web sources into the n0 authored model, and it is not evidence
//! that the mature semantics already lower through `rframe` or the final
//! chassis. Meaning remains in `htmlcss`. The host owns only arguments, file
//! I/O, an explicit raster size (also used as the standalone-SVG container
//! size), ambient system-font selection, CPU rasterization, and PNG encode.
//! It intentionally accepts only self-contained files today: local/remote
//! images and external stylesheets are not resolved. Directory input and
//! non-PNG output are not yet admitted.
//!
//! Usage:
//!   cargo run -p n0_cli --bin n0 -- <input.svg|input.html> <out.png> <WxH>
//!
//! Examples:
//!   cargo run -p n0_cli --bin n0 -- \
//!     fixtures/test-svg/L0/basic-shapes.svg /tmp/shapes.png 500x500
//!   cargo run -p n0_cli --bin n0 -- \
//!     fixtures/test-html/L0/svg-inline-basic.html /tmp/page.png 800x600

use std::path::Path;
use std::process::ExitCode;

use skia_safe::textlayout::FontCollection;
use skia_safe::{Color, EncodedImageFormat, FontMgr, Picture, surfaces};

struct SystemFontCollection(FontCollection);

impl SystemFontCollection {
    fn new() -> Self {
        let mut fonts = FontCollection::new();
        fonts.set_default_font_manager(FontMgr::new(), None);
        fonts.enable_font_fallback();
        Self(fonts)
    }
}

impl htmlcss::SkiaFontCollectionProvider for SystemFontCollection {
    fn font_collection(&self) -> &FontCollection {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SourceKind {
    Html,
    Svg,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() != 3 {
        eprintln!(
            "usage: n0 <input.svg|input.html> <out.png> <WxH>\n\
             renders the extracted static Web implementation to a CPU PNG."
        );
        return ExitCode::from(2);
    }
    let input = &args[0];
    let output = &args[1];
    let Some((w, h)) = parse_size(&args[2]) else {
        eprintln!("error: size must look like 128x128 and be positive");
        return ExitCode::from(2);
    };

    let kind = match source_kind(Path::new(input)) {
        Ok(kind) => kind,
        Err(message) => {
            eprintln!("error: {message}");
            return ExitCode::from(2);
        }
    };
    if !has_extension(Path::new(output), "png") {
        eprintln!("error: output must have a .png extension");
        return ExitCode::from(2);
    }

    let source = match std::fs::read_to_string(input) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read {input}: {e}");
            return ExitCode::FAILURE;
        }
    };

    let png = match render_source_to_png(&source, kind, w, h) {
        Ok(png) => png,
        Err(e) => {
            eprintln!("error: render failed: {e}");
            return ExitCode::FAILURE;
        }
    };

    if let Err(e) = std::fs::write(output, &png) {
        eprintln!("error: cannot write {output}: {e}");
        return ExitCode::FAILURE;
    }
    eprintln!(
        "rendered {input} -> {output} ({w}x{h}, {} bytes)",
        png.len()
    );
    ExitCode::SUCCESS
}

fn parse_size(s: &str) -> Option<(i32, i32)> {
    let (w, h) = s.split_once(['x', 'X'])?;
    let size = (w.trim().parse().ok()?, h.trim().parse().ok()?);
    (size.0 > 0 && size.1 > 0).then_some(size)
}

fn source_kind(path: &Path) -> Result<SourceKind, String> {
    if has_extension(path, "html") || has_extension(path, "htm") {
        return Ok(SourceKind::Html);
    }
    if has_extension(path, "svg") {
        return Ok(SourceKind::Svg);
    }
    Err(format!(
        "unsupported input extension for {}; expected .html, .htm, or .svg",
        path.display()
    ))
}

fn has_extension(path: &Path, expected: &str) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case(expected))
}

fn render_source_to_png(
    source: &str,
    kind: SourceKind,
    width: i32,
    height: i32,
) -> Result<Vec<u8>, String> {
    let picture = match kind {
        SourceKind::Html => {
            let fonts = SystemFontCollection::new();
            htmlcss::render(
                source,
                width as f32,
                height as f32,
                &fonts,
                &htmlcss::NoImages,
            )
        }
        SourceKind::Svg => htmlcss::render_svg(source, width as f32, height as f32),
    }?;
    picture_to_png(&picture, width, height)
}

fn picture_to_png(picture: &Picture, width: i32, height: i32) -> Result<Vec<u8>, String> {
    let mut surface = surfaces::raster_n32_premul((width, height))
        .ok_or_else(|| format!("cannot allocate {width}x{height} CPU raster"))?;
    let canvas = surface.canvas();
    canvas.clear(Color::TRANSPARENT);
    canvas.draw_picture(picture, None, None);
    let image = surface.image_snapshot();
    let png = image
        .encode(None, EncodedImageFormat::PNG, None)
        .ok_or_else(|| format!("cannot encode {width}x{height} PNG"))?;
    Ok(png.as_bytes().to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use skia_safe::image::CachingHint;
    use skia_safe::{AlphaType, ColorType, Data, IPoint, Image, ImageInfo};

    struct TestRaster {
        width: i32,
        height: i32,
        pixels: Vec<u8>,
    }

    impl TestRaster {
        fn at(&self, x: i32, y: i32) -> [u8; 4] {
            let offset = ((y * self.width + x) * 4) as usize;
            self.pixels[offset..offset + 4]
                .try_into()
                .expect("RGBA pixel")
        }
    }

    fn decode_png(bytes: &[u8]) -> Option<TestRaster> {
        let image = Image::from_encoded(Data::new_copy(bytes))?;
        let width = image.width();
        let height = image.height();
        let info = ImageInfo::new(
            (width, height),
            ColorType::RGBA8888,
            AlphaType::Unpremul,
            None,
        );
        let row_bytes = width as usize * 4;
        let mut pixels = vec![0; row_bytes * height as usize];
        image
            .read_pixels(
                &info,
                &mut pixels,
                row_bytes,
                IPoint::new(0, 0),
                CachingHint::Disallow,
            )
            .then_some(TestRaster {
                width,
                height,
                pixels,
            })
    }

    #[test]
    fn input_and_output_contract_is_strict() {
        assert_eq!(source_kind(Path::new("page.HTML")), Ok(SourceKind::Html));
        assert_eq!(source_kind(Path::new("icon.svg")), Ok(SourceKind::Svg));
        assert!(source_kind(Path::new("scene.n0.xml")).is_err());
        assert!(has_extension(Path::new("out.PNG"), "png"));
        assert_eq!(parse_size("320x200"), Some((320, 200)));
        assert_eq!(parse_size("320X200"), Some((320, 200)));
        assert_eq!(parse_size("0x200"), None);
        assert_eq!(parse_size("auto"), None);
    }

    #[test]
    fn committed_html_and_svg_fixtures_render_deterministically() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        for (relative, kind, size) in [
            (
                "fixtures/test-svg/L0/basic-shapes.svg",
                SourceKind::Svg,
                (500, 500),
            ),
            (
                "fixtures/test-html/L0/svg-inline-basic.html",
                SourceKind::Html,
                (800, 600),
            ),
            (
                "fixtures/test-svg/probe/circle-fill-probe.svg",
                SourceKind::Svg,
                (64, 64),
            ),
            (
                "fixtures/test-html/probe/inline-svg-flex-probe.html",
                SourceKind::Html,
                (96, 48),
            ),
        ] {
            let input = root.join(relative);
            let source = std::fs::read_to_string(&input)
                .unwrap_or_else(|error| panic!("read {}: {error}", input.display()));
            let first = render_source_to_png(&source, kind, size.0, size.1)
                .unwrap_or_else(|error| panic!("first render {relative}: {error}"));
            let second = render_source_to_png(&source, kind, size.0, size.1)
                .unwrap_or_else(|error| panic!("second render {relative}: {error}"));
            assert_eq!(first, second, "{relative} must be byte-deterministic");

            let raster =
                decode_png(&first).unwrap_or_else(|| panic!("decode rendered PNG for {relative}"));
            assert_eq!((raster.width, raster.height), size, "{relative} dimensions");
            assert!(
                raster.pixels.chunks_exact(4).any(|pixel| pixel[3] != 0),
                "{relative} must paint at least one non-transparent pixel"
            );
            match relative {
                "fixtures/test-svg/L0/basic-shapes.svg"
                | "fixtures/test-html/L0/svg-inline-basic.html" => {}
                "fixtures/test-svg/probe/circle-fill-probe.svg" => {
                    assert_eq!(
                        raster.at(32, 32),
                        [22, 163, 74, 255],
                        "the standalone SVG circle probe must render"
                    );
                    assert_eq!(raster.at(4, 4), [255, 255, 255, 255]);
                }
                "fixtures/test-html/probe/inline-svg-flex-probe.html" => {
                    assert_eq!(
                        raster.at(24, 24),
                        [239, 68, 68, 255],
                        "the CSS-positioned first inline SVG must render"
                    );
                    assert_eq!(
                        raster.at(64, 24),
                        [37, 99, 235, 255],
                        "flex layout must place the second inline SVG beside the first"
                    );
                    assert_eq!(raster.at(4, 4), [255, 255, 255, 255]);
                }
                _ => unreachable!("fixture table and probes must advance together"),
            }

            let output = std::env::temp_dir().join(format!(
                "n0-cli-render-{}-{}.png",
                std::process::id(),
                match kind {
                    SourceKind::Html => "html",
                    SourceKind::Svg => "svg",
                }
            ));
            std::fs::write(&output, &first)
                .unwrap_or_else(|error| panic!("write {}: {error}", output.display()));
            let written = std::fs::read(&output)
                .unwrap_or_else(|error| panic!("read {}: {error}", output.display()));
            assert_eq!(written, first, "written PNG bytes for {relative}");
            let _ = std::fs::remove_file(output);
        }
    }
}
