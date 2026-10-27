//! Hogwarts' background: the Great Hall at night, after the home page's. The
//! enchanted ceiling (a night sky, two layers of twinkling stars, clouds
//! drifting both ways, now and then a shooting star), the four house banners
//! swaying, candles floating at three depths with flickering flames, a little
//! golden dust. Now and then a candle gutters, a draught bends every flame and
//! stirs the banners, the Golden Snitch darts across; when an event takes the
//! screen, the owl post flies through and drops a letter.
//!
//! Every shape comes baked (`render::Hall`): a frame places images with a
//! transform; only sparks, dust and the snitch are drawn as small paths.

// drawing code: x, y, w, h for a box, c for the canvas, as in the SVG it follows
#![allow(clippy::many_single_char_names)]

use std::f32::consts::{PI, TAU};

use femtovg::{Color, Paint, Path};

use crate::anim::unit;
use crate::look::{fade, rgb};
use crate::render::{CANDLE_HEIGHTS, FLAME_BASE, Gfx, Hall, WICK};

const W: f32 = 800.0;
const H: f32 = 480.0;
const MOTES: usize = 14;
const MAX_SPARKS: usize = 180;
/// Flame flicker keyframes: time, scale x, scale y, skew (degrees), alpha.
const FLICKER: [[f32; 5]; 7] = [
    [0.0, 1.0, 1.0, 0.0, 1.0],
    [0.16, 0.96, 1.07, 1.6, 1.0],
    [0.31, 1.03, 0.95, -1.2, 1.0],
    [0.48, 0.97, 1.09, 2.2, 1.0],
    [0.63, 1.02, 0.97, -1.8, 1.0],
    [0.81, 0.98, 1.04, 0.8, 1.0],
    [1.0, 1.0, 1.0, 0.0, 1.0],
];
/// A candle guttering: the flame shrinks nearly out, then leaps back.
const GUTTER: [[f32; 5]; 5] = [
    [0.0, 1.0, 1.0, 0.0, 1.0],
    [0.22, 0.75, 0.55, -8.0, 1.0],
    [0.40, 0.35, 0.22, 0.0, 0.35],
    [0.75, 1.05, 1.15, 0.0, 1.0],
    [1.0, 1.0, 1.0, 0.0, 1.0],
];
const GUTTER_TIME: f32 = 1.9;
/// A draught: banners stir (time, rotate, skew in degrees).
const STIR: [[f32; 5]; 5] = [
    [0.0, 0.0, 0.0, 0.0, 0.0],
    [0.25, -3.5, 4.0, 0.0, 0.0],
    [0.5, 1.8, 0.0, 0.0, 0.0],
    [0.75, -1.2, 0.0, 0.0, 0.0],
    [1.0, 0.0, 0.0, 0.0, 0.0],
];
const DRAUGHT_TIME: f32 = 2.8;
/// Banner centres, across the top left of the hall, above the clock.
const BANNERS_X: [f32; 4] = [40.0, 103.0, 166.0, 229.0];
const BANNER_W: f32 = 36.0;
const BANNER_TOP: f32 = 40.0;
/// Owl and letter: the owl sprite is 110 x 80, drawn at this scale.
const OWL_SCALE: f32 = 0.72;
const LETTER_LIFE: f32 = 4.5;

fn smooth(k: f32) -> f32 {
    let k = k.clamp(0.0, 1.0);
    k * k * (3.0 - 2.0 * k)
}

/// Values of keyframes at `k` (0 → 1), eased between frames.
pub fn keyframes(frames: &[[f32; 5]], k: f32) -> [f32; 4] {
    let k = k.clamp(0.0, 1.0);
    for pair in frames.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        if k <= b[0] {
            let local = smooth((k - a[0]) / (b[0] - a[0]).max(1e-4));
            return [1, 2, 3, 4].map(|i| a[i] + (b[i] - a[i]) * local);
        }
    }
    let last = frames[frames.len() - 1];
    [last[1], last[2], last[3], last[4]]
}

/// A depth of candles: where they float, and the ranges of their width,
/// alpha and bob.
type Depth<'a> = (&'a [(f32, f32)], f32, f32, f32, f32, f32, f32);

struct Candle {
    x: f32,
    y: f32,
    /// Width on screen; the candle is 24 units wide.
    w: f32,
    variant: usize,
    alpha: f32,
    bob: f32,
    bob_period: f32,
    phase: f32,
    flicker_period: f32,
    flicker_phase: f32,
    halo_period: f32,
}

struct Mote {
    x: f32,
    y: f32,
    vy: f32,
    sway: f32,
    phase: f32,
}

struct Spark {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    life: f32,
    span: f32,
}

struct Shooting {
    start: f32,
    x: f32,
    y: f32,
    dx: f32,
    dy: f32,
    travel: f32,
}

struct Owl {
    start: f32,
    duration: f32,
    dir: f32,
    y0: f32,
    dip: f32,
    drop: f32,
    dropped: bool,
}

struct Letter {
    born: f32,
    x: f32,
    y: f32,
    vy: f32,
    dir: f32,
}

struct Snitch {
    start: f32,
    duration: f32,
    points: [(f32, f32); 5],
}

/// When the next of each ambient event comes (seconds of `Gfx::t`).
struct Next {
    star: f32,
    owl: f32,
    snitch: f32,
    gutter: f32,
    draught: f32,
}

pub struct GreatHall {
    candles: Vec<Candle>,
    motes: Vec<Mote>,
    sparks: Vec<Spark>,
    heat: f32,
    seed: u64,
    next: Option<Next>,
    shooting: Option<Shooting>,
    owl: Option<Owl>,
    owl_asked: bool,
    letter: Option<Letter>,
    snitch: Option<Snitch>,
    gutter: Option<(usize, f32)>,
    draught: Option<f32>,
}

impl GreatHall {
    pub fn new() -> Self {
        // three depths: far and small and dim, near and big, at the edges
        // where they do not cross the words
        let far = [(150.0, 52.0), (330.0, 46.0), (470.0, 58.0), (628.0, 50.0)];
        let mid = [
            (62.0, 64.0),
            (252.0, 70.0),
            (402.0, 60.0),
            (552.0, 72.0),
            (716.0, 62.0),
        ];
        let near = [(14.0, 74.0), (262.0, 58.0), (318.0, 44.0), (786.0, 66.0)];
        let mut candles = Vec::new();
        let layers: [Depth; 3] = [
            (&far, 6.0, 8.5, 0.42, 0.6, 4.0, 7.0),
            (&mid, 10.0, 13.0, 0.75, 0.9, 6.0, 10.0),
            (&near, 15.0, 20.0, 0.95, 1.0, 8.0, 13.0),
        ];
        let mut k = 0_u64;
        for (places, w0, w1, a0, a1, b0, b1) in layers {
            for &(x, y) in places {
                k += 1;
                candles.push(Candle {
                    x,
                    y,
                    w: w0 + unit(k, 1) * (w1 - w0),
                    variant: (unit(k, 2) * 3.0) as usize % 3,
                    alpha: a0 + unit(k, 3) * (a1 - a0),
                    bob: b0 + unit(k, 4) * (b1 - b0),
                    bob_period: 5.0 + unit(k, 5) * 4.0,
                    phase: unit(k, 6),
                    flicker_period: 1.3 + unit(k, 7) * 1.3,
                    flicker_phase: unit(k, 8),
                    halo_period: 1.8 + unit(k, 9) * 1.6,
                });
            }
        }
        let motes = (0..MOTES as u64)
            .map(|i| Mote {
                x: unit(i, 11) * W,
                y: unit(i, 12) * H,
                vy: 3.0 + unit(i, 13) * 6.0,
                sway: 6.0 + unit(i, 14) * 12.0,
                phase: unit(i, 15) * TAU,
            })
            .collect();
        Self {
            candles,
            motes,
            sparks: Vec::new(),
            heat: 0.0,
            seed: 11,
            next: None,
            shooting: None,
            owl: None,
            owl_asked: false,
            letter: None,
            snitch: None,
            gutter: None,
            draught: None,
        }
    }

    fn rand(&mut self) -> f32 {
        self.seed += 1;
        unit(self.seed, 37)
    }

    fn between(&mut self, low: f32, high: f32) -> f32 {
        low + self.rand() * (high - low)
    }

    /// A scene starts: a shower of sparks; a big one (an event, a film)
    /// sends the owl post.
    pub fn burst(&mut self, power: f32) {
        self.heat = (self.heat + power).min(1.5);
        let count = (60.0 * power) as usize;
        for _ in 0..count {
            if self.sparks.len() >= MAX_SPARKS {
                break;
            }
            let angle = self.rand() * TAU;
            let speed = 50.0 + self.rand() * 170.0;
            let span = 0.9 + self.rand() * 1.2;
            self.sparks.push(Spark {
                x: W * 0.5,
                y: H * 0.5,
                vx: angle.cos() * speed,
                vy: angle.sin() * speed - 40.0,
                life: span,
                span,
            });
        }
        if power >= 1.0 {
            self.owl_asked = true;
        }
    }

    fn schedule(&mut self, t: f32) {
        let next = self.next.get_or_insert(Next {
            star: t + 6.0,
            owl: t + 50.0,
            snitch: t + 20.0,
            gutter: t + 8.0,
            draught: t + 30.0,
        });
        let (star, owl, snitch, gutter, draught) = (
            t >= next.star,
            t >= next.owl,
            t >= next.snitch,
            t >= next.gutter,
            t >= next.draught,
        );
        if star {
            let (wait, x, y, travel) = (
                self.between(14.0, 30.0),
                self.between(60.0, 740.0),
                self.between(14.0, 120.0),
                self.between(260.0, 520.0),
            );
            let left = self.rand() < 0.5;
            let angle = if left {
                self.between(8.0, 30.0)
            } else {
                self.between(150.0, 172.0)
            }
            .to_radians();
            self.shooting = Some(Shooting {
                start: t,
                x,
                y,
                dx: angle.cos(),
                dy: angle.sin(),
                travel,
            });
            if let Some(next) = &mut self.next {
                next.star = t + wait;
            }
        }
        if owl || (self.owl_asked && self.owl.is_none()) {
            self.owl_asked = false;
            if self.owl.is_none() {
                let flight = Owl {
                    start: t,
                    duration: self.between(7.0, 10.0),
                    dir: if self.rand() < 0.5 { 1.0 } else { -1.0 },
                    y0: self.between(0.06, 0.2) * H + 30.0,
                    dip: self.between(0.18, 0.3) * H,
                    drop: self.between(0.35, 0.6),
                    dropped: false,
                };
                self.owl = Some(flight);
            }
            let wait = self.between(50.0, 110.0);
            if let Some(next) = &mut self.next {
                next.owl = t + wait;
            }
        }
        if snitch {
            let mut points = [(0.0, 0.0); 5];
            let from_left = self.rand() < 0.5;
            for (i, point) in points.iter_mut().enumerate() {
                let along = i as f32 / 4.0;
                let x = if from_left { along } else { 1.0 - along } * (W + 80.0) - 40.0;
                *point = (x + self.between(-60.0, 60.0), self.between(60.0, 300.0));
            }
            let (duration, wait) = (self.between(3.6, 5.0), self.between(45.0, 90.0));
            self.snitch = Some(Snitch {
                start: t,
                duration,
                points,
            });
            if let Some(next) = &mut self.next {
                next.snitch = t + wait;
            }
        }
        if gutter {
            let (index, wait) = (
                (self.rand() * self.candles.len() as f32) as usize % self.candles.len(),
                self.between(8.0, 16.0),
            );
            self.gutter = Some((index, t));
            if let Some(next) = &mut self.next {
                next.gutter = t + wait;
            }
        }
        if draught {
            let wait = self.between(30.0, 60.0);
            self.draught = Some(t);
            if let Some(next) = &mut self.next {
                next.draught = t + wait;
            }
        }
    }

    pub fn draw(&mut self, gfx: &mut Gfx, dt: f32) {
        let Some(hall) = gfx.hall else { return };
        let t = gfx.t;
        self.schedule(t);
        let heat = self.heat;
        self.heat *= (-dt * 0.45).exp();

        // the enchanted ceiling
        gfx.image(hall.sky, 0.0, 0.0, W, H, 1.0);
        let twinkle = |period: f32, offset: f32| {
            0.45 + 0.55 * (0.5 + 0.5 * ((t / period + offset) * TAU).sin())
        };
        gfx.image(hall.stars[0], 0.0, 0.0, W, 374.0, twinkle(4.6, 0.0));
        gfx.image(hall.stars[1], 0.0, 0.0, W, 374.0, twinkle(3.3, 0.37));
        // the front bank drifts left (1400 px in 190 s), the back one right (in 120 s)
        let front = (t * 1400.0 / 190.0) % 1400.0;
        let back = (t * 1400.0 / 120.0) % 1400.0;
        for dx in [0.0, 1400.0] {
            gfx.image(
                hall.clouds[1],
                back - 1400.0 + dx - 300.0,
                0.0,
                1400.0,
                170.0,
                0.6,
            );
        }
        for dx in [0.0, 1400.0] {
            gfx.image(hall.clouds[0], -front + dx, 0.0, 1400.0, 170.0, 0.75);
        }
        self.draw_shooting_star(gfx, t);

        // a draught bends every flame and stirs the banners
        let draught = self
            .draught
            .map(|start| (t - start) / DRAUGHT_TIME)
            .filter(|k| (0.0..1.0).contains(k));
        if self.draught.is_some() && draught.is_none() {
            self.draught = None;
        }
        self.draw_banners(gfx, hall, t);
        self.draw_candles(gfx, hall, t, draught);
        self.draw_snitch(gfx, t);
        self.draw_owl(gfx, hall, t, dt);

        // a little golden dust in the candlelight
        let mut dust = Path::new();
        for mote in &mut self.motes {
            mote.y -= mote.vy * dt * (1.0 + heat);
            if mote.y < -4.0 {
                mote.y = H + 4.0;
            }
            let x = mote.x + (t * 0.5 + mote.phase).sin() * mote.sway;
            dust.circle(x, mote.y, 0.9);
        }
        gfx.fill(&dust, fade(rgb(0xf0c75a), 0.35));
        self.draw_sparks(gfx, dt);
    }

    fn draw_banners(&mut self, gfx: &mut Gfx, hall: &Hall, t: f32) {
        let periods = [7.0, 8.2, 7.6, 9.0];
        let scale = BANNER_W / 100.0;
        for (index, &x) in BANNERS_X.iter().enumerate() {
            let swing = ((t / periods[index] + index as f32 * 0.27) * TAU).sin();
            let (mut rotate, mut skew) = (0.8 * swing, -0.6 * swing);
            if let Some(start) = self.draught {
                let k = (t - start - 0.15 * index as f32) / DRAUGHT_TIME;
                if (0.0..1.0).contains(&k) {
                    let stir = keyframes(&STIR, k);
                    rotate += stir[0];
                    skew += stir[1];
                }
            }
            gfx.canvas.save();
            gfx.canvas.translate(x, BANNER_TOP);
            gfx.canvas.rotate(rotate.to_radians());
            gfx.canvas.skew_x(skew.to_radians());
            // the rod's top is 19 units down the image (cord above it)
            gfx.image(
                hall.banners[index],
                -BANNER_W / 2.0,
                -19.0 * scale,
                BANNER_W,
                316.0 * scale,
                0.92,
            );
            gfx.canvas.restore();
        }
    }

    fn draw_candles(&mut self, gfx: &mut Gfx, hall: &Hall, t: f32, draught: Option<f32>) {
        if self
            .gutter
            .is_some_and(|(_, start)| t - start > GUTTER_TIME)
        {
            self.gutter = None;
        }
        for (index, candle) in self.candles.iter().enumerate() {
            let s = candle.w / 24.0;
            let swing = ((t / candle.bob_period + candle.phase) * TAU).sin();
            let bob = candle.bob * 0.5 * (1.0 + swing);
            let mut flame = keyframes(
                &FLICKER,
                (t / candle.flicker_period + candle.flicker_phase).fract(),
            );
            if let Some((which, start)) = self.gutter
                && which == index
            {
                flame = keyframes(&GUTTER, (t - start) / GUTTER_TIME);
            }
            if let Some(k) = draught {
                flame[2] -= 14.0 * (PI * k).sin();
            }
            let glow = 0.86 + 0.14 * ((t / candle.halo_period + candle.phase) * TAU).sin();
            gfx.canvas.save();
            gfx.canvas.translate(candle.x, candle.y + bob);
            gfx.canvas.rotate((0.7 * swing).to_radians());
            let glow_size = 7.0 * candle.w * (0.9 + 0.1 * flame[1]);
            let flame_middle = (WICK - 11.6) * s;
            gfx.image(
                hall.halo,
                -glow_size / 2.0,
                flame_middle - glow_size / 2.0,
                glow_size,
                glow_size,
                candle.alpha * glow * flame[3],
            );
            gfx.image(
                hall.candles[candle.variant],
                -12.0 * s,
                0.0,
                24.0 * s,
                CANDLE_HEIGHTS[candle.variant] * s,
                candle.alpha,
            );
            gfx.canvas.translate(0.0, WICK * s);
            gfx.canvas.scale(flame[0], flame[1]);
            gfx.canvas.skew_x(flame[2].to_radians());
            gfx.image(
                hall.flame,
                -12.0 * s,
                -FLAME_BASE * s,
                24.0 * s,
                48.0 * s,
                candle.alpha * flame[3],
            );
            gfx.canvas.restore();
        }
    }

    fn draw_shooting_star(&mut self, gfx: &mut Gfx, t: f32) {
        let Some(star) = &self.shooting else { return };
        let k = (t - star.start) / 1.2;
        if k >= 1.0 {
            self.shooting = None;
            return;
        }
        let head = (
            star.x + star.dx * star.travel * k,
            star.y + star.dy * star.travel * k,
        );
        let tail = (head.0 - star.dx * 170.0, head.1 - star.dy * 170.0);
        let fade_out = if k > 0.75 { (1.0 - k) / 0.25 } else { 1.0 };
        let mut streak = Path::new();
        streak.move_to(tail.0, tail.1);
        streak.line_to(head.0, head.1);
        let paint = Paint::linear_gradient_stops(
            tail.0,
            tail.1,
            head.0,
            head.1,
            [
                (0.0, Color::rgbaf(0.86, 0.9, 1.0, 0.0)),
                (0.6, Color::rgbaf(0.86, 0.9, 1.0, 0.5 * fade_out)),
                (1.0, Color::rgbaf(1.0, 1.0, 1.0, fade_out)),
            ],
        )
        .with_line_width(2.0);
        gfx.canvas.stroke_path(&streak, &paint);
    }

    fn draw_snitch(&mut self, gfx: &mut Gfx, t: f32) {
        let Some(snitch) = &self.snitch else { return };
        let k = (t - snitch.start) / snitch.duration;
        if k >= 1.0 {
            self.snitch = None;
            return;
        }
        // dart from point to point, quick in the middle of each leg
        let leg = (k * 4.0).min(3.999);
        let (index, local) = (leg as usize, smooth(leg.fract()));
        let (a, b) = (snitch.points[index], snitch.points[index + 1]);
        let (x, y) = (a.0 + (b.0 - a.0) * local, a.1 + (b.1 - a.1) * local);
        gfx.halo(x, y, 14.0, Color::rgbaf(1.0, 0.85, 0.4, 0.35));
        let beat = (t * 60.0).sin();
        for side in [-1.0_f32, 1.0] {
            gfx.canvas.save();
            gfx.canvas.translate(x + side * 2.5, y - 1.0);
            gfx.canvas.rotate(side * (0.5 + 0.6 * beat));
            let mut wing = Path::new();
            wing.ellipse(side * 6.0, 0.0, 6.5, 2.0);
            gfx.fill(&wing, Color::rgbaf(0.94, 0.95, 1.0, 0.75));
            gfx.canvas.restore();
        }
        let mut ball = Path::new();
        ball.circle(x, y, 3.2);
        let paint = Paint::radial_gradient_stops(
            x - 1.0,
            y - 1.2,
            0.0,
            3.6,
            [
                (0.0, rgb(0xfff3bf)),
                (0.6, rgb(0xe8bd52)),
                (1.0, rgb(0x8a5e10)),
            ],
        );
        gfx.canvas.fill_path(&ball, &paint);
    }

    fn draw_owl(&mut self, gfx: &mut Gfx, hall: &Hall, t: f32, dt: f32) {
        let mut spawned = None;
        if let Some(owl) = &mut self.owl {
            let k = (t - owl.start) / owl.duration;
            if k >= 1.0 {
                self.owl = None;
            } else {
                // after the home page's flight: across, dipping, a slow climb
                let x = if owl.dir > 0.0 {
                    -130.0 + (W + 260.0) * k
                } else {
                    W + 130.0 - (W + 260.0) * k
                };
                let y = owl.y0 + (k * PI).sin() * owl.dip - k * 40.0 + (k * 18.0).sin() * 4.0;
                let glide = k > 0.3 && k < 0.42;
                // flapping 0.42 s each way, eased
                let beat = 0.5 - 0.5 * ((t / 0.42) * PI).cos();
                let frame = if glide {
                    1
                } else if beat < 0.33 {
                    0
                } else if beat < 0.66 {
                    1
                } else {
                    2
                };
                let tilt = (k * PI).cos() * -12.0 * owl.dir;
                gfx.canvas.save();
                gfx.canvas.translate(x, y + 40.0 * OWL_SCALE);
                gfx.canvas.rotate(tilt.to_radians());
                gfx.canvas.scale(owl.dir * OWL_SCALE, OWL_SCALE);
                if !owl.dropped {
                    // the letter in its claws
                    gfx.canvas.save();
                    gfx.canvas.translate(-55.0 + 62.0, -40.0 + 68.0);
                    gfx.canvas.rotate((-10.0_f32).to_radians());
                    gfx.image(hall.letter, -12.0, -8.0, 24.0, 16.0, 1.0);
                    gfx.canvas.restore();
                }
                gfx.image(hall.owl[frame], -55.0, -40.0, 110.0, 80.0, 1.0);
                gfx.canvas.restore();
                if !owl.dropped && k > owl.drop {
                    owl.dropped = true;
                    spawned = Some(Letter {
                        born: t,
                        x: x + 8.0 * owl.dir,
                        y: y + 62.0 * OWL_SCALE,
                        vy: 10.0,
                        dir: owl.dir,
                    });
                }
            }
        }
        if spawned.is_some() {
            self.letter = spawned;
        }
        if let Some(letter) = &mut self.letter {
            let age = t - letter.born;
            if age >= LETTER_LIFE {
                self.letter = None;
                return;
            }
            // falls like a leaf: no faster than 70 px/s, swaying and rocking
            letter.vy = (letter.vy + 120.0 * dt).min(70.0);
            letter.y += letter.vy * dt;
            letter.x += (letter.dir * 10.0 + 38.0 * (2.6 * age).cos()) * dt;
            let rock = 32.0 * (2.6 * age).sin();
            let alpha = ((LETTER_LIFE - age) / 1.2).min(1.0);
            gfx.canvas.save();
            gfx.canvas.translate(letter.x, letter.y);
            gfx.canvas.rotate(rock.to_radians());
            gfx.image(hall.letter, -11.0, -7.7, 22.0, 15.4, alpha);
            gfx.canvas.restore();
        }
    }

    fn draw_sparks(&mut self, gfx: &mut Gfx, dt: f32) {
        if self.sparks.is_empty() {
            return;
        }
        let mut trails = Path::new();
        let mut heads = Path::new();
        for spark in &mut self.sparks {
            spark.vy += 70.0 * dt;
            spark.x += spark.vx * dt;
            spark.y += spark.vy * dt;
            spark.life -= dt;
            trails.move_to(spark.x, spark.y);
            trails.line_to(spark.x - spark.vx * 0.04, spark.y - spark.vy * 0.04);
            heads.circle(spark.x, spark.y, 1.3);
        }
        let alive = self
            .sparks
            .iter()
            .map(|spark| spark.life / spark.span)
            .fold(0.0_f32, f32::max);
        gfx.stroke(&trails, fade(rgb(0xd4a849), 0.6 * alive), 1.0);
        gfx.fill(&heads, fade(rgb(0xfff3bf), 0.9 * alive));
        self.sparks.retain(|spark| spark.life > 0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyframes_start_and_end_on_their_values() {
        assert_eq!(keyframes(&FLICKER, 0.0), [1.0, 1.0, 0.0, 1.0]);
        assert_eq!(keyframes(&FLICKER, 1.0), [1.0, 1.0, 0.0, 1.0]);
        let low = keyframes(&GUTTER, 0.40);
        for (value, expected) in low.iter().zip([0.35, 0.22, 0.0, 0.35]) {
            assert!((value - expected).abs() < 1e-5);
        }
    }

    #[test]
    fn keyframes_ease_between_two_frames() {
        let middle = keyframes(&STIR, 0.125);
        assert!(middle[0] < 0.0 && middle[0] > -3.5);
    }
}
