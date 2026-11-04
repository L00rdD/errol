//! Cyberpunk's background: a perspective grid scrolling to a red horizon,
//! columns of data falling above it, glitch blocks flashing.

use femtovg::Path;

use crate::anim::{hash, unit};
use crate::look::fade;
use crate::render::{Anchor, Face, Gfx, Type};

const W: f32 = 800.0;
const H: f32 = 480.0;
const HORIZON: f32 = H * 0.58;
// the fonts are Latin only: hex digits and symbols, no katakana
const GLYPHS: &[&str] = &[
    "0", "1", "2", "7", "9", "A", "C", "E", "F", "#", "$", "%", "&", "@", "<", ">", "/", "=", "+",
    "X", "?",
];

struct Column {
    x: f32,
    y: f32,
    speed: f32,
    on: bool,
}

struct Block {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    life: f32,
    colour: u8,
}

pub struct Grid {
    columns: Vec<Column>,
    blocks: Vec<Block>,
    heat: f32,
    scroll: f32,
    seed: u64,
}

impl Grid {
    pub fn new() -> Self {
        let columns = (0..34u64)
            .map(|i| Column {
                x: i as f32 * 24.0 + 6.0,
                y: (unit(i, 1) * 2.0 - 1.0) * H,
                speed: 36.0 + unit(i, 2) * 70.0,
                on: unit(i, 3) < 0.35,
            })
            .collect();
        Self {
            columns,
            blocks: Vec::new(),
            heat: 0.0,
            scroll: 0.0,
            seed: 3,
        }
    }

    fn rand(&mut self) -> f32 {
        self.seed += 1;
        unit(self.seed, 7)
    }

    pub fn burst(&mut self, power: f32) {
        self.heat = (self.heat + power).min(1.5);
        for index in 0..self.columns.len() {
            if self.rand() < 0.6 {
                self.columns[index].on = true;
            }
        }
    }

    pub fn draw(&mut self, gfx: &mut Gfx, dt: f32) {
        let heat = self.heat;
        self.heat *= (-dt * 0.45).exp();
        self.floor(gfx, dt, heat);
        self.columns(gfx, dt, heat);
        self.glitch(gfx, dt, heat);
    }

    /// The horizon glows red; floor lines come closer, faster when hot.
    fn floor(&mut self, gfx: &mut Gfx, dt: f32, heat: f32) {
        let red = gfx.palette.accent;
        gfx.vertical_gradient(
            0.0,
            HORIZON - 140.0,
            W,
            140.0,
            fade(red, 0.0),
            fade(red, 0.18 + heat * 0.2),
        );
        self.scroll = (self.scroll + dt * (36.0 + heat * 120.0)) % 40.0;
        let mut floor = Path::new();
        for k in 0..14 {
            let z = (k as f32 * 40.0 + self.scroll) / 560.0;
            let y = HORIZON + (H - HORIZON) * z * z;
            floor.move_to(0.0, y);
            floor.line_to(W, y);
        }
        for k in -12..=12 {
            floor.move_to(W / 2.0 + k as f32 * 12.0, HORIZON);
            floor.line_to(W / 2.0 + k as f32 * 110.0, H);
        }
        gfx.stroke(&floor, fade(red, 0.3 + heat * 0.3), 1.0);
    }

    /// Columns of data falling; a column that wakes starts falling, one that
    /// fell below the horizon starts again from the top.
    fn columns(&mut self, gfx: &mut Gfx, dt: f32, heat: f32) {
        let tick = (gfx.t * 12.0) as u64;
        for index in 0..self.columns.len() {
            let wake = self.rand() < (0.06 + heat * 1.5) * dt;
            let column = &mut self.columns[index];
            column.on |= wake;
            if !column.on {
                continue;
            }
            column.y += column.speed * dt * (1.0 + heat * 2.0);
            let (column_x, tip) = (column.x, column.y);
            trail(gfx, index, tick, (column_x, tip));
            if tip - 130.0 > HORIZON {
                self.respawn(index);
            }
        }
    }

    fn respawn(&mut self, index: usize) {
        let (y, speed, on) = (
            self.rand() * -200.0,
            36.0 + self.rand() * 70.0,
            self.rand() < 0.5,
        );
        let column = &mut self.columns[index];
        (column.y, column.speed, column.on) = (y, speed, on);
    }

    /// Glitch blocks: one now and then, more when hot, gone in a blink.
    fn glitch(&mut self, gfx: &mut Gfx, dt: f32, heat: f32) {
        if self.rand() < (0.9 + heat * 12.0) * dt {
            let block = Block {
                x: self.rand() * W,
                y: self.rand() * H,
                w: 20.0 + self.rand() * 140.0,
                h: 2.0 + self.rand() * 8.0,
                life: 0.1 + self.rand() * 0.25,
                colour: (self.rand() * 3.0) as u8,
            };
            self.blocks.push(block);
        }
        let palette = [gfx.palette.accent, gfx.palette.accent2, gfx.palette.hot];
        for block in &mut self.blocks {
            let colour = palette[usize::from(block.colour.min(2))];
            gfx.rect(block.x, block.y, block.w, block.h, fade(colour, 0.35));
            block.life -= dt;
        }
        self.blocks.retain(|block| block.life > 0.0);
    }
}

/// A column's glyphs, its tip at (x, tip): yellow at the tip, fading cyan
/// above, only above the horizon.
fn trail(gfx: &mut Gfx, index: usize, tick: u64, (x, tip): (f32, f32)) {
    let (cyan, yellow) = (gfx.palette.accent2, gfx.palette.hot);
    for k in 0..9 {
        let y = tip - k as f32 * 14.0;
        if !(0.0..=HORIZON).contains(&y) {
            continue;
        }
        let colour = if k == 0 {
            fade(yellow, 0.9)
        } else {
            fade(cyan, 0.5 - k as f32 * 0.05)
        };
        gfx.text(
            x,
            y,
            glyph(tick, k, index),
            Type::new(Face::Body, 12.0),
            colour,
            Anchor::TOP_LEFT,
        );
    }
}

/// The `k`-th glyph of column `index`, changing twelve times a second.
fn glyph(tick: u64, k: u64, index: usize) -> &'static str {
    GLYPHS[(hash(tick + k, index as u64) % GLYPHS.len() as u64) as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_glyph_stays_put_within_a_tick() {
        assert_eq!(glyph(5, 1, 3), glyph(5, 1, 3));
        assert!(GLYPHS.contains(&glyph(9, 8, 33)));
    }

    #[test]
    fn a_column_falls_back_to_the_top() {
        let mut grid = Grid::new();
        grid.respawn(0);
        assert!(grid.columns[0].y <= 0.0 && grid.columns[0].y >= -200.0);
        assert!((36.0..=106.0).contains(&grid.columns[0].speed));
    }

    #[test]
    fn a_burst_wakes_columns_and_heats() {
        let mut grid = Grid::new();
        grid.burst(2.0);
        assert!((grid.heat - 1.5).abs() < f32::EPSILON);
        assert!(grid.columns.iter().filter(|column| column.on).count() > 12);
    }
}
