//! The thin `n0` CLI host for Web rendering.
//!
//! This host is the executable adoption seam for the mature renderer. It does
//! not convert Web sources into the n0 authored model, and it is not evidence
//! that every mature semantic already lowers through the shared frame.
//! Unqualified HTML/SVG rendering remains on `htmlcss`; explicit SVG Base and
//! Sample requests use the retained `websem -> rframe -> n0` rect-x slice. The
//! route is never selected silently. The host owns arguments, file I/O, an
//! explicit raster size, ambient system-font selection for the mature route,
//! CPU rasterization, and PNG encoding.
//! Local/remote images and external stylesheets are not resolved; directory
//! input and non-PNG output remain outside the admitted host contract.
//!
//! Usage:
//!   cargo run -p n0_cli --bin n0 -- <input.svg|input.html> <out.png> <WxH>
//!   cargo run -p n0_cli --bin n0 -- <input.svg> <out.png> <WxH> --base
//!   cargo run -p n0_cli --bin n0 -- <input.svg> <out.png> <WxH> --time-ns <i64>
//!
//! Examples:
//!   cargo run -p n0_cli --bin n0 -- \
//!     fixtures/test-svg/L0/basic-shapes.svg /tmp/shapes.png 500x500
//!   cargo run -p n0_cli --bin n0 -- \
//!     fixtures/test-html/L0/svg-inline-basic.html /tmp/page.png 800x600

use std::path::Path;
use std::process::ExitCode;

use animation_sampling::SampleTime;
use n0::paint::PaintCtx;
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FramePolicy {
    /// Transitional mature static route. This preserves existing HTML/SVG
    /// coverage while capabilities move behind the shared frame.
    MatureStatic,
    /// Authored state; animation contributes no value.
    Base,
    /// One exact signed-nanosecond sample.
    Sample(SampleTime),
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if !matches!(args.len(), 3..=5) {
        eprintln!(
            "usage:\n\
             n0 <input.svg|input.html> <out.png> <WxH>\n\
             n0 <input.svg> <out.png> <WxH> --base\n\
             n0 <input.svg> <out.png> <WxH> --time-ns <signed-nanoseconds>"
        );
        return ExitCode::from(2);
    }
    let input = &args[0];
    let output = &args[1];
    let Some((w, h)) = parse_size(&args[2]) else {
        eprintln!("error: size must look like 128x128 and be positive");
        return ExitCode::from(2);
    };
    let policy = match parse_frame_policy(&args[3..]) {
        Ok(policy) => policy,
        Err(message) => {
            eprintln!("error: {message}");
            return ExitCode::from(2);
        }
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

    let png = match render_source_to_png(&source, kind, w, h, policy) {
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
        "rendered {input} -> {output} ({w}x{h}, {}, {} bytes)",
        policy.label(),
        png.len()
    );
    ExitCode::SUCCESS
}

impl FramePolicy {
    const fn label(self) -> &'static str {
        match self {
            Self::MatureStatic => "static-mature",
            Self::Base => "base-shared-frame",
            Self::Sample(_) => "sample-shared-frame",
        }
    }
}

fn parse_frame_policy(args: &[String]) -> Result<FramePolicy, String> {
    match args {
        [] => Ok(FramePolicy::MatureStatic),
        [flag] if flag == "--base" => Ok(FramePolicy::Base),
        [flag, nanoseconds] if flag == "--time-ns" => nanoseconds
            .parse::<i64>()
            .map(|value| FramePolicy::Sample(SampleTime::from_nanoseconds(value)))
            .map_err(|_| {
                format!("--time-ns requires a signed 64-bit nanosecond value, got {nanoseconds:?}")
            }),
        _ => Err(
            "frame policy must be omitted, `--base`, or `--time-ns <signed-nanoseconds>`"
                .to_string(),
        ),
    }
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
    policy: FramePolicy,
) -> Result<Vec<u8>, String> {
    if kind == SourceKind::Html && policy != FramePolicy::MatureStatic {
        return Err(
            "explicit Base/Sample currently admits the retained standalone SVG rect-x slice only"
                .to_string(),
        );
    }
    if kind == SourceKind::Svg && policy != FramePolicy::MatureStatic {
        let retained = websem::SvgFrameSource::from_bare_svg_scaffold(source)
            .map_err(|error| error.to_string())?;
        let frame = match policy {
            FramePolicy::Base => retained.base_frame(),
            FramePolicy::Sample(time) => retained
                .sample_frame(time)
                .map_err(|error| error.to_string())?,
            FramePolicy::MatureStatic => unreachable!("handled by the mature route below"),
        };
        return frame_to_png(frame, width, height);
    }

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

fn frame_to_png(frame: rframe::Frame, width: i32, height: i32) -> Result<Vec<u8>, String> {
    let context = PaintCtx::new(None);
    let product = n0::glyphless::compile(frame).map_err(|error| error.to_string())?;
    let mut surface = surfaces::raster_n32_premul((width, height))
        .ok_or_else(|| format!("cannot allocate {width}x{height} CPU raster"))?;
    surface.canvas().clear(Color::TRANSPARENT);
    product
        .execute(
            surface.canvas(),
            &math2::transform::AffineTransform::identity(),
            &context,
        )
        .map_err(|error| error.to_string())?;
    surface_to_png(&mut surface, width, height)
}

fn picture_to_png(picture: &Picture, width: i32, height: i32) -> Result<Vec<u8>, String> {
    let mut surface = surfaces::raster_n32_premul((width, height))
        .ok_or_else(|| format!("cannot allocate {width}x{height} CPU raster"))?;
    let canvas = surface.canvas();
    canvas.clear(Color::TRANSPARENT);
    canvas.draw_picture(picture, None, None);
    surface_to_png(&mut surface, width, height)
}

fn surface_to_png(
    surface: &mut skia_safe::Surface,
    width: i32,
    height: i32,
) -> Result<Vec<u8>, String> {
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
        assert_eq!(parse_frame_policy(&[]), Ok(FramePolicy::MatureStatic));
        assert_eq!(
            parse_frame_policy(&["--base".to_string()]),
            Ok(FramePolicy::Base)
        );
        assert_eq!(
            parse_frame_policy(&["--time-ns".to_string(), "-1".to_string()]),
            Ok(FramePolicy::Sample(SampleTime::from_nanoseconds(-1)))
        );
        assert!(parse_frame_policy(&["--time-ns".to_string()]).is_err());
        assert!(parse_frame_policy(&["--time-ns".to_string(), "1.5".to_string()]).is_err());
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
            let first =
                render_source_to_png(&source, kind, size.0, size.1, FramePolicy::MatureStatic)
                    .unwrap_or_else(|error| panic!("first render {relative}: {error}"));
            let second =
                render_source_to_png(&source, kind, size.0, size.1, FramePolicy::MatureStatic)
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

    #[test]
    fn retained_svg_base_and_exact_time_render_through_the_shared_frame() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let source = std::fs::read_to_string(
            root.join("fixtures/web-first/animation/svg-rect-x-animation.svg"),
        )
        .expect("read animation fixture");
        for (policy, expected_black_x, oracle) in [
            (
                FramePolicy::Base,
                4,
                "fixtures/web-first/animation/chromium/base.png",
            ),
            (
                FramePolicy::Sample(SampleTime::from_nanoseconds(0)),
                20,
                "fixtures/web-first/animation/chromium/sample-0ns.png",
            ),
            (
                FramePolicy::Sample(SampleTime::from_nanoseconds(1_000_000_000)),
                32,
                "fixtures/web-first/animation/chromium/sample-1000000000ns.png",
            ),
            (
                FramePolicy::Sample(SampleTime::from_nanoseconds(2_000_000_000)),
                44,
                "fixtures/web-first/animation/chromium/sample-2000000000ns.png",
            ),
        ] {
            let first = render_source_to_png(&source, SourceKind::Svg, 64, 32, policy)
                .unwrap_or_else(|error| panic!("render {policy:?}: {error}"));
            let second = render_source_to_png(&source, SourceKind::Svg, 64, 32, policy)
                .unwrap_or_else(|error| panic!("repeat {policy:?}: {error}"));
            assert_eq!(first, second, "{policy:?} encoded determinism");
            let raster = decode_png(&first).expect("decode exact-time PNG");
            let expected = decode_png(
                &std::fs::read(root.join(oracle))
                    .unwrap_or_else(|error| panic!("read {oracle}: {error}")),
            )
            .unwrap_or_else(|| panic!("decode {oracle}"));
            assert_eq!(raster.pixels, expected.pixels, "{policy:?} Chromium RGBA");
            assert_eq!(raster.at(expected_black_x + 4, 16), [0, 0, 0, 255]);
            let authored_x = if expected_black_x == 4 { 4 } else { 8 };
            assert_eq!(
                raster.at(authored_x, 16),
                if expected_black_x == 4 {
                    [0, 0, 0, 255]
                } else {
                    [255, 255, 255, 255]
                }
            );
        }
        assert!(
            render_source_to_png("<html></html>", SourceKind::Html, 64, 32, FramePolicy::Base)
                .is_err()
        );
    }
}
