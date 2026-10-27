//! What is drawn, frame by frame: the background, the header, the scene of the
//! moment (idle, event, cinema), the ticker, the scanlines and the flash that
//! marks a change of scene. `Stage` keeps what must last between frames.

mod cinema;
mod event;
mod header;
mod idle;
mod text;
mod ticker;

use errol_domain::strategies::Scene;
use errol_domain::usecases::View;
use femtovg::Path;

use crate::anim::{ease_out, progress};
use crate::fx::Background;
use crate::look::fade;
use crate::render::{Gfx, Textures};

/// Top of the scenes, under the header.
pub const TOP: f32 = 38.0;
/// Bottom of the scenes, above the ticker.
pub const BOTTOM: f32 = 454.0;
pub const WIDTH: f32 = 800.0;
pub const HEIGHT: f32 = 480.0;

pub struct Stage {
    background: Background,
    /// Identifies the scene shown ("idle", "event:<id>", "cinema:<id>").
    key: String,
    /// When it started (seconds of `Gfx::t`).
    since: f32,
    idle: idle::Carousel,
    ticker: ticker::Ticker,
}

impl Stage {
    pub fn new(look: crate::look::Look) -> Self {
        Self {
            background: Background::new(look),
            key: String::new(),
            since: 0.0,
            idle: idle::Carousel::default(),
            ticker: ticker::Ticker::default(),
        }
    }

    pub fn draw(&mut self, gfx: &mut Gfx, view: &View, textures: &mut Textures, dt: f32) {
        let key = match &view.scene {
            Scene::Idle => "idle".to_owned(),
            Scene::Event { event, .. } => format!("event:{}", event.id.as_str()),
            Scene::Cinema(playing) => format!("cinema:{}", playing.id),
        };
        if key != self.key {
            self.key = key;
            self.since = gfx.t;
            self.background.burst(if matches!(view.scene, Scene::Idle) {
                0.35
            } else {
                1.2
            });
        }
        let age = gfx.t - self.since;
        let bg = gfx.palette.bg;
        gfx.rect(0.0, 0.0, WIDTH, HEIGHT, bg);
        self.background.draw(gfx, dt);

        // a cyberpunk change of scene jolts sideways for half a second
        let jolt = if gfx.cyber() && age < 0.5 {
            ((age * 40.0).sin() * 6.0 * (1.0 - age * 2.0)).round()
        } else {
            0.0
        };
        gfx.canvas.save();
        gfx.canvas.translate(jolt, 0.0);
        gfx.canvas.set_global_alpha(ease_out(progress(age, 0.7)));
        match &view.scene {
            Scene::Idle => self.idle.draw(gfx, view, textures, age),
            Scene::Event {
                event,
                started,
                ends,
            } => {
                let length = ends.secs_since(*started) as f32;
                event::draw(gfx, view, event, age, length, textures);
            }
            Scene::Cinema(playing) => cinema::draw(gfx, view, playing, age, textures),
        }
        gfx.canvas.restore();

        header::draw(gfx, view);
        self.ticker.draw(gfx, view, dt);
        flash(gfx, age);
        scanlines(gfx);
    }
}

/// The light (friday) or the coloured bands (cyberpunk) of a change of scene.
fn flash(gfx: &mut Gfx, age: f32) {
    let left = 1.0 - progress(age, 0.6);
    if left <= 0.0 {
        return;
    }
    if gfx.cyber() {
        let (red, cyan, yellow) = (gfx.palette.accent, gfx.palette.accent2, gfx.palette.hot);
        for (y, h, colour) in [(0.20, 0.03, cyan), (0.61, 0.04, red), (0.80, 0.02, yellow)] {
            gfx.rect(0.0, HEIGHT * y, WIDTH, HEIGHT * h, fade(colour, 0.5 * left));
        }
    } else {
        gfx.halo(
            WIDTH / 2.0,
            HEIGHT / 2.0,
            460.0,
            fade(gfx.palette.accent, 0.35 * left),
        );
    }
}

/// Fine horizontal lines over everything, like an old monitor.
fn scanlines(gfx: &mut Gfx) {
    let mut path = Path::new();
    let mut y = 0.5;
    while y < HEIGHT {
        path.move_to(0.0, y);
        path.line_to(WIDTH, y);
        y += 3.0;
    }
    let strength = if gfx.cyber() && (gfx.t % 6.0) > 5.88 {
        0.08
    } else {
        0.2
    };
    gfx.stroke(&path, femtovg::Color::rgbaf(0.0, 0.0, 0.0, strength), 1.0);
}
