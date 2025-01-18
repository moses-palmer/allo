use std::fs::read_to_string;
use std::path::{Path, PathBuf};

use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg::{Options, Tree};

/// The errors that can occur when converting images.
#[derive(Debug)]
struct ConversionError(String);

impl<T> From<T> for ConversionError
where
    T: ToString,
{
    fn from(source: T) -> Self {
        Self(source.to_string())
    }
}

/// Converts an SVG image to PNG.
///
/// # Arguments
/// *  `source` - The source SVG.
/// *  `target` - The target PNG.
/// *  `size` - An explicit size. If this is not specified, the dimensions are
///    extracted from the source image.
///
/// # Panics
/// This function will panic if any of the paths cannot be converted to a
/// string.
fn svg_to_png<P>(
    source: P,
    target: P,
    size: Option<(u32, u32)>,
) -> Result<(), ConversionError>
where
    P: AsRef<Path>,
{
    println!(
        "cargo:rerun-if-changed={}",
        source.as_ref().to_str().unwrap()
    );
    println!(
        "cargo:rerun-if-changed={}",
        target.as_ref().to_str().unwrap()
    );

    let svg = Tree::from_str(&read_to_string(source)?, &Options::default())?;
    let size = size.unwrap_or_else(|| {
        (
            svg.size().width().ceil() as u32,
            svg.size().height().ceil() as u32,
        )
    });

    let mut pixmap = Pixmap::new(size.0, size.1).ok_or_else(|| {
        format!("invalid dimensions: {} × {}", size.0, size.1)
    })?;
    resvg::render(&svg, Transform::default(), &mut pixmap.as_mut());
    pixmap.save_png(target)?;

    Ok(())
}

pub fn main() {
    let frontend = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("frontend");
    let templates = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("templates");
    svg_to_png(
        frontend.join("icon.svg"),
        templates.join("logo.png"),
        Some((120, 120)),
    )
    .unwrap();
}
