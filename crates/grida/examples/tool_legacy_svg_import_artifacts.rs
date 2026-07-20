//! Produce the two exact artifacts consumed by the legacy SVG import sweep.
//!
//! This is an internal equivalence driver, not a renderer conformance tool.
//! It accepts one explicit SVG and surface, fails on the first error, writes
//! raw `.grida` FlatBuffers bytes, decodes those exact bytes, and exports the
//! decoded legacy scene through the CPU raster path with a transparent
//! background and embedded fonts only.

use grida::export::{
    export_as_image::export_node_as_image, ExportAsImage, ExportAsPNG, ExportSize, Exported,
};
use grida::import::svg::grida::svg_to_grida_bytes;
use grida::io::io_grida_file;
use grida::resources::ByteStore;
use grida::runtime::{font_repository::FontRepository, image_repository::ImageRepository};
use math2::rect::Rectangle;
use std::error::Error;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(ErrorKind::InvalidInput, message.into())
}

fn parse_edge(raw: &str, name: &str) -> Result<u32, io::Error> {
    let value = raw
        .parse::<u32>()
        .map_err(|error| invalid(format!("invalid {name} {raw:?}: {error}")))?;
    if value == 0 || value > i32::MAX as u32 {
        return Err(invalid(format!(
            "{name} must be in 1..={}: {value}",
            i32::MAX
        )));
    }
    Ok(value)
}

fn create_parent(path: &Path) -> Result<(), io::Error> {
    let parent = path
        .parent()
        .ok_or_else(|| invalid(format!("output has no parent: {}", path.display())))?;
    std::fs::create_dir_all(parent)
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args_os();
    let program = args
        .next()
        .unwrap_or_else(|| "tool_legacy_svg_import_artifacts".into());
    let svg_path = PathBuf::from(args.next().ok_or_else(|| {
        invalid(format!(
            "usage: {} <input.svg> <output.grida> <output.png> <width> <height>",
            Path::new(&program).display()
        ))
    })?);
    let grida_path = PathBuf::from(
        args.next()
            .ok_or_else(|| invalid("missing .grida output path"))?,
    );
    let png_path = PathBuf::from(
        args.next()
            .ok_or_else(|| invalid("missing PNG output path"))?,
    );
    let width = parse_edge(
        &args
            .next()
            .ok_or_else(|| invalid("missing surface width"))?
            .to_string_lossy(),
        "surface width",
    )?;
    let height = parse_edge(
        &args
            .next()
            .ok_or_else(|| invalid("missing surface height"))?
            .to_string_lossy(),
        "surface height",
    )?;
    if args.next().is_some() {
        return Err(invalid("unexpected trailing argument").into());
    }
    if grida_path == png_path {
        return Err(invalid(".grida and PNG outputs must differ").into());
    }
    for output in [&grida_path, &png_path] {
        if output.exists() {
            return Err(invalid(format!(
                "refusing to overwrite existing artifact: {}",
                output.display()
            ))
            .into());
        }
    }

    let svg = std::fs::read_to_string(&svg_path)?;
    let grida = svg_to_grida_bytes(&svg).map_err(|error| {
        io::Error::new(
            ErrorKind::InvalidData,
            format!("SVG import failed for {}: {error}", svg_path.display()),
        )
    })?;
    let scene = io_grida_file::decode(&grida).map_err(|error| {
        io::Error::new(
            ErrorKind::InvalidData,
            format!(
                "decoding generated .grida failed for {}: {error}",
                svg_path.display()
            ),
        )
    })?;

    let store = Arc::new(Mutex::new(ByteStore::new()));
    let mut fonts = FontRepository::new(store.clone());
    fonts.register_embedded_fonts();
    let images = ImageRepository::new(store);
    let exported = export_node_as_image(
        &scene,
        &fonts,
        &images,
        ExportSize {
            width: width as f32,
            height: height as f32,
        },
        Rectangle {
            x: 0.0,
            y: 0.0,
            width: width as f32,
            height: height as f32,
        },
        ExportAsImage::PNG(ExportAsPNG::default()),
    )
    .ok_or_else(|| {
        io::Error::other(format!(
            "legacy CPU PNG export failed for {} at {width}x{height}",
            svg_path.display()
        ))
    })?;
    let png = match exported {
        Exported::PNG(data) => data,
        _ => return Err(io::Error::other("PNG export returned another format").into()),
    };

    create_parent(&grida_path)?;
    create_parent(&png_path)?;
    std::fs::write(&grida_path, grida)?;
    std::fs::write(&png_path, png)?;
    Ok(())
}
