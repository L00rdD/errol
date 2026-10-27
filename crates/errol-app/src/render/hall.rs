//! The Great Hall's sprites, for the hogwarts look, after the home page's own
//! Great Hall: the night sky, the stars, the clouds, the candles' wax, a flame
//! and its halo, the four house banners and their crests, the parchment, the
//! wax seals, the gilt frame of the portraits, the house-point hourglasses,
//! the owl and its letter.
//!
//! Everything that does not move by itself is drawn once, into GPU images,
//! when the look starts (`Hall::bake`). A frame then only places images with
//! a transform; what flickers or flows (flames' shape, gem piles, sparks) is
//! a transform of a baked image or a few small paths.

// drawing code: x, y, w, h for a box, c for the canvas, as in the SVG it follows
#![allow(clippy::many_single_char_names)]

use femtovg::renderer::OpenGl;
use femtovg::{
    Align, Baseline, Canvas, Color, CompositeOperation, ImageFlags, ImageId, LineCap, Paint, Path,
    PixelFormat, RenderTarget, Solidity,
};

use super::crests::{self, Ink, Mark};
use super::svg;
use super::{Fonts, Gfx};
use crate::anim::unit;
use crate::look::{fade, rgb};

/// The parchment's size: the idle cards'.
pub const SHEET: (f32, f32) = (490.0, 350.0);
/// Room around the sheet, for its shadow and its torn edges.
pub const SHEET_MARGIN: f32 = 18.0;
/// The gilt frame's width around a 200 x 300 picture, and the room above it
/// for the shell.
pub const FRAME: f32 = 22.0;
pub const FRAME_TOP: f32 = 6.0;
/// Candle heights, in units of a candle 24 wide.
pub const CANDLE_HEIGHTS: [f32; 3] = [112.0, 140.0, 168.0];
/// The wick's tip, where the flame sits, in the same units.
pub const WICK: f32 = 9.0;
/// The flame image is 24 x 48 units; its base sits on the wick.
pub const FLAME_BASE: f32 = 41.6;
/// The seal image: a 60-unit seal with room for its shadow.
const SEAL_BOX: f32 = 68.0;
/// The hourglass glass, as on the home page (100 x 260 units).
const GLASS: &str = "M24 30C24 70 31 100 46.5 123L46.5 135C31 158 24 188 24 228H76C76 188 69 158 53.5 135L53.5 123C69 100 76 70 76 30Z";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum House {
    Gryffindor,
    Slytherin,
    Ravenclaw,
    Hufflepuff,
}

impl House {
    pub fn index(self) -> usize {
        self as usize
    }

    fn cloth(self) -> Color {
        rgb([0x8f1414, 0x1d5133, 0x1c2c6e, 0xd9a81e][self.index()])
    }

    fn trim(self) -> Color {
        rgb([0xd9a93f, 0xb9c0c9, 0xb07a3e, 0x2a2418][self.index()])
    }

    fn metal(self) -> Color {
        rgb([0xe8bd52, 0xd3d8df, 0xc98d4c, 0x1d1a14][self.index()])
    }

    /// Gems: base, highlight, dark.
    fn gems(self) -> [Color; 3] {
        let [base, light, dark] = [
            [0xc8102e, 0xff7a8a, 0x5a0010],
            [0x0f8a4b, 0x7dffb8, 0x03361c],
            [0x1d4fd0, 0x8fb4ff, 0x081c5a],
            [0xf2c230, 0xfff3b0, 0x8a5e00],
        ][self.index()];
        [rgb(base), rgb(light), rgb(dark)]
    }

    fn wax(self) -> Color {
        rgb([0x9c1620, 0x1e6a3c, 0x23408e, 0xc99512][self.index()])
    }

    fn name(self) -> &'static str {
        ["GRYFFINDOR", "SLYTHERIN", "RAVENCLAW", "HUFFLEPUFF"][self.index()]
    }

    fn marks(self) -> &'static [Mark] {
        [
            crests::GRYFFINDOR,
            crests::SLYTHERIN,
            crests::RAVENCLAW,
            crests::HUFFLEPUFF,
        ][self.index()]
    }

    /// The hourglass' metal, light to dark: gold, silver for Slytherin,
    /// bronze for Ravenclaw.
    fn frame_metal(self) -> [Color; 4] {
        let stops = match self {
            Self::Slytherin => [0xffffff, 0xd3d8df, 0x8d96a3, 0x3c424c],
            Self::Ravenclaw => [0xffe0b8, 0xc98d4c, 0x8a5a26, 0x4a2e10],
            Self::Gryffindor | Self::Hufflepuff => [0xfff3bf, 0xf0c75a, 0xc28a26, 0x6e4710],
        };
        stops.map(rgb)
    }
}

/// The baked images of the Great Hall.
pub struct Hall {
    pub sky: ImageId,
    pub stars: [ImageId; 2],
    pub clouds: [ImageId; 2],
    pub candles: [ImageId; 3],
    pub flame: ImageId,
    pub halo: ImageId,
    pub banners: [ImageId; 4],
    sheet: ImageId,
    seals: [ImageId; 4],
    portrait: ImageId,
    hourglasses: [ImageId; 4],
    glass: ImageId,
    gems: [ImageId; 4],
    pub owl: [ImageId; 3],
    pub letter: ImageId,
}

fn clear() -> Color {
    Color::rgbaf(0.0, 0.0, 0.0, 0.0)
}

fn rgba(hex: u32, alpha: f32) -> Color {
    fade(rgb(hex), alpha)
}

fn black(alpha: f32) -> Color {
    Color::rgbaf(0.0, 0.0, 0.0, alpha)
}

fn white(alpha: f32) -> Color {
    Color::rgbaf(1.0, 1.0, 1.0, alpha)
}

/// Draws `draw` (in units) into a new image of `w` x `h` units at `scale`
/// pixels per unit. The image holds premultiplied colours, as femtovg renders,
/// upside down, as OpenGL renders into a texture.
fn bake(
    canvas: &mut Canvas<OpenGl>,
    w: f32,
    h: f32,
    scale: f32,
    flags: ImageFlags,
    draw: impl FnOnce(&mut Canvas<OpenGl>),
) -> Result<ImageId, String> {
    let (pw, ph) = ((w * scale).ceil() as u32, (h * scale).ceil() as u32);
    let image = canvas
        .create_image_empty(
            pw as usize,
            ph as usize,
            PixelFormat::Rgba8,
            // rendered by OpenGL bottom row first: drawn back the right way up
            flags | ImageFlags::PREMULTIPLIED | ImageFlags::FLIP_Y,
        )
        .map_err(|error| format!("bake: {error:?}"))?;
    canvas.set_render_target(RenderTarget::Image(image));
    canvas.save();
    canvas.reset_transform();
    canvas.reset_scissor();
    canvas.set_global_alpha(1.0);
    canvas.clear_rect(0, 0, pw, ph, clear());
    canvas.scale(scale, scale);
    draw(canvas);
    canvas.restore();
    canvas.set_render_target(RenderTarget::Screen);
    Ok(image)
}

fn fill(canvas: &mut Canvas<OpenGl>, path: &Path, color: Color) {
    canvas.fill_path(path, &Paint::color(color));
}

fn stroke(canvas: &mut Canvas<OpenGl>, path: &Path, color: Color, width: f32) {
    canvas.stroke_path(
        path,
        &Paint::color(color)
            .with_line_width(width)
            .with_line_cap(LineCap::Round),
    );
}

fn circle(x: f32, y: f32, r: f32) -> Path {
    let mut path = Path::new();
    path.circle(x, y, r);
    path
}

fn rect(x: f32, y: f32, w: f32, h: f32) -> Path {
    let mut path = Path::new();
    path.rect(x, y, w, h);
    path
}

impl Hall {
    /// Draws every sprite once. Call `flush` on the canvas before using them.
    pub fn bake(canvas: &mut Canvas<OpenGl>, fonts: &Fonts) -> Result<Self, String> {
        let plain = ImageFlags::empty();
        let houses = |canvas: &mut Canvas<OpenGl>,
                      w: f32,
                      h: f32,
                      scale: f32,
                      flags: ImageFlags,
                      draw: &dyn Fn(&mut Canvas<OpenGl>, House)|
         -> Result<[ImageId; 4], String> {
            Ok([
                bake(canvas, w, h, scale, flags, |c| draw(c, House::Gryffindor))?,
                bake(canvas, w, h, scale, flags, |c| draw(c, House::Slytherin))?,
                bake(canvas, w, h, scale, flags, |c| draw(c, House::Ravenclaw))?,
                bake(canvas, w, h, scale, flags, |c| draw(c, House::Hufflepuff))?,
            ])
        };
        let repeat = ImageFlags::REPEAT_X | ImageFlags::REPEAT_Y;
        Ok(Self {
            sky: bake(canvas, 800.0, 480.0, 1.0, plain, sky)?,
            stars: [
                bake(canvas, 800.0, 374.0, 1.0, plain, |c| stars(c, 0))?,
                bake(canvas, 800.0, 374.0, 1.0, plain, |c| stars(c, 1))?,
            ],
            clouds: [
                bake(canvas, 1400.0, 170.0, 1.0, plain, |c| clouds(c, 0))?,
                bake(canvas, 1400.0, 170.0, 1.0, plain, |c| clouds(c, 1))?,
            ],
            candles: [
                bake(canvas, 24.0, CANDLE_HEIGHTS[0], 2.0, plain, |c| {
                    candle(c, 0);
                })?,
                bake(canvas, 24.0, CANDLE_HEIGHTS[1], 2.0, plain, |c| {
                    candle(c, 1);
                })?,
                bake(canvas, 24.0, CANDLE_HEIGHTS[2], 2.0, plain, |c| {
                    candle(c, 2);
                })?,
            ],
            flame: bake(canvas, 24.0, 48.0, 2.5, plain, flame)?,
            halo: bake(canvas, 128.0, 128.0, 1.0, plain, halo)?,
            banners: houses(canvas, 100.0, 316.0, 0.6, plain, &|c, house| {
                banner(c, house, fonts);
            })?,
            sheet: bake(
                canvas,
                SHEET.0 + 2.0 * SHEET_MARGIN,
                SHEET.1 + 2.0 * SHEET_MARGIN,
                1.0,
                plain,
                sheet,
            )?,
            seals: houses(canvas, SEAL_BOX, SEAL_BOX, 2.0, plain, &|c, house| {
                seal(c, house, fonts);
            })?,
            portrait: bake(
                canvas,
                200.0 + 2.0 * FRAME,
                300.0 + 2.0 * FRAME + FRAME_TOP,
                1.0,
                plain,
                portrait,
            )?,
            hourglasses: houses(canvas, 100.0, 260.0, 0.8, plain, &hourglass)?,
            glass: bake(canvas, 100.0, 260.0, 0.8, plain, glass_front)?,
            gems: houses(canvas, 9.0, 8.0, 4.0, repeat, &gems)?,
            owl: [
                bake(canvas, 110.0, 80.0, 1.0, plain, |c| owl(c, -30.0, 1.0))?,
                bake(canvas, 110.0, 80.0, 1.0, plain, |c| owl(c, -2.0, 0.78))?,
                bake(canvas, 110.0, 80.0, 1.0, plain, |c| owl(c, 26.0, 0.55))?,
            ],
            letter: bake(canvas, 30.0, 21.0, 2.0, plain, letter)?,
        })
    }
}

// ---------- the ceiling ----------

fn sky(c: &mut Canvas<OpenGl>) {
    let all = rect(0.0, 0.0, 800.0, 480.0);
    let paint = Paint::linear_gradient_stops(
        0.0,
        0.0,
        0.0,
        480.0,
        [
            (0.0, rgb(0x0e1530)),
            (0.16, rgb(0x121a37)),
            (0.40, rgb(0x0e1127)),
            (0.68, rgb(0x0b0a12)),
            (1.0, rgb(0x0b0a12)),
        ],
    );
    c.fill_path(&all, &paint);
    let moon = Paint::radial_gradient(640.0, 14.0, 0.0, 330.0, rgba(0xc4d2f8, 0.16), clear());
    c.fill_path(&all, &moon);
}

/// A layer of stars over the top of the hall, fading out by 78 % of the
/// screen's height; a few bright ones get a four-point glint.
fn stars(c: &mut Canvas<OpenGl>, layer: u64) {
    let (count, height) = (if layer == 0 { 110 } else { 100 }, 374.0);
    for i in 0..count {
        let k = i + layer * 1000;
        let (x, y) = (unit(k, 1) * 800.0, unit(k, 2) * height);
        let r = 0.45 + unit(k, 3) * 0.65;
        let mask = if y < 168.0 {
            1.0
        } else {
            1.0 - (y - 168.0) / (height - 168.0)
        };
        let alpha = (0.35 + unit(k, 4) * 0.65) * mask;
        let colour = if unit(k, 5) < 0.2 {
            rgb(0xdfe8ff)
        } else {
            rgb(0xfff8e6)
        };
        fill(c, &circle(x, y, r), fade(colour, alpha));
    }
    for i in 0..3 {
        let k = 500 + i + layer * 1000;
        let (x, y, size) = (
            40.0 + unit(k, 1) * 720.0,
            12.0 + unit(k, 2) * 130.0,
            3.0 + unit(k, 3) * 2.5,
        );
        let mut glint = Path::new();
        for (dx, dy) in [(size, 0.0), (0.0, size)] {
            glint.move_to(x - dx, y - dy);
            glint.line_to(x + dy * 0.18, y + dx * 0.18 - dy * 0.0);
            glint.line_to(x + dx, y + dy);
            glint.line_to(x - dy * 0.18, y - dx * 0.18);
            glint.close();
        }
        fill(c, &glint, white(0.75));
        fill(c, &circle(x, y, 1.1), white(1.0));
    }
}

/// A bank of soft clouds, 1400 wide, seamless when tiled side by side.
fn clouds(c: &mut Canvas<OpenGl>, bank: u64) {
    let (colour, lit) = if bank == 0 {
        (rgb(0x7d8fb8), Some(rgb(0xc9d6f0)))
    } else {
        (rgb(0x5b6a90), None)
    };
    for i in 0..18 {
        let k = i + bank * 100;
        let (x, y, r) = (
            unit(k, 1) * 1400.0,
            34.0 + unit(k, 2) * 80.0,
            50.0 + unit(k, 3) * 70.0,
        );
        let alpha = if bank == 0 {
            0.25 + unit(k, 4) * 0.35
        } else {
            0.2 + unit(k, 4) * 0.3
        };
        for dx in [-1400.0, 0.0, 1400.0] {
            blob(c, x + dx, y, r, fade(colour, alpha));
            if let Some(lit) = lit {
                blob(c, x + dx, y - r * 0.16, r * 0.6, fade(lit, alpha * 0.35));
            }
        }
    }
}

fn blob(c: &mut Canvas<OpenGl>, x: f32, y: f32, r: f32, colour: Color) {
    c.save();
    c.translate(x, y);
    c.scale(1.0, 0.36);
    let paint = Paint::radial_gradient(0.0, 0.0, r * 0.1, r, colour, fade(colour, 0.0));
    c.fill_path(&circle(0.0, 0.0, r), &paint);
    c.restore();
}

// ---------- candles ----------

/// A tall ivory taper, 24 units wide: wax lit from the front and from its own
/// flame above, an uneven rim with a wax pool, a wick, two or three drips.
fn candle(c: &mut Canvas<OpenGl>, variant: usize) {
    let height = CANDLE_HEIGHTS[variant];
    let mut body = Path::new();
    body.move_to(5.0, 13.4);
    body.bezier_to(7.0, 12.6, 9.5, 12.0, 11.0, 12.2);
    // one lip of the rim stands higher, where the wax ran over
    body.line_to(12.6, 10.4);
    body.line_to(14.2, 10.8);
    body.bezier_to(15.5, 12.0, 17.8, 12.3, 19.0, 12.9);
    body.line_to(19.0, height);
    body.line_to(5.0, height);
    body.close();
    let wax = Paint::linear_gradient_stops(
        5.0,
        0.0,
        19.0,
        0.0,
        [
            (0.0, rgb(0x76654a)),
            (0.16, rgb(0xc7b590)),
            (0.38, rgb(0xf5ecd6)),
            (0.50, rgb(0xfffaf0)),
            (0.66, rgb(0xebdfc2)),
            (0.86, rgb(0xb39d77)),
            (1.0, rgb(0x65543b)),
        ],
    );
    c.fill_path(&body, &wax);
    let light = Paint::linear_gradient_stops(
        0.0,
        10.4,
        0.0,
        height,
        [
            (0.0, rgba(0xffb860, 0.65)),
            (18.0 / height, rgba(0xffd59a, 0.25)),
            (42.0 / height, clear()),
            (0.55, rgba(0x140c06, 0.12)),
            (0.85, rgba(0x140c06, 0.5)),
            (1.0, rgba(0x0b0a12, 0.85)),
        ],
    );
    c.fill_path(&body, &light);
    let drips: &[(f32, f32)] = match variant {
        0 => &[(6.9, 17.0), (16.4, 27.0)],
        1 => &[(6.6, 24.0), (10.2, 15.0), (16.8, 33.0)],
        _ => &[(7.2, 33.0), (13.0, 19.0), (16.6, 26.0)],
    };
    for &(x, length) in drips {
        let mut shadow = Path::new();
        shadow.rounded_rect(x - 1.2 + 0.7, 12.6, 2.4, length, 1.2);
        shadow.circle(x + 0.7, 12.0 + length + 0.6, 1.7);
        fill(c, &shadow, rgba(0x3a2a16, 0.35));
        let mut drip = Path::new();
        drip.rounded_rect(x - 1.2, 12.0, 2.4, length, 1.2);
        drip.circle(x, 12.0 + length, 1.7);
        let paint = Paint::linear_gradient_stops(
            x - 1.7,
            0.0,
            x + 1.7,
            0.0,
            [
                (0.0, rgb(0xa8936e)),
                (0.35, rgb(0xfffaf0)),
                (0.7, rgb(0xe6d8b8)),
                (1.0, rgb(0x8f7b5a)),
            ],
        );
        c.fill_path(&drip, &paint);
    }
    let mut pool = Path::new();
    pool.ellipse(12.0, 12.6, 5.3, 1.8);
    let paint = Paint::radial_gradient_stops(
        12.0,
        12.6,
        0.0,
        5.3,
        [
            (0.0, rgb(0xfff6d8)),
            (0.5, rgb(0xffd58a)),
            (1.0, rgb(0xc99650)),
        ],
    );
    c.fill_path(&pool, &paint);
    let mut wick = Path::new();
    wick.move_to(12.0, 12.8);
    wick.line_to(12.4, 8.4);
    stroke(c, &wick, rgb(0x24160c), 1.1);
    fill(c, &circle(12.4, 8.6, 0.75), rgb(0xff8a2a));
}

/// A teardrop, `h` tall from its base, `half` wide on each side.
fn drop_shape(cx: f32, base: f32, half: f32, h: f32) -> Path {
    let mut path = Path::new();
    path.move_to(cx, base - h);
    path.bezier_to(
        cx + half * 0.35,
        base - h * 0.62,
        cx + half,
        base - h * 0.3,
        cx + half * 0.92,
        base - h * 0.1,
    );
    path.bezier_to(
        cx + half * 0.8,
        base + half * 0.55,
        cx - half * 0.8,
        base + half * 0.55,
        cx - half * 0.92,
        base - h * 0.1,
    );
    path.bezier_to(
        cx - half,
        base - h * 0.3,
        cx - half * 0.35,
        base - h * 0.62,
        cx,
        base - h,
    );
    path.close();
    path
}

/// The flame, in a 24 x 48 box, its base at y 41.6: an orange mantle, a
/// white-yellow body, a white core and a little blue at the base.
fn flame(c: &mut Canvas<OpenGl>) {
    let mantle = drop_shape(12.0, FLAME_BASE + 1.0, 7.4, 33.0);
    let paint = Paint::radial_gradient_stops(
        12.0,
        32.0,
        0.0,
        18.0,
        [
            (0.0, rgba(0xff8a24, 0.5)),
            (0.7, rgba(0xff8a24, 0.22)),
            (1.0, rgba(0xff8a24, 0.0)),
        ],
    );
    c.fill_path(&mantle, &paint);
    let body = drop_shape(12.0, FLAME_BASE, 5.0, 28.0);
    let paint = Paint::radial_gradient_stops(
        12.0,
        35.0,
        0.0,
        17.0,
        [
            (0.0, white(1.0)),
            (0.22, rgb(0xfff6cc)),
            (0.46, rgb(0xffd05c)),
            (0.74, rgb(0xff9428)),
            (1.0, rgba(0xff6a10, 0.0)),
        ],
    );
    c.fill_path(&body, &paint);
    let core = Paint::radial_gradient_stops(
        12.0,
        33.5,
        0.0,
        7.0,
        [
            (0.0, white(1.0)),
            (0.5, rgba(0xfffbea, 0.8)),
            (1.0, rgba(0xfffbea, 0.0)),
        ],
    );
    c.fill_path(&circle(12.0, 33.5, 7.0), &core);
    let blue = Paint::radial_gradient(
        12.0,
        40.4,
        0.0,
        2.2,
        rgba(0x8fb8ff, 0.7),
        rgba(0x8fb8ff, 0.0),
    );
    c.fill_path(&circle(12.0, 40.4, 2.2), &blue);
}

fn halo(c: &mut Canvas<OpenGl>) {
    let paint = Paint::radial_gradient_stops(
        64.0,
        64.0,
        0.0,
        64.0,
        [
            (0.0, rgba(0xffd68c, 0.42)),
            (0.38, rgba(0xffb250, 0.16)),
            (0.68, rgba(0xffb250, 0.05)),
            (1.0, rgba(0xffb250, 0.0)),
        ],
    );
    c.fill_path(&circle(64.0, 64.0, 64.0), &paint);
}

// ---------- banners ----------

/// A house banner, 100 x 316 units: its cord, a gilt rod, the cloth with its
/// trim, folds and shade, the crest and the house's name.
fn banner(c: &mut Canvas<OpenGl>, house: House, fonts: &Fonts) {
    c.translate(0.0, 14.0);
    let cloth = svg::path("M6 12H94V262L50 296 6 262Z");
    fill(c, &cloth, house.cloth());
    stroke(
        c,
        &svg::path("M12 12V258L50 288 88 258V12"),
        house.trim(),
        2.4,
    );
    stroke(
        c,
        &svg::path("M16 12V255L50 282 84 255V12"),
        fade(house.trim(), 0.8),
        0.8,
    );
    let folds = Paint::linear_gradient_stops(
        6.0,
        0.0,
        94.0,
        0.0,
        [
            (0.0, black(0.55)),
            (0.08, black(0.15)),
            (0.2, white(0.14)),
            (0.32, black(0.22)),
            (0.46, white(0.1)),
            (0.6, black(0.18)),
            (0.74, white(0.16)),
            (0.88, black(0.2)),
            (1.0, black(0.6)),
        ],
    );
    c.fill_path(&cloth, &folds);
    let shade = Paint::linear_gradient_stops(
        0.0,
        12.0,
        0.0,
        296.0,
        [
            (0.0, black(0.35)),
            (0.12, black(0.0)),
            (0.75, black(0.05)),
            (1.0, black(0.45)),
        ],
    );
    c.fill_path(&cloth, &shade);
    stroke(c, &svg::path("M50 296V300"), house.trim(), 1.6);
    fill(c, &circle(50.0, 299.0, 2.4), house.trim());
    let rod = Paint::linear_gradient_stops(
        0.0,
        4.0,
        0.0,
        13.0,
        [
            (0.0, rgb(0xfff1b8)),
            (0.45, rgb(0xc9952e)),
            (1.0, rgb(0x5e3f10)),
        ],
    );
    let mut bar = Path::new();
    bar.rounded_rect(0.0, 5.0, 100.0, 7.0, 3.5);
    bar.circle(2.0, 8.5, 4.5);
    bar.circle(98.0, 8.5, 4.5);
    c.fill_path(&bar, &rod);
    stroke(c, &svg::path("M30 5L50 -12 70 5"), rgb(0x8a6a2a), 1.0);

    c.save();
    c.translate(16.0, 40.0);
    c.scale(1.13, 1.13);
    let ink = |ink: Ink| match ink {
        Ink::Metal => house.metal(),
        Ink::Cloth => house.cloth(),
        Ink::Dark => black(0.5),
    };
    for mark in house.marks() {
        let path = svg::path(mark.d);
        if let Some(colour) = mark.fill {
            fill(c, &path, ink(colour));
        }
        if let Some((colour, width)) = mark.stroke {
            let mut paint = Paint::color(ink(colour))
                .with_line_width(width)
                .with_line_cap(LineCap::Round);
            if mark.dash {
                paint.set_line_dash(&[1.4, 1.6]);
            }
            c.stroke_path(&path, &paint);
        }
    }
    c.restore();
    let name = Paint::color(house.metal())
        .with_font(&[fonts.cinzel])
        .with_font_size(7.4)
        .with_letter_spacing(0.9)
        .with_text_align(Align::Center)
        .with_text_baseline(Baseline::Alphabetic);
    let _ = c.fill_text(50.0, 148.0, house.name(), &name);
}

// ---------- parchment ----------

/// A parchment sheet of `SHEET` size, inside a `SHEET_MARGIN` border: torn
/// edges, a soft shadow, stains, fibres, grain and foxing, a scorched rim and
/// a corner curling up.
fn sheet(c: &mut Canvas<OpenGl>) {
    let (w, h, m) = (SHEET.0, SHEET.1, SHEET_MARGIN);
    let (x0, y0, x1, y1) = (m, m, m + w, m + h);
    let curl = 19.0;
    let tear = |k: u64| {
        let depth = 0.6 + unit(k, 7) * 5.9;
        if unit(k, 8) < 0.06 {
            depth + 2.5
        } else {
            depth
        }
    };
    let mut edge = Path::new();
    let step = 8.0;
    let mut k = 0_u64;
    let mut x = x0;
    edge.move_to(x0 + tear(k), y0 + tear(k + 1));
    while x < x1 {
        k += 1;
        edge.line_to(x, y0 + tear(k));
        x += step;
    }
    let mut y = y0;
    while y < y1 - curl {
        k += 1;
        edge.line_to(x1 - tear(k), y);
        y += step;
    }
    edge.line_to(x1 - 1.0, y1 - curl);
    edge.line_to(x1 - curl, y1 - 1.0);
    let mut x = x1 - curl;
    while x > x0 {
        k += 1;
        edge.line_to(x, y1 - tear(k));
        x -= step;
    }
    let mut y = y1;
    while y > y0 {
        k += 1;
        edge.line_to(x0 + tear(k), y);
        y -= step;
    }
    edge.close();

    let base = Paint::radial_gradient_stops(
        x0 + w * 0.45,
        y0 + h * 0.35,
        0.0,
        w * 0.72,
        [
            (0.0, rgb(0xf1e2bb)),
            (0.5, rgb(0xe6cf9c)),
            (1.0, rgb(0xcaa66a)),
        ],
    );
    c.set_shadow_color(black(0.55));
    c.set_shadow_blur(10.0);
    c.set_shadow_offset(0.0, 8.0);
    c.fill_path(&edge, &base);
    c.set_shadow_color(clear());
    c.set_shadow_blur(0.0);

    // what follows stays on the paper
    c.global_composite_operation(CompositeOperation::Atop);
    for i in 0..4 {
        let (sx, sy, r) = (
            x0 + unit(i, 41) * w,
            y0 + unit(i, 42) * h,
            60.0 + unit(i, 43) * 90.0,
        );
        let stain = Paint::radial_gradient(
            sx,
            sy,
            r * 0.2,
            r,
            rgba(0x75470f, 0.1 + unit(i, 44) * 0.12),
            rgba(0x75470f, 0.0),
        );
        c.fill_path(&circle(sx, sy, r), &stain);
    }
    // a water stain: a faint ring
    c.save();
    c.translate(x0 + w * 0.68, y0 + h * 0.62);
    c.scale(1.0, 0.77);
    let ring = Paint::radial_gradient_stops(
        0.0,
        0.0,
        0.0,
        w * 0.13,
        [
            (0.0, clear()),
            (0.78, rgba(0x75470f, 0.025)),
            (0.94, rgba(0x75470f, 0.07)),
            (1.0, rgba(0x75470f, 0.0)),
        ],
    );
    c.fill_path(&circle(0.0, 0.0, w * 0.13), &ring);
    c.restore();
    let mut fibres = Path::new();
    for i in 0..70 {
        let (fx, fy, length) = (
            x0 + unit(i, 51) * w,
            y0 + unit(i, 52) * h,
            30.0 + unit(i, 53) * 140.0,
        );
        fibres.move_to(fx, fy);
        fibres.line_to(fx + length, fy + (unit(i, 54) - 0.5) * 1.5);
    }
    c.stroke_path(
        &fibres,
        &Paint::color(rgba(0x6a4a1c, 0.1)).with_line_width(0.6),
    );
    let mut grain = Path::new();
    for i in 0..700 {
        grain.circle(x0 + unit(i, 61) * w, y0 + unit(i, 62) * h, 0.45);
    }
    fill(c, &grain, rgba(0x4a3010, 0.16));
    let mut specks = Path::new();
    for i in 0..26 {
        specks.circle(
            x0 + unit(i, 71) * w,
            y0 + unit(i, 72) * h,
            0.6 + unit(i, 73) * 0.7,
        );
    }
    fill(c, &specks, rgba(0x2a1a08, 0.35));
    for i in 0..3 {
        let (fx, fy) = (x0 + unit(i, 81) * w, y0 + unit(i, 82) * h);
        let halo =
            Paint::radial_gradient(fx, fy, 1.5, 7.0, rgba(0x8a4a1a, 0.12), rgba(0x8a4a1a, 0.0));
        c.fill_path(&circle(fx, fy, 7.0), &halo);
        fill(c, &circle(fx, fy, 1.8), rgba(0x8a3a10, 0.25));
    }
    // scorched rim, from each edge
    let scorch = [
        (x0, y0, x0, y0 + 30.0, rect(x0, y0, w, 30.0)),
        (x0, y1, x0, y1 - 30.0, rect(x0, y1 - 30.0, w, 30.0)),
        (x0, y0, x0 + 30.0, y0, rect(x0, y0, 30.0, h)),
        (x1, y0, x1 - 30.0, y0, rect(x1 - 30.0, y0, 30.0, h)),
    ];
    for (sx, sy, ex, ey, band) in scorch {
        let paint = Paint::linear_gradient_stops(
            sx,
            sy,
            ex,
            ey,
            [
                (0.0, rgba(0x341a06, 0.7)),
                (8.0 / 30.0, rgba(0x6e4216, 0.22)),
                (1.0, clear()),
            ],
        );
        c.fill_path(&band, &paint);
    }
    let vignette = Paint::radial_gradient(
        x0 + w / 2.0,
        y0 + h / 2.0,
        h * 0.4,
        w * 0.62,
        clear(),
        rgba(0x784816, 0.22),
    );
    c.fill_path(&rect(x0, y0, w, h), &vignette);
    c.global_composite_operation(CompositeOperation::SourceOver);

    // the curled corner: the back of the sheet, folded over
    let mut flap = Path::new();
    flap.move_to(x1 - curl, y1 - 1.0);
    flap.line_to(x1 - 1.0, y1 - curl);
    flap.line_to(x1 - curl - 2.0, y1 - curl - 2.0);
    flap.close();
    let back = Paint::linear_gradient_stops(
        x1 - curl - 2.0,
        y1 - curl - 2.0,
        x1 - curl / 2.0,
        y1 - curl / 2.0,
        [
            (0.0, rgb(0xc9a868)),
            (0.4, rgb(0xe9d4a2)),
            (0.7, rgb(0xf3e5c2)),
            (1.0, rgb(0xdcc08a)),
        ],
    );
    c.set_shadow_color(black(0.35));
    c.set_shadow_blur(3.0);
    c.set_shadow_offset(-2.0, -2.0);
    c.fill_path(&flap, &back);
    c.set_shadow_color(clear());
    c.set_shadow_blur(0.0);
}

// ---------- seals ----------

/// A wax seal, after the home page's: an irregular blob of wax in the house
/// colour, shaded, a ring, an embossed H, a highlight.
fn seal(c: &mut Canvas<OpenGl>, house: House, fonts: &Fonts) {
    c.translate(4.0, 2.0);
    let blob = svg::path(
        "M56.6 26.4Q58.7 30.0 56.8 33.7Q54.9 37.3 54.0 41.1Q53.2 44.9 50.0 47.2Q46.8 49.4 44.1 52.1Q41.3 54.8 37.7 56.7Q34.1 58.5 30.2 56.9Q26.4 55.3 22.5 55.0Q18.7 54.7 16.0 52.0Q13.3 49.3 10.3 46.9Q7.4 44.5 5.3 41.2Q3.3 37.8 3.8 33.9Q4.3 30.0 4.3 26.2Q4.2 22.4 5.3 18.6Q6.3 14.7 9.7 12.6Q13.1 10.5 15.7 7.3Q18.2 4.2 22.2 3.5Q26.1 2.7 30.1 2.2Q34.1 1.7 37.7 3.4Q41.4 5.1 44.4 7.4Q47.5 9.8 51.8 11.5Q56.0 13.3 55.3 18.0Q54.5 22.8 56.6 26.4Z",
    );
    c.set_shadow_color(black(0.45));
    c.set_shadow_blur(3.0);
    c.set_shadow_offset(0.0, 3.0);
    fill(c, &blob, house.wax());
    c.set_shadow_color(clear());
    c.set_shadow_blur(0.0);
    let shade = Paint::radial_gradient_stops(
        21.0,
        18.0,
        0.0,
        45.0,
        [(0.0, white(0.35)), (0.45, white(0.0)), (1.0, black(0.4))],
    );
    c.fill_path(&blob, &shade);
    stroke(c, &circle(30.0, 30.0, 19.0), black(0.35), 2.4);
    stroke(c, &circle(29.2, 29.2, 19.0), white(0.28), 1.0);
    fill(c, &circle(30.0, 30.0, 15.5), black(0.12));
    let letter = |colour: Color| {
        Paint::color(colour)
            .with_font(&[fonts.cinzel])
            .with_font_size(21.0)
            .with_text_align(Align::Center)
            .with_text_baseline(Baseline::Alphabetic)
    };
    let _ = c.fill_text(30.0, 37.5, "H", &letter(black(0.4)));
    let _ = c.fill_text(29.2, 36.7, "H", &letter(white(0.3)));
    stroke(c, &svg::path("M18 16C22 12 28 11 33 12"), white(0.45), 2.0);
}

// ---------- the gilt frame of portraits ----------

/// A carved gilt frame around a 200 x 300 picture (the hole), with the
/// varnish and sheen of an old painting over it, acanthus in the corners
/// and a shell on top.
fn portrait(c: &mut Canvas<OpenGl>) {
    let (b, top) = (FRAME, FRAME_TOP);
    let (w, h) = (200.0 + 2.0 * b, 300.0 + 2.0 * b);
    c.translate(0.0, top);
    let band = |outer: f32, inner: f32| {
        let mut path = Path::new();
        path.rect(outer, outer, w - 2.0 * outer, h - 2.0 * outer);
        path.rect(inner, inner, w - 2.0 * inner, h - 2.0 * inner);
        path.solidity(Solidity::Hole);
        path
    };
    let diagonal = |stops: &[(f32, u32)], reverse: bool| {
        let (sx, sy, ex, ey) = if reverse {
            (w, h, 0.0, 0.0)
        } else {
            (0.0, 0.0, w, h)
        };
        Paint::linear_gradient_stops(
            sx,
            sy,
            ex,
            ey,
            stops.iter().map(|&(at, hex)| (at, rgb(hex))),
        )
    };
    c.fill_path(
        &band(0.0, b),
        &diagonal(&[(0.0, 0x8a5f1a), (0.5, 0x4a300a), (1.0, 0x2a1a04)], false),
    );
    c.fill_path(
        &band(3.0, 9.0),
        &diagonal(
            &[
                (0.0, 0xfff0b0),
                (0.3, 0xe2b043),
                (0.55, 0x9c6a1c),
                (0.8, 0xe8bc55),
                (1.0, 0x6e4710),
            ],
            false,
        ),
    );
    c.fill_path(
        &band(9.0, 16.5),
        &diagonal(&[(0.0, 0xf6d77a), (0.5, 0xa87420), (1.0, 0x5e3c0c)], true),
    );
    let mut bead = Path::new();
    bead.rect(12.5, 12.5, w - 25.0, h - 25.0);
    let mut paint = Paint::color(rgba(0xfff0b0, 0.85))
        .with_line_width(1.4)
        .with_line_cap(LineCap::Round);
    paint.set_line_dash(&[1.0, 2.4]);
    c.stroke_path(&bead, &paint);
    c.fill_path(
        &band(16.5, b),
        &diagonal(&[(0.0, 0x5a3a0c), (0.5, 0xc79634), (1.0, 0xf3d47a)], false),
    );
    // the painting's old varnish and a sheen from the top left
    let canvas_box = rect(b, b, 200.0, 300.0);
    let varnish = Paint::radial_gradient_stops(
        b + 100.0,
        b + 126.0,
        0.0,
        215.0,
        [(0.55, rgba(0x2a1a06, 0.0)), (1.0, rgba(0x1a0e02, 0.55))],
    );
    c.fill_path(&canvas_box, &varnish);
    let sheen = Paint::linear_gradient_stops(
        b,
        b,
        b + 200.0,
        b + 300.0,
        [(0.0, rgba(0xfff6d8, 0.16)), (0.35, rgba(0xfff6d8, 0.0))],
    );
    c.fill_path(&canvas_box, &sheen);
    stroke(c, &rect(b, b, 200.0, 300.0), rgb(0x3a2406), 2.0);
    // ornaments
    let ornament = |c: &mut Canvas<OpenGl>, path: &Path| {
        let paint = Paint::radial_gradient_stops(
            10.0,
            9.0,
            0.0,
            26.0,
            [
                (0.0, rgb(0xfff3c0)),
                (0.5, rgb(0xd9a640)),
                (1.0, rgb(0x7a5212)),
            ],
        );
        c.fill_path(path, &paint);
        c.stroke_path(path, &Paint::color(rgb(0x6a440e)).with_line_width(0.8));
    };
    for (sx, sy, tx, ty) in [
        (1.0, 1.0, 0.0, 0.0),
        (-1.0, 1.0, w, 0.0),
        (1.0, -1.0, 0.0, h),
        (-1.0, -1.0, w, h),
    ] {
        c.save();
        c.translate(tx, ty);
        c.scale(sx * 0.78, sy * 0.78);
        ornament(
            c,
            &svg::path("M2 2C14 0 26 4 32 14 26 12 20 14 18 20 14 14 8 14 4 18 6 12 4 6 2 2Z"),
        );
        ornament(
            c,
            &svg::path("M2 2C0 14 4 26 14 32 12 26 14 20 20 18 14 14 14 8 18 4 12 6 6 4 2 2Z"),
        );
        ornament(c, &circle(16.0, 16.0, 4.5));
        c.restore();
    }
    c.save();
    c.translate(w / 2.0 - 100.0 * 0.78, 1.0);
    c.scale(0.78, 0.78);
    ornament(
        c,
        &svg::path("M78 6C86 -6 114 -6 122 6 116 4 112 8 110 12 106 6 94 6 90 12 88 8 84 4 78 6Z"),
    );
    stroke(
        c,
        &svg::path("M84 4L92 -2M100 -4V4M116 4L108 -2"),
        rgb(0x6a440e),
        0.8,
    );
    c.restore();
}

// ---------- hourglasses ----------

/// The hourglass behind its gems: the glass' tint, the posts with their
/// rings, the caps with their gem medallions.
fn hourglass(c: &mut Canvas<OpenGl>, house: House) {
    let glass = svg::path(GLASS);
    fill(c, &glass, rgba(0xa0bee6, 0.07));
    let [m0, m1, m2, m3] = house.frame_metal();
    let horizontal = |x: f32, w: f32| {
        Paint::linear_gradient_stops(
            x,
            0.0,
            x + w,
            0.0,
            [(0.0, m1), (0.4, m0), (0.75, m2), (1.0, m3)],
        )
    };
    let vertical = |y: f32, h: f32| {
        Paint::linear_gradient_stops(
            0.0,
            y,
            0.0,
            y + h,
            [(0.0, m0), (0.35, m1), (0.7, m2), (1.0, m3)],
        )
    };
    for x in [8.0, 86.0] {
        let mut post = Path::new();
        post.rounded_rect(x, 24.0, 6.0, 212.0, 2.0);
        c.fill_path(&post, &horizontal(x, 6.0));
    }
    for (y, r) in [(80.0, 4.2), (130.0, 4.6), (180.0, 4.2)] {
        for x in [11.0, 89.0] {
            c.fill_path(&circle(x, y, r), &vertical(y - r, 2.0 * r));
        }
    }
    let top =
        svg::path("M2 8C2 3 6 0 12 0H88C94 0 98 3 98 8V24C98 28 94 30 90 30H10C6 30 2 28 2 24Z");
    c.fill_path(&top, &vertical(0.0, 30.0));
    let bottom = svg::path(
        "M2 236C2 232 6 230 10 230H90C94 230 98 232 98 236V252C98 257 94 260 88 260H12C6 260 2 257 2 252Z",
    );
    c.fill_path(&bottom, &vertical(230.0, 30.0));
    stroke(c, &svg::path("M6 14H94M6 246H94"), black(0.35), 1.2);
    stroke(c, &svg::path("M8 6H92M8 238H92"), white(0.45), 1.0);
    let [gem, light, _] = house.gems();
    for (y, r, width) in [(15.0, 9.0, 2.4), (245.0, 7.0, 2.0)] {
        fill(c, &circle(50.0, y, r), gem);
        c.stroke_path(
            &circle(50.0, y, r),
            &Paint::color(m1).with_line_width(width),
        );
    }
    fill(c, &circle(47.0, 12.0, 2.6), fade(light, 0.8));
}

/// The front of the glass, over the gems: its rim and highlights.
fn glass_front(c: &mut Canvas<OpenGl>) {
    stroke(c, &svg::path(GLASS), rgba(0xdcebff, 0.45), 1.2);
    stroke(
        c,
        &svg::path("M30 40C30 70 35 94 44 112M30 220C30 196 34 172 42 154"),
        white(0.5),
        2.2,
    );
    stroke(c, &svg::path("M70 46C70 64 67 80 62 94"), white(0.22), 1.4);
}

/// A 9 x 8 tile of gems on their dark ground, repeated to fill the bulbs.
fn gems(c: &mut Canvas<OpenGl>, house: House) {
    let [base, light, dark] = house.gems();
    fill(c, &rect(0.0, 0.0, 9.0, 8.0), dark);
    for (x, y, r) in [
        (2.4, 2.2, 2.3),
        (7.0, 2.6, 2.0),
        (4.6, 6.0, 2.4),
        (9.0, 6.6, 1.8),
        (0.0, 6.6, 1.8),
    ] {
        fill(c, &circle(x, y, r), base);
    }
    for (x, y, r) in [(1.8, 1.5, 0.7), (6.4, 1.9, 0.6), (3.9, 5.2, 0.7)] {
        fill(c, &circle(x, y, r), light);
    }
}

/// Where the sand stands for `value` (0: all in the top bulb, 1: all fallen),
/// in the hourglass' units: the top bulb's surface and the bottom pile's top.
pub fn hourglass_levels(value: f32) -> (f32, f32) {
    let value = value.clamp(0.0, 1.0);
    (34.0 + 90.0 * value, 228.0 - 88.0 * value)
}

fn bezier(p: [(f32, f32); 4], t: f32) -> (f32, f32) {
    let u = 1.0 - t;
    let (a, b, c, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
    (
        a * p[0].0 + b * p[1].0 + c * p[2].0 + d * p[3].0,
        a * p[0].1 + b * p[1].1 + c * p[2].1 + d * p[3].1,
    )
}

/// The left wall of the glass at height `y` (the right wall is 100 - x).
pub fn glass_x(y: f32) -> f32 {
    let curve = if y < 123.0 {
        [(24.0, 30.0), (24.0, 70.0), (31.0, 100.0), (46.5, 123.0)]
    } else if y > 135.0 {
        [(46.5, 135.0), (31.0, 158.0), (24.0, 188.0), (24.0, 228.0)]
    } else {
        return 46.5;
    };
    // walk the curve: y grows with t on both halves
    let mut previous = bezier(curve, 0.0);
    for step in 1..=48 {
        let point = bezier(curve, step as f32 / 48.0);
        if point.1 >= y {
            let k = (y - previous.1) / (point.1 - previous.1).max(1e-4);
            return previous.0 + (point.0 - previous.0) * k.clamp(0.0, 1.0);
        }
        previous = point;
    }
    previous.0
}

/// The wall points of one half of the glass between two heights.
fn wall(from: f32, to: f32) -> Vec<(f32, f32)> {
    let steps = 14;
    (0..=steps)
        .map(|i| {
            let y = from + (to - from) * i as f32 / steps as f32;
            (glass_x(y), y)
        })
        .collect()
}

impl Gfx<'_> {
    /// An image stretched over a box.
    pub fn image(&mut self, image: ImageId, x: f32, y: f32, w: f32, h: f32, alpha: f32) {
        let paint = Paint::image(image, x, y, w, h, 0.0, alpha);
        self.canvas.fill_path(&rect(x, y, w, h), &paint);
    }

    /// The parchment, its paper filling the box (its torn edges and shadow
    /// spill a little outside).
    pub fn parchment(&mut self, x: f32, y: f32, w: f32, h: f32) {
        let Some(hall) = self.hall else { return };
        let (sx, sy) = (w / SHEET.0, h / SHEET.1);
        self.image(
            hall.sheet,
            x - SHEET_MARGIN * sx,
            y - SHEET_MARGIN * sy,
            w + 2.0 * SHEET_MARGIN * sx,
            h + 2.0 * SHEET_MARGIN * sy,
            1.0,
        );
    }

    /// A wax seal centred on (`x`, `y`), `size` wide, turned by `degrees`.
    pub fn seal(&mut self, house: House, x: f32, y: f32, size: f32, degrees: f32) {
        let Some(hall) = self.hall else { return };
        let full = size * SEAL_BOX / 60.0;
        self.canvas.save();
        self.canvas.translate(x, y);
        self.canvas.rotate(degrees.to_radians());
        self.image(
            hall.seals[house.index()],
            -full / 2.0,
            -full / 2.0,
            full,
            full,
            1.0,
        );
        self.canvas.restore();
    }

    /// The gilt frame around a picture drawn in the box (any 2:3 box).
    pub fn portrait(&mut self, x: f32, y: f32, w: f32, h: f32, alpha: f32) {
        let Some(hall) = self.hall else { return };
        let (sx, sy) = (w / 200.0, h / 300.0);
        self.image(
            hall.portrait,
            x - FRAME * sx,
            y - (FRAME + FRAME_TOP) * sy,
            w + 2.0 * FRAME * sx,
            h + (2.0 * FRAME + FRAME_TOP) * sy,
            alpha,
        );
    }

    /// A house-point hourglass `height` tall with its top left at (`x`, `y`),
    /// filled to `value` (0: none fallen, 1: all fallen); `trickle` lets gems
    /// keep falling through the neck.
    pub fn hourglass(
        &mut self,
        house: House,
        x: f32,
        y: f32,
        height: f32,
        value: f32,
        trickle: bool,
    ) {
        let Some(hall) = self.hall else { return };
        let scale = height / 260.0;
        let i = house.index();
        self.image(hall.hourglasses[i], x, y, 100.0 * scale, height, 1.0);
        let (surface, pile) = hourglass_levels(value);
        self.canvas.save();
        self.canvas.translate(x, y);
        self.canvas.scale(scale, scale);
        let gems = Paint::image(hall.gems[i], 0.0, 0.0, 9.0, 8.0, 0.0, 1.0);
        if surface < 121.0 {
            let side = wall(surface, 123.0);
            let mut sand = Path::new();
            sand.move_to(side[0].0, side[0].1);
            for &(px, py) in &side[1..] {
                sand.line_to(px, py);
            }
            for &(px, py) in side.iter().rev() {
                sand.line_to(100.0 - px, py);
            }
            // the surface is dished: lower in the middle
            sand.quad_to(50.0, surface + 24.0, side[0].0, side[0].1);
            sand.close();
            self.canvas.fill_path(&sand, &gems);
        }
        if pile < 226.0 {
            let side = wall(pile.max(136.0), 228.0);
            let mut heap = Path::new();
            heap.move_to(side[0].0, side[0].1);
            for &(px, py) in &side[1..] {
                heap.line_to(px, py);
            }
            for &(px, py) in side.iter().rev() {
                heap.line_to(100.0 - px, py);
            }
            // the pile is domed: higher in the middle
            heap.quad_to(50.0, side[0].1 - 24.0, side[0].0, side[0].1);
            heap.close();
            self.canvas.fill_path(&heap, &gems);
        }
        if trickle && value > 0.0 && value < 1.0 {
            let [base, _, _] = house.gems();
            let mut stream = Path::new();
            for k in 0..4 {
                let f = (self.t * 1.7 + k as f32 / 4.0).fract();
                stream.circle(
                    50.0 + (k as f32 * 1.3).sin() * 0.5,
                    132.0 + f * (pile - 140.0).max(8.0),
                    1.5,
                );
            }
            self.fill(&stream, base);
        }
        self.canvas.restore();
        self.image(hall.glass, x, y, 100.0 * scale, height, 1.0);
    }
}

// ---------- the owl and its letter ----------

/// The owl post, after the home page's: a pale owl facing right, its wings
/// turned by `degrees` and squashed by `squash` around their roots.
fn owl(c: &mut Canvas<OpenGl>, degrees: f32, squash: f32) {
    let wing = |c: &mut Canvas<OpenGl>,
                pivot: (f32, f32),
                shape: &str,
                lines: &str,
                colour: u32,
                ink: u32| {
        c.save();
        c.translate(pivot.0, pivot.1);
        c.rotate(degrees.to_radians());
        c.scale(1.0, squash);
        c.translate(-pivot.0, -pivot.1);
        fill(c, &svg::path(shape), rgb(colour));
        stroke(c, &svg::path(lines), rgb(ink), 0.8);
        c.restore();
    };
    wing(
        c,
        (52.0, 40.0),
        "M52 34C44 20 30 9 10 6 18 12 22 16 25 21 18 19 13 19 8 21 17 24 24 28 30 33 25 33 20 35 17 38 28 40 40 40 50 40Z",
        "M14 9L22 17M12 20L25 24M20 36L33 37",
        0xcdbfa4,
        0x9c8a6c,
    );
    fill(
        c,
        &svg::path("M30 44C24 45 19 48 15 52 22 52 28 51 34 49Z"),
        rgb(0xd9ccb2),
    );
    stroke(
        c,
        &svg::path("M17 51L24 49M20 52L27 50"),
        rgb(0xa8977a),
        0.6,
    );
    let feathers = |x: f32, y: f32, r: f32| {
        Paint::radial_gradient_stops(
            x,
            y,
            0.0,
            r,
            [
                (0.0, rgb(0xfbf7ec)),
                (0.7, rgb(0xe2d6bf)),
                (1.0, rgb(0xb9a98c)),
            ],
        )
    };
    c.fill_path(
        &svg::path(
            "M30 46C33 35 44 30 57 31 70 32 78 38 78 46 78 54 68 58 55 58 42 58 32 54 30 46Z",
        ),
        &feathers(49.0, 40.0, 38.0),
    );
    stroke(
        c,
        &svg::path(
            "M45 40l1.6 1.6M52 44l1.6 1.6M59 40l1.6 1.6M48 50l1.6 1.6M56 51l1.6 1.6M64 46l1.6 1.6",
        ),
        rgba(0x9a845e, 0.7),
        1.1,
    );
    c.fill_path(&circle(80.0, 36.0, 12.5), &feathers(77.0, 32.0, 20.0));
    fill(
        c,
        &svg::path(
            "M80 26C86 26 91 31 91 37 91 43 86 47 80 47 76 45 74 41 74 36 74 31 76 27 80 26Z",
        ),
        rgb(0xfffdf6),
    );
    stroke(
        c,
        &svg::path("M80 26C86 26 91 31 91 37 91 43 86 47 80 47"),
        rgb(0xc4a87a),
        1.2,
    );
    fill(c, &circle(85.0, 34.5, 3.1), rgb(0x1a1006));
    fill(c, &circle(85.9, 33.6, 0.9), white(1.0));
    fill(
        c,
        &svg::path("M88.6 38.4L92.4 40.6 88.4 42Z"),
        rgb(0x5c4630),
    );
    stroke(
        c,
        &svg::path("M60 57L58 63M64 57L64 63"),
        rgb(0x7a6248),
        1.6,
    );
    wing(
        c,
        (56.0, 41.0),
        "M56 35C50 18 37 4 14 0 23 7 27 12 30 18 22 16 15 16 9 19 20 22 28 27 34 33 28 33 22 35 18 39 31 41 44 41 56 40Z",
        "M19 3L28 13M14 18L29 22M22 37L36 38M33 27L44 33",
        0xfbf8f0,
        0xb6a586,
    );
}

fn letter(c: &mut Canvas<OpenGl>) {
    let mut paper = Path::new();
    paper.rounded_rect(0.5, 0.5, 29.0, 20.0, 1.0);
    fill(c, &paper, rgb(0xf1e2bb));
    c.stroke_path(&paper, &Paint::color(rgb(0xa88a50)).with_line_width(0.7));
    c.stroke_path(
        &svg::path("M.5 .5L15 12 29.5 .5"),
        &Paint::color(rgb(0xa88a50)).with_line_width(0.7),
    );
    fill(c, &circle(15.0, 12.0, 3.2), rgb(0xa3211f));
    fill(c, &circle(14.2, 11.2, 1.0), rgb(0xd4544c));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hourglass_empties_the_top_into_the_bottom() {
        assert_eq!(hourglass_levels(0.0), (34.0, 228.0));
        assert_eq!(hourglass_levels(1.0), (124.0, 140.0));
        assert_eq!(hourglass_levels(0.5), (79.0, 184.0));
    }

    #[test]
    fn values_outside_zero_to_one_are_held_at_the_ends() {
        assert_eq!(hourglass_levels(-2.0), hourglass_levels(0.0));
        assert_eq!(hourglass_levels(7.0), hourglass_levels(1.0));
    }

    #[test]
    fn the_glass_walls_follow_the_home_page_outline() {
        assert!((glass_x(30.0) - 24.0).abs() < 0.1);
        assert!((glass_x(123.0) - 46.5).abs() < 0.1);
        assert!((glass_x(228.0) - 24.0).abs() < 0.1);
        // narrower towards the neck
        assert!(glass_x(110.0) > glass_x(60.0));
        assert!(glass_x(150.0) > glass_x(200.0));
    }
}
