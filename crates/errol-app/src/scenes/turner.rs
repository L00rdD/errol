//! Hermione's Time-Turner, the Map's clock, after Pensieve's: on its chain,
//! the outer ring engraved with its inscription and turning slowly, the hours
//! round it (the current one in red ink), the inner rings turning on
//! themselves, and the hourglass in the middle emptying as the hour goes by.
//! Drawn in a 400-unit design, its centre at (200, 236), scaled to fit.

use std::f32::consts::TAU;

use femtovg::{Color, Path};

use crate::look::{fade, rgb};
use crate::render::map::{RING, gold};
use crate::render::{Anchor, Face, Gfx, Type, ink};

const CENTRE: (f32, f32) = (200.0, 236.0);
const ROMAN: [&str; 12] = [
    "XII", "I", "II", "III", "IV", "V", "VI", "VII", "VIII", "IX", "X", "XI",
];

/// The Time-Turner centred on (`cx`, `cy`), `scale` units to a pixel, for
/// `hour` (0–11) with `progress` (0 → 1) of it gone by.
pub fn draw(gfx: &mut Gfx, cx: f32, cy: f32, scale: f32, hour: usize, progress: f32) {
    let t = gfx.t;
    gfx.canvas.save();
    gfx.canvas.translate(cx, cy);
    gfx.canvas.scale(scale, scale);
    gfx.canvas.translate(-CENTRE.0, -CENTRE.1);
    let (x, y) = CENTRE;
    chain(gfx);
    // its glow on the parchment
    gfx.halo(x, y, 178.0, fade(rgb(0xffd27a), 0.5));
    outer_ring(gfx, t);
    hours(gfx, hour);
    inner_rings(gfx, t);
    hourglass_at(gfx, progress, t);
    gfx.canvas.restore();
}

/// The chain it hangs from: links alternately seen flat and edge-on.
fn chain(gfx: &mut Gfx) {
    let mut chain = Path::new();
    for k in 0..6 {
        chain.ellipse(200.0, 8.0 + k as f32 * 11.0, link_half_width(k), 7.0);
    }
    gfx.canvas
        .stroke_path(&chain, &gold(0.0, 0.0, 400.0, 400.0).with_line_width(3.0));
}

/// Half the width of the `k`th link: every other one is seen edge-on.
fn link_half_width(k: usize) -> f32 {
    if k % 2 == 1 { 3.0 } else { 6.0 }
}

/// The outer ring, engraved, turning; a plain gold band when the Map's
/// images could not be baked.
fn outer_ring(gfx: &mut Gfx, t: f32) {
    let (x, y) = CENTRE;
    if let Some(map) = gfx.map {
        gfx.canvas.save();
        gfx.canvas.translate(x, y);
        gfx.canvas.rotate((t * 4.0).to_radians());
        gfx.image(map.ring, -RING / 2.0, -RING / 2.0, RING, RING, 1.0);
        gfx.canvas.restore();
    } else {
        let mut band = Path::new();
        band.circle(x, y, 166.0);
        gfx.canvas
            .stroke_path(&band, &gold(0.0, 0.0, 400.0, 400.0).with_line_width(26.0));
    }
}

/// The hours round the dial, the current one larger and in red ink.
fn hours(gfx: &mut Gfx, hour: usize) {
    let (x, y) = CENTRE;
    gfx.disc(x, y, 146.0, fade(rgb(0x46280a), 0.08));
    gfx.ring(x, y, 146.0, 1.0, rgb(0x6b4512), &[], 0.0, TAU);
    for (i, numeral) in ROMAN.iter().enumerate() {
        let a = i as f32 * TAU / 12.0;
        let (colour, size) = numeral_style(i, hour);
        gfx.text(
            x + a.sin() * 130.0,
            y + 6.0 - a.cos() * 130.0,
            numeral,
            Type::new(Face::Body, size),
            colour,
            Anchor::MIDDLE_CENTER,
        );
    }
}

/// The colour and size of the `i`th numeral: the current hour stands out.
fn numeral_style(i: usize, hour: usize) -> (Color, f32) {
    if i == hour {
        (ink::red(), 21.0)
    } else {
        (rgb(0x4a2c0c), 17.0)
    }
}

/// The inner rings, turning on themselves.
fn inner_rings(gfx: &mut Gfx, t: f32) {
    let (x, y) = CENTRE;
    for (radius, period, width) in [(112.0, 7.0, 9.0), (96.0, -11.0, 6.0)] {
        gfx.canvas.save();
        gfx.canvas.translate(x, y);
        gfx.canvas.scale((t / period * TAU).cos(), 1.0);
        let mut ring = Path::new();
        ring.circle(0.0, 0.0, radius);
        gfx.canvas.stroke_path(
            &ring,
            &gold(-200.0, -236.0, 400.0, 400.0).with_line_width(width),
        );
        gfx.canvas.restore();
    }
}

/// The hourglass alone, its top left at (`x`, `y`), `scale` units to a pixel
/// (it is 80 x 172 units), `progress` (0 → 1) of its sand fallen.
pub fn hourglass(gfx: &mut Gfx, x: f32, y: f32, scale: f32, progress: f32) {
    let t = gfx.t;
    gfx.canvas.save();
    gfx.canvas.translate(x, y);
    gfx.canvas.scale(scale, scale);
    gfx.canvas.translate(-160.0, -150.0);
    hourglass_at(gfx, progress, t);
    gfx.canvas.restore();
}

/// The hourglass in design units: gold caps and posts, the glass, the sand
/// above draining into the pile below, grains falling through the neck.
fn hourglass_at(gfx: &mut Gfx, progress: f32, t: f32) {
    let progress = progress.clamp(0.0, 1.0);
    let metal = gold(0.0, 0.0, 400.0, 400.0);
    let mut caps = Path::new();
    caps.rounded_rect(160.0, 150.0, 80.0, 9.0, 3.0);
    caps.rounded_rect(160.0, 313.0, 80.0, 9.0, 3.0);
    gfx.canvas.fill_path(&caps, &metal);
    let mut glass = Path::new();
    glass.move_to(167.0, 159.0);
    glass.line_to(233.0, 159.0);
    glass.line_to(204.0, 234.0);
    glass.line_to(204.0, 238.0);
    glass.line_to(233.0, 313.0);
    glass.line_to(167.0, 313.0);
    glass.line_to(196.0, 238.0);
    glass.line_to(196.0, 234.0);
    glass.close();
    gfx.fill(&glass, fade(rgb(0xfff6dd), 0.3));
    gfx.stroke(&glass, rgb(0x8f6220), 1.4);
    let sand = rgb(0xd9a441);
    let top = 70.0 * (1.0 - progress);
    let hw = 33.0 * top / 75.0;
    if top > 1.0 {
        let mut above = Path::new();
        above.move_to(200.0 - hw, 234.0 - top);
        above.line_to(200.0 + hw, 234.0 - top);
        above.line_to(201.0, 234.0);
        above.line_to(199.0, 234.0);
        above.close();
        gfx.fill(&above, sand);
    }
    let pile = 64.0 * progress.sqrt();
    let pw = 33.0 * (1.0 - pile / 75.0);
    let mut heap = Path::new();
    heap.move_to(167.0, 313.0);
    heap.line_to(233.0, 313.0);
    heap.line_to(200.0 + pw, 313.0 - pile);
    heap.quad_to(200.0, 313.0 - pile - 6.0, 200.0 - pw, 313.0 - pile);
    heap.close();
    gfx.fill(&heap, rgb(0xc98f2e));
    if progress > 0.0 && progress < 1.0 {
        let mut grains = Path::new();
        let mut gy = 236.0 + (t * 10.0) % 6.0;
        while gy < 313.0 - pile - 4.0 {
            grains.rect(199.2, gy, 1.6, 3.0);
            gy += 6.0;
        }
        gfx.fill(&grains, sand);
    }
    let mut posts = Path::new();
    posts.move_to(163.0, 159.0);
    posts.line_to(163.0, 313.0);
    posts.move_to(237.0, 159.0);
    posts.line_to(237.0, 313.0);
    gfx.canvas.stroke_path(&posts, &metal.with_line_width(3.0));
}

/// Keeps the sand's colour in one place for the cards that print it.
pub fn sand() -> Color {
    rgb(0xd9a441)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_links_alternate() {
        assert!((link_half_width(0) - 6.0).abs() < f32::EPSILON);
        assert!((link_half_width(1) - 3.0).abs() < f32::EPSILON);
    }

    #[test]
    fn the_current_hour_is_larger_and_red() {
        assert_eq!(numeral_style(3, 3), (ink::red(), 21.0));
        assert_eq!(numeral_style(4, 3), (rgb(0x4a2c0c), 17.0));
    }
}
