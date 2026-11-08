//! What the three backgrounds share around their own animation: how lively
//! they are (a film calms them, the night holds them still), the sky outside
//! (rain or snow falling through them) and, for the small news of a banner,
//! the tone and the place each kind of news answers with.

use errol_domain::entities::{EventKind, PrintPhase, Sky};
use femtovg::{Color, Path};

use crate::anim::unit;
use crate::look::{Palette, fade};
use crate::render::Gfx;

const W: f32 = 800.0;
const H: f32 = 480.0;
/// Liveliness moves this much a second: from lively to still in two seconds.
const EASE: f32 = 0.5;
/// A piece of news wakes a still background for this many seconds.
const STIR: f32 = 4.0;

/// The background's liveliness, easing toward the one asked for.
pub struct Calm {
    now: f32,
    target: f32,
    /// Nothing asked yet: the first wish is granted at once.
    fresh: bool,
}

impl Calm {
    pub fn new() -> Self {
        Self {
            now: 1.0,
            target: 1.0,
            fresh: true,
        }
    }

    pub fn aim(&mut self, liveliness: f32) {
        self.target = liveliness.clamp(0.0, 1.0);
        if self.fresh {
            self.fresh = false;
            self.now = self.target;
        }
    }

    /// Eases toward the target over `dt` seconds; returns the liveliness.
    pub fn ease(&mut self, dt: f32) -> f32 {
        self.now = ease_toward(self.now, self.target, dt);
        self.now
    }
}

/// `now` moved toward `target` at `EASE` a second, without overshooting.
fn ease_toward(now: f32, target: f32, dt: f32) -> f32 {
    let step = EASE * dt;
    if (target - now).abs() <= step {
        target
    } else {
        now + step.copysign(target - now)
    }
}

/// How lively a background is, the sky it shows, and its own clock, which
/// stands still when the background does.
pub struct Ambience {
    calm: Calm,
    pub sky: Option<Sky>,
    /// Seconds of liveliness lived: what falling rain and lightning follow.
    pub clock: f32,
    /// 1 when news just came, wearing off over `STIR` seconds.
    stir: f32,
    /// The liveliness of the last frame.
    pub live: f32,
}

impl Ambience {
    pub fn new() -> Self {
        Self {
            calm: Calm::new(),
            sky: None,
            clock: 0.0,
            stir: 0.0,
            live: 1.0,
        }
    }

    pub fn aim(&mut self, liveliness: f32) {
        self.calm.aim(liveliness);
    }

    /// News came: the background wakes for a few seconds, even at night.
    pub fn stir(&mut self) {
        self.stir = 1.0;
    }

    /// A frame of `dt` seconds passed: returns the background's own `dt`,
    /// slower when calm, zero when still.
    pub fn tick(&mut self, dt: f32) -> f32 {
        self.stir = (self.stir - dt / STIR).max(0.0);
        self.live = self.calm.ease(dt).max(self.stir);
        self.clock += dt * self.live;
        dt * self.live
    }
}

/// What the news is like, each look colouring it its own way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// A lamp lit.
    Warm,
    /// A lamp went out.
    Dim,
    /// The projection screen or the projector.
    Plain,
    /// Something arrived or finished well.
    Good,
    /// Something wants an answer or failed.
    Alert,
}

pub fn tone(kind: &EventKind) -> Tone {
    match kind {
        EventKind::Light { on: true, .. } => Tone::Warm,
        EventKind::Light { on: false, .. } => Tone::Dim,
        EventKind::Download { .. } | EventKind::Import(_) | EventKind::Scan(_) => Tone::Good,
        EventKind::Waiting { .. } => Tone::Alert,
        EventKind::Print(news) => match news.phase {
            PrintPhase::Started => Tone::Warm,
            PrintPhase::Done => Tone::Good,
            PrintPhase::Cancelled => Tone::Alert,
        },
        EventKind::Cover { .. } | EventKind::Projector { .. } | EventKind::Play { .. } => {
            Tone::Plain
        }
    }
}

/// Where on the screen the news happens: lamps to the left, the cinema
/// (screen, projector, film) to the right, what arrives along the top, the
/// printer top left, a waiting session in the middle.
pub fn place(kind: &EventKind) -> (f32, f32) {
    match kind {
        EventKind::Light { .. } => (190.0, 300.0),
        EventKind::Cover { .. } => (630.0, 130.0),
        EventKind::Projector { .. } => (650.0, 250.0),
        EventKind::Play { .. } => (600.0, 340.0),
        EventKind::Download { .. } => (400.0, 90.0),
        EventKind::Import(_) => (520.0, 100.0),
        EventKind::Scan(_) => (280.0, 100.0),
        EventKind::Print(_) => (130.0, 130.0),
        EventKind::Waiting { .. } => (400.0, 260.0),
    }
}

/// The colours a look picks for a tone: warm, dim, plain, good, alert.
pub fn tone_colour(tone: Tone, colours: [Color; 5]) -> Color {
    colours[tone as usize]
}

/// The palette's colours for the tones, in the order of `Tone`.
pub fn tones(palette: &Palette, warm: Color, plain: Color, alert: Color) -> [Color; 5] {
    [warm, palette.muted, plain, palette.ok, alert]
}

/// Rain or snow falling through a background: so many drops, falling so
/// fast (pixels a second), each a streak so long and slanted (0 long: a dot
/// swaying as it falls).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fall {
    pub count: usize,
    pub speed: f32,
    pub length: f32,
    pub slant: f32,
}

/// Where drop `i` is at `clock`: it falls, wraps at the bottom, a dot sways.
pub fn drop_at(i: usize, clock: f32, fall: Fall) -> (f32, f32) {
    let i = i as u64;
    let speed = fall.speed * (0.7 + 0.6 * unit(i, 13));
    let span = H + fall.length;
    let y = (unit(i, 12) * span + clock * speed) % span - fall.length;
    let sway = if fall.length > 0.0 {
        -fall.slant * y / H * 30.0
    } else {
        (clock * 0.9 + i as f32).sin() * 9.0
    };
    (
        (unit(i, 11) * (W + 60.0) - 30.0 + sway).rem_euclid(W + 60.0) - 30.0,
        y,
    )
}

/// The drops, all in one path: streaks stroked, dots filled.
pub fn draw_fall(gfx: &mut Gfx, fall: Fall, clock: f32, colour: Color, alpha: f32) {
    let mut path = Path::new();
    for i in 0..fall.count {
        let (x, y) = drop_at(i, clock, fall);
        if fall.length > 0.0 {
            path.move_to(x, y);
            path.line_to(x - fall.slant * fall.length, y + fall.length);
        } else {
            path.circle(x, y, 1.3);
        }
    }
    if fall.length > 0.0 {
        gfx.stroke(&path, fade(colour, alpha), 1.0);
    } else {
        gfx.fill(&path, fade(colour, alpha));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use errol_domain::entities::PrintNews;

    fn light(on: bool) -> EventKind {
        EventKind::Light {
            name: "Salon".into(),
            on,
        }
    }

    fn print(phase: PrintPhase) -> EventKind {
        EventKind::Print(PrintNews {
            phase,
            printer: "Saturn".into(),
            file: "owl.ctb".into(),
            image: None,
            layers: None,
            remaining: None,
            started: None,
        })
    }

    #[test]
    fn calm_eases_over_two_seconds() {
        assert!((ease_toward(1.0, 0.0, 1.0) - 0.5).abs() < 1e-6);
        assert!(ease_toward(1.0, 0.0, 2.0).abs() < 1e-6);
        assert!((ease_toward(0.0, 0.45, 0.5) - 0.25).abs() < 1e-6);
        assert!((ease_toward(0.4, 0.45, 0.5) - 0.45).abs() < 1e-6);
    }

    #[test]
    fn the_first_wish_is_granted_at_once_the_next_ones_eased() {
        let mut calm = Calm::new();
        calm.aim(0.0);
        assert!(calm.ease(0.0).abs() < 1e-6);
        calm.aim(1.0);
        assert!((calm.ease(1.0) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn a_still_background_keeps_its_clock_until_news_stirs_it() {
        let mut ambience = Ambience::new();
        ambience.aim(0.0);
        assert!(ambience.tick(0.1).abs() < 1e-6);
        assert!(ambience.clock.abs() < 1e-6);
        ambience.stir();
        assert!(ambience.tick(0.1) > 0.09);
        for _ in 0..50 {
            ambience.tick(0.1);
        }
        assert!(ambience.tick(0.1).abs() < 1e-6);
    }

    #[test]
    fn each_kind_of_news_has_its_tone() {
        assert_eq!(tone(&light(true)), Tone::Warm);
        assert_eq!(tone(&light(false)), Tone::Dim);
        assert_eq!(tone(&print(PrintPhase::Started)), Tone::Warm);
        assert_eq!(tone(&print(PrintPhase::Done)), Tone::Good);
        assert_eq!(tone(&print(PrintPhase::Cancelled)), Tone::Alert);
        let download = EventKind::Download {
            name: "x".into(),
            size: None,
        };
        assert_eq!(tone(&download), Tone::Good);
        let waiting = EventKind::Waiting {
            name: "errol".into(),
            project: None,
            room: None,
        };
        assert_eq!(tone(&waiting), Tone::Alert);
        assert_eq!(tone(&EventKind::Play { title: "x".into() }), Tone::Plain);
    }

    #[test]
    fn lamps_happen_left_the_cinema_right() {
        assert!(place(&light(true)).0 < 400.0);
        let cover = EventKind::Cover {
            id: "c".into(),
            name: "Écran".into(),
            down: true,
        };
        let projector = EventKind::Projector {
            name: "p".into(),
            on: true,
            app: None,
        };
        assert!(place(&cover).0 > 400.0 && place(&projector).0 > 400.0);
        assert!(place(&print(PrintPhase::Done)).1 < 240.0);
    }

    #[test]
    fn a_look_colours_the_tones_its_own_way() {
        let palette = crate::look::Look::Friday.palette();
        let colours = tones(&palette, palette.accent2, palette.accent, palette.hot);
        assert_eq!(tone_colour(Tone::Warm, colours), palette.accent2);
        assert_eq!(tone_colour(Tone::Dim, colours), palette.muted);
        assert_eq!(tone_colour(Tone::Good, colours), palette.ok);
        assert_eq!(tone_colour(Tone::Alert, colours), palette.hot);
    }

    #[test]
    fn drops_fall_and_wrap_within_the_screen() {
        let rain = Fall {
            count: 10,
            speed: 400.0,
            length: 12.0,
            slant: 0.2,
        };
        let snow = Fall {
            length: 0.0,
            ..rain
        };
        for clock in [0.0, 1.0, 33.3] {
            for fall in [rain, snow] {
                let (x, y) = drop_at(3, clock, fall);
                assert!((-30.0..=W + 30.0).contains(&x), "{x}");
                assert!((-12.0..=H).contains(&y), "{y}");
            }
        }
        assert_ne!(drop_at(3, 0.0, rain), drop_at(3, 0.01, rain));
    }
}
