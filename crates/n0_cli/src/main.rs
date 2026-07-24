//! The thin `n0` CLI host for the SVG engine of record.
//!
//! Per D-N (docs/wg/consolidation/svg-engine-of-record.md), every render
//! goes through the one pipeline: the websem compiler lowers standalone SVG
//! or inline-HTML SVG from the retained document session to the shared
//! frame, which the n0 engine compiles and paints. Static renders are the
//! Base view (animation contributes no value); `--time-ns` renders one
//! exact signed-nanosecond Sample of the same compile. Time changes
//! effective values only — it selects no route.
//!
//! Beyond-slice constructs refuse loudly with the unsupported construct
//! named. The HTML entry compiles exactly the document's first inline SVG:
//! when that subtree is admitted the render succeeds and the surrounding
//! page contributes nothing (a pinned contract, not a silent drop); when it
//! is not, the host refuses by name. The host owns arguments,
//! file I/O, an explicit raster size, CPU rasterization, and PNG encoding.
//! Local/remote images and external stylesheets are not resolved; directory
//! input and non-PNG output remain outside the admitted host contract.
//!
//! Usage:
//!   cargo run -p n0_cli --bin n0 -- <input.svg|input.html> <out.png> <WxH>
//!   cargo run -p n0_cli --bin n0 -- <input.svg|input.html> <out.png> <WxH> --base
//!   cargo run -p n0_cli --bin n0 -- <input.svg> <out.png> <WxH> --time-ns <i64>
//!
//! Examples:
//!   cargo run -p n0_cli --bin n0 -- \
//!     fixtures/web-first/svg-fill-named-rect.svg /tmp/rect.png 64x64
//!   cargo run -p n0_cli --bin n0 -- \
//!     fixtures/web-first/animation/svg-rect-x-animation.svg /tmp/t1s.png 64x32 --time-ns 1000000000

use std::path::Path;
use std::process::ExitCode;

use animation_sampling::SampleTime;
use n0::paint::PaintCtx;
use skia_safe::{Color, EncodedImageFormat, surfaces};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SourceKind {
    Html,
    Svg,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FramePolicy {
    /// Authored state; animation contributes no value. The default.
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
             n0 <input.svg|input.html> <out.png> <WxH> --base\n\
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
            Self::Base => "base-shared-frame",
            Self::Sample(_) => "sample-shared-frame",
        }
    }
}

fn parse_frame_policy(args: &[String]) -> Result<FramePolicy, String> {
    match args {
        [] => Ok(FramePolicy::Base),
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
    let retained = match kind {
        SourceKind::Svg => websem::SvgFrameSource::from_standalone_svg(source),
        SourceKind::Html => websem::SvgFrameSource::from_html_inline_svg(source),
    }
    .map_err(|error| error.to_string())?;
    let frame = match policy {
        FramePolicy::Base => retained.base_frame(),
        FramePolicy::Sample(time) => retained
            .sample_frame(time)
            .map_err(|error| error.to_string())?,
    };
    frame_to_png(frame, width, height)
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
            assert!(x >= 0 && x < self.width && y >= 0 && y < self.height);
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
        assert_eq!(parse_frame_policy(&[]), Ok(FramePolicy::Base));
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

    /// D-N's capability statement: inputs the retired mature route rendered
    /// refuse loudly on the engine of record, with the unsupported construct
    /// named — never wrong pixels. Each pinned reason is the current
    /// capability edge; an evolution rung that admits the construct must
    /// update the pin alongside its Chromium-baked fixtures.
    #[test]
    fn beyond_slice_legacy_corpus_refuses_by_name() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        for (relative, kind, size, named) in [
            (
                "fixtures/test-svg/L0/basic-shapes.svg",
                SourceKind::Svg,
                (500, 500),
                "unsupported element <title>",
            ),
            (
                "fixtures/test-svg/probe/circle-fill-probe.svg",
                SourceKind::Svg,
                (64, 64),
                "unsupported element <circle>",
            ),
            (
                "fixtures/test-html/probe/inline-svg-flex-probe.html",
                SourceKind::Html,
                (96, 48),
                "unsupported SVG viewport sizing: missing width",
            ),
        ] {
            let input = root.join(relative);
            let source = std::fs::read_to_string(&input)
                .unwrap_or_else(|error| panic!("read {}: {error}", input.display()));
            let error = render_source_to_png(&source, kind, size.0, size.1, FramePolicy::Base)
                .expect_err("beyond-slice input must refuse, not render");
            assert!(
                error.contains(named),
                "{relative} must refuse with the construct named; got: {error}"
            );
        }
    }

    /// The legacy multi-SVG page pins the other half of the capability
    /// statement: the engine of record compiles the FIRST inline SVG of an
    /// HTML document — here the admitted rect — and nothing else on the
    /// page. The mature route rendered the whole page; that surface returns
    /// only through evolution rungs.
    #[test]
    fn legacy_multi_svg_page_renders_its_first_admitted_svg_only() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let source =
            std::fs::read_to_string(root.join("fixtures/test-html/L0/svg-inline-basic.html"))
                .expect("read legacy inline-svg page");
        let first = render_source_to_png(&source, SourceKind::Html, 800, 600, FramePolicy::Base)
            .expect("the first inline SVG is the admitted rect");
        let second = render_source_to_png(&source, SourceKind::Html, 800, 600, FramePolicy::Base)
            .expect("repeat render");
        assert_eq!(first, second, "encoded determinism");
        let raster = decode_png(&first).expect("decode rendered PNG");
        assert_eq!(
            raster.at(50, 50),
            [239, 68, 68, 255],
            "the first SVG's authored rect paints"
        );
        assert_eq!(
            raster.at(5, 5),
            [0, 0, 0, 0],
            "the page body (flex frames, labels) contributes nothing"
        );
        assert_eq!(
            raster.at(200, 200),
            [0, 0, 0, 0],
            "the later SVG cells contribute nothing"
        );
    }

    #[test]
    fn admitted_primitives_render_exactly_through_the_one_engine() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        for (relative, kind, size, oracle) in [
            (
                "fixtures/web-first/svg-fill-named-rect.svg",
                SourceKind::Svg,
                (64, 64),
                "fixtures/web-first/chromium/svg-fill-named-rect.png",
            ),
            (
                "fixtures/web-first/html-inline-svg-currentcolor-rect.html",
                SourceKind::Html,
                (64, 64),
                "fixtures/web-first/chromium/html-inline-svg-currentcolor-rect.png",
            ),
            (
                "fixtures/web-first/html-webpage-mockup.html",
                SourceKind::Html,
                (640, 400),
                "fixtures/web-first/chromium/html-webpage-mockup.png",
            ),
        ] {
            let source = std::fs::read_to_string(root.join(relative))
                .unwrap_or_else(|error| panic!("read {relative}: {error}"));
            let first = render_source_to_png(&source, kind, size.0, size.1, FramePolicy::Base)
                .unwrap_or_else(|error| panic!("render {relative}: {error}"));
            let second = render_source_to_png(&source, kind, size.0, size.1, FramePolicy::Base)
                .unwrap_or_else(|error| panic!("repeat {relative}: {error}"));
            assert_eq!(first, second, "{relative} encoded determinism");
            let raster = decode_png(&first).expect("decode rendered PNG");
            let expected = decode_png(
                &std::fs::read(root.join(oracle))
                    .unwrap_or_else(|error| panic!("read {oracle}: {error}")),
            )
            .unwrap_or_else(|| panic!("decode {oracle}"));
            assert_eq!(
                (raster.width, raster.height),
                (expected.width, expected.height),
                "{relative} dimensions"
            );
            assert_eq!(raster.pixels, expected.pixels, "{relative} Chromium RGBA");
        }
    }

    #[test]
    fn base_and_exact_time_render_through_the_one_engine() {
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
    }

    /// Sampling inline HTML refuses loudly through websem's own dynamic
    /// inventory; a document with no inline SVG refuses at construction.
    #[test]
    fn html_sampling_and_svgless_documents_refuse_loudly() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let source = std::fs::read_to_string(
            root.join("fixtures/web-first/html-inline-svg-currentcolor-rect.html"),
        )
        .expect("read inline-svg fixture");
        let error = render_source_to_png(
            &source,
            SourceKind::Html,
            64,
            64,
            FramePolicy::Sample(SampleTime::ZERO),
        )
        .expect_err("inline-HTML sampling is not admitted");
        assert!(
            error.contains("inline HTML"),
            "the refusal names the entry: {error}"
        );
        let error =
            render_source_to_png("<html></html>", SourceKind::Html, 64, 32, FramePolicy::Base)
                .expect_err("a document with no inline SVG refuses at construction");
        assert!(
            error.contains("no <svg> element"),
            "the refusal names the missing root: {error}"
        );
    }
}
