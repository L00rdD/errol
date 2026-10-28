//! What is drawn, frame by frame: the background, the header, the scene of the
//! moment (idle, event, cinema), the ticker, the scanlines and the flash that
//! marks a change of scene. `Stage` keeps what must last between frames.

mod cinema;
mod event;
mod header;
mod idle;
mod text;
mod ticker;
mod turner;

use errol_domain::strategies::Scene;
use errol_domain::usecases::View;
use femtovg::Path;

use crate::anim::{ease_back, ease_in_out, ease_out, progress, unit};
use crate::fx::Background;
use crate::look::fade;
use crate::render::{Gfx, Textures};

/// Top of the scenes, under the header.
pub const TOP: f32 = 38.0;
/// Bottom of the scenes, above the ticker.
pub const BOTTOM: f32 = 454.0;
pub const WIDTH: f32 = 800.0;
pub const HEIGHT: f32 = 480.0;

/// How long the scene that leaves takes to go, in seconds.
const LEAVE: f32 = 0.45;
/// How long the scene that comes takes to settle.
const ENTER: f32 = 0.7;

pub struct Stage {
    background: Background,
    /// Identifies the scene shown ("idle", "event:<id>", "cinema:<id>").
    key: String,
    /// The scene shown, as it was when it came: it is drawn again as it leaves.
    shown: Option<Scene>,
    /// When it started (seconds of `Gfx::t`).
    since: f32,
    /// The scene that leaves, and when it had started, while it goes.
    leaving: Option<(Scene, f32)>,
    idle: idle::Carousel,
    ticker: ticker::Ticker,
}

impl Stage {
    pub fn new(look: crate::look::Look) -> Self {
        Self {
            background: Background::new(look),
            key: String::new(),
            shown: None,
            since: 0.0,
            leaving: None,
            idle: idle::Carousel::default(),
            ticker: ticker::Ticker::default(),
        }
    }

    /// Changes look without restarting: a new background, and the next frame
    /// counts as a new scene (flash, burst), with nothing leaving: the old
    /// scene belongs to the old look.
    pub fn set_look(&mut self, look: crate::look::Look) {
        self.background = Background::new(look);
        self.key.clear();
        self.shown = None;
    }

    pub fn draw(&mut self, gfx: &mut Gfx, view: &View, textures: &mut Textures, dt: f32) {
        let key = match &view.scene {
            Scene::Idle => "idle".to_owned(),
            Scene::Event { event, .. } => format!("event:{}", event.id.as_str()),
            Scene::Cinema(playing) => format!("cinema:{}", playing.id),
        };
        if key != self.key {
            self.key = key;
            // a scene cut short while it was still coming leaves from where it was
            self.leaving = self.shown.take().map(|scene| (scene, self.since));
            self.shown = Some(view.scene.clone());
            self.since = gfx.t;
            self.background.burst(if matches!(view.scene, Scene::Idle) {
                0.35
            } else {
                1.2
            });
        }
        let age = gfx.t - self.since;
        if age >= LEAVE {
            self.leaving = None;
        }
        let bg = gfx.palette.bg;
        gfx.rect(0.0, 0.0, WIDTH, HEIGHT, bg);
        self.background.draw(gfx, dt);

        if let Some((scene, since)) = self.leaving.take() {
            gfx.canvas.save();
            leave(gfx, age);
            self.scene(gfx, view, &scene, gfx.t - since, textures);
            gfx.canvas.restore();
            self.leaving = Some((scene, since));
        }
        gfx.canvas.save();
        enter(gfx, age);
        self.scene(gfx, view, &view.scene, age, textures);
        gfx.canvas.restore();
        gfx.alpha = 1.0;
        if gfx.cyber() {
            wipe_edge(gfx, age);
        }

        header::draw(gfx, view);
        self.ticker.draw(gfx, view, dt);
        flash(gfx, age);
        if gfx.hogwarts() {
            // no screen in that world: no scanlines, the parchment's burnt edges
            gfx.burnt_edges();
        } else {
            scanlines(gfx);
        }
        // the carousel turned: footsteps walk from the clock to the new card
        if let Some((from, to)) = self.idle.take_walk() {
            self.background.walk(from, to);
        }
    }

    fn scene(
        &mut self,
        gfx: &mut Gfx,
        view: &View,
        scene: &Scene,
        age: f32,
        textures: &mut Textures,
    ) {
        match scene {
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
    }
}

/// Middle of the scenes, the point they grow from and into.
const MIDDLE: (f32, f32) = (WIDTH / 2.0, f32::midpoint(TOP, BOTTOM));

fn zoom(gfx: &mut Gfx, scale: f32) {
    gfx.canvas.translate(MIDDLE.0, MIDDLE.1);
    gfx.canvas.scale(scale, scale);
    gfx.canvas.translate(-MIDDLE.0, -MIDDLE.1);
}

fn set_alpha(gfx: &mut Gfx, alpha: f32) {
    gfx.alpha = alpha;
    gfx.canvas.set_global_alpha(alpha);
}

/// How the scene that leaves goes, `age` seconds after the change: friday
/// zooms it through the glass, cyberpunk wipes it off to the right behind a
/// jittering edge, hogwarts lets it rise and fade like smoke.
fn leave(gfx: &mut Gfx, age: f32) {
    let k = progress(age, LEAVE);
    if gfx.cyber() {
        let edge = WIDTH * ease_in_out(k);
        let jitter = ((age * 50.0).sin() * 5.0).round();
        gfx.canvas.scissor(edge, 0.0, WIDTH - edge, HEIGHT);
        gfx.canvas.translate(jitter, 0.0);
        set_alpha(gfx, 1.0 - 0.4 * k);
    } else if gfx.hogwarts() {
        gfx.canvas.translate(0.0, -18.0 * ease_out(k));
        set_alpha(gfx, 1.0 - ease_out(k));
    } else {
        zoom(gfx, 1.0 + 0.08 * ease_out(k));
        set_alpha(gfx, 1.0 - ease_out(k));
    }
}

/// How the scene that comes settles: friday draws it in from a little
/// smaller, cyberpunk reveals it behind the wiping edge and jolts it,
/// hogwarts lets it rise from below once the smoke has thinned.
fn enter(gfx: &mut Gfx, age: f32) {
    let k = progress(age, ENTER);
    if gfx.cyber() {
        let edge = WIDTH * ease_in_out(progress(age, LEAVE));
        gfx.canvas.scissor(0.0, 0.0, edge, HEIGHT);
        // the change of scene jolts sideways for half a second
        let jolt = if age < 0.5 {
            ((age * 40.0).sin() * 6.0 * (1.0 - age * 2.0)).round()
        } else {
            0.0
        };
        gfx.canvas.translate(jolt, 0.0);
        set_alpha(gfx, ease_out(k));
    } else if gfx.hogwarts() {
        let k = progress(age - 0.15, ENTER + 0.2);
        gfx.canvas.translate(0.0, 16.0 * (1.0 - ease_out(k)));
        set_alpha(gfx, ease_in_out(k));
    } else {
        zoom(gfx, 0.94 + 0.06 * ease_back(k));
        set_alpha(gfx, ease_out(k));
    }
}

/// The edge of the cyberpunk wipe: a bright bar with a trail of glitch.
fn wipe_edge(gfx: &mut Gfx, age: f32) {
    let k = progress(age, LEAVE);
    if k >= 1.0 {
        return;
    }
    let edge = WIDTH * ease_in_out(k);
    let (red, cyan, yellow) = (gfx.palette.accent, gfx.palette.accent2, gfx.palette.hot);
    gfx.rect(edge - 2.0, TOP, 3.0, BOTTOM - TOP, fade(yellow, 0.9));
    gfx.rect(edge - 14.0, TOP, 10.0, BOTTOM - TOP, fade(cyan, 0.18));
    let tick = (age * 25.0) as u64;
    for n in 0..6u64 {
        let y = TOP + (BOTTOM - TOP) * unit(tick, n);
        let w = 20.0 + 70.0 * unit(tick, n + 10);
        let colour = if n % 2 == 0 { red } else { cyan };
        gfx.rect(
            edge - w,
            y,
            w,
            3.0 + 6.0 * unit(tick, n + 20),
            fade(colour, 0.45),
        );
    }
}

/// The light (friday), the coloured bands (cyberpunk) or the paper catching
/// the light (hogwarts) of a change of scene.
fn flash(gfx: &mut Gfx, age: f32) {
    let left = 1.0 - progress(age, 0.6);
    if left <= 0.0 {
        return;
    }
    if gfx.hogwarts() {
        gfx.halo(
            WIDTH / 2.0,
            HEIGHT / 2.0,
            460.0,
            fade(femtovg::Color::rgb(0xff, 0xf4, 0xd8), 0.45 * left),
        );
    } else if gfx.cyber() {
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
