//! Cyberpunk's background: a perspective grid scrolling to a red horizon,
//! columns of data falling above it, glitch blocks flashing. News makes the
//! data rain rush (or slow, for a lamp going out), flares a column in its
//! colour and sweeps a scan line down the grid; rain falls in thin neon
//! streaks, a storm flashes now and then.

use errol_domain::entities::{EventKind, Sky};
use femtovg::{Color, Path};

use super::ambience::{self, Ambience, Fall, Tone};
use crate::anim::{hash, unit};
use crate::look::fade;
use crate::render::{Anchor, Face, Gfx, Type};

const W: f32 = 800.0;
const H: f32 = 480.0;
const HORIZON: f32 = H * 0.58;
const COLUMNS: usize = 34;
/// A scan line takes this long down the grid; a flared column stays lit
/// this long at most.
const SCAN: f32 = 1.1;
const FLARE: f32 = 5.0;
/// Seconds between two chances of lightning in a storm.
const STORM_BEAT: f32 = 9.0;
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

/// A column lit in the colour of a piece of news.
struct Flare {
    column: usize,
    tone: Tone,
    life: f32,
}

pub struct Grid {
    columns: Vec<Column>,
    blocks: Vec<Block>,
    heat: f32,
    scroll: f32,
    seed: u64,
    /// The data rain rushes when positive, slows when negative, back to 0.
    rush: f32,
    flare: Option<Flare>,
    /// The scan line sweeping down: its age and tone.
    scan: Option<(f32, Tone)>,
    pub ambience: Ambience,
}

impl Grid {
    pub fn new() -> Self {
        let columns = (0..COLUMNS as u64)
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
            rush: 0.0,
            flare: None,
            scan: None,
            ambience: Ambience::new(),
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

    /// News: the column above where it happens drops afresh in its colour,
    /// its neighbours wake, the rain rushes and a scan line sweeps the grid.
    pub fn react(&mut self, kind: &EventKind) {
        let tone = ambience::tone(kind);
        let column = column_at(ambience::place(kind).0);
        for index in column.saturating_sub(2)..(column + 3).min(COLUMNS) {
            self.columns[index].on = true;
        }
        let flared = &mut self.columns[column];
        (flared.y, flared.speed) = (-10.0, 150.0);
        self.flare = Some(Flare {
            column,
            tone,
            life: FLARE,
        });
        self.rush = rush(tone);
        self.scan = Some((0.0, tone));
    }

    pub fn draw(&mut self, gfx: &mut Gfx, real_dt: f32) {
        let dt = self.ambience.tick(real_dt);
        let heat = self.heat;
        self.heat *= (-dt * 0.45).exp();
        self.rush *= (-real_dt * 0.7).exp();
        let palette = gfx.palette;
        let colours = ambience::tones(&palette, palette.hot, palette.accent2, palette.accent);
        self.floor(gfx, dt, heat);
        self.weather(gfx);
        self.draw_scan(gfx, colours, real_dt);
        self.columns(gfx, (dt, real_dt), heat, colours);
        self.glitch(gfx, (dt, real_dt), heat);
    }

    /// Thin neon rain, or snow; in a storm, lightning now and then.
    fn weather(&self, gfx: &mut Gfx) {
        let clock = self.ambience.clock;
        if let Some(fall) = fall(self.ambience.sky) {
            ambience::draw_fall(gfx, fall, clock, gfx.palette.accent2, 0.22);
        }
        if self.ambience.sky == Some(Sky::Storm) {
            let flash = lightning(clock) * self.ambience.live;
            if flash > 0.0 {
                let light = gfx.palette.text;
                gfx.rect(0.0, 0.0, W, H, fade(light, 0.16 * flash));
            }
        }
    }

    /// The scan line, a bright rule with a glow above it, going down the grid.
    fn draw_scan(&mut self, gfx: &mut Gfx, colours: [Color; 5], dt: f32) {
        let Some((age, tone)) = &mut self.scan else {
            return;
        };
        *age += dt;
        let Some((y, alpha)) = scan_line(*age) else {
            self.scan = None;
            return;
        };
        let colour = ambience::tone_colour(*tone, colours);
        gfx.vertical_gradient(
            0.0,
            y - 26.0,
            W,
            26.0,
            fade(colour, 0.0),
            fade(colour, alpha * 0.3),
        );
        gfx.rect(0.0, y - 1.0, W, 2.0, fade(colour, alpha));
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
    /// fell below the horizon starts again from the top. The flared one is
    /// in the colour of its news. `dt` is the background's own time, then
    /// the real one.
    fn columns(
        &mut self,
        gfx: &mut Gfx,
        (dt, real_dt): (f32, f32),
        heat: f32,
        colours: [Color; 5],
    ) {
        let tick = (self.ambience.clock * 12.0) as u64;
        let pace = pace(heat, self.rush);
        let plain = (gfx.palette.accent2, gfx.palette.hot);
        let flared = self.burn(real_dt);
        for index in 0..self.columns.len() {
            let Some(tip) = self.fall_column(index, dt, heat, pace) else {
                continue;
            };
            let colour = if flared == Some(index) {
                let flare = self.flare.as_ref().map_or(Tone::Plain, |flare| flare.tone);
                let colour = ambience::tone_colour(flare, colours);
                (colour, colour)
            } else {
                plain
            };
            trail(gfx, index, tick, (self.columns[index].x, tip), colour);
            if tip - 130.0 > HORIZON {
                self.respawn(index);
            }
        }
    }

    /// The flare burns `dt` seconds more; returns its column while it lasts.
    fn burn(&mut self, dt: f32) -> Option<usize> {
        let flare = self.flare.as_mut()?;
        flare.life -= dt;
        if flare.life > 0.0 {
            Some(flare.column)
        } else {
            self.flare = None;
            None
        }
    }

    /// Column `index` may wake, and falls `dt` seconds at `pace`; returns
    /// where its tip is, if it is falling.
    fn fall_column(&mut self, index: usize, dt: f32, heat: f32, pace: f32) -> Option<f32> {
        let wake = self.rand() < (0.06 + heat * 1.5) * dt;
        let column = &mut self.columns[index];
        column.on |= wake;
        if !column.on {
            return None;
        }
        column.y += column.speed * dt * pace;
        Some(column.y)
    }

    fn respawn(&mut self, index: usize) {
        let (y, speed, on) = (
            self.rand() * -200.0,
            36.0 + self.rand() * 70.0,
            self.rand() < 0.5,
        );
        let column = &mut self.columns[index];
        (column.y, column.speed, column.on) = (y, speed, on);
        if self
            .flare
            .as_ref()
            .is_some_and(|flare| flare.column == index)
        {
            self.flare = None;
        }
    }

    /// Glitch blocks: one now and then, more when hot, gone in a blink (a
    /// real one: they do not linger on a still background).
    fn glitch(&mut self, gfx: &mut Gfx, (dt, real_dt): (f32, f32), heat: f32) {
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
            block.life -= real_dt;
        }
        self.blocks.retain(|block| block.life > 0.0);
    }
}

/// A column's glyphs, its tip at (x, tip): `yellow` at the tip, fading
/// `cyan` above, only above the horizon.
fn trail(
    gfx: &mut Gfx,
    index: usize,
    tick: u64,
    (x, tip): (f32, f32),
    (cyan, yellow): (Color, Color),
) {
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

/// The column falling at `x`.
fn column_at(x: f32) -> usize {
    (((x - 6.0) / 24.0).round().max(0.0) as usize).min(COLUMNS - 1)
}

/// How the data rain answers news: it rushes, or slows when a lamp goes out.
fn rush(tone: Tone) -> f32 {
    if tone == Tone::Dim { -0.6 } else { 1.4 }
}

/// How fast the columns fall: faster when hot or rushing, never stopped.
fn pace(heat: f32, rush: f32) -> f32 {
    (1.0 + heat * 2.0 + rush).max(0.25)
}

/// Where the scan line is `age` seconds after the news, and how bright;
/// `None` once it has gone down.
fn scan_line(age: f32) -> Option<(f32, f32)> {
    if age >= SCAN {
        return None;
    }
    let k = age / SCAN;
    Some((HORIZON + (H - HORIZON) * k * k, 0.7 * (1.0 - k)))
}

/// Thin streaks for rain, slanted more as it gets wilder; dots for snow.
fn fall(sky: Option<Sky>) -> Option<Fall> {
    let (count, speed, length, slant) = match sky? {
        Sky::Drizzle => (30, 520.0, 10.0, 0.15),
        Sky::Rain => (70, 700.0, 16.0, 0.2),
        Sky::Storm => (110, 850.0, 20.0, 0.3),
        Sky::Snow => (50, 40.0, 0.0, 0.0),
        _ => return None,
    };
    Some(Fall {
        count,
        speed,
        length,
        slant,
    })
}

/// How bright the lightning is at `clock`: in two beats out of three, a
/// double flicker at a moment of the beat that changes each time.
fn lightning(clock: f32) -> f32 {
    let beat = (clock / STORM_BEAT).floor();
    let n = beat as u64;
    if unit(n, 31) < 0.33 {
        return 0.0;
    }
    let k = clock - beat * STORM_BEAT - (1.0 + unit(n, 32) * 6.0);
    if (0.0..0.08).contains(&k) {
        1.0
    } else if (0.16..0.26).contains(&k) {
        0.6
    } else {
        0.0
    }
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

    #[test]
    fn news_flares_its_column_rushes_the_rain_and_scans() {
        let mut grid = Grid::new();
        grid.react(&EventKind::Waiting {
            name: "errol".into(),
            project: None,
            room: None,
        });
        let flare = grid.flare.as_ref().expect("a flare");
        assert_eq!((flare.column, flare.tone), (column_at(400.0), Tone::Alert));
        assert!(grid.columns[flare.column].on && grid.columns[flare.column].y < 0.0);
        assert!(grid.rush > 1.0);
        assert!(grid.scan.is_some());
        grid.respawn(column_at(400.0));
        assert!(grid.flare.is_none());
    }

    #[test]
    fn a_flare_burns_out() {
        let mut grid = Grid::new();
        assert_eq!(grid.burn(0.1), None);
        grid.react(&EventKind::Play { title: "x".into() });
        let column = column_at(600.0);
        assert_eq!(grid.burn(1.0), Some(column));
        assert_eq!(grid.burn(FLARE), None);
        assert!(grid.flare.is_none());
    }

    #[test]
    fn a_still_column_stays_a_sleeping_one_waits() {
        let mut grid = Grid::new();
        grid.columns[0].on = true;
        let y = grid.columns[0].y;
        assert_eq!(grid.fall_column(0, 0.0, 0.0, 1.0), Some(y));
        assert!(grid.fall_column(0, 0.5, 0.0, 1.0).expect("falling") > y);
        grid.columns[1].on = false;
        assert_eq!(grid.fall_column(1, 0.0, 0.0, 1.0), None);
    }

    #[test]
    fn a_lamp_going_out_slows_the_rain_without_stopping_it() {
        assert!(rush(Tone::Dim) < 0.0 && rush(Tone::Warm) > 0.0);
        assert!(pace(0.0, rush(Tone::Dim)) < 1.0);
        assert!((pace(0.0, -5.0) - 0.25).abs() < f32::EPSILON);
        assert!(pace(1.0, rush(Tone::Good)) > 3.0);
    }

    #[test]
    fn columns_are_found_by_where_they_fall() {
        assert_eq!(column_at(-50.0), 0);
        assert_eq!(column_at(6.0 + 24.0 * 5.0), 5);
        assert_eq!(column_at(2000.0), COLUMNS - 1);
    }

    #[test]
    fn the_scan_line_goes_down_the_grid_and_fades() {
        let (y0, a0) = scan_line(0.0).expect("a line");
        let (y1, a1) = scan_line(SCAN * 0.9).expect("a line");
        assert!((y0 - HORIZON).abs() < 1e-3 && y1 > y0 && y1 < H);
        assert!(a1 < a0);
        assert_eq!(scan_line(SCAN), None);
    }

    #[test]
    fn rain_falls_thicker_as_the_sky_darkens() {
        assert_eq!(fall(None), None);
        assert_eq!(fall(Some(Sky::Overcast)), None);
        let counts: Vec<usize> = [Sky::Drizzle, Sky::Rain, Sky::Storm]
            .into_iter()
            .map(|sky| fall(Some(sky)).expect("rain").count)
            .collect();
        assert!(counts[0] < counts[1] && counts[1] < counts[2]);
        assert!(fall(Some(Sky::Snow)).expect("snow").length.abs() < f32::EPSILON);
    }

    #[test]
    fn lightning_flickers_now_and_then() {
        let frames: Vec<f32> = (0..30 * 120).map(|i| lightning(i as f32 / 30.0)).collect();
        let lit = frames.iter().filter(|&&flash| flash > 0.0).count();
        assert!(lit > 0, "no lightning in two minutes");
        assert!(lit < frames.len() / 20, "too much lightning: {lit}");
        assert!(frames.iter().all(|&flash| (0.0..=1.0).contains(&flash)));
    }
}
