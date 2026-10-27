//! Drawing helpers over femtovg: text in the looks' fonts, frames, rings,
//! glows, pictures. Every scene draws through a `Gfx`.

use femtovg::renderer::OpenGl;
use femtovg::{Align, Baseline, Canvas, Color, FontId, ImageId, LineCap, Paint, Path, Solidity};

use crate::look::{Look, Palette, fade};

pub struct Fonts {
    pub michroma: FontId,
    pub medium: FontId,
    pub semibold: FontId,
    pub bold: FontId,
    pub cinzel: FontId,
    pub fell: FontId,
    pub fell_italic: FontId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Face {
    /// Titles: Michroma in friday, Rajdhani Bold in cyberpunk, Cinzel in hogwarts.
    Display,
    Body,
    Strong,
    Bold,
    /// Hogwarts' italic hand (IM Fell English); the body face elsewhere.
    Italic,
}

/// Where a text is anchored.
#[derive(Debug, Clone, Copy)]
pub struct Anchor {
    pub align: Align,
    pub baseline: Baseline,
}

impl Anchor {
    pub const TOP_LEFT: Self = Self {
        align: Align::Left,
        baseline: Baseline::Top,
    };
    pub const TOP_CENTER: Self = Self {
        align: Align::Center,
        baseline: Baseline::Top,
    };
    pub const TOP_RIGHT: Self = Self {
        align: Align::Right,
        baseline: Baseline::Top,
    };
    pub const MIDDLE_LEFT: Self = Self {
        align: Align::Left,
        baseline: Baseline::Middle,
    };
    pub const MIDDLE_CENTER: Self = Self {
        align: Align::Center,
        baseline: Baseline::Middle,
    };
    pub const MIDDLE_RIGHT: Self = Self {
        align: Align::Right,
        baseline: Baseline::Middle,
    };
}

/// A text style: face, size, letter spacing.
#[derive(Debug, Clone, Copy)]
pub struct Type {
    pub face: Face,
    pub size: f32,
    pub spacing: f32,
}

impl Type {
    pub const fn new(face: Face, size: f32) -> Self {
        Self {
            face,
            size,
            spacing: 0.0,
        }
    }

    #[must_use]
    pub const fn spaced(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }
}

pub struct Gfx<'a> {
    pub canvas: &'a mut Canvas<OpenGl>,
    pub fonts: &'a Fonts,
    pub look: Look,
    pub palette: Palette,
    /// Seconds since the screen started (drives every loop).
    pub t: f32,
    /// The Great Hall's baked sprites, while the look is hogwarts.
    pub hall: Option<&'a super::Hall>,
}

impl Gfx<'_> {
    pub fn cyber(&self) -> bool {
        self.look == Look::Cyberpunk
    }

    pub fn hogwarts(&self) -> bool {
        self.look == Look::Hogwarts
    }

    /// What a title is made of while it appears: hex digits in friday, symbols
    /// in cyberpunk, nothing in hogwarts (the letters are written in ink).
    pub fn glyphs(&self) -> &'static str {
        match self.look {
            Look::Friday => "0123456789ABCDEF·:/",
            Look::Cyberpunk => "#$%&@!?/<>=+01",
            Look::Hogwarts => "",
        }
    }

    fn font(&self, face: Face) -> [FontId; 2] {
        // Michroma has no ◆, no arrows: Rajdhani catches what it lacks
        let primary = match (face, self.look) {
            (Face::Display, Look::Friday) => self.fonts.michroma,
            (Face::Display | Face::Strong | Face::Bold, Look::Hogwarts) => self.fonts.cinzel,
            (Face::Body, Look::Hogwarts) => self.fonts.fell,
            (Face::Italic, Look::Hogwarts) => self.fonts.fell_italic,
            (Face::Display | Face::Bold, _) => self.fonts.bold,
            (Face::Strong, _) => self.fonts.semibold,
            (Face::Body | Face::Italic, _) => self.fonts.medium,
        };
        [primary, self.fonts.semibold]
    }

    fn text_paint(&self, style: Type, color: Color, anchor: Anchor) -> Paint {
        Paint::color(color)
            .with_font(&self.font(style.face))
            .with_font_size(style.size)
            .with_letter_spacing(style.spacing)
            .with_text_align(anchor.align)
            .with_text_baseline(anchor.baseline)
    }

    /// Draws a line of text; returns its width.
    pub fn text(
        &mut self,
        x: f32,
        y: f32,
        text: &str,
        style: Type,
        color: Color,
        anchor: Anchor,
    ) -> f32 {
        let paint = self.text_paint(style, color, anchor);
        self.canvas
            .fill_text(x, y, text, &paint)
            .map(|metrics| metrics.width())
            .unwrap_or_default()
    }

    pub fn measure(&self, text: &str, style: Type) -> f32 {
        let paint = self.text_paint(style, Color::white(), Anchor::TOP_LEFT);
        self.canvas
            .measure_text(0.0, 0.0, text, &paint)
            .map(|metrics| metrics.width())
            .unwrap_or_default()
    }

    /// A text with a soft halo: the same text, faint, a few times around it.
    pub fn glow_text(
        &mut self,
        x: f32,
        y: f32,
        text: &str,
        style: Type,
        color: Color,
        anchor: Anchor,
        halo: f32,
    ) {
        let faint = fade(color, 0.16);
        for (dx, dy) in [(-halo, 0.0), (halo, 0.0), (0.0, -halo), (0.0, halo)] {
            self.text(x + dx, y + dy, text, style, faint, anchor);
        }
        self.text(x, y, text, style, color, anchor);
    }

    /// Cyberpunk's chromatic split: the text in the two accents, offset,
    /// under the text itself. `jitter` widens it during a glitch.
    pub fn split_text(
        &mut self,
        x: f32,
        y: f32,
        text: &str,
        style: Type,
        color: Color,
        anchor: Anchor,
        jitter: f32,
    ) {
        let offset = 2.0 + jitter;
        let (left, right) = (self.palette.accent, self.palette.accent2);
        self.text(x + offset, y, text, style, fade(left, 0.9), anchor);
        self.text(x - offset, y, text, style, fade(right, 0.8), anchor);
        self.text(x, y, text, style, color, anchor);
    }

    /// The title of a look: glow in friday, chromatic split in cyberpunk,
    /// gold lit by candles in hogwarts.
    pub fn title(&mut self, x: f32, y: f32, text: &str, style: Type, color: Color, anchor: Anchor) {
        if self.cyber() {
            self.split_text(x, y, text, style, color, anchor, 0.0);
        } else if self.hogwarts() {
            let warm = Color::rgbaf(1.0, 0.77, 0.36, 1.0);
            self.glow_text(
                x,
                y,
                text,
                style,
                crate::look::mix(color, warm, 0.15),
                anchor,
                1.6,
            );
        } else {
            self.glow_text(x, y, text, style, color, anchor, 1.2);
        }
    }

    /// Hogwarts' frame: a double gilt rule, like the portraits of the castle.
    pub fn gilt_frame(&mut self, x: f32, y: f32, w: f32, h: f32) {
        let gold = self.palette.accent;
        self.rect_line(x, y, w, h, fade(gold, 0.85), 1.5);
        self.rect_line(x + 4.0, y + 4.0, w - 8.0, h - 8.0, fade(gold, 0.4), 1.0);
    }

    /// A small diamond, hogwarts' star between words.
    pub fn diamond(&mut self, x: f32, y: f32, r: f32, color: Color) {
        let mut path = Path::new();
        path.move_to(x, y - r);
        path.line_to(x + r * 0.7, y);
        path.line_to(x, y + r);
        path.line_to(x - r * 0.7, y);
        path.close();
        self.fill(&path, color);
    }

    /// Lines of `text` wrapped in `width`, at most `max_lines` (the last one
    /// ends with … when cut). Returns the lines.
    pub fn wrap(&self, text: &str, width: f32, style: Type, max_lines: usize) -> Vec<String> {
        let paint = self.text_paint(style, Color::white(), Anchor::TOP_LEFT);
        let ranges = self
            .canvas
            .break_text_vec(width, text, &paint)
            .unwrap_or_default();
        let mut lines: Vec<String> = ranges
            .iter()
            .map(|range| text[range.clone()].trim().to_owned())
            .collect();
        if lines.len() > max_lines {
            lines.truncate(max_lines);
            if let Some(last) = lines.last_mut() {
                while !last.is_empty() && self.measure(&format!("{last}…"), style) > width {
                    last.pop();
                }
                last.push('…');
            }
        }
        lines
    }

    /// Draws wrapped text; returns the height used.
    pub fn paragraph(
        &mut self,
        x: f32,
        y: f32,
        text: &str,
        width: f32,
        style: Type,
        line: f32,
        color: Color,
        max_lines: usize,
    ) -> f32 {
        let lines = self.wrap(text, width, style, max_lines);
        for (index, row) in lines.iter().enumerate() {
            self.text(
                x,
                y + index as f32 * line,
                row,
                style,
                color,
                Anchor::TOP_LEFT,
            );
        }
        lines.len() as f32 * line
    }

    pub fn fill(&mut self, path: &Path, color: Color) {
        self.canvas.fill_path(path, &Paint::color(color));
    }

    pub fn stroke(&mut self, path: &Path, color: Color, width: f32) {
        self.canvas
            .stroke_path(path, &Paint::color(color).with_line_width(width));
    }

    pub fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: Color) {
        let mut path = Path::new();
        path.rect(x, y, w, h);
        self.fill(&path, color);
    }

    pub fn rect_line(&mut self, x: f32, y: f32, w: f32, h: f32, color: Color, width: f32) {
        let mut path = Path::new();
        path.rect(x + width / 2.0, y + width / 2.0, w - width, h - width);
        self.stroke(&path, color, width);
    }

    pub fn disc(&mut self, x: f32, y: f32, r: f32, color: Color) {
        let mut path = Path::new();
        path.circle(x, y, r);
        self.fill(&path, color);
    }

    /// A soft round light fading out from `color` at the centre.
    pub fn halo(&mut self, x: f32, y: f32, r: f32, color: Color) {
        let mut path = Path::new();
        path.circle(x, y, r);
        let paint = Paint::radial_gradient(x, y, 0.0, r, color, fade(color, 0.0));
        self.canvas.fill_path(&path, &paint);
    }

    /// A ring, possibly dashed, turned by `angle`, from `from` to `to` (radians).
    #[allow(clippy::too_many_arguments)]
    pub fn ring(
        &mut self,
        x: f32,
        y: f32,
        r: f32,
        width: f32,
        color: Color,
        dash: &[f32],
        angle: f32,
        sweep: f32,
    ) {
        let mut path = Path::new();
        if sweep >= std::f32::consts::TAU - 1e-3 {
            path.circle(x, y, r);
        } else {
            path.arc(x, y, r, angle, angle + sweep, Solidity::Hole);
        }
        let mut paint = Paint::color(color)
            .with_line_width(width)
            .with_line_cap(LineCap::Butt);
        if !dash.is_empty() {
            paint.set_line_dash(dash);
            // the dash pattern turns with the ring
            paint.set_line_dash_offset(-angle * r);
        }
        self.canvas.stroke_path(&path, &paint);
    }

    /// Corner brackets around a box (friday's frames).
    pub fn brackets(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        length: f32,
        color: Color,
        width: f32,
    ) {
        let mut path = Path::new();
        path.move_to(x, y + length);
        path.line_to(x, y);
        path.line_to(x + length, y);
        path.move_to(x + w - length, y + h);
        path.line_to(x + w, y + h);
        path.line_to(x + w, y + h - length);
        self.stroke(&path, color, width);
    }

    /// A box with its top right corner cut (cyberpunk's panels).
    pub fn cut_box(x: f32, y: f32, w: f32, h: f32, cut: f32) -> Path {
        let mut path = Path::new();
        path.move_to(x, y);
        path.line_to(x + w - cut, y);
        path.line_to(x + w, y + cut);
        path.line_to(x + w, y + h);
        path.line_to(x, y + h);
        path.close();
        path
    }

    /// A horizontal gradient fill.
    pub fn gradient_rect(&mut self, x: f32, y: f32, w: f32, h: f32, from: Color, to: Color) {
        let mut path = Path::new();
        path.rect(x, y, w, h);
        let paint = Paint::linear_gradient(x, y, x + w, y, from, to);
        self.canvas.fill_path(&path, &paint);
    }

    pub fn vertical_gradient(&mut self, x: f32, y: f32, w: f32, h: f32, from: Color, to: Color) {
        let mut path = Path::new();
        path.rect(x, y, w, h);
        let paint = Paint::linear_gradient(x, y, x, y + h, from, to);
        self.canvas.fill_path(&path, &paint);
    }

    /// A picture fitted in a box, cropped to fill it (CSS object-fit: cover).
    pub fn picture(&mut self, image: ImageId, x: f32, y: f32, w: f32, h: f32, alpha: f32) {
        let Ok((width, height)) = self.canvas.image_size(image) else {
            return;
        };
        let scale = (w / width as f32).max(h / height as f32);
        let (iw, ih) = (width as f32 * scale, height as f32 * scale);
        let paint = Paint::image(
            image,
            x + (w - iw) / 2.0,
            y + (h - ih) / 2.0,
            iw,
            ih,
            0.0,
            alpha,
        );
        let mut path = Path::new();
        path.rect(x, y, w, h);
        self.canvas.fill_path(&path, &paint);
    }

    /// A small tag: a framed label in friday, a slanted filled one in
    /// cyberpunk, a gilded plate in hogwarts.
    pub fn tag(&mut self, x: f32, y: f32, text: &str) -> f32 {
        let style = Type::new(Face::Strong, 14.0).spaced(1.2);
        let w = self.measure(text, style) + 20.0;
        if self.hogwarts() {
            let plate = Paint::linear_gradient(
                x,
                y,
                x,
                y + 22.0,
                Color::rgbaf(0.83, 0.66, 0.29, 0.28),
                Color::rgbaf(0.83, 0.66, 0.29, 0.08),
            );
            let mut path = Path::new();
            path.rect(x, y, w, 22.0);
            self.canvas.fill_path(&path, &plate);
            self.rect_line(x, y, w, 22.0, fade(self.palette.accent, 0.8), 1.0);
            self.text(
                x + w / 2.0,
                y + 11.5,
                text,
                Type::new(Face::Strong, 12.5).spaced(1.4),
                self.palette.accent2,
                Anchor::MIDDLE_CENTER,
            );
        } else if self.cyber() {
            let mut path = Path::new();
            path.move_to(x + 6.0, y);
            path.line_to(x + w, y);
            path.line_to(x + w - 6.0, y + 22.0);
            path.line_to(x, y + 22.0);
            path.close();
            self.fill(&path, self.palette.accent);
            self.text(
                x + w / 2.0,
                y + 11.5,
                text,
                style,
                Color::black(),
                Anchor::MIDDLE_CENTER,
            );
        } else {
            self.rect_line(x, y, w, 22.0, fade(self.palette.accent, 0.45), 1.0);
            self.text(
                x + w / 2.0,
                y + 11.5,
                text,
                style,
                self.palette.text,
                Anchor::MIDDLE_CENTER,
            );
        }
        w
    }

    /// The small caps line above a title, with its dash (a gold star in hogwarts).
    pub fn kicker(&mut self, x: f32, y: f32, text: &str, color: Color) {
        if self.hogwarts() {
            self.diamond(x + 9.0, y + 6.0, 5.0, color);
        } else {
            self.rect(x, y + 5.0, 18.0, 2.0, color);
        }
        self.text(
            x + 26.0,
            y,
            &text.to_uppercase(),
            Type::new(Face::Strong, 12.0).spaced(3.2),
            color,
            Anchor::TOP_LEFT,
        );
    }
}
