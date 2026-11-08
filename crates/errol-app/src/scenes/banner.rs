//! Small news over the scene: a lamp, the projection screen, the projector
//! changing app, and whatever happens while a video plays. Like breaking news
//! on television it covers the ticker, low on the right, where it hides
//! nothing essential (the cards' feet, the cinema's last line at most): a
//! strip of the HUD in friday, a hard-edged strip that glitches in in
//! cyberpunk, an owl's note pinned on the Map.

use errol_domain::entities::Event;
use errol_domain::strategies::Banner;
use errol_domain::usecases::View;
use femtovg::{Color, Paint};

use super::{BOTTOM, HEIGHT, WIDTH, event};
use crate::anim::{decode, ease_in_out, ease_out, progress, unit};
use crate::look::{Look, fade};
use crate::render::{Anchor, Face, Gfx, Textures, Type, ink};

/// How long a banner takes to come and to go, in seconds.
const SLIDE: f32 = 0.35;

/// A box: x, y, w, h.
type Slip = (f32, f32, f32, f32);

/// The banner shown, to tell a new one when it comes.
#[derive(Debug, Default)]
pub struct Banners {
    /// The id of its event; empty while there is none.
    shown: String,
}

impl Banners {
    /// True once, the frame a new banner comes.
    pub fn arrived(&mut self, banner: Option<&Banner>) -> bool {
        let id = banner.map_or("", |banner| banner.event.id.as_str());
        if id == self.shown {
            return false;
        }
        id.clone_into(&mut self.shown);
        !id.is_empty()
    }
}

/// Draws the banner, `alpha` dimming it (at night).
pub fn draw(gfx: &mut Gfx, view: &View, banner: &Banner, textures: &mut Textures, alpha: f32) {
    let age = view.now.secs_since(banner.started) as f32;
    let length = banner.ends.secs_since(banner.started) as f32;
    let shown = presence(age, length);
    if shown <= 0.0 {
        return;
    }
    let slip = slip(gfx.look);
    if !gfx.hogwarts() {
        hush(gfx, slip, alpha * shown);
    }
    gfx.canvas.save();
    arrive(gfx, slip, age, shown, alpha);
    paper(gfx, slip);
    let (x, y, size) = icon(gfx.look, slip);
    event::thumbnail(gfx, view, &banner.event, age, textures, (x, y, size));
    tile(gfx, (x, y, size));
    lines(gfx, slip, &banner.event, age);
    drain(gfx, slip, 1.0 - progress(age, length));
    if gfx.cyber() {
        glitch(gfx, slip, age, length);
    }
    gfx.canvas.restore();
    gfx.alpha = 1.0;
}

/// How much of the banner shows, `age` seconds into its `length`: it comes
/// in `SLIDE` seconds and goes in as many before its end.
fn presence(age: f32, length: f32) -> f32 {
    ease_out(progress(age, SLIDE)).min(ease_out(progress(length - age, SLIDE)))
}

/// Where the banner lies: over the ticker, on the right, under the cards;
/// on the Map a smaller note, clear of the burnt corner.
fn slip(look: Look) -> Slip {
    match look {
        Look::Hogwarts => (392.0, HEIGHT - 88.0, 348.0, 82.0),
        _ => (288.0, HEIGHT - 66.0, 488.0, 62.0),
    }
}

/// How the banner comes and goes, `shown` from 0 to 1: friday slides it in
/// from the right, cyberpunk reveals it from the left behind a jolt, the ink
/// of the note on the Map darkens as it rises a little.
fn arrive(gfx: &mut Gfx, (x, y, w, h): Slip, age: f32, shown: f32, alpha: f32) {
    if gfx.cyber() {
        gfx.canvas
            .intersect_scissor(x - 4.0, y - 4.0, (w + 8.0) * shown, h + 8.0);
        gfx.canvas.translate(jolt(age), 0.0);
    } else if gfx.hogwarts() {
        gfx.canvas.translate(0.0, 6.0 * (1.0 - shown));
    } else {
        gfx.canvas.translate(40.0 * (1.0 - shown), 0.0);
    }
    let alpha = alpha * fading(gfx.cyber(), shown);
    gfx.alpha = alpha;
    gfx.canvas.set_global_alpha(alpha);
}

/// The opacity of a banner `shown` that far: cyberpunk's strip is all there
/// as soon as it is revealed, the others fade.
fn fading(cyber: bool, shown: f32) -> f32 {
    if cyber { 1.0 } else { shown }
}

/// The sideways jolt of cyberpunk's strip as it comes, for half a second.
fn jolt(age: f32) -> f32 {
    if age < 0.5 {
        ((age * 50.0).sin() * 5.0 * (1.0 - age * 2.0)).round()
    } else {
        0.0
    }
}

/// What the banner is written on: glass and brackets in friday, a cut panel
/// in red and yellow in cyberpunk, a note pinned on the Map.
fn paper(gfx: &mut Gfx, slip: Slip) {
    match gfx.look {
        Look::Friday => glass(gfx, slip),
        Look::Cyberpunk => panel(gfx, slip),
        Look::Hogwarts => note(gfx, slip),
    }
}

/// The ticker hushed beside the strip: its end would run out from under it.
fn hush(gfx: &mut Gfx, (x, _, w, _): Slip, shown: f32) {
    let bg = fade(gfx.palette.bg, shown);
    gfx.rect(x + w, BOTTOM, WIDTH - x - w, HEIGHT - BOTTOM, bg);
}

/// Friday's strip: dark glass lit from the left, a thin edge, the accent's
/// corner brackets.
fn glass(gfx: &mut Gfx, (x, y, w, h): Slip) {
    let palette = gfx.palette;
    gfx.rect(x, y, w, h, fade(palette.bg, 0.97));
    gfx.gradient_rect(
        x,
        y,
        w,
        h,
        fade(palette.accent, 0.14),
        fade(palette.accent, 0.02),
    );
    gfx.rect_line(x, y, w, h, fade(palette.accent, 0.2), 1.0);
    gfx.brackets(
        x - 1.0,
        y - 1.0,
        w + 2.0,
        h + 2.0,
        14.0,
        palette.accent,
        2.0,
    );
}

/// Cyberpunk's strip: black, its corner cut, a red bar on the left and a
/// yellow edge.
fn panel(gfx: &mut Gfx, (x, y, w, h): Slip) {
    let palette = gfx.palette;
    let path = Gfx::cut_box(x, y, w, h, 16.0);
    let paint = Paint::linear_gradient(
        x,
        0.0,
        x + w,
        0.0,
        fade(palette.accent, 0.3),
        fade(palette.bg, 0.92),
    );
    gfx.fill(&path, fade(palette.bg, 0.97));
    gfx.canvas.fill_path(&path, &paint);
    gfx.stroke(&path, fade(palette.hot, 0.75), 1.0);
    gfx.rect(x, y, 4.0, h, palette.accent);
}

/// The Map's note: a slip of paper pinned askew, in a thin rule.
fn note(gfx: &mut Gfx, (x, y, w, h): Slip) {
    gfx.pinned(x, y, w, h, -1.2);
    gfx.rect_line(
        x + 6.0,
        y + 6.0,
        w - 12.0,
        h - 12.0,
        fade(ink::news_ink(), 0.5),
        1.0,
    );
}

/// The icon's centre and size: on the left of the strip.
fn icon(look: Look, (x, y, _, h): Slip) -> (f32, f32, f32) {
    if look == Look::Hogwarts {
        (x + 44.0, y + h / 2.0 + 1.0, 56.0)
    } else {
        (x + 34.0, y + h / 2.0, 48.0)
    }
}

/// The thin frame of the icon, which the picture fills: the accent's in
/// friday, yellow in cyberpunk, a rule of ink on the Map.
fn tile(gfx: &mut Gfx, (x, y, size): (f32, f32, f32)) {
    let colour = match gfx.look {
        Look::Friday => fade(gfx.palette.accent, 0.35),
        Look::Cyberpunk => fade(gfx.palette.hot, 0.5),
        Look::Hogwarts => fade(ink::news_ink(), 0.6),
    };
    gfx.rect_line(x - size / 2.0, y - size / 2.0, size, size, colour, 1.0);
}

/// Where the words start, right of the icon.
fn words_x(look: Look, (x, _, _, _): Slip) -> f32 {
    if look == Look::Hogwarts {
        x + 84.0
    } else {
        x + 72.0
    }
}

/// How a look writes a banner's title: its style, its colour, in capitals.
fn title_hand(look: Look) -> (Type, bool) {
    match look {
        Look::Friday => (Type::new(Face::Display, 15.0), false),
        Look::Cyberpunk => (Type::new(Face::Bold, 22.0), true),
        Look::Hogwarts => (Type::new(Face::Italic, 21.0), false),
    }
}

/// How far down the kicker, the title and the subtitle are: the title comes
/// down a little when nothing goes under it.
fn rows(look: Look, has_sub: bool) -> (f32, f32, f32) {
    match (look, has_sub) {
        (Look::Hogwarts, true) => (11.0, 35.0, 55.0),
        (Look::Hogwarts, false) => (14.0, 43.0, 0.0),
        (Look::Cyberpunk, true) => (7.0, 17.0, 41.0),
        (Look::Cyberpunk, false) => (11.0, 27.0, 0.0),
        (Look::Friday, true) => (8.0, 22.0, 42.0),
        (Look::Friday, false) => (12.0, 30.0, 0.0),
    }
}

/// The kicker, a line of title and the subtitle, as `event.rs` words them.
fn lines(gfx: &mut Gfx, slip: Slip, event: &Event, age: f32) {
    let (kicker, title, sub, _) = event::words(event);
    let (x, y, w, _) = slip;
    let left = words_x(gfx.look, slip);
    let (top, title_y, sub_y) = rows(gfx.look, !sub.is_empty());
    kick(gfx, left, y + top, &kicker);
    let (style, capitals) = title_hand(gfx.look);
    let title = if capitals {
        title.to_uppercase()
    } else {
        title
    };
    let width = x + w - 16.0 - left;
    let line = first_line(gfx.wrap(&title, width, style, 1));
    let shown = decode(&line, age - 0.1, 0.8, gfx.glyphs());
    let colour = title_colour(gfx);
    gfx.title(left, y + title_y, &shown, style, colour, Anchor::TOP_LEFT);
    if !sub.is_empty() {
        let sub = first_line(gfx.wrap(&sub, width, Type::new(Face::Italic, 14.0), 1));
        gfx.text(
            left,
            y + sub_y,
            &sub,
            Type::new(Face::Italic, 14.0),
            sub_colour(gfx),
            Anchor::TOP_LEFT,
        );
    }
}

/// The first of wrapped lines, or nothing.
fn first_line(lines: Vec<String>) -> String {
    lines.into_iter().next().unwrap_or_default()
}

/// The kicker: the accent's dash in friday, yellow in cyberpunk, a red ink
/// stamp on the Map.
fn kick(gfx: &mut Gfx, x: f32, y: f32, kicker: &str) {
    match gfx.look {
        Look::Hogwarts => {
            gfx.stamp(x, y, kicker, -2.5);
        }
        Look::Cyberpunk => gfx.kicker(x, y, kicker, gfx.palette.hot),
        Look::Friday => gfx.kicker(x, y, kicker, gfx.palette.accent),
    }
}

fn title_colour(gfx: &Gfx) -> Color {
    match gfx.look {
        Look::Friday => gfx.palette.text,
        Look::Cyberpunk => gfx.palette.hot,
        Look::Hogwarts => ink::news_ink(),
    }
}

fn sub_colour(gfx: &Gfx) -> Color {
    if gfx.hogwarts() {
        fade(ink::news_ink(), 0.75)
    } else {
        gfx.palette.muted
    }
}

/// The line of the time left, draining: along the strip's foot, inside the
/// note's rule on the Map.
fn drain_line(look: Look, (x, y, w, h): Slip) -> (f32, f32, f32) {
    if look == Look::Hogwarts {
        (x + 12.0, y + h - 11.0, w - 24.0)
    } else {
        (x + 4.0, y + h - 3.0, w - 8.0)
    }
}

/// The time left to the banner, `left` from 1 to 0.
fn drain(gfx: &mut Gfx, slip: Slip, left: f32) {
    let (x, y, w) = drain_line(gfx.look, slip);
    let colour = match gfx.look {
        Look::Friday => gfx.palette.accent,
        Look::Cyberpunk => gfx.palette.hot,
        Look::Hogwarts => fade(ink::red(), 0.7),
    };
    gfx.rect(x, y, w, 2.0, fade(colour, 0.12));
    gfx.rect(x, y, w * left, 2.0, colour);
}

/// Cyberpunk's glitch as the strip comes and goes: bars of red and cyan
/// across it, changing 25 times a second.
fn glitch(gfx: &mut Gfx, (x, y, w, h): Slip, age: f32, length: f32) {
    let strength = glitch_strength(age, length);
    if strength <= 0.0 {
        return;
    }
    let (red, cyan) = (gfx.palette.accent, gfx.palette.accent2);
    let tick = (age * 25.0) as u64;
    for n in 0..4u64 {
        let colour = if n % 2 == 0 { red } else { cyan };
        gfx.rect(
            x + w * unit(tick, n),
            y + h * unit(tick, n + 10),
            40.0 + 120.0 * unit(tick, n + 20),
            2.0 + 4.0 * unit(tick, n + 30),
            fade(colour, 0.55 * strength),
        );
    }
}

/// How hard the strip glitches: in its first half second and its last.
fn glitch_strength(age: f32, length: f32) -> f32 {
    let coming = 1.0 - progress(age, 0.5);
    let going = 1.0 - progress(length - age, 0.5);
    ease_in_out(coming.max(going))
}

#[cfg(test)]
mod tests {
    use super::*;
    use errol_domain::entities::{EventId, EventKind, Timestamp};

    fn banner(id: &str) -> Banner {
        Banner {
            event: Event {
                id: EventId::new(id),
                at: Timestamp::from_secs(0),
                kind: EventKind::Light {
                    name: "Néon".into(),
                    on: true,
                },
            },
            started: Timestamp::from_secs(0),
            ends: Timestamp::from_secs(8),
        }
    }

    #[test]
    fn a_new_banner_is_told_once() {
        let mut banners = Banners::default();
        assert!(!banners.arrived(None));
        assert!(banners.arrived(Some(&banner("a"))));
        assert!(!banners.arrived(Some(&banner("a"))));
        assert!(banners.arrived(Some(&banner("b"))));
        assert!(!banners.arrived(None));
        assert!(banners.arrived(Some(&banner("b"))));
    }

    #[test]
    fn a_banner_comes_stays_and_goes() {
        assert!(presence(0.0, 8.0).abs() < 1e-5);
        assert!((presence(4.0, 8.0) - 1.0).abs() < 1e-5);
        assert!(presence(8.0, 8.0).abs() < 1e-5);
        assert!(presence(9.0, 8.0) <= 0.0);
        assert!(presence(0.2, 8.0) > 0.5);
    }

    #[test]
    fn the_banner_lies_low_and_clear_of_the_ticker() {
        for look in [Look::Friday, Look::Cyberpunk, Look::Hogwarts] {
            let (x, y, w, h) = slip(look);
            assert!(y + h <= HEIGHT - 4.0 && y > 380.0, "{look:?}");
            assert!(x + w <= 780.0, "{look:?}");
            let (ix, _, size) = icon(look, (x, y, w, h));
            assert!(ix + size / 2.0 < words_x(look, (x, y, w, h)), "{look:?}");
            let (dx, dy, dw) = drain_line(look, (x, y, w, h));
            assert!(dx > x && dx + dw < x + w && dy < y + h, "{look:?}");
            for has_sub in [true, false] {
                let (kicker, title, sub) = rows(look, has_sub);
                assert!(kicker < title && (sub == 0.0 || title < sub), "{look:?}");
                assert!(sub < h - 18.0, "{look:?}");
            }
        }
    }

    #[test]
    fn each_look_writes_its_title_in_its_hand() {
        assert!(title_hand(Look::Cyberpunk).1);
        assert!(!title_hand(Look::Friday).1);
        assert_eq!(title_hand(Look::Hogwarts).0.face, Face::Italic);
        assert_eq!(first_line(vec!["a".into(), "b".into()]), "a");
        assert_eq!(first_line(Vec::new()), "");
    }

    #[test]
    fn cyberpunk_jolts_and_glitches_only_at_the_edges() {
        assert!(jolt(1.0).abs() < f32::EPSILON);
        assert!(jolt(0.03).abs() > 0.0);
        assert!(glitch_strength(0.0, 8.0) > 0.99);
        assert!(glitch_strength(4.0, 8.0).abs() < f32::EPSILON);
        assert!(glitch_strength(8.0, 8.0) > 0.99);
        assert!((fading(true, 0.2) - 1.0).abs() < f32::EPSILON);
        assert!((fading(false, 0.2) - 0.2).abs() < f32::EPSILON);
    }
}
