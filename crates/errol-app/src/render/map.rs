//! The Marauder's Map, for the hogwarts look, after Pensieve's own: warm
//! parchment (stains, folds, grain) with the castle's rooms, corridors, stairs
//! and names in faint ink; the Time-Turner's engraved outer ring; the halftone
//! the Gazette prints its photographs with. These are drawn once, into GPU
//! images, when the look starts (`Map::bake`). The ink every scene draws with
//! (rooms, rules, ribbons, stamps, fleurons, pins, printed photographs) is a
//! few small paths a frame.

// drawing code: x, y, w, h for a box, c for the canvas
#![allow(clippy::many_single_char_names)]

use femtovg::renderer::OpenGl;
use femtovg::{
    Align, Baseline, Canvas, Color, FontId, ImageFlags, ImageId, LineCap, Paint, Path, PixelFormat,
    RenderTarget,
};

use super::{Anchor, Face, Fonts, Gfx, Type};
use crate::anim::unit;
use crate::look::{fade, rgb};

/// The Map's warm parchment and its inks.
pub mod ink {
    use crate::look::rgb;
    use femtovg::Color;

    pub fn ink() -> Color {
        rgb(0x3f210b)
    }
    pub fn red() -> Color {
        rgb(0x9c1f12)
    }
    pub fn paper() -> Color {
        rgb(0xf6dda6)
    }
    pub fn newsprint() -> Color {
        rgb(0xf2dfb6)
    }
    pub fn news_ink() -> Color {
        rgb(0x2a1608)
    }
    /// A room's floor.
    pub fn room() -> Color {
        Color::rgbaf(0.965, 0.871, 0.675, 0.88)
    }
    /// The paper of a ribbon that names someone.
    pub fn ribbon() -> Color {
        rgb(0xf6e7c2)
    }
}

/// The ring image: 364 units around its centre.
pub const RING: f32 = 364.0;
const RING_CENTRE: (f32, f32) = (200.0, 236.0);
const INSCRIPTION: &str = "JE MARQUE LES HEURES, UNE À UNE · ET JAMAIS NE DEVANCE LE SOLEIL · MON PRIX EST CE QUE TU EN FAIS · ";

/// The baked images of the Map.
pub struct Map {
    pub parchment: ImageId,
    pub ring: ImageId,
    halftone: ImageId,
}

fn clear() -> Color {
    Color::rgbaf(0.0, 0.0, 0.0, 0.0)
}

fn rgba(hex: u32, alpha: f32) -> Color {
    fade(rgb(hex), alpha)
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
            .with_line_cap(LineCap::Butt),
    );
}

fn rect(x: f32, y: f32, w: f32, h: f32) -> Path {
    let mut path = Path::new();
    path.rect(x, y, w, h);
    path
}

impl Map {
    /// Draws every image once. Call `flush` on the canvas before using them.
    pub fn bake(canvas: &mut Canvas<OpenGl>, fonts: &Fonts) -> Result<Self, String> {
        let plain = ImageFlags::empty();
        let repeat = ImageFlags::REPEAT_X | ImageFlags::REPEAT_Y;
        Ok(Self {
            parchment: bake(canvas, 800.0, 480.0, 1.0, plain, |c| {
                parchment(c, fonts.fell_italic);
            })?,
            ring: bake(canvas, RING, RING, 1.0, plain, |c| ring(c, fonts.fell))?,
            halftone: bake(canvas, 4.0, 4.0, 2.0, repeat, halftone)?,
        })
    }
}

/// The gold of the Time-Turner, light to dark and back, across a 400-unit box.
pub fn gold(x: f32, y: f32, w: f32, h: f32) -> Paint {
    Paint::linear_gradient_stops(
        x,
        y + h * 0.15,
        x + w,
        y + h * 1.05,
        [
            (0.0, rgb(0xffe7a3)),
            (0.25, rgb(0xb8862f)),
            (0.5, rgb(0xf6d27a)),
            (0.75, rgb(0x8f6220)),
            (1.0, rgb(0xe8b95a)),
        ],
    )
}

// ---------- the parchment and the castle ----------

/// Warm parchment: a gradient darkening to the edges, stains, two folds down
/// and one across, grain; then the castle in faint ink.
fn parchment(c: &mut Canvas<OpenGl>, italic: FontId) {
    let (w, h) = (800.0, 480.0);
    let paper = Paint::radial_gradient_stops(
        w / 2.0,
        h * 0.45,
        0.0,
        w * 0.62,
        [
            (0.0, rgb(0xf6dda6)),
            (0.52, rgb(0xe7bf7c)),
            (0.88, rgb(0xc58a43)),
            (1.0, rgb(0x9a5e24)),
        ],
    );
    c.fill_path(&rect(0.0, 0.0, w, h), &paper);
    // stains
    for (fx, fy, r, a) in [
        (0.18, 0.72, 75.0, 0.22),
        (0.82, 0.22, 57.0, 0.18),
        (0.63, 0.88, 92.0, 0.2),
        (0.4, 0.15, 101.0, 0.14),
    ] {
        let (x, y) = (fx * w, fy * h);
        let stain = Paint::radial_gradient(x, y, 0.0, r, rgba(0x96501e, a), rgba(0x96501e, 0.0));
        let mut path = Path::new();
        path.circle(x, y, r);
        c.fill_path(&path, &stain);
    }
    // folds: two down, one across
    for x in [w / 3.0, w * 2.0 / 3.0] {
        let fold = Paint::linear_gradient_stops(
            x - 6.0,
            0.0,
            x + 6.0,
            0.0,
            [
                (0.0, rgba(0x5a3c14, 0.0)),
                (0.45, rgba(0x5a3c14, 0.2)),
                (0.55, rgba(0xfffae6, 0.3)),
                (1.0, rgba(0xfffae6, 0.0)),
            ],
        );
        c.fill_path(&rect(x - 6.0, 0.0, 12.0, h), &fold);
    }
    let fold = Paint::linear_gradient_stops(
        0.0,
        h / 2.0 - 5.0,
        0.0,
        h / 2.0 + 5.0,
        [
            (0.0, rgba(0x5a3c14, 0.0)),
            (0.45, rgba(0x5a3c14, 0.16)),
            (0.55, rgba(0xfffae6, 0.24)),
            (1.0, rgba(0xfffae6, 0.0)),
        ],
    );
    c.fill_path(&rect(0.0, h / 2.0 - 5.0, w, 10.0), &fold);
    // grain
    let mut dark = Path::new();
    let mut light = Path::new();
    for i in 0..7000u64 {
        let (x, y) = (unit(i, 1) * w, unit(i, 2) * h);
        if unit(i, 3) < 0.5 {
            &mut dark
        } else {
            &mut light
        }
        .rect(x, y, 0.9, 0.9);
    }
    fill(c, &dark, rgba(0x6b3a10, 0.06));
    fill(c, &light, rgba(0xfff4d8, 0.06));
    castle(c, italic);
}

/// Rooms, corridors (double lines) and stairs, in faint ink, and the names
/// of a few places in an italic hand.
fn castle(c: &mut Canvas<OpenGl>, italic: FontId) {
    let line = rgba(0x5a2d0a, 0.24);
    let mut rooms = Path::new();
    for i in 0..26u64 {
        let (x, y) = (unit(i, 11) * 792.0, unit(i, 12) * 440.0);
        let (rw, rh) = (35.0 + unit(i, 13) * 97.0, 26.0 + unit(i, 14) * 70.0);
        rooms.rect(x, y, rw, rh);
        if unit(i, 15) < 0.4 {
            rooms.rect(x + 2.2, y + 2.2, rw - 4.4, rh - 4.4);
        }
    }
    stroke(c, &rooms, line, 1.0);
    let mut corridors = Path::new();
    for i in 0..34u64 {
        let (x, y) = (unit(i, 21) * 800.0, unit(i, 22) * 480.0);
        let len = 88.0 + unit(i, 23) * 264.0;
        if unit(i, 24) < 0.5 {
            corridors.move_to(x, y);
            corridors.line_to(x + len, y);
            corridors.move_to(x, y + 11.5);
            corridors.line_to(x + len, y + 11.5);
        } else {
            corridors.move_to(x, y);
            corridors.line_to(x, y + len);
            corridors.move_to(x + 11.5, y);
            corridors.line_to(x + 11.5, y + len);
        }
    }
    for i in 0..8u64 {
        let (x, y) = (unit(i, 31) * 814.0, unit(i, 32) * 449.0);
        for k in 0..9 {
            corridors.move_to(x + k as f32 * 3.1, y);
            corridors.line_to(x + k as f32 * 3.1, y + 18.0);
        }
    }
    stroke(c, &corridors, line, 0.7);
    let paint = Paint::color(rgba(0x3a2312, 0.35))
        .with_font(&[italic])
        .with_font_size(11.0)
        .with_text_align(Align::Left)
        .with_text_baseline(Baseline::Alphabetic);
    for (name, x, y) in [
        ("Grande Salle", 167.0, 110.0),
        ("Tour d'astronomie", 660.0, 62.0),
        ("Cachots", 114.0, 431.0),
        ("Bibliothèque", 722.0, 396.0),
        ("Saule cogneur", 519.0, 458.0),
    ] {
        let _ = c.fill_text(x, y, name, &paint);
    }
}

// ---------- the Time-Turner's ring ----------

/// The outer ring with its inscription, 364 x 364 around its centre.
fn ring(c: &mut Canvas<OpenGl>, face: FontId) {
    c.translate(RING / 2.0 - RING_CENTRE.0, RING / 2.0 - RING_CENTRE.1);
    let (cx, cy) = RING_CENTRE;
    let mut band = Path::new();
    band.circle(cx, cy, 166.0);
    c.stroke_path(&band, &gold(0.0, 0.0, 400.0, 400.0).with_line_width(26.0));
    let mut edges = Path::new();
    edges.circle(cx, cy, 179.0);
    edges.circle(cx, cy, 153.0);
    stroke(c, &edges, rgb(0x6b4512), 1.2);
    // the inscription, letter by letter round the ring, read clockwise
    let paint = Paint::color(rgb(0x4a2c0c))
        .with_font(&[face])
        .with_font_size(15.5)
        .with_text_align(Align::Center)
        .with_text_baseline(Baseline::Alphabetic);
    let radius = 160.0;
    let mut angle = 0.0_f32;
    for letter in INSCRIPTION.chars() {
        let text = letter.to_string();
        let width = c
            .measure_text(0.0, 0.0, &text, &paint)
            .map_or(8.0, |m| m.width())
            + 1.55;
        angle += width / 2.0 / radius;
        c.save();
        c.translate(cx + angle.cos() * radius, cy + angle.sin() * radius);
        c.rotate(angle + std::f32::consts::FRAC_PI_2);
        let _ = c.fill_text(0.0, 0.0, &text, &paint);
        c.restore();
        angle += width / 2.0 / radius;
    }
}

/// The dots the Gazette prints its photographs with: one every 4 units.
fn halftone(c: &mut Canvas<OpenGl>) {
    let mut dot = Path::new();
    dot.circle(2.0, 2.0, 1.0);
    fill(c, &dot, rgba(0x1e140a, 0.22));
}

// ---------- the ink every scene draws with ----------

impl Gfx<'_> {
    /// An image stretched over a box.
    pub fn image(&mut self, image: ImageId, x: f32, y: f32, w: f32, h: f32, alpha: f32) {
        let paint = Paint::image(image, x, y, w, h, 0.0, alpha);
        self.canvas.fill_path(&rect(x, y, w, h), &paint);
    }

    /// A room of the castle: its floor, double walls with a door in the middle
    /// of the bottom wall.
    pub fn room(&mut self, x: f32, y: f32, w: f32, h: f32) {
        self.rect(x, y, w, h, ink::room());
        let door = w * 0.24;
        let (dl, dr) = (x + (w - door) / 2.0, x + f32::midpoint(w, door));
        let walls = |inset: f32| {
            let (l, t, r, b) = (x + inset, y + inset, x + w - inset, y + h - inset);
            let mut path = Path::new();
            path.move_to(dl, b);
            path.line_to(l, b);
            path.line_to(l, t);
            path.line_to(r, t);
            path.line_to(r, b);
            path.line_to(dr, b);
            path
        };
        self.stroke(&walls(1.2), ink::ink(), 2.2);
        self.stroke(&walls(5.5), ink::ink(), 1.0);
        let mut jambs = Path::new();
        jambs.move_to(dl, y + h - 5.5);
        jambs.line_to(dl, y + h);
        jambs.move_to(dr, y + h - 5.5);
        jambs.line_to(dr, y + h);
        self.stroke(&jambs, ink::ink(), 1.0);
    }

    /// A name on its ribbon, as the Map labels people, centred on `cx` with
    /// its top at `y`; returns its width.
    pub fn ribbon(&mut self, cx: f32, y: f32, text: &str, alpha: f32) -> f32 {
        let style = Type::new(Face::Italic, 12.0);
        let w = self.measure(text, style) + 16.0;
        self.rect(cx - w / 2.0, y, w, 18.0, fade(ink::ribbon(), alpha));
        self.rect_line(cx - w / 2.0, y, w, 18.0, fade(ink::ink(), alpha), 1.2);
        self.text(
            cx,
            y + 9.5,
            text,
            style,
            fade(ink::ink(), alpha),
            Anchor::MIDDLE_CENTER,
        );
        w
    }

    /// A word stamped in red ink, a little askew; returns its width.
    pub fn stamp(&mut self, x: f32, y: f32, text: &str, degrees: f32) -> f32 {
        let style = Type::new(Face::Body, 11.5).spaced(1.2);
        let text = text.to_uppercase();
        let w = self.measure(&text, style) + 14.0;
        let red = fade(ink::red(), 0.9);
        self.canvas.save();
        self.canvas.translate(x + w / 2.0, y + 11.0);
        self.canvas.rotate(degrees.to_radians());
        self.rect_line(-w / 2.0, -11.0, w, 22.0, red, 1.6);
        self.text(0.0, 0.5, &text, style, red, Anchor::MIDDLE_CENTER);
        self.canvas.restore();
        w
    }

    /// The fleuron that opens a label: a heart with a leaf's tail, `size`
    /// tall, centred on (`x`, `y`).
    pub fn fleuron(&mut self, x: f32, y: f32, size: f32, color: Color) {
        let s = size / 10.0;
        let mut path = Path::new();
        path.move_to(x, y + 4.5 * s);
        path.bezier_to(
            x - 6.0 * s,
            y + 0.5 * s,
            x - 5.0 * s,
            y - 5.0 * s,
            x - 0.5 * s,
            y - 2.0 * s,
        );
        path.bezier_to(
            x + 4.0 * s,
            y - 5.5 * s,
            x + 6.0 * s,
            y + 0.5 * s,
            x,
            y + 4.5 * s,
        );
        path.close();
        self.fill(&path, color);
        let mut tail = Path::new();
        tail.move_to(x, y + 4.0 * s);
        tail.bezier_to(
            x + 1.0 * s,
            y + 1.0 * s,
            x - 2.5 * s,
            y - 1.0 * s,
            x - 3.5 * s,
            y - 4.0 * s,
        );
        self.stroke(&tail, color, 0.9 * s);
    }

    /// A dashed rule, as the Map draws under its labels.
    pub fn dashed_rule(&mut self, x: f32, y: f32, w: f32, color: Color) {
        let mut path = Path::new();
        path.move_to(x, y);
        path.line_to(x + w, y);
        let paint = Paint::color(color)
            .with_line_width(1.0)
            .with_line_dash(&[4.0, 3.5]);
        self.canvas.stroke_path(&path, &paint);
    }

    /// The Gazette's double rule around a box.
    pub fn double_rule(&mut self, x: f32, y: f32, w: f32, h: f32, color: Color) {
        self.rect_line(x, y, w, h, color, 1.0);
        self.rect_line(x + 4.0, y + 4.0, w - 8.0, h - 8.0, color, 1.0);
    }

    /// A page of the Gazette pinned to the Map: newsprint over a soft shadow,
    /// turned by `degrees`, its red pin at the top. What is written on it
    /// stays square (turned text would be resampled, and blur).
    pub fn pinned(&mut self, x: f32, y: f32, w: f32, h: f32, degrees: f32) {
        let (cx, cy) = (x + w / 2.0, y + h / 2.0);
        self.canvas.save();
        self.canvas.translate(cx, cy);
        self.canvas.rotate(degrees.to_radians());
        self.canvas.translate(-cx, -cy);
        let mut shadow = Path::new();
        shadow.rect(x - 10.0, y - 8.0, w + 20.0, h + 22.0);
        let soft = Paint::box_gradient(
            x + 2.0,
            y + 4.0,
            w,
            h,
            6.0,
            14.0,
            rgba(0x3a1e08, 0.4),
            rgba(0x3a1e08, 0.0),
        );
        self.canvas.fill_path(&shadow, &soft);
        self.rect(x, y, w, h, ink::newsprint());
        self.canvas.restore();
        self.pin(cx, y + 3.0);
    }

    /// The red pin that holds a page on the Map.
    pub fn pin(&mut self, x: f32, y: f32) {
        let mut path = Path::new();
        path.circle(x, y, 6.0);
        let head = Paint::radial_gradient_stops(
            x - 2.0,
            y - 2.0,
            0.0,
            7.5,
            [
                (0.0, rgb(0xe0605a)),
                (0.6, rgb(0x8c1d22)),
                (1.0, rgb(0x4a0c10)),
            ],
        );
        self.canvas.fill_path(&path, &head);
    }

    /// A photograph as the Gazette prints it: the picture (sepia, fetched
    /// with `Finish::Print`), its halftone, and a thin rule.
    pub fn photo(&mut self, image: ImageId, x: f32, y: f32, w: f32, h: f32, alpha: f32) {
        self.picture(image, x, y, w, h, alpha);
        if let Some(map) = self.map {
            let dots = Paint::image(map.halftone, x, y, 4.0, 4.0, 0.0, alpha);
            self.canvas.fill_path(&rect(x, y, w, h), &dots);
        }
        self.rect_line(x, y, w, h, fade(ink::news_ink(), alpha), 1.0);
    }

    /// The parchment's burnt edges, darkening towards the corners.
    pub fn burnt_edges(&mut self) {
        let (w, h) = (800.0, 480.0);
        let burn = Paint::radial_gradient_stops(
            w / 2.0,
            h / 2.0,
            w * 0.62 * 0.72,
            w * 0.62,
            [
                (0.0, rgba(0x6e320a, 0.0)),
                (0.64, rgba(0x6e320a, 0.35)),
                (1.0, rgba(0x461a05, 0.72)),
            ],
        );
        self.canvas.save();
        self.canvas.translate(w / 2.0, h / 2.0);
        self.canvas.scale(1.0, h / w * 1.1);
        self.canvas.translate(-w / 2.0, -h / 2.0);
        self.canvas.fill_path(&rect(0.0, -h, w, h * 3.0), &burn);
        self.canvas.restore();
    }
}
