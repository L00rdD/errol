//! The idle cards added to the first ones: the weather and the sun, the
//! episodes to watch, the day's recap. The sky is drawn, not loaded: a sun,
//! a moon, clouds and what falls from them, inked as each look inks.

use std::f32::consts::{PI, TAU};

use errol_domain::entities::{Episode, Sky, Snapshot, Timestamp, Today, Weather};
use femtovg::{Color, Paint, Path};

use super::idle::{Box, CARD_W, POSTER, Sheet, big, poster, row, scoreboard};
use super::{text, turner};
use crate::anim::{decode, hash};
use crate::look::{Look, Palette, fade, mix};
use crate::render::{Anchor, Face, Gfx, Textures, Type, ink};

/// The width inside a card.
const WIDTH: f32 = CARD_W - 44.0;
/// Seconds an episode stays on the card after it started airing.
const AIRED: f64 = 3600.0;

// ---------- the sky ----------

/// How the sky is inked in a look.
#[derive(Debug, Clone, Copy)]
struct Inks {
    /// The outlines.
    line: Color,
    /// Inside the clouds.
    cloud: Color,
    /// The sun and the moon, and their outline.
    sun: Color,
    rim: Color,
    /// Rain, snow, fog.
    fall: Color,
    /// Cyberpunk splits its lines in its second colour.
    ghost: Option<Color>,
}

fn inks(look: Look) -> Inks {
    let p = look.palette();
    match look {
        Look::Friday => Inks {
            line: p.accent,
            cloud: mix(p.bg, p.accent, 0.14),
            sun: fade(p.accent2, 0.4),
            rim: p.accent2,
            fall: p.accent,
            ghost: None,
        },
        Look::Cyberpunk => Inks {
            line: p.accent,
            cloud: mix(p.bg, p.accent, 0.1),
            sun: p.hot,
            rim: p.hot,
            fall: p.accent2,
            ghost: Some(fade(p.accent2, 0.6)),
        },
        Look::Hogwarts => Inks {
            line: ink::ink(),
            cloud: ink::ribbon(),
            sun: turner::sand(),
            rim: ink::ink(),
            fall: ink::ink(),
            ghost: None,
        },
    }
}

/// The inks and the width of the line they are drawn with.
#[derive(Debug, Clone, Copy)]
struct Pen {
    inks: Inks,
    width: f32,
}

impl Pen {
    /// The look's pen for an icon of radius `r`: finer as it gets smaller.
    fn new(look: Look, r: f32) -> Self {
        Self {
            inks: inks(look),
            width: (r / 24.0).clamp(1.0, 2.0),
        }
    }
}

/// What falls from a cloud.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fall {
    Drizzle,
    Rain,
    Snow,
    Fog,
}

/// What a sky is made of, as shares of the icon's radius: the sun (or the
/// moon by night) and the cloud before it, each where it stands and how
/// big; a second cloud behind; what falls; a bolt of lightning.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Parts {
    body: Option<(f32, f32, f32)>,
    cloud: Option<(f32, f32, f32)>,
    back: bool,
    fall: Option<Fall>,
    bolt: bool,
}

const BARE: Parts = Parts {
    body: None,
    cloud: None,
    back: false,
    fall: None,
    bolt: false,
};
/// A cloud raised to leave room for what falls from it.
const HIGH: Option<(f32, f32, f32)> = Some((0.0, -0.2, 0.62));

fn parts(sky: Sky) -> Parts {
    match sky {
        Sky::Clear => Parts {
            body: Some((0.0, 0.0, 0.48)),
            ..BARE
        },
        Sky::Cloudy => Parts {
            body: Some((-0.3, -0.3, 0.4)),
            cloud: Some((0.1, 0.16, 0.6)),
            ..BARE
        },
        Sky::Overcast => Parts {
            cloud: Some((-0.08, 0.14, 0.62)),
            back: true,
            ..BARE
        },
        Sky::Fog => falling_from(Fall::Fog, false),
        Sky::Drizzle => falling_from(Fall::Drizzle, false),
        Sky::Rain => falling_from(Fall::Rain, true),
        Sky::Snow => falling_from(Fall::Snow, false),
        Sky::Storm => Parts {
            bolt: true,
            ..falling_from(Fall::Rain, true)
        },
    }
}

/// A high cloud, another behind it when `back`, and `fall` under them.
fn falling_from(fall: Fall, back: bool) -> Parts {
    Parts {
        cloud: HIGH,
        back,
        fall: Some(fall),
        ..BARE
    }
}

/// The sky drawn in a box of radius `r` around (`x`, `y`): the sun, or the
/// moon by night, clouds and what falls from them, inked as the look inks.
pub(super) fn sky_icon(gfx: &mut Gfx, (x, y, r): (f32, f32, f32), sky: Sky, night: bool) {
    let pen = Pen::new(gfx.look, r);
    let parts = parts(sky);
    if let Some((dx, dy, size)) = parts.body {
        body(gfx, pen, (x + dx * r, y + dy * r, size * r), night);
    }
    if let Some((dx, dy, size)) = parts.cloud {
        let (cx, cy, s) = (x + dx * r, y + dy * r, size * r);
        if parts.back {
            cloud(gfx, pen, (cx + 0.55 * s, cy - 0.5 * s, 0.7 * s), 0.55);
        }
        cloud(gfx, pen, (cx, cy, s), 1.0);
        below(gfx, pen, (cx, cy + 0.55 * s, s), parts);
    }
}

/// The sun, or the moon by night.
fn body(gfx: &mut Gfx, pen: Pen, at: (f32, f32, f32), night: bool) {
    if night {
        moon(gfx, pen, at);
    } else {
        sun(gfx, pen, at);
    }
}

/// Strokes a path, split first in cyberpunk's second colour.
fn ink_line(gfx: &mut Gfx, pen: Pen, path: &Path, colour: Color) {
    if let Some(ghost) = pen.inks.ghost {
        gfx.canvas.save();
        gfx.canvas.translate(-1.5, 0.0);
        gfx.stroke(path, ghost, pen.width);
        gfx.canvas.restore();
    }
    gfx.stroke(path, colour, pen.width);
}

/// The sun, its rays turning slowly.
fn sun(gfx: &mut Gfx, pen: Pen, (x, y, r): (f32, f32, f32)) {
    gfx.halo(x, y, r * 2.4, fade(pen.inks.rim, 0.25));
    let turn = gfx.t * 0.2;
    let mut rays = Path::new();
    for k in 0..8 {
        let (sin, cos) = (turn + k as f32 * TAU / 8.0).sin_cos();
        rays.move_to(x + cos * r * 1.3, y + sin * r * 1.3);
        rays.line_to(x + cos * r * 1.7, y + sin * r * 1.7);
    }
    ink_line(gfx, pen, &rays, pen.inks.rim);
    let mut disc = Path::new();
    disc.circle(x, y, r);
    gfx.fill(&disc, pen.inks.sun);
    ink_line(gfx, pen, &disc, pen.inks.rim);
}

/// A crescent moon.
fn moon(gfx: &mut Gfx, pen: Pen, (x, y, r): (f32, f32, f32)) {
    gfx.halo(x, y, r * 2.0, fade(pen.inks.rim, 0.18));
    let mut path = Path::new();
    for (index, (px, py)) in crescent(r).into_iter().enumerate() {
        if index == 0 {
            path.move_to(x + px, y + py);
        } else {
            path.line_to(x + px, y + py);
        }
    }
    path.close();
    gfx.fill(&path, pen.inks.sun);
    ink_line(gfx, pen, &path, pen.inks.rim);
}

/// The outline of a crescent of radius `r` around the origin: the disc less
/// the same disc moved up and to the right.
fn crescent(r: f32) -> Vec<(f32, f32)> {
    let d = 0.6 * r;
    let theta = (d / (2.0 * r)).acos();
    let (sin, cos) = (-PI / 4.0).sin_cos();
    let steps = 16;
    let share = |k: usize| k as f32 / steps as f32;
    let outer = (0..=steps).map(|k| {
        let a = theta + (TAU - 2.0 * theta) * share(k);
        (r * a.cos(), r * a.sin())
    });
    let inner = (0..=steps).map(|k| {
        let a = PI + theta - 2.0 * theta * share(k);
        (d + r * a.cos(), r * a.sin())
    });
    outer
        .chain(inner)
        .map(|(x, y)| (x * cos - y * sin, x * sin + y * cos))
        .collect()
}

/// A cloud's outline, in shares of its scale, from its bottom left corner.
const CLOUD: [[(f32, f32); 3]; 4] = [
    [(-1.3, 0.45), (-1.3, -0.15), (-0.8, -0.1)],
    [(-0.8, -0.65), (-0.1, -0.8), (0.1, -0.35)],
    [(0.3, -0.7), (1.05, -0.6), (0.9, -0.05)],
    [(1.3, 0.0), (1.3, 0.45), (0.9, 0.45)],
];

/// A cloud of scale `s` around (`x`, `y`), its line faded by `alpha`.
fn cloud(gfx: &mut Gfx, pen: Pen, (x, y, s): (f32, f32, f32), alpha: f32) {
    let mut path = Path::new();
    path.move_to(x - 0.9 * s, y + 0.45 * s);
    for [a, b, c] in CLOUD {
        path.bezier_to(
            x + a.0 * s,
            y + a.1 * s,
            x + b.0 * s,
            y + b.1 * s,
            x + c.0 * s,
            y + c.1 * s,
        );
    }
    path.close();
    gfx.fill(&path, pen.inks.cloud);
    ink_line(gfx, pen, &path, fade(pen.inks.line, alpha));
}

/// What falls from the cloud, and the lightning.
fn below(gfx: &mut Gfx, pen: Pen, at: (f32, f32, f32), parts: Parts) {
    if let Some(fall) = parts.fall {
        falling(gfx, pen, at, fall);
    }
    if parts.bolt {
        bolt(gfx, pen, at);
    }
}

/// How many drops fall, how long a streak is (none for flakes), how fast.
fn fall_style(fall: Fall) -> (usize, f32, f32) {
    match fall {
        Fall::Drizzle => (3, 0.14, 0.7),
        Fall::Rain => (4, 0.34, 1.3),
        Fall::Snow => (4, 0.0, 0.45),
        Fall::Fog => (3, 0.0, 0.2),
    }
}

/// Where each of `count` drops is under a cloud of scale `s` at `clock`
/// (seconds times their speed), and how far down it is, 0 to 1.
fn drops(count: usize, clock: f32, s: f32) -> Vec<(f32, f32, f32)> {
    (0..count)
        .map(|k| {
            let phase = (clock + k as f32 * 0.37).fract();
            let dx = (k as f32 - (count as f32 - 1.0) / 2.0) * 0.42 * s;
            (dx, phase * 0.62 * s, phase)
        })
        .collect()
}

/// Rain or drizzle in streaks, snow in flakes, fog in drifting lines.
fn falling(gfx: &mut Gfx, pen: Pen, (x, y, s): (f32, f32, f32), fall: Fall) {
    let (count, length, speed) = fall_style(fall);
    let clock = gfx.t * speed;
    if fall == Fall::Fog {
        fog(gfx, pen, (x, y, s), clock);
        return;
    }
    for (dx, dy, phase) in drops(count, clock, s) {
        let colour = fade(pen.inks.fall, 1.0 - phase * 0.8);
        let mut path = Path::new();
        if length > 0.0 {
            path.move_to(x + dx, y + dy);
            path.line_to(x + dx - length * s * 0.3, y + dy + length * s);
            ink_line(gfx, pen, &path, colour);
        } else {
            path.circle(x + dx, y + dy + 0.1 * s, 0.08 * s);
            gfx.fill(&path, colour);
        }
    }
}

/// Three lines of fog drifting under the cloud.
fn fog(gfx: &mut Gfx, pen: Pen, (x, y, s): (f32, f32, f32), clock: f32) {
    let mut path = Path::new();
    for k in 0..3 {
        let drift = (clock * TAU + k as f32 * 2.0).sin() * 0.15 * s;
        let half = (1.0 - k as f32 * 0.18) * s;
        let ly = y + 0.12 * s + k as f32 * 0.24 * s;
        path.move_to(x - half + drift, ly);
        path.line_to(x + half + drift, ly);
    }
    ink_line(gfx, pen, &path, fade(pen.inks.fall, 0.8));
}

/// A bolt of lightning, in the sun's colour.
const BOLT: [(f32, f32); 7] = [
    (0.12, -0.12),
    (-0.22, 0.4),
    (0.02, 0.4),
    (-0.14, 0.82),
    (0.3, 0.26),
    (0.06, 0.26),
    (0.22, -0.12),
];

fn bolt(gfx: &mut Gfx, pen: Pen, (x, y, s): (f32, f32, f32)) {
    let mut path = Path::new();
    path.move_to(x + BOLT[0].0 * s, y + BOLT[0].1 * s);
    for (bx, by) in &BOLT[1..] {
        path.line_to(x + bx * s, y + by * s);
    }
    path.close();
    gfx.fill(&path, pen.inks.sun);
    ink_line(gfx, pen, &path, pen.inks.rim);
}

// ---------- the weather ----------

/// How far the sun has gone across the sky, 0 at sunrise, 1 at sunset;
/// None while it is down, or without sunrise and sunset.
fn sun_share(weather: &Weather, now: Timestamp) -> Option<f32> {
    let (rise, set) = (weather.sunrise?, weather.sunset?);
    let k = now.secs_since(rise) / set.secs_since(rise).max(1.0);
    (0.0..=1.0).contains(&k).then_some(k as f32)
}

/// The point `k` of the way along the sun's arc, over the box's bottom edge.
fn arc_point((x, base, w, h): Box, k: f32) -> (f32, f32) {
    (x + w * k, base - h * (PI * k).sin())
}

/// "max 12° · min 6°", with what is known.
fn high_low(weather: &Weather) -> Option<String> {
    let high = weather
        .high
        .map(|c| format!("max {}", text::temperature(c)));
    let low = weather.low.map(|c| format!("min {}", text::temperature(c)));
    let both: Vec<String> = high.into_iter().chain(low).collect();
    (!both.is_empty()).then(|| both.join(" · "))
}

/// The chance of rain and the wind, as rows, with what is known.
fn weather_facts(weather: &Weather) -> Vec<(&'static str, String)> {
    let rain = weather
        .rain
        .map(|rain| ("Risque de pluie", format!("{rain:.0} %")));
    let wind = weather.wind.map(|wind| ("Vent", format!("{wind:.0} km/h")));
    rain.into_iter().chain(wind).collect()
}

/// The plates of the Map's board: rain, wind, highest and lowest; unlit
/// when unknown.
fn weather_plates(weather: &Weather) -> [(String, String, bool); 3] {
    let plate = |value: Option<String>, name: &str| {
        (
            value.clone().unwrap_or_else(|| "–".to_owned()),
            name.to_owned(),
            value.is_some(),
        )
    };
    let span = weather
        .high
        .zip(weather.low)
        .map(|(high, low)| format!("{} · {}", text::temperature(high), text::temperature(low)));
    [
        plate(weather.rain.map(|rain| format!("{rain:.0} %")), "pluie"),
        plate(weather.wind.map(|wind| format!("{wind:.0} km/h")), "vent"),
        plate(span, "max · min"),
    ]
}

/// How the temperature is written: large, in each look's figures.
fn temperature_type(look: Look) -> Type {
    match look {
        Look::Friday => Type::new(Face::Display, 50.0),
        Look::Cyberpunk => Type::new(Face::Bold, 76.0),
        Look::Hogwarts => Type::new(Face::Display, 64.0),
    }
}

/// The colour of a card's big figure: the text in friday, cyberpunk's
/// yellow, ink on the Map.
fn figure_colour(look: Look, palette: Palette) -> Color {
    match look {
        Look::Friday => palette.text,
        Look::Cyberpunk => palette.hot,
        Look::Hogwarts => palette.accent,
    }
}

/// The weather now: the sky drawn, the temperature, the highest and lowest,
/// the rain and the wind, and the sun's arc from its rising to its setting.
pub(super) fn weather_card(gfx: &mut Gfx, sheet: &Sheet, _: &mut Textures) {
    let Some(weather) = &sheet.snapshot.weather else {
        return;
    };
    let (left, top, now) = (sheet.left, sheet.top, sheet.view.now);
    let sky = weather.sky();
    gfx.kicker(
        left,
        top,
        &format!("Météo · {}", text::sky(sky)),
        sheet.kicker,
    );
    let night = weather.daylight(now) == Some(false);
    sky_icon(gfx, (left + 54.0, top + 82.0, 50.0), sky, night);
    temperature(gfx, left + 128.0, top + 24.0, weather);
    if gfx.hogwarts() {
        let plates = weather_plates(weather);
        scoreboard(gfx, left, top + 128.0, WIDTH, &plates, 3);
    } else {
        for (index, (label, value)) in weather_facts(weather).iter().enumerate() {
            let text_colour = gfx.palette.text;
            row(
                gfx,
                left,
                top + 136.0 + index as f32 * 32.0,
                label,
                value,
                text_colour,
            );
        }
    }
    day_arc(
        gfx,
        (left + 20.0, top + 290.0, WIDTH - 40.0, 66.0),
        weather,
        now,
    );
}

/// The temperature in big figures, and under it the highest and lowest; on
/// the Map, the sky in a word.
fn temperature(gfx: &mut Gfx, x: f32, y: f32, weather: &Weather) {
    let palette = gfx.palette;
    let style = temperature_type(gfx.look);
    let colour = figure_colour(gfx.look, palette);
    degrees(
        gfx,
        (x, y + style.size * 0.5),
        weather.temperature,
        style,
        colour,
    );
    let note = if gfx.hogwarts() {
        Some(capitalised(text::sky(weather.sky())))
    } else {
        high_low(weather)
    };
    gfx.text(
        x + 2.0,
        y + style.size * 0.95 + 8.0,
        note.as_deref().unwrap_or_default(),
        Type::new(Face::Italic, 18.0),
        palette.muted,
        Anchor::TOP_LEFT,
    );
}

/// The figures of a temperature and the gap and ring of its degree: the
/// faces' own degree is too large in some, missing in others.
fn degree_parts(gfx: &Gfx, celsius: f64, style: Type) -> (String, f32, f32, f32) {
    let figure = (celsius.round() as i64).to_string();
    let width = gfx.measure(&figure, style);
    (figure, width, style.size * 0.07, style.size * 0.09)
}

/// How wide `degrees` writes a temperature.
pub(super) fn degrees_width(gfx: &Gfx, celsius: f64, style: Type) -> f32 {
    let (_, width, gap, r) = degree_parts(gfx, celsius, style);
    width + gap + 2.0 * r
}

/// A temperature in figures, centred on `y`, as the look writes its titles.
pub(super) fn degrees(gfx: &mut Gfx, (x, y): (f32, f32), celsius: f64, style: Type, colour: Color) {
    let (figure, width, ..) = degree_parts(gfx, celsius, style);
    gfx.title(x, y, &figure, style, colour, Anchor::MIDDLE_LEFT);
    degree_ring(gfx, (x + width, y), style, colour);
}

/// The degree after figures that end at `x`, centred on `y`: a small ring at
/// the height of their tops.
pub(super) fn degree_ring(gfx: &mut Gfx, (x, y): (f32, f32), style: Type, colour: Color) {
    let (gap, r) = (style.size * 0.07, style.size * 0.09);
    let line = (style.size * 0.045).max(1.2);
    gfx.ring(
        x + gap + r,
        y - style.size * 0.32 + r,
        r,
        line,
        colour,
        &[],
        0.0,
        TAU,
    );
}

/// "Pluie" for "pluie".
fn capitalised(words: &str) -> String {
    let mut chars = words.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

/// The sun's arc over the horizon, the part it went along drawn in full,
/// the sun where it is (the moon over it by night), and the hours of
/// sunrise and sunset at its ends.
fn day_arc(gfx: &mut Gfx, (x, base, w, h): Box, weather: &Weather, now: Timestamp) {
    let pen = Pen::new(gfx.look, 12.0);
    let line = pen.inks.line;
    gfx.dashed_rule(x - 10.0, base, w + 20.0, fade(line, 0.5));
    let dashed = Paint::color(fade(line, 0.4))
        .with_line_width(1.0)
        .with_line_dash(&[3.0, 4.0]);
    gfx.canvas
        .stroke_path(&arc((x, base, w, h), 0.0, 1.0), &dashed);
    if let Some(k) = sun_share(weather, now) {
        ink_line(gfx, pen, &arc((x, base, w, h), 0.0, k), line);
        let (sx, sy) = arc_point((x, base, w, h), k);
        sun(gfx, pen, (sx, sy, 7.0));
    } else {
        moon(gfx, pen, (x + w / 2.0, base - h * 0.55, 9.0));
    }
    sun_hours(gfx, (x, base + 10.0, w), weather);
}

/// The arc from `from` to `to` of the way along.
fn arc(at: Box, from: f32, to: f32) -> Path {
    let mut path = Path::new();
    let (sx, sy) = arc_point(at, from);
    path.move_to(sx, sy);
    for step in 1..=32 {
        let (px, py) = arc_point(at, from + (to - from) * step as f32 / 32.0);
        path.line_to(px, py);
    }
    path
}

/// "lever 07:52" and "coucher 17:21" under the ends of the arc, the length
/// of the day between them.
fn sun_hours(gfx: &mut Gfx, (x, y, w): (f32, f32, f32), weather: &Weather) {
    let style = Type::new(Face::Italic, 14.0);
    let muted = gfx.palette.muted;
    let words = sun_words(weather);
    let anchors = [Anchor::TOP_LEFT, Anchor::TOP_CENTER, Anchor::TOP_RIGHT];
    for (index, (word, anchor)) in words.iter().zip(anchors).enumerate() {
        gfx.text(x + w * index as f32 / 2.0, y, word, style, muted, anchor);
    }
}

/// What is said under the arc: the sunrise, the day's length, the sunset.
fn sun_words(weather: &Weather) -> [String; 3] {
    let rise = weather
        .sunrise
        .map(|at| format!("lever {}", text::clock(at)));
    let set = weather
        .sunset
        .map(|at| format!("coucher {}", text::clock(at)));
    let day = weather
        .sunrise
        .zip(weather.sunset)
        .map(|(rise, set)| format!("{} de jour", text::hours(set.secs_since(rise))));
    [rise, day, set].map(Option::unwrap_or_default)
}

// ---------- tonight, and what comes next ----------

/// What the card lists: the episodes airing from an hour ago on, soonest
/// first, then Jellyfin's next up.
pub(super) fn lineup(snapshot: &Snapshot, now: Timestamp) -> Vec<&Episode> {
    let mut airing: Vec<&Episode> = snapshot
        .airing
        .iter()
        .filter(|episode| episode.at.is_some_and(|at| now.secs_since(at) < AIRED))
        .collect();
    airing.sort_by_key(|episode| episode.at);
    airing.extend(&snapshot.next_up);
    airing
}

/// "Ce soir" when an episode airs tonight, else "À suivre".
fn lineup_kicker(lineup: &[&Episode], now: Timestamp) -> &'static str {
    let tonight = lineup
        .iter()
        .filter_map(|episode| episode.at)
        .any(|at| text::day_word(at, now) == "ce soir");
    if tonight {
        "Ce soir · à suivre"
    } else {
        "À suivre"
    }
}

/// "S02E05 · Blisters and Bedrock".
fn episode_label(episode: &Episode) -> String {
    let number = text::episode(episode.season, episode.episode);
    match &episode.title {
        Some(title) => format!("{number} · {title}"),
        None => number,
    }
}

/// When an episode comes, as a row says it: "demain 21:00", or "à suivre"
/// for one already in the library.
fn when(episode: &Episode, now: Timestamp) -> String {
    episode
        .at
        .map_or_else(|| "à suivre".to_owned(), |at| text::airs(at, now))
}

/// The episodes to watch: the next one with its poster, when it airs in
/// big figures, its series and its title; the others in rows under it.
pub(super) fn tonight_card(gfx: &mut Gfx, sheet: &Sheet, textures: &mut Textures) {
    let now = sheet.view.now;
    let lineup = lineup(sheet.snapshot, now);
    let Some((first, rest)) = lineup.split_first() else {
        return;
    };
    let (left, top) = (sheet.left, sheet.top);
    gfx.kicker(left, top, lineup_kicker(&lineup, now), sheet.kicker);
    let (pw, ph, _) = POSTER;
    poster(gfx, textures, first.poster.as_ref(), left, top + 30.0);
    hero(
        gfx,
        sheet,
        first,
        (left + pw + 18.0, top + 30.0, WIDTH - pw - 18.0),
    );
    for (index, episode) in rest.iter().take(3).enumerate() {
        let y = top + 48.0 + ph + index as f32 * 31.0;
        let label = format!("{} · {}", episode.series, episode_label(episode));
        short_row(gfx, (left, y), &label, &when(episode, now));
    }
}

/// The next episode beside its poster: when, in big figures, its series and
/// its title.
fn hero(gfx: &mut Gfx, sheet: &Sheet, episode: &Episode, (x, y, w): (f32, f32, f32)) {
    let now = sheet.view.now;
    let day = episode.at.map_or_else(
        || "prochain épisode".to_owned(),
        |at| text::day_word(at, now),
    );
    when_label(gfx, x, y, &day);
    let mut y = y + 28.0;
    if let Some(at) = episode.at {
        let colour = figure_colour(gfx.look, gfx.palette);
        big(gfx, x, y - 4.0, &text::clock(at), colour);
        y += 58.0;
    }
    y += series_title(gfx, (x, y, w), &episode.series, sheet.age);
    let face = [Face::Body, Face::Italic][usize::from(gfx.hogwarts())];
    let muted = gfx.palette.muted;
    let style = Type::new(face, 16.0);
    gfx.paragraph(
        x,
        y + 4.0,
        &episode_label(episode),
        w,
        style,
        20.0,
        muted,
        2,
    );
}

/// When an episode airs, over its figures: a red stamp on the Map, small
/// capitals elsewhere.
fn when_label(gfx: &mut Gfx, x: f32, y: f32, day: &str) {
    if gfx.hogwarts() {
        gfx.stamp(x, y - 2.0, day, -3.0);
        return;
    }
    gfx.text(
        x,
        y + 2.0,
        &day.to_uppercase(),
        Type::new(Face::Strong, 13.0).spaced(2.5),
        gfx.palette.accent2,
        Anchor::TOP_LEFT,
    );
}

/// How a series' name is written beside its poster, and whether in capitals.
fn series_type(look: Look) -> (Type, bool) {
    match look {
        Look::Friday => (Type::new(Face::Display, 17.0), false),
        Look::Cyberpunk => (Type::new(Face::Bold, 24.0), true),
        Look::Hogwarts => (Type::new(Face::Display, 26.0), false),
    }
}

/// The series' name on two lines at most, written in as it appears;
/// returns the height used.
fn series_title(gfx: &mut Gfx, (x, y, w): (f32, f32, f32), series: &str, age: f32) -> f32 {
    let (style, capitals) = series_type(gfx.look);
    let name = if capitals {
        series.to_uppercase()
    } else {
        series.to_owned()
    };
    let colour = [gfx.palette.text, gfx.palette.accent][usize::from(gfx.hogwarts())];
    let glyphs = gfx.glyphs();
    let lines = gfx.wrap(&name, w, style, 2);
    let line = style.size * 1.25;
    for (index, text) in lines.iter().enumerate() {
        let shown = decode(text, age, 0.7, glyphs);
        gfx.title(
            x,
            y + index as f32 * line,
            &shown,
            style,
            colour,
            Anchor::TOP_LEFT,
        );
    }
    lines.len() as f32 * line
}

/// A row whose label is cut with … before it meets its value.
fn short_row(gfx: &mut Gfx, (x, y): (f32, f32), label: &str, value: &str) {
    let room = WIDTH - gfx.measure(value, Type::new(Face::Strong, 16.0)) - 18.0;
    let lines = gfx.wrap(label, room, Type::new(Face::Body, 16.0), 1);
    let colour = gfx.palette.accent2;
    let label = lines.first().map_or("", String::as_str);
    row(gfx, x, y, label, value, colour);
}

// ---------- the day ----------

/// The day's figures: how many, the board's word, and the Prophet's words
/// for one and for more.
fn tallies(today: &Today) -> [(u32, &'static str, &'static str, &'static str); 6] {
    [
        (
            today.imports,
            "AJOUTS",
            "nouveauté entrée à la bibliothèque",
            "nouveautés entrées à la bibliothèque",
        ),
        (
            today.downloads,
            "TÉLÉCHARGEMENTS",
            "téléchargement achevé",
            "téléchargements achevés",
        ),
        (
            today.plays,
            "LECTURES",
            "séance de projection",
            "séances de projection",
        ),
        (today.lights, "LUMIÈRES", "lampe allumée", "lampes allumées"),
        (
            today.prints,
            "IMPRESSIONS",
            "impression sortie de la cuve",
            "impressions sorties de la cuve",
        ),
        (
            today.questions,
            "QUESTIONS",
            "question venue du Mac",
            "questions venues du Mac",
        ),
    ]
}

/// The Prophet's headlines: each figure that is not zero, and its words.
fn headlines(today: &Today) -> Vec<(String, String)> {
    tallies(today)
        .into_iter()
        .filter(|(count, ..)| *count > 0)
        .map(|(count, _, one, many)| {
            let words = if count > 1 { many } else { one };
            (count.to_string(), words.to_owned())
        })
        .collect()
}

/// The Prophet's edition at `hour`.
fn edition(hour: u32) -> &'static str {
    match hour {
        0..12 => "Édition du matin",
        12..18 => "Édition de l'après-midi",
        _ => "Édition du soir",
    }
}

/// The day's recap: an airport's split-flap board, or the Prophet's edition
/// on the Map.
pub(super) fn today_card(gfx: &mut Gfx, sheet: &Sheet, _: &mut Textures) {
    let Some(today) = sheet.snapshot.today else {
        return;
    };
    if gfx.hogwarts() {
        prophet(gfx, sheet, &today);
    } else {
        board(gfx, sheet, &today);
    }
}

/// Flaps of a word on the board, of a figure, the width of a flap and the
/// height of a row.
const WORD_FLAPS: usize = 15;
const FIGURE_FLAPS: usize = 3;
const FLAP: (f32, f32, f32) = (20.0, 28.0, 22.0);
const FLAP_ROW: f32 = 37.0;
/// What the flaps show while they turn.
const FLAP_GLYPHS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

/// The character a flap shows `age` seconds into the card: letters turning
/// until its `delay`, then `target`. A blank stays blank.
fn flap_char(target: char, age: f32, delay: f32, seed: u64) -> char {
    if age >= delay || target == ' ' {
        return target;
    }
    let step = (age * 24.0) as u64;
    FLAP_GLYPHS[(hash(seed, step) % FLAP_GLYPHS.len() as u64) as usize] as char
}

/// When the flap of `row` and `column` stops turning.
fn flap_delay(row: usize, column: usize) -> f32 {
    0.3 + row as f32 * 0.16 + column as f32 * 0.035
}

/// The flaps' colours: the flap, the figure on it, unlit, and the split
/// across it.
fn flap_inks(look: Look, palette: Palette) -> (Color, Color, Color, Color) {
    let figure = figure_colour(look, palette);
    let split = [palette.bg, fade(palette.accent, 0.6)][usize::from(look == Look::Cyberpunk)];
    (
        mix(palette.bg, palette.accent, 0.12),
        figure,
        fade(palette.muted, 0.6),
        split,
    )
}

/// The split-flap board: a row a figure, lit when it is not zero.
fn board(gfx: &mut Gfx, sheet: &Sheet, today: &Today) {
    let (left, top) = (sheet.left, sheet.top);
    gfx.kicker(left, top, "Aujourd'hui · depuis minuit", sheet.kicker);
    let heads = Type::new(Face::Strong, 10.5).spaced(2.0);
    let muted = gfx.palette.muted;
    gfx.text(left, top + 30.0, "ACTIVITÉ", heads, muted, Anchor::TOP_LEFT);
    gfx.text(
        left + WIDTH,
        top + 30.0,
        "NOMBRE",
        heads,
        muted,
        Anchor::TOP_RIGHT,
    );
    for (index, (count, word, ..)) in tallies(today).iter().enumerate() {
        let y = top + 48.0 + index as f32 * FLAP_ROW;
        let figure = format!("{count:>3}");
        let lit = *count > 0;
        flaps(gfx, (left, y), (index, 0), word, lit, sheet.age);
        let fx = left + WIDTH - FIGURE_FLAPS as f32 * FLAP.2 + 2.0;
        flaps(gfx, (fx, y), (index, WORD_FLAPS), &figure, lit, sheet.age);
    }
    let note = format!("mis à jour à {}", text::clock(sheet.view.now));
    let style = Type::new(Face::Body, 13.0);
    gfx.text(left, top + 276.0, &note, style, muted, Anchor::TOP_LEFT);
}

/// A word on its flaps, from the flap (`row`, `column`) of the board; the
/// flaps it does not fill stay blank.
fn flaps(
    gfx: &mut Gfx,
    (x, y): (f32, f32),
    (row, column): (usize, usize),
    word: &str,
    lit: bool,
    age: f32,
) {
    let (fw, fh, step) = FLAP;
    let count = if column == 0 {
        WORD_FLAPS
    } else {
        FIGURE_FLAPS
    };
    let (tile, figure, unlit, split) = flap_inks(gfx.look, gfx.palette);
    let mut tiles = Path::new();
    let mut splits = Path::new();
    for k in 0..count {
        tiles.rect(x + k as f32 * step, y, fw, fh);
        splits.rect(x + k as f32 * step, y + fh / 2.0 - 0.5, fw, 1.0);
    }
    gfx.fill(&tiles, tile);
    let style = Type::new(Face::Bold, 18.0);
    let colour = if lit { figure } else { unlit };
    for (k, target) in word.chars().enumerate() {
        let seed = (row * 32 + column + k) as u64;
        let shown = flap_char(target, age, flap_delay(row, column + k), seed);
        let cx = x + k as f32 * step + fw / 2.0;
        gfx.text(
            cx,
            y + fh / 2.0 + 1.0,
            &shown.to_string(),
            style,
            colour,
            Anchor::MIDDLE_CENTER,
        );
    }
    gfx.fill(&splits, split);
}

/// The Prophet's evening edition: its masthead, and the day's figures as
/// headlines in two columns.
fn prophet(gfx: &mut Gfx, sheet: &Sheet, today: &Today) {
    let (left, top) = (sheet.left, sheet.top);
    masthead(gfx, left, top, sheet.view.now);
    let news = headlines(today);
    let half = WIDTH / 2.0;
    for (index, (figure, words)) in news.iter().take(6).enumerate() {
        let x = left + (index % 2) as f32 * (half + 8.0);
        let y = top + 76.0 + (index / 2) as f32 * 64.0;
        headline(gfx, (x, y, half - 16.0), figure, words);
    }
    let mut column = Path::new();
    column.move_to(left + half, top + 74.0);
    column.line_to(
        left + half,
        top + 76.0 + news.len().min(6).div_ceil(2) as f32 * 64.0 - 14.0,
    );
    gfx.stroke(&column, fade(ink::news_ink(), 0.4), 1.0);
    let style = Type::new(Face::Italic, 13.0);
    let line = "« Tout ce qui s'est passé au château depuis minuit. »";
    let muted = gfx.palette.muted;
    gfx.text(
        left + half,
        top + 280.0,
        line,
        style,
        muted,
        Anchor::TOP_CENTER,
    );
}

/// The Prophet's name, its rules, and the edition with the date.
fn masthead(gfx: &mut Gfx, left: f32, top: f32, now: Timestamp) {
    let news = ink::news_ink();
    let centre = left + WIDTH / 2.0;
    let title = Type::new(Face::Display, 30.0);
    gfx.text(
        centre,
        top - 4.0,
        "La Gazette du sorcier",
        title,
        news,
        Anchor::TOP_CENTER,
    );
    gfx.rect(left, top + 36.0, WIDTH, 2.0, news);
    gfx.rect(left, top + 40.0, WIDTH, 0.8, news);
    let hour = chrono::Timelike::hour(&text::local(now));
    let line = format!("{} · {}", edition(hour), text::date(now)).to_uppercase();
    let style = Type::new(Face::Body, 11.0).spaced(2.0);
    gfx.text(
        centre,
        top + 47.0,
        &line,
        style,
        ink::red(),
        Anchor::TOP_CENTER,
    );
    gfx.rect(left, top + 64.0, WIDTH, 0.8, news);
}

/// A headline: its figure large, its words beside it.
fn headline(gfx: &mut Gfx, (x, y, w): (f32, f32, f32), figure: &str, words: &str) {
    let news = ink::news_ink();
    let fw = gfx.text(
        x,
        y - 4.0,
        figure,
        Type::new(Face::Display, 38.0),
        news,
        Anchor::TOP_LEFT,
    );
    let style = Type::new(Face::Italic, 15.0);
    let colour = gfx.palette.text;
    let room = w - fw - 10.0;
    // one line sits level with the figure, two around its middle
    let lines = gfx.wrap(words, room, style, 2).len() as f32;
    gfx.paragraph(
        x + fw + 10.0,
        y + 17.0 - lines * 9.0,
        words,
        room,
        style,
        18.0,
        colour,
        2,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use errol_domain::entities::PictureRef;

    fn weather() -> Weather {
        Weather {
            temperature: 9.4,
            code: 61,
            wind: Some(18.0),
            high: Some(12.0),
            low: Some(6.0),
            rain: Some(70.0),
            sunrise: Some(Timestamp::from_secs(1000)),
            sunset: Some(Timestamp::from_secs(2000)),
        }
    }

    fn episode(series: &str, at: Option<i64>) -> Episode {
        Episode {
            series: series.into(),
            season: Some(2),
            episode: Some(5),
            title: Some("Blisters and Bedrock".into()),
            poster: Some(PictureRef::new("/p.jpg")),
            at: at.map(Timestamp::from_secs),
        }
    }

    #[test]
    fn every_sky_has_its_parts() {
        assert_eq!(parts(Sky::Clear).cloud, None);
        assert!(parts(Sky::Cloudy).body.is_some() && parts(Sky::Cloudy).cloud.is_some());
        assert!(parts(Sky::Overcast).back);
        assert_eq!(parts(Sky::Fog).fall, Some(Fall::Fog));
        assert_eq!(parts(Sky::Drizzle).fall, Some(Fall::Drizzle));
        assert_eq!(parts(Sky::Snow).fall, Some(Fall::Snow));
        let rain = parts(Sky::Rain);
        assert!(rain.back && !rain.bolt && rain.body.is_none());
        let storm = parts(Sky::Storm);
        assert!(storm.bolt && storm.fall == Some(Fall::Rain));
    }

    #[test]
    fn each_look_inks_its_sky() {
        assert!(inks(Look::Cyberpunk).ghost.is_some());
        assert!(inks(Look::Friday).ghost.is_none());
        assert_eq!(inks(Look::Hogwarts).line, ink::ink());
        assert!((Pen::new(Look::Friday, 8.0).width - 1.0).abs() < 1e-6);
        assert!((Pen::new(Look::Friday, 100.0).width - 2.0).abs() < 1e-6);
    }

    #[test]
    fn the_crescent_closes_on_itself() {
        let points = crescent(10.0);
        let (first, last) = (points[0], points[points.len() - 1]);
        assert!((first.0 - last.0).abs() < 1e-3 && (first.1 - last.1).abs() < 1e-3);
        assert!(points.iter().all(|(x, y)| x.hypot(*y) <= 10.0 + 1e-3));
    }

    #[test]
    fn drops_fall_in_columns_and_take_their_turn() {
        for fall in [Fall::Drizzle, Fall::Rain, Fall::Snow, Fall::Fog] {
            assert!(fall_style(fall).0 >= 3);
        }
        let drops = drops(3, 0.5, 10.0);
        assert_eq!(drops.len(), 3);
        assert!((drops[1].0).abs() < 1e-6);
        assert!(
            drops
                .iter()
                .all(|(_, dy, phase)| (0.0..1.0).contains(phase) && *dy >= 0.0)
        );
    }

    #[test]
    fn the_sun_goes_across_the_day_only() {
        let weather = weather();
        assert_eq!(sun_share(&weather, Timestamp::from_secs(1500)), Some(0.5));
        assert_eq!(sun_share(&weather, Timestamp::from_secs(2500)), None);
        let unknown = Weather {
            sunset: None,
            ..weather
        };
        assert_eq!(sun_share(&unknown, Timestamp::from_secs(1500)), None);
        let (x, y) = arc_point((0.0, 100.0, 200.0, 50.0), 0.5);
        assert!((x - 100.0).abs() < 1e-4 && (y - 50.0).abs() < 1e-4);
    }

    #[test]
    fn the_weather_says_what_it_knows() {
        let full = weather();
        assert_eq!(high_low(&full).as_deref(), Some("max 12° · min 6°"));
        assert_eq!(
            weather_facts(&full),
            [
                ("Risque de pluie", "70 %".to_owned()),
                ("Vent", "18 km/h".to_owned())
            ]
        );
        assert_eq!(
            weather_plates(&full)[2],
            ("12° · 6°".into(), "max · min".into(), true)
        );
        let bare = Weather {
            wind: None,
            high: None,
            low: None,
            rain: None,
            sunrise: None,
            sunset: None,
            ..full
        };
        assert_eq!(high_low(&bare), None);
        assert_eq!(weather_facts(&bare), Vec::new());
        assert_eq!(
            weather_plates(&bare)[0],
            ("–".into(), "pluie".into(), false)
        );
        assert_eq!(sun_words(&bare), ["", "", ""].map(str::to_owned));
        assert!(sun_words(&full)[1].ends_with("de jour"));
        assert_eq!(capitalised("pluie"), "Pluie");
        assert_eq!(capitalised(""), "");
    }

    #[test]
    fn each_look_writes_its_figures() {
        let palette = Look::Cyberpunk.palette();
        assert_eq!(figure_colour(Look::Cyberpunk, palette), palette.hot);
        assert!(temperature_type(Look::Cyberpunk).size > temperature_type(Look::Friday).size);
        assert_eq!(temperature_type(Look::Hogwarts).face, Face::Display);
        assert!(series_type(Look::Cyberpunk).1);
        assert!(!series_type(Look::Friday).1 && !series_type(Look::Hogwarts).1);
        let hogwarts = Look::Hogwarts.palette();
        assert_eq!(figure_colour(Look::Hogwarts, hogwarts), hogwarts.accent);
        assert_eq!(
            figure_colour(Look::Friday, Look::Friday.palette()),
            Look::Friday.palette().text
        );
        assert_eq!(
            flap_inks(Look::Friday, Look::Friday.palette()).3,
            Look::Friday.palette().bg
        );
    }

    #[test]
    fn the_lineup_puts_what_airs_first_and_drops_what_aired() {
        let now = Timestamp::from_secs(10_000);
        let snapshot = Snapshot {
            airing: vec![
                episode("Later", Some(20_000)),
                episode("Gone", Some(5_000)),
                episode("Soon", Some(12_000)),
            ],
            next_up: vec![episode("Next", None)],
            ..Snapshot::default()
        };
        let names: Vec<&str> = lineup(&snapshot, now)
            .iter()
            .map(|e| e.series.as_str())
            .collect();
        assert_eq!(names, ["Soon", "Later", "Next"]);
        assert_eq!(lineup(&Snapshot::default(), now), Vec::<&Episode>::new());
    }

    #[test]
    fn an_episode_says_its_number_and_when() {
        let next = episode("Arcane", None);
        assert_eq!(episode_label(&next), "S02E05 · Blisters and Bedrock");
        let bare = Episode {
            title: None,
            ..next.clone()
        };
        assert_eq!(episode_label(&bare), "S02E05");
        assert_eq!(when(&next, Timestamp(0)), "à suivre");
        let now = Timestamp::from_secs(0);
        let soon = episode("Arcane", Some(60));
        assert_eq!(when(&soon, now), text::airs(Timestamp::from_secs(60), now));
        assert_eq!(lineup_kicker(&[&next], now), "À suivre");
        let late = text::local(now)
            .date_naive()
            .and_hms_opt(21, 0, 0)
            .and_then(|at| at.and_local_timezone(chrono::Local).single())
            .map(|at| at.timestamp())
            .unwrap();
        let tonight = episode("Arcane", Some(late));
        assert_eq!(lineup_kicker(&[&tonight], now), "Ce soir · à suivre");
    }

    #[test]
    fn the_day_tells_what_happened() {
        let today = Today {
            imports: 3,
            lights: 1,
            ..Today::default()
        };
        assert_eq!(tallies(&today)[0].1, "AJOUTS");
        assert_eq!(
            headlines(&today),
            [
                (
                    "3".to_owned(),
                    "nouveautés entrées à la bibliothèque".to_owned()
                ),
                ("1".to_owned(), "lampe allumée".to_owned()),
            ]
        );
        assert_eq!(edition(8), "Édition du matin");
        assert_eq!(edition(14), "Édition de l'après-midi");
        assert_eq!(edition(21), "Édition du soir");
    }

    #[test]
    fn the_flaps_turn_then_settle() {
        assert_eq!(flap_char('A', 2.0, 1.0, 7), 'A');
        assert_eq!(flap_char(' ', 0.0, 1.0, 7), ' ');
        let turning = flap_char('A', 0.1, 1.0, 7);
        assert!(FLAP_GLYPHS.contains(&(turning as u8)));
        assert!(flap_delay(1, 0) > flap_delay(0, 0));
        assert!(flap_delay(0, 1) > flap_delay(0, 0));
    }
}
