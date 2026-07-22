//! A thin CLI host for the Web-first pipeline: render an SVG or an
//! HTML-with-inline-SVG file to a PNG.
//!
//! Meaning lives in the engine — this host does only argument parsing, file
//! I/O, and choosing the raster size. It calls `websem` to compile the source
//! into the source-neutral `rframe::Frame`, then `rframe` to render + encode.
//! Unsupported constructs fail explicitly (a non-zero exit + a clear message);
//! there are no silent shims.
//!
//! Usage:
//!   cargo run -p websem --example render -- <input.svg|input.html> <out.png> [WxH]
//!
//! Examples:
//!   cargo run -p websem --example render -- \
//!     fixtures/web-first/svg-currentcolor-rect.svg /tmp/out.png
//!   cargo run -p websem --example render -- \
//!     fixtures/web-first/html-inline-svg-currentcolor-rect.html /tmp/out.png 128x128

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 2 || args.len() > 3 {
        eprintln!(
            "usage: render <input.svg|input.html> <out.png> [WxH]\n\
             renders the Web-first pipeline (websem compile -> rframe::Frame -> PNG)."
        );
        return ExitCode::from(2);
    }
    let input = &args[0];
    let output = &args[1];
    let size = args.get(2).map(|s| parse_size(s));

    let source = match std::fs::read_to_string(input) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read {input}: {e}");
            return ExitCode::FAILURE;
        }
    };

    let is_html = input
        .rsplit('.')
        .next()
        .is_some_and(|e| e.eq_ignore_ascii_case("html") || e.eq_ignore_ascii_case("htm"));
    let compiled = if is_html {
        websem::compile_html_inline_svg(&source)
    } else {
        websem::compile_standalone_svg(&source)
    };

    let frame = match compiled {
        Ok(frame) => frame,
        Err(e) => {
            // Explicit failure — the Web-first slice supports only what it
            // supports, and says so rather than shimming.
            eprintln!("error: unsupported source: {e}");
            return ExitCode::FAILURE;
        }
    };

    let (w, h) = match size {
        Some(Some(wh)) => wh,
        Some(None) => {
            eprintln!("error: size must look like 128x128");
            return ExitCode::FAILURE;
        }
        None => (
            frame.bounds.width.ceil() as i32,
            frame.bounds.height.ceil() as i32,
        ),
    };
    if w <= 0 || h <= 0 {
        eprintln!("error: non-positive raster size {w}x{h} (give an explicit WxH)");
        return ExitCode::FAILURE;
    }

    let png = rframe::render_png(&frame, w, h);
    if let Err(e) = std::fs::write(output, &png) {
        eprintln!("error: cannot write {output}: {e}");
        return ExitCode::FAILURE;
    }
    eprintln!(
        "rendered {input} -> {output} ({w}x{h}, {} nodes, {} bytes)",
        frame.nodes.len(),
        png.len()
    );
    ExitCode::SUCCESS
}

fn parse_size(s: &str) -> Option<(i32, i32)> {
    let (w, h) = s.split_once(['x', 'X'])?;
    Some((w.trim().parse().ok()?, h.trim().parse().ok()?))
}
