//! Turning the shipped artwork into the pixels a panel wants.
//!
//! The controller is rendered from its SVG at run time rather than shipped as
//! a set of PNGs. A panel asks for whichever size its bar happens to be, and
//! the same one path has to come back white, green, or black depending on the
//! bar's colour and whether a game is running. Committing that many rasters is
//! more files than the renderer that replaces them.

use resvg::tiny_skia::{Color, FillRule, IntSize, Paint, PathBuilder, Pixmap, Transform};
use resvg::usvg::{Options, Tree};

use crate::infra::errors::IconError;
use crate::infra::ink::Ink;

/// The shipped artwork, anchored to the crate root so moving this file cannot
/// silently break the include.
const CONTROLLER: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/gameready.svg"));

/// The sizes a panel is offered. StatusNotifierItem lets the host pick, and
/// these cover the bar heights the common desktops use.
const SIZES: [u32; 3] = [22, 32, 48];

/// How wide the status dot beside a menu row is drawn, in pixels.
const DOT: u32 = 16;

/// Half a pixel in from the dot's edge, so antialiasing has somewhere to land
/// and the circle does not read as a clipped square at 16px.
const DOT_INSET: f32 = 1.5;

/// The controller, at every size a panel might ask for, in `ink`.
pub fn controller(ink: Ink) -> Result<Vec<ksni::Icon>, IconError> {
    let tree = Tree::from_str(CONTROLLER, &Options::default())?;
    SIZES
        .iter()
        .map(|&size| render(&tree, size).map(|pixmap| tint(&pixmap, ink)))
        .collect()
}

/// The dot drawn beside one menu row, as the PNG bytes dbusmenu wants.
pub fn dot(ink: Ink) -> Result<Vec<u8>, IconError> {
    let mut pixmap = Pixmap::new(DOT, DOT).ok_or(IconError::Allocate { size: DOT })?;
    let centre = DOT as f32 / 2.0;
    let circle = PathBuilder::from_circle(centre, centre, centre - DOT_INSET)
        .ok_or(IconError::Allocate { size: DOT })?;

    let mut paint = Paint::default();
    paint.set_color(colour(ink));
    paint.anti_alias = true;
    pixmap.fill_path(
        &circle,
        &paint,
        FillRule::Winding,
        Transform::identity(),
        None,
    );

    pixmap
        .encode_png()
        .map_err(|_| IconError::Encode { size: DOT })
}

/// Rasterizes the artwork to a square of `size`, fitted to its wider side.
fn render(tree: &Tree, size: u32) -> Result<Pixmap, IconError> {
    let mut pixmap = Pixmap::new(size, size).ok_or(IconError::Allocate { size })?;
    let square = IntSize::from_wh(size, size).ok_or(IconError::Allocate { size })?;
    let fitted = tree.size().to_int_size().scale_to(square);
    let scale = f32::from(u16::try_from(fitted.width()).unwrap_or(u16::MAX)) / tree.size().width();
    // Centred vertically: the controller is much wider than it is tall, so a
    // top-left render leaves it sitting on the bar's upper edge.
    let top = (size as f32 - tree.size().height() * scale) / 2.0;
    resvg::render(
        tree,
        Transform::from_scale(scale, scale).post_translate(0.0, top),
        &mut pixmap.as_mut(),
    );
    Ok(pixmap)
}

/// Repaints a rendered controller in one flat colour, keeping its alpha.
///
/// The artwork is a single black fill with no colour of its own, which is
/// invisible on the dark bar most desktops ship. Using the render purely as a
/// mask means one asset covers every ink without editing SVG text, which is
/// the fragile way to do this.
fn tint(pixmap: &Pixmap, ink: Ink) -> ksni::Icon {
    let colour = colour(ink);
    let mut argb = Vec::with_capacity(pixmap.data().len());
    for pixel in pixmap.pixels() {
        // ARGB32, network byte order, premultiplied to match the alpha the
        // renderer already applied.
        let alpha = pixel.alpha();
        let lit = f32::from(alpha) / 255.0;
        argb.push(alpha);
        argb.push(premultiplied(colour.red(), lit));
        argb.push(premultiplied(colour.green(), lit));
        argb.push(premultiplied(colour.blue(), lit));
    }
    ksni::Icon {
        width: i32::try_from(pixmap.width()).unwrap_or(i32::MAX),
        height: i32::try_from(pixmap.height()).unwrap_or(i32::MAX),
        data: argb,
    }
}

/// One colour channel scaled by the coverage the renderer produced.
fn premultiplied(channel: f32, lit: f32) -> u8 {
    (channel * lit * 255.0).round().clamp(0.0, 255.0) as u8
}

/// What each ink is worth in pixels.
fn colour(ink: Ink) -> Color {
    let (red, green, blue) = ink.rgb();
    Color::from_rgba8(red, green, blue, u8::MAX)
}

#[cfg(test)]
#[path = "icon_test.rs"]
mod icon_test;
