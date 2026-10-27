//! Hogwarts' background: the Great Hall at night. The enchanted ceiling
//! twinkles, candles float and drift with flickering flames, golden dust rises
//! through the air, and a burst throws a shower of sparks (Lumos).

use femtovg::{Color, Paint, Path};

use crate::anim::unit;
use crate::look::fade;
use crate::render::Gfx;

const W: f32 = 800.0;
const H: f32 = 480.0;
const STARS: u64 = 70;
const CANDLES: u64 = 12;
const MOTES: usize = 46;
const MAX_SPARKS: usize = 220;

struct Candle {
    x: f32,
    y: f32,
    height: f32,
    phase: f32,
    speed: f32,
}

struct Mote {
    x: f32,
    y: f32,
    vy: f32,
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

pub struct Candles {
    lights: Vec<Candle>,
    motes: Vec<Mote>,
    sparks: Vec<Spark>,
    heat: f32,
    seed: u64,
}

impl Candles {
    pub fn new() -> Self {
        // a band of candles high in the hall, above the words: parchment text
        // over cream wax would not read
        let lights = (0..CANDLES)
            .map(|i| Candle {
                x: 30.0 + (i as f32 + unit(i, 1) * 0.6) * (W - 60.0) / CANDLES as f32,
                y: 52.0 + unit(i, 2) * 40.0,
                height: 16.0 + unit(i, 3) * 16.0,
                phase: unit(i, 4) * 6.3,
                speed: 0.25 + unit(i, 5) * 0.25,
            })
            .collect();
        let motes = (0..MOTES as u64)
            .map(|i| Mote {
                x: unit(i, 11) * W,
                y: unit(i, 12) * H,
                vy: 5.0 + unit(i, 13) * 10.0,
                phase: unit(i, 14) * 6.3,
            })
            .collect();
        Self {
            lights,
            motes,
            sparks: Vec::new(),
            heat: 0.0,
            seed: 5,
        }
    }

    fn rand(&mut self) -> f32 {
        self.seed += 1;
        unit(self.seed, 31)
    }

    pub fn burst(&mut self, power: f32) {
        self.heat = (self.heat + power).min(1.5);
        let count = (70.0 * power) as usize;
        for _ in 0..count {
            if self.sparks.len() >= MAX_SPARKS {
                break;
            }
            let angle = self.rand() * std::f32::consts::TAU;
            let speed = 50.0 + self.rand() * 190.0;
            let span = 0.9 + self.rand() * 1.3;
            self.sparks.push(Spark {
                x: W * 0.5,
                y: H * 0.5,
                vx: angle.cos() * speed,
                vy: angle.sin() * speed - 40.0,
                life: span,
                span,
            });
        }
    }

    pub fn draw(&mut self, gfx: &mut Gfx, dt: f32) {
        let heat = self.heat;
        self.heat *= (-dt * 0.45).exp();
        let (gold, parchment) = (gfx.palette.accent, gfx.palette.accent2);
        let t = gfx.t;

        // the enchanted ceiling: a night sky, faint, over the top of the hall
        let mut stars = Path::new();
        let mut bright = Path::new();
        for i in 0..STARS {
            let (x, y) = (unit(i, 21) * W, unit(i, 22) * H * 0.55);
            let twinkle = 0.5 + 0.5 * (t * (0.6 + unit(i, 23)) + unit(i, 24) * 6.3).sin();
            if twinkle > 0.85 {
                bright.circle(x, y, 1.3);
            } else {
                stars.circle(x, y, 0.8);
            }
        }
        gfx.fill(&stars, fade(parchment, 0.22));
        gfx.fill(&bright, fade(parchment, 0.6));

        // candles: glow first, then wax and flame
        for (index, candle) in self.lights.iter().enumerate() {
            let drift = (t * candle.speed + candle.phase).sin() * 5.0;
            let (x, top) = (candle.x, candle.y + drift);
            let flicker = 0.82
                + 0.1 * (t * 11.0 + candle.phase).sin()
                + 0.08 * (t * 17.3 + index as f32).sin();
            gfx.halo(
                x,
                top - 6.0,
                46.0 * flicker,
                Color::rgbaf(1.0, 0.72, 0.3, 0.16 * flicker),
            );
            let wax = Paint::linear_gradient(
                x - 4.0,
                0.0,
                x + 4.0,
                0.0,
                Color::rgbaf(0.95, 0.91, 0.8, 0.55),
                Color::rgbaf(0.72, 0.66, 0.54, 0.55),
            );
            let mut body = Path::new();
            body.rounded_rect(x - 3.0, top, 6.0, candle.height, 1.5);
            gfx.canvas.fill_path(&body, &wax);
            // the flame: a drop that stretches and leans as it flickers
            let (fh, lean) = (10.0 * flicker, (t * 3.1 + candle.phase).sin() * 1.2);
            let mut flame = Path::new();
            flame.move_to(x + lean, top - 2.0 - fh);
            flame.bezier_to(
                x + 4.0,
                top - 2.0 - fh * 0.4,
                x + 3.5,
                top - 1.0,
                x,
                top - 1.0,
            );
            flame.bezier_to(
                x - 3.5,
                top - 1.0,
                x - 4.0,
                top - 2.0 - fh * 0.4,
                x + lean,
                top - 2.0 - fh,
            );
            let fire = Paint::linear_gradient(
                x,
                top - 2.0 - fh,
                x,
                top - 1.0,
                Color::rgbaf(1.0, 0.98, 0.85, 0.95),
                Color::rgbaf(1.0, 0.6, 0.15, 0.9),
            );
            gfx.canvas.fill_path(&flame, &fire);
        }

        // golden dust drifting up
        let mut dust = Path::new();
        for mote in &mut self.motes {
            mote.y -= mote.vy * dt * (1.0 + heat * 2.0);
            mote.x += (t * 0.7 + mote.phase).sin() * 6.0 * dt;
            if mote.y < -4.0 {
                mote.y = H + 4.0;
            }
            dust.circle(mote.x, mote.y, 1.1);
        }
        gfx.fill(&dust, fade(gold, 0.45 + 0.2 * (t * 1.3).sin().abs()));

        // sparks: a short trail each, falling a little, fading out
        let mut trails = Path::new();
        let mut heads = Path::new();
        for spark in &mut self.sparks {
            spark.vy += 70.0 * dt;
            spark.x += spark.vx * dt;
            spark.y += spark.vy * dt;
            spark.life -= dt;
            trails.move_to(spark.x, spark.y);
            trails.line_to(spark.x - spark.vx * 0.04, spark.y - spark.vy * 0.04);
            heads.circle(spark.x, spark.y, 1.4);
        }
        let alive = self
            .sparks
            .iter()
            .map(|spark| spark.life / spark.span)
            .fold(0.0_f32, f32::max);
        gfx.stroke(&trails, fade(gold, 0.6 * alive), 1.0);
        gfx.fill(&heads, fade(parchment, 0.9 * alive));
        self.sparks.retain(|spark| spark.life > 0.0);
    }
}
