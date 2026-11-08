//! What is drawn, frame by frame: the background, the header, the scene of the
//! moment (idle, event, cinema), the ticker, the banner of small news, the
//! night's veil, the scanlines and the flash that marks a change of scene.
//! `Stage` keeps what must last between frames.

mod banner;
mod cards;
mod cinema;
mod event;
mod header;
mod idle;
mod printer;
mod text;
mod ticker;
mod turner;
mod xmansion;

use chrono::Timelike;
use errol_domain::entities::{Sky, Weather};
use errol_domain::strategies::{Mode, Scene};
use errol_domain::usecases::View;
use femtovg::{Color, Paint, Path};

use crate::anim::{ease_back, ease_in_out, ease_out, progress, unit};
use crate::fx::Background;
use crate::look::{Look, fade, rgb};
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
/// How long the night's veil takes to be drawn or lifted, in seconds.
const VEIL_FADE: f32 = 1.5;
/// How dark the veil is, drawn.
const VEIL: f32 = 0.94;
/// The frame rate while the screen sleeps.
const NIGHT_FPS: u32 = 5;

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
    banners: banner::Banners,
    /// The night is forced (`--night`), for previews and shots.
    night: bool,
    /// How far the night's veil is drawn, 0 to 1.
    veil: f32,
    /// Nothing but the idle scene shows: no event, no banner, nothing leaving.
    quiet: bool,
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
            banners: banner::Banners::default(),
            night: false,
            veil: 0.0,
            quiet: false,
        }
    }

    /// Forces the night, whatever the hour and the house (`--night`).
    #[must_use]
    pub fn with_night(mut self, night: bool) -> Self {
        self.night = night;
        self
    }

    /// The frame rate the stage asks for, below the usual one: low while the
    /// screen sleeps and nothing moves.
    pub fn fps_cap(&self) -> Option<u32> {
        night_fps(self.veil, self.quiet)
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
        self.follow(gfx, view);
        let age = gfx.t - self.since;
        if age >= LEAVE {
            self.leaving = None;
        }
        self.follow_house(view, dt, age);
        let bg = gfx.palette.bg;
        gfx.rect(0.0, 0.0, WIDTH, HEIGHT, bg);
        self.background.draw(gfx, dt);
        self.draw_scenes(gfx, view, textures, age);
        self.draw_overlays(gfx, view, textures, (dt, age));
        // the carousel turned: footsteps walk from the clock to the new card
        if let Some((from, to)) = self.idle.take_walk() {
            self.background.walk(from, to);
        }
    }

    /// Notices a change of scene: the one shown starts to leave, the new one
    /// comes, and the background bursts.
    fn follow(&mut self, gfx: &Gfx, view: &View) {
        let key = scene_key(&view.scene);
        if key == self.key {
            return;
        }
        self.key = key;
        // a scene cut short while it was still coming leaves from where it was
        self.leaving = self.shown.take().map(|scene| (scene, self.since));
        self.shown = Some(view.scene.clone());
        self.since = gfx.t;
        self.background.burst(burst_strength(&view.scene));
    }

    /// What the house is doing: how lively the background is, the sky, the
    /// night's veil (lifted while an event shows), and a banner that comes.
    fn follow_house(&mut self, view: &View, dt: f32, age: f32) {
        let mode = house_mode(self.night, view);
        let event = matches!(view.scene, Scene::Event { .. });
        self.background.set_calm(liveliness(mode, event));
        self.background.set_sky(sky(view));
        self.veil = approach(self.veil, veil_target(mode, event), dt / VEIL_FADE);
        self.quiet = matches!(view.scene, Scene::Idle) && view.banner.is_none() && age >= ENTER;
        if self.banners.arrived(view.banner.as_ref())
            && let Some(banner) = &view.banner
        {
            self.background.react(&banner.event.kind);
            self.background.burst(0.5);
        }
    }

    /// The scene that leaves, if any, then the scene that comes.
    fn draw_scenes(&mut self, gfx: &mut Gfx, view: &View, textures: &mut Textures, age: f32) {
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
    }

    /// What lies over the scenes: the wipe's edge, the header, the ticker, the
    /// night's veil, the banner, the flash, and the scanlines or the
    /// parchment's burnt edges.
    fn draw_overlays(
        &mut self,
        gfx: &mut Gfx,
        view: &View,
        textures: &mut Textures,
        (dt, age): (f32, f32),
    ) {
        if gfx.cyber() {
            wipe_edge(gfx, age);
        }
        header::draw(gfx, view);
        self.ticker.draw(gfx, view, dt);
        if self.veil > 0.0 {
            veil(gfx, self.veil);
        }
        if let Some(banner) = &view.banner {
            banner::draw(gfx, view, banner, textures, banner_alpha(self.veil));
        }
        flash(gfx, age);
        edges(gfx, self.veil);
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

/// Identifies a scene: "idle", "event:<id>", "cinema:<id>".
fn scene_key(scene: &Scene) -> String {
    match scene {
        Scene::Idle => "idle".to_owned(),
        Scene::Event { event, .. } => format!("event:{}", event.id.as_str()),
        Scene::Cinema(playing) => format!("cinema:{}", playing.id),
    }
}

/// How hard the background bursts when a scene comes: softly for the idle
/// recap, strongly for news.
fn burst_strength(scene: &Scene) -> f32 {
    if matches!(scene, Scene::Idle) {
        0.35
    } else {
        1.2
    }
}

/// The mode of the house at the view's hour, or the night when it is forced.
fn house_mode(night: bool, view: &View) -> Mode {
    if night {
        return Mode::Night;
    }
    Mode::of(view.snapshot.as_deref(), text::local(view.now).hour())
}

/// The sky outside, when the feed knows the weather.
fn sky(view: &View) -> Option<Sky> {
    view.snapshot
        .as_ref()
        .and_then(|snapshot| snapshot.weather.as_ref())
        .map(Weather::sky)
}

/// How lively the background is: as usual by day, quieter for a film, still
/// at night unless an event woke the screen.
fn liveliness(mode: Mode, event: bool) -> f32 {
    match mode {
        Mode::Cinema => 0.45,
        Mode::Night if !event => 0.0,
        _ => 1.0,
    }
}

/// The night's veil is drawn at night, and lifted while an event shows.
fn veil_target(mode: Mode, event: bool) -> f32 {
    if mode == Mode::Night && !event {
        1.0
    } else {
        0.0
    }
}

/// `value` moved towards `target` by at most `step`.
fn approach(value: f32, target: f32, step: f32) -> f32 {
    if value < target {
        (value + step).min(target)
    } else {
        (value - step).max(target)
    }
}

/// A banner at night shows, dimmed by half once the veil is drawn.
fn banner_alpha(veil: f32) -> f32 {
    1.0 - 0.5 * veil
}

/// The frame rate while the veil is drawn and nothing but the idle scene
/// shows; none (the usual) otherwise.
fn night_fps(veil: f32, quiet: bool) -> Option<u32> {
    (veil >= 1.0 && quiet).then_some(NIGHT_FPS)
}

/// The veil's colour: deep night in friday and cyberpunk, dark sepia on the Map.
fn veil_colour(look: Look) -> Color {
    match look {
        Look::Friday => rgb(0x01_040a),
        Look::Cyberpunk => rgb(0x05_0203),
        Look::Hogwarts => rgb(0x12_0a04),
    }
}

/// Where the idle clock is (centre and radius): the veil thins over it, so
/// only a faint clock shows.
fn clock_window(look: Look) -> (f32, f32, f32) {
    if look == Look::Hogwarts {
        (140.0, 250.0, 150.0)
    } else {
        (140.0, 245.0, 115.0)
    }
}

/// The night's veil, `drawn` from 0 to 1: near black over everything, a
/// little thinner over the clock.
fn veil(gfx: &mut Gfx, drawn: f32) {
    let k = ease_in_out(drawn);
    let colour = veil_colour(gfx.look);
    let (x, y, r) = clock_window(gfx.look);
    let paint = Paint::radial_gradient(
        x,
        y,
        r * 0.7,
        r * 1.3,
        fade(colour, 0.6 * k),
        fade(colour, VEIL * k),
    );
    let mut path = Path::new();
    path.rect(0.0, 0.0, WIDTH, HEIGHT);
    gfx.canvas.fill_path(&path, &paint);
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

/// The scanlines, or on the Map the parchment's burnt edges (no screen in
/// that world), which the night's veil hides.
fn edges(gfx: &mut Gfx, veil: f32) {
    if gfx.hogwarts() {
        gfx.canvas.save();
        gfx.canvas.set_global_alpha(1.0 - veil);
        gfx.burnt_edges();
        gfx.canvas.restore();
    } else {
        scanlines(gfx);
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

#[cfg(test)]
mod tests {
    use super::*;
    use errol_domain::entities::{Event, EventId, EventKind, Timestamp};

    fn event() -> Scene {
        Scene::Event {
            event: Event {
                id: EventId::new("7-1"),
                at: Timestamp::from_secs(0),
                kind: EventKind::Play {
                    title: "Dune".into(),
                },
            },
            started: Timestamp::from_secs(0),
            ends: Timestamp::from_secs(60),
        }
    }

    #[test]
    fn a_scene_is_known_by_its_kind_and_id() {
        assert_eq!(scene_key(&Scene::Idle), "idle");
        assert_eq!(scene_key(&event()), "event:7-1");
    }

    fn view(scene: Scene) -> View {
        View {
            snapshot: None,
            read_at: Timestamp::from_secs(0),
            scene,
            banner: None,
            now: Timestamp::from_secs(0),
            warning: None,
        }
    }

    #[test]
    fn the_night_is_forced_or_comes_from_the_house() {
        assert_eq!(house_mode(true, &view(Scene::Idle)), Mode::Night);
        // without a snapshot, it is day whatever the hour
        assert_eq!(house_mode(false, &view(Scene::Idle)), Mode::Day);
        assert_eq!(sky(&view(Scene::Idle)), None);
    }

    #[test]
    fn the_background_calms_for_a_film_and_sleeps_at_night() {
        assert!((liveliness(Mode::Day, false) - 1.0).abs() < f32::EPSILON);
        assert!((liveliness(Mode::Cinema, false) - 0.45).abs() < f32::EPSILON);
        assert!(liveliness(Mode::Night, false).abs() < f32::EPSILON);
        assert!((liveliness(Mode::Night, true) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn an_event_lifts_the_veil_slowly() {
        assert!((veil_target(Mode::Night, false) - 1.0).abs() < f32::EPSILON);
        assert!(veil_target(Mode::Night, true).abs() < f32::EPSILON);
        assert!(veil_target(Mode::Day, false).abs() < f32::EPSILON);
        assert!((approach(0.0, 1.0, 0.25) - 0.25).abs() < f32::EPSILON);
        assert!((approach(0.9, 1.0, 0.25) - 1.0).abs() < f32::EPSILON);
        assert!((approach(1.0, 0.0, 0.25) - 0.75).abs() < f32::EPSILON);
        assert!((banner_alpha(1.0) - 0.5).abs() < f32::EPSILON);
        assert!((banner_alpha(0.0) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn the_screen_slows_down_only_asleep_and_still() {
        assert_eq!(night_fps(1.0, true), Some(NIGHT_FPS));
        assert_eq!(night_fps(0.6, true), None);
        assert_eq!(night_fps(1.0, false), None);
        let stage = Stage::new(Look::Friday).with_night(true);
        assert!(stage.night);
        assert_eq!(stage.fps_cap(), None);
    }

    #[test]
    fn the_veil_thins_over_the_clock_of_each_look() {
        for look in [Look::Friday, Look::Cyberpunk, Look::Hogwarts] {
            let (x, y, r) = clock_window(look);
            assert!(x + r < WIDTH / 2.0 && y > TOP && y < BOTTOM, "{look:?}");
            assert!(veil_colour(look).r < 0.1, "{look:?}");
        }
    }

    #[test]
    fn news_bursts_harder_than_the_recap() {
        assert!((burst_strength(&Scene::Idle) - 0.35).abs() < f32::EPSILON);
        assert!((burst_strength(&event()) - 1.2).abs() < f32::EPSILON);
    }
}
