//! The thin `n0` CLI host for the SVG engine of record.
//!
//! The n0 path is this repository's SVG engine of record
//! (docs/wg/consolidation/svg-engine-of-record.md), so every render
//! goes through the one pipeline: the websem compiler lowers standalone SVG
//! or inline-HTML SVG from the retained document session to the shared
//! frame, which the n0 engine compiles and paints. Static renders are the
//! Base view (animation contributes no value); `--time-ns` renders one
//! exact signed-nanosecond Sample of the same compile. Time changes
//! effective values only — it selects no route.
//!
//! The default admission is best-effort: the admitted subset renders and
//! every beyond-slice construct is declared on stderr with its node path
//! and reason — skipped elements leave holes, a beyond-inventory dynamic
//! surface samples as the Base view. Never silent, never guessed pixels.
//! `--strict` keeps the refusal harness: the first beyond-slice construct
//! fails loudly with the construct named — the dev/TODO surface (the
//! refusal pins run strict; the gates hold both admissions frame-identical
//! across the full oracle corpus and byte-identical at the host's spot
//! checks; the Chromium bakes drive the browser only). Document-level
//! contracts — no `<svg>` root, malformed standalone XML, the script-free
//! parses, the outer viewport grammar and root patrols — refuse identically
//! in both admissions.
//!
//! The requested `WxH` is the initial viewport (SVG2 §8.2) of a standalone
//! document — the window it is loaded into. Explicit root `width`/`height`
//! win; a missing dimension is `auto` and resolves to 100% of `WxH`, so
//! viewBox-only documents render at the requested raster, mapped under the
//! full `preserveAspectRatio` grammar.
//!
//! The HTML entry compiles exactly the document's first inline SVG: the
//! surrounding page contributes nothing (a pinned contract, not a silent
//! drop). Sampling inline HTML refuses under `--strict` and samples as the
//! Base view (declared) by default. The host owns arguments, file I/O, an
//! explicit raster size, CPU rasterization, and PNG encoding. Local/remote
//! images and external stylesheets are not resolved; directory input and
//! non-PNG output remain outside the admitted host contract.
//!
//! Usage:
//!   cargo run -p n0_cli --bin n0 -- <input.svg|input.html> <out.png> <WxH>
//!   cargo run -p n0_cli --bin n0 -- <input.svg|input.html> <out.png> <WxH> --base
//!   cargo run -p n0_cli --bin n0 -- <input.svg|input.html> <out.png> <WxH> --time-ns <i64>
//!   ... any form plus --strict (refuse beyond-slice) or --best-effort (the
//!   explicit spelling of the default)
//!
//! Examples:
//!   cargo run -p n0_cli --bin n0 -- \
//!     fixtures/web-first/svg-fill-named-rect.svg /tmp/rect.png 64x64
//!   cargo run -p n0_cli --bin n0 -- \
//!     fixtures/web-first/animation/svg-rect-x-animation.svg /tmp/t1s.png 64x32 --time-ns 1000000000
//!   cargo run -p n0_cli --bin n0 -- \
//!     fixtures/web-first/animation/svg-scene-cub-animation.svg /tmp/cub.png 96x96 --strict

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Admission {
    /// Render the admitted subset; declare every beyond-slice construct on
    /// stderr. The default.
    BestEffort,
    /// Refuse loudly on the first beyond-slice construct — the dev harness
    /// and TODO surface.
    Strict,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if !matches!(args.len(), 3..=6) {
        eprintln!(
            "usage:\n\
             n0 <input.svg|input.html> <out.png> <WxH>\n\
             n0 <input.svg|input.html> <out.png> <WxH> --base\n\
             n0 <input.svg|input.html> <out.png> <WxH> --time-ns <signed-nanoseconds>\n\
             any form may add --strict (refuse beyond-slice constructs)\n\
             or --best-effort (declare and render; the default)"
        );
        return ExitCode::from(2);
    }
    let input = &args[0];
    let output = &args[1];
    let Some((w, h)) = parse_size(&args[2]) else {
        eprintln!("error: size must look like 128x128 and be positive");
        return ExitCode::from(2);
    };
    let (policy, admission) = match parse_render_options(&args[3..]) {
        Ok(options) => options,
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

    let (png, degradations) = match render_source_to_png(&source, kind, w, h, policy, admission) {
        Ok(rendered) => rendered,
        Err(e) => {
            eprintln!("error: render failed: {e}");
            return ExitCode::FAILURE;
        }
    };

    if let Err(e) = std::fs::write(output, &png) {
        eprintln!("error: cannot write {output}: {e}");
        return ExitCode::FAILURE;
    }
    for degradation in &degradations {
        eprintln!("degraded: {degradation}");
    }
    if degradations.is_empty() {
        eprintln!(
            "rendered {input} -> {output} ({w}x{h}, {}, {} bytes)",
            policy.label(),
            png.len()
        );
    } else {
        eprintln!(
            "rendered {input} -> {output} ({w}x{h}, {}, {} degraded, {} bytes)",
            policy.label(),
            degradations.len(),
            png.len()
        );
    }
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

/// Parse the trailing options: an optional frame policy (`--base` or
/// `--time-ns <ns>`) and an optional admission (`--strict` or
/// `--best-effort`), in any order.
fn parse_render_options(args: &[String]) -> Result<(FramePolicy, Admission), String> {
    let mut admission = None;
    let mut policy_args = Vec::new();
    for arg in args {
        let flag = match arg.as_str() {
            "--strict" => Some(Admission::Strict),
            "--best-effort" => Some(Admission::BestEffort),
            _ => None,
        };
        match flag {
            Some(_) if admission.is_some() => {
                return Err(format!("duplicate or conflicting admission flag {arg:?}"));
            }
            Some(flag) => admission = Some(flag),
            None => policy_args.push(arg.clone()),
        }
    }
    let policy = parse_frame_policy(&policy_args)?;
    Ok((policy, admission.unwrap_or(Admission::BestEffort)))
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

/// Render one source through the engine of record and return the PNG plus
/// the degradations relevant to this request: skips always apply, a
/// samples-as-base declaration only describes `--time-ns` requests.
fn render_source_to_png(
    source: &str,
    kind: SourceKind,
    width: i32,
    height: i32,
    policy: FramePolicy,
    admission: Admission,
) -> Result<(Vec<u8>, Vec<websem::Degradation>), String> {
    // The requested raster is the initial viewport (SVG2 §8.2) a standalone
    // document resolves auto sizing against — the CLI's WxH is the window.
    let initial_viewport = websem::InitialViewport::new(width as f32, height as f32);
    let retained = match (kind, admission) {
        (SourceKind::Svg, Admission::Strict) => {
            websem::SvgFrameSource::from_standalone_svg(source, initial_viewport)
        }
        (SourceKind::Svg, Admission::BestEffort) => {
            websem::SvgFrameSource::from_standalone_svg_best_effort(source, initial_viewport)
        }
        (SourceKind::Html, Admission::Strict) => {
            websem::SvgFrameSource::from_html_inline_svg(source)
        }
        (SourceKind::Html, Admission::BestEffort) => {
            websem::SvgFrameSource::from_html_inline_svg_best_effort(source)
        }
    }
    .map_err(|error| error.to_string())?;
    let frame = match policy {
        FramePolicy::Base => retained.base_frame(),
        FramePolicy::Sample(time) => retained
            .sample_frame(time)
            .map_err(|error| error.to_string())?,
    };
    let degradations = retained
        .degradations()
        .iter()
        .filter(|degradation| match degradation.action() {
            // Both change what the raster shows, at Base as much as at a
            // sample: a skipped element leaves a hole, an ignored
            // declaration renders without a value Chromium honors.
            websem::DegradationAction::Skipped | websem::DegradationAction::DeclarationIgnored => {
                true
            }
            websem::DegradationAction::SamplesAsBase => {
                matches!(policy, FramePolicy::Sample(_))
            }
        })
        .cloned()
        .collect();
    frame_to_png(frame, width, height).map(|png| (png, degradations))
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
        assert_eq!(
            parse_render_options(&[]),
            Ok((FramePolicy::Base, Admission::BestEffort)),
            "the unqualified default is best-effort Base"
        );
        assert_eq!(
            parse_render_options(&["--base".to_string()]),
            Ok((FramePolicy::Base, Admission::BestEffort))
        );
        assert_eq!(
            parse_render_options(&["--strict".to_string()]),
            Ok((FramePolicy::Base, Admission::Strict))
        );
        assert_eq!(
            parse_render_options(&["--best-effort".to_string()]),
            Ok((FramePolicy::Base, Admission::BestEffort))
        );
        assert_eq!(
            parse_render_options(&[
                "--time-ns".to_string(),
                "-1".to_string(),
                "--strict".to_string()
            ]),
            Ok((
                FramePolicy::Sample(SampleTime::from_nanoseconds(-1)),
                Admission::Strict
            ))
        );
        assert_eq!(
            parse_render_options(&[
                "--strict".to_string(),
                "--time-ns".to_string(),
                "7".to_string()
            ]),
            Ok((
                FramePolicy::Sample(SampleTime::from_nanoseconds(7)),
                Admission::Strict
            )),
            "flag order is free"
        );
        assert!(
            parse_render_options(&["--strict".to_string(), "--best-effort".to_string()]).is_err(),
            "one admission flag at most"
        );
        assert!(parse_render_options(&["--time-ns".to_string()]).is_err());
        assert!(parse_render_options(&["--time-ns".to_string(), "1.5".to_string()]).is_err());
    }

    /// The strict admission keeps the refusal harness: inputs the retired
    /// mature route rendered refuse loudly under `--strict`, with the
    /// unsupported construct named — never wrong pixels. Each pinned reason
    /// is the current capability edge; the change that admits the construct
    /// must update the pin alongside its Chromium-baked fixtures.
    #[test]
    fn strict_admission_refuses_beyond_slice_by_name() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        for (relative, kind, size, named) in [
            (
                // The strokes rung consumed this fixture's stroked shapes and
                // its two `<line>`s, so strict now reaches the labels: the
                // first refusal is `<text>`, the next rung on the ladder.
                "fixtures/test-svg/L0/basic-shapes.svg",
                SourceKind::Svg,
                (500, 500),
                "unsupported element <text>",
            ),
            (
                // The probe's `.mark` class CSS-sizes the inline <svg>:
                // the root patrol names the cascaded width instead of
                // painting at a size Chromium would not use.
                "fixtures/test-html/probe/inline-svg-flex-probe.html",
                SourceKind::Html,
                (96, 48),
                "unsupported computed style: CSS width",
            ),
        ] {
            let input = root.join(relative);
            let source = std::fs::read_to_string(&input)
                .unwrap_or_else(|error| panic!("read {}: {error}", input.display()));
            let error = render_source_to_png(
                &source,
                kind,
                size.0,
                size.1,
                FramePolicy::Base,
                Admission::Strict,
            )
            .expect_err("beyond-slice input must refuse under --strict, not render");
            assert!(
                error.contains(named),
                "{relative} must refuse with the construct named; got: {error}"
            );
        }
    }

    /// The default admission renders the admitted subset of the same legacy
    /// corpus and declares every beyond-slice construct — holes with names,
    /// never wrong pixels for admitted nodes, exit success.
    #[test]
    fn best_effort_default_renders_the_admitted_subset_and_declares_the_rest() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");

        // basic-shapes.svg: the white background rect is admitted, the eight
        // groups are compiled and their beyond-slice *descendants* are declared
        // individually at nested paths, and `<title>`/`<desc>` declare nothing
        // because Chromium paints nothing for them either. After the points
        // rung the polygons and the polyline render too, so what remains is
        // the labels and one rounded rect (`rx` is not consumed).
        let source = std::fs::read_to_string(root.join("fixtures/test-svg/L0/basic-shapes.svg"))
            .expect("read basic-shapes");
        let (png, degradations) = render_source_to_png(
            &source,
            SourceKind::Svg,
            800,
            400,
            FramePolicy::Base,
            Admission::BestEffort,
        )
        .expect("best-effort renders the admitted subset");
        let raster = decode_png(&png).expect("decode PNG");
        assert_eq!(
            raster.at(400, 200),
            [255, 255, 255, 255],
            "the admitted background rect paints"
        );
        let skipped: Vec<&str> = degradations.iter().map(|d| d.path()).collect();
        assert_eq!(
            skipped,
            vec![
                "svg/g[1]/text[1]",
                "svg/g[2]/rect[1]",
                "svg/g[2]/text[1]",
                "svg/g[3]/text[1]",
                "svg/g[4]/text[1]",
                "svg/g[5]/text[1]",
                "svg/g[6]/text[1]",
                "svg/g[7]/text[1]",
                "svg/g[8]/text[1]",
            ],
            "every beyond-slice descendant is declared at its nested path"
        );
        assert!(
            degradations
                .iter()
                .any(|d| d.reason() == "unsupported element <text>"),
            "reasons name the construct"
        );
        assert!(
            !degradations.iter().any(|d| d.path() == "svg/g[1]"),
            "a container is compiled, not a hole"
        );

        // circle-fill-probe.svg: the basic-shapes rung admitted <circle> —
        // the probe pixel paints Chromium's green in both admissions,
        // byte-identically, with nothing degraded.
        let source =
            std::fs::read_to_string(root.join("fixtures/test-svg/probe/circle-fill-probe.svg"))
                .expect("read circle probe");
        let (png, degradations) = render_source_to_png(
            &source,
            SourceKind::Svg,
            64,
            64,
            FramePolicy::Base,
            Admission::BestEffort,
        )
        .expect("best-effort renders the admitted circle");
        let raster = decode_png(&png).expect("decode PNG");
        assert_eq!(raster.at(4, 4), [255, 255, 255, 255], "background paints");
        assert_eq!(
            raster.at(32, 32),
            [22, 163, 74, 255],
            "the admitted circle paints its fill"
        );
        assert!(
            degradations.is_empty(),
            "nothing degrades: {degradations:?}"
        );
        let (strict_png, _) = render_source_to_png(
            &source,
            SourceKind::Svg,
            64,
            64,
            FramePolicy::Base,
            Admission::Strict,
        )
        .expect("strict admits the circle too");
        assert_eq!(png, strict_png, "admissions are byte-identical");

        // path-fill-probe.svg: the paths rung admitted <path> — the probe
        // pixel paints Chromium's green in both admissions, byte-identically,
        // with nothing degraded.
        let source =
            std::fs::read_to_string(root.join("fixtures/test-svg/probe/path-fill-probe.svg"))
                .expect("read path probe");
        let (png, degradations) = render_source_to_png(
            &source,
            SourceKind::Svg,
            64,
            64,
            FramePolicy::Base,
            Admission::BestEffort,
        )
        .expect("best-effort renders the admitted path");
        let raster = decode_png(&png).expect("decode PNG");
        assert_eq!(raster.at(4, 4), [255, 255, 255, 255], "background paints");
        assert_eq!(
            raster.at(32, 40),
            [22, 163, 74, 255],
            "the admitted path paints its fill"
        );
        assert!(
            degradations.is_empty(),
            "nothing degrades: {degradations:?}"
        );
        let (strict_png, _) = render_source_to_png(
            &source,
            SourceKind::Svg,
            64,
            64,
            FramePolicy::Base,
            Admission::Strict,
        )
        .expect("strict admits the path too");
        assert_eq!(png, strict_png, "admissions are byte-identical");

        // polygon-fill-probe.svg: the points rung admitted <polygon> — the
        // probe pixel paints Chromium's green in both admissions,
        // byte-identically, with nothing degraded.
        let source =
            std::fs::read_to_string(root.join("fixtures/test-svg/probe/polygon-fill-probe.svg"))
                .expect("read polygon probe");
        let (png, degradations) = render_source_to_png(
            &source,
            SourceKind::Svg,
            64,
            64,
            FramePolicy::Base,
            Admission::BestEffort,
        )
        .expect("best-effort renders the admitted polygon");
        let raster = decode_png(&png).expect("decode PNG");
        assert_eq!(raster.at(4, 4), [255, 255, 255, 255], "background paints");
        assert_eq!(
            raster.at(32, 40),
            [22, 163, 74, 255],
            "the admitted polygon paints its fill"
        );
        assert!(
            degradations.is_empty(),
            "nothing degrades: {degradations:?}"
        );
        let (strict_png, _) = render_source_to_png(
            &source,
            SourceKind::Svg,
            64,
            64,
            FramePolicy::Base,
            Admission::Strict,
        )
        .expect("strict admits the polygon too");
        assert_eq!(png, strict_png, "admissions are byte-identical");

        // Document-level contracts hold in both admissions: the flex
        // probe's <svg> is CSS-sized (`.mark { width: 32px }`), and the
        // root patrol refuses the cascaded width by name rather than paint
        // at a size Chromium would not use; best-effort never invents the
        // canvas. (A standalone viewBox-only document sizes to WxH instead
        // — pinned below.)
        let source = std::fs::read_to_string(
            root.join("fixtures/test-html/probe/inline-svg-flex-probe.html"),
        )
        .expect("read flex probe");
        for admission in [Admission::BestEffort, Admission::Strict] {
            let error = render_source_to_png(
                &source,
                SourceKind::Html,
                96,
                48,
                FramePolicy::Base,
                admission,
            )
            .expect_err("root sizing is document-level in both admissions");
            assert!(
                error.contains("unsupported computed style: CSS width"),
                "{admission:?}: {error}"
            );
        }
    }

    /// The viewport rung at the host: a viewBox-only standalone document
    /// sizes to the requested `WxH` — the CLI's raster IS the initial
    /// viewport (SVG2 §8.2) — and both admissions produce byte-identical
    /// output with nothing degraded.
    #[test]
    fn viewbox_only_svg_sizes_to_the_requested_raster() {
        let source = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 16">
  <rect width="32" height="16" fill="#ffffff"/>
  <rect x="2" y="2" width="4" height="4" fill="#16a34a"/>
</svg>"##;
        let (png, degradations) = render_source_to_png(
            source,
            SourceKind::Svg,
            64,
            32,
            FramePolicy::Base,
            Admission::BestEffort,
        )
        .expect("viewBox-only renders by default");
        assert!(
            degradations.is_empty(),
            "sizing is the document contract, not a degradation"
        );
        let raster = decode_png(&png).expect("decode PNG");
        // 2x mapping: the green rect (2,2,4,4) covers (4,4)..(12,12).
        assert_eq!(
            raster.at(8, 8),
            [22, 163, 74, 255],
            "user units scale to the raster"
        );
        assert_eq!(
            raster.at(20, 8),
            [255, 255, 255, 255],
            "the background paints beyond the green rect"
        );

        let (strict_png, _) = render_source_to_png(
            source,
            SourceKind::Svg,
            64,
            32,
            FramePolicy::Base,
            Admission::Strict,
        )
        .expect("strict admits the same document");
        assert_eq!(png, strict_png, "admissions agree byte-for-byte");

        // A larger raster is a larger window: the same document scales.
        let (large, _) = render_source_to_png(
            source,
            SourceKind::Svg,
            256,
            128,
            FramePolicy::Base,
            Admission::BestEffort,
        )
        .expect("the raster is the initial viewport");
        let raster = decode_png(&large).expect("decode PNG");
        assert_eq!(
            raster.at(32, 32),
            [22, 163, 74, 255],
            "an 8x window maps the same rect"
        );
        assert_eq!(
            raster.at(250, 4),
            [255, 255, 255, 255],
            "the background fills the window"
        );
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
        let (first, degradations) = render_source_to_png(
            &source,
            SourceKind::Html,
            800,
            600,
            FramePolicy::Base,
            Admission::BestEffort,
        )
        .expect("the first inline SVG is the admitted rect");
        assert!(
            degradations.is_empty(),
            "the first SVG is fully admitted; the first-SVG-only boundary is \
             a pinned entry contract, not a degradation"
        );
        let (second, _) = render_source_to_png(
            &source,
            SourceKind::Html,
            800,
            600,
            FramePolicy::Base,
            Admission::Strict,
        )
        .expect("repeat render (strict admission)");
        assert_eq!(first, second, "encoded determinism across admissions");
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
            let (first, degradations) = render_source_to_png(
                &source,
                kind,
                size.0,
                size.1,
                FramePolicy::Base,
                Admission::BestEffort,
            )
            .unwrap_or_else(|error| panic!("render {relative}: {error}"));
            assert!(
                degradations.is_empty(),
                "{relative} is fully admitted; nothing degrades"
            );
            let (second, _) = render_source_to_png(
                &source,
                kind,
                size.0,
                size.1,
                FramePolicy::Base,
                Admission::Strict,
            )
            .unwrap_or_else(|error| panic!("repeat {relative}: {error}"));
            assert_eq!(first, second, "{relative} byte-identical across admissions");
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
                "fixtures/web-first/animation/chromium/svg-rect-x-animation/base.png",
            ),
            (
                FramePolicy::Sample(SampleTime::from_nanoseconds(0)),
                20,
                "fixtures/web-first/animation/chromium/svg-rect-x-animation/sample-0ns.png",
            ),
            (
                FramePolicy::Sample(SampleTime::from_nanoseconds(1_000_000_000)),
                32,
                "fixtures/web-first/animation/chromium/svg-rect-x-animation/\
                 sample-1000000000ns.png",
            ),
            (
                FramePolicy::Sample(SampleTime::from_nanoseconds(2_000_000_000)),
                44,
                "fixtures/web-first/animation/chromium/svg-rect-x-animation/\
                 sample-2000000000ns.png",
            ),
        ] {
            let (first, degradations) = render_source_to_png(
                &source,
                SourceKind::Svg,
                64,
                32,
                policy,
                Admission::BestEffort,
            )
            .unwrap_or_else(|error| panic!("render {policy:?}: {error}"));
            assert!(
                degradations.is_empty(),
                "the admitted x animation degrades nothing"
            );
            let (second, _) =
                render_source_to_png(&source, SourceKind::Svg, 64, 32, policy, Admission::Strict)
                    .unwrap_or_else(|error| panic!("repeat {policy:?}: {error}"));
            assert_eq!(first, second, "{policy:?} byte-identical across admissions");
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

    /// The cub scene is the sampling slice over a whole composition: seventeen
    /// nodes of containers, path curves, strokes and a `<line>`, sized from the
    /// requested raster because its root is viewBox-only. Every frame is
    /// byte-exact to its own Chromium oracle, and — the point of a scene — an
    /// exact-time sample moves the animated node's pixels and no others.
    #[test]
    fn the_cub_scene_samples_move_only_the_animated_node() {
        const TEAL: [u8; 4] = [0x0f, 0x76, 0x6e, 0xff];
        const BACKDROP: [u8; 4] = [0xfe, 0xf3, 0xc7, 0xff];
        /// `viewBox="0 0 48 48"` rendered into the requested 96x96 raster.
        const SCALE: i32 = 2;
        /// The animated rect is authored `y="38" height="5"`, in user units.
        const BLOCK_ROWS: std::ops::Range<i32> = 38 * SCALE..43 * SCALE;
        const AUTHORED_X: i32 = 12;

        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let read = |relative: &str| {
            std::fs::read_to_string(root.join(relative))
                .unwrap_or_else(|error| panic!("read {relative}: {error}"))
        };
        let animated = read("fixtures/web-first/animation/svg-scene-cub-animation.svg");
        let projection = read("fixtures/web-first/animation/svg-scene-cub-base.svg");
        let render = |source: &str, policy: FramePolicy| {
            let (best_effort, degradations) = render_source_to_png(
                source,
                SourceKind::Svg,
                96,
                96,
                policy,
                Admission::BestEffort,
            )
            .unwrap_or_else(|error| panic!("render {policy:?}: {error}"));
            assert!(
                degradations.is_empty(),
                "the cub scene is fully admitted; {policy:?} declares {degradations:?}"
            );
            let (strict, _) =
                render_source_to_png(source, SourceKind::Svg, 96, 96, policy, Admission::Strict)
                    .unwrap_or_else(|error| panic!("strict render {policy:?}: {error}"));
            assert_eq!(
                best_effort, strict,
                "{policy:?} byte-identical across admissions"
            );
            best_effort
        };

        let mut frames = Vec::new();
        for (policy, expected_x, oracle) in [
            (
                FramePolicy::Base,
                AUTHORED_X,
                "fixtures/web-first/animation/chromium/svg-scene-cub/base.png",
            ),
            (
                FramePolicy::Sample(SampleTime::from_nanoseconds(0)),
                6,
                "fixtures/web-first/animation/chromium/svg-scene-cub/sample-0ns.png",
            ),
            (
                FramePolicy::Sample(SampleTime::from_nanoseconds(1_000_000_000)),
                22,
                "fixtures/web-first/animation/chromium/svg-scene-cub/sample-1000000000ns.png",
            ),
            (
                FramePolicy::Sample(SampleTime::from_nanoseconds(2_000_000_000)),
                38,
                "fixtures/web-first/animation/chromium/svg-scene-cub/sample-2000000000ns.png",
            ),
            (
                FramePolicy::Sample(SampleTime::from_nanoseconds(3_000_000_000)),
                38,
                "fixtures/web-first/animation/chromium/svg-scene-cub/sample-3000000000ns.png",
            ),
        ] {
            let png = render(&animated, policy);
            let raster = decode_png(&png).expect("decode cub PNG");
            assert_eq!((raster.width, raster.height), (96, 96));
            let expected = decode_png(
                &std::fs::read(root.join(oracle))
                    .unwrap_or_else(|error| panic!("read {oracle}: {error}")),
            )
            .unwrap_or_else(|| panic!("decode {oracle}"));
            assert_eq!(raster.pixels, expected.pixels, "{policy:?} Chromium RGBA");

            // The block is where the resolved `x` puts it, and nowhere else.
            let row = BLOCK_ROWS.start + 2;
            assert_eq!(
                raster.at(expected_x * SCALE + 4, row),
                TEAL,
                "{policy:?} paints the block at x={expected_x}"
            );
            if expected_x != AUTHORED_X {
                assert_eq!(
                    raster.at(AUTHORED_X * SCALE + 4, row),
                    BACKDROP,
                    "{policy:?} leaves the authored position empty"
                );
            }
            frames.push((format!("{policy:?}"), raster));
        }

        // Rendering the animated document at Base IS the static projection —
        // the same pixels the separate Base source file produces.
        assert_eq!(
            decode_png(&render(&projection, FramePolicy::Base))
                .expect("decode projection PNG")
                .pixels,
            frames[0].1.pixels,
            "the Base view of the animated scene is its static projection"
        );

        // Time touches one node: every pixel that differs between the Base
        // frame and a sample lies in the animated rect's own rows. Seventeen
        // nodes of curves and strokes render identically at every time.
        for (label, sampled) in &frames[1..] {
            let mut differing = 0usize;
            for (index, (base_pixel, sample_pixel)) in frames[0]
                .1
                .pixels
                .chunks_exact(4)
                .zip(sampled.pixels.chunks_exact(4))
                .enumerate()
            {
                if base_pixel == sample_pixel {
                    continue;
                }
                differing += 1;
                let y = index as i32 / 96;
                assert!(
                    BLOCK_ROWS.contains(&y),
                    "{label} differs from Base at row {y}, outside the animated node"
                );
            }
            assert_ne!(
                differing, 0,
                "{label} must move the block off its authored x"
            );
        }

        // The animation freezes: the last two samples are the same frame.
        assert_eq!(frames[3].1.pixels, frames[4].1.pixels, "fill=freeze holds");
        for pair in [(1, 2), (1, 3), (2, 3)] {
            assert_ne!(
                frames[pair.0].1.pixels, frames[pair.1].1.pixels,
                "samples {} and {} must differ",
                frames[pair.0].0, frames[pair.1].0
            );
        }
    }

    /// Sampling inline HTML refuses loudly under `--strict` through websem's
    /// own dynamic inventory; the default admission samples as the Base view
    /// and declares it. A document with no inline SVG refuses at
    /// construction in both admissions.
    #[test]
    fn html_sampling_refuses_strictly_and_falls_back_by_default() {
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
            Admission::Strict,
        )
        .expect_err("inline-HTML sampling is not admitted under --strict");
        assert!(
            error.contains("inline HTML"),
            "the refusal names the entry: {error}"
        );

        let (sampled, degradations) = render_source_to_png(
            &source,
            SourceKind::Html,
            64,
            64,
            FramePolicy::Sample(SampleTime::ZERO),
            Admission::BestEffort,
        )
        .expect("the default admission samples as the Base view");
        assert_eq!(degradations.len(), 1, "declared exactly once");
        assert!(
            degradations[0].reason().contains("inline HTML"),
            "the declaration names the entry: {}",
            degradations[0].reason()
        );
        let (base, base_degradations) = render_source_to_png(
            &source,
            SourceKind::Html,
            64,
            64,
            FramePolicy::Base,
            Admission::BestEffort,
        )
        .expect("Base render");
        assert!(
            base_degradations.is_empty(),
            "a Base request is not degraded by the sampling boundary"
        );
        assert_eq!(sampled, base, "the fallback sample IS the Base view");

        for admission in [Admission::BestEffort, Admission::Strict] {
            let error = render_source_to_png(
                "<html></html>",
                SourceKind::Html,
                64,
                32,
                FramePolicy::Base,
                admission,
            )
            .expect_err("a document with no inline SVG refuses at construction");
            assert!(
                error.contains("no <svg> element"),
                "{admission:?} names the missing root: {error}"
            );
        }
    }
}
