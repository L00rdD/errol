//! The bar at the top: the AI's name, the mode, three status chips, the
//! weather, the time.

use errol_domain::entities::{Snapshot, Timestamp, Weather};
use errol_domain::strategies::Scene;
use errol_domain::usecases::View;

use super::{cards, text};
use crate::look::{Look, fade};
use crate::render::{Anchor, Face, Gfx, Type};

const BRAND_FRIDAY: &str = "F.R.I.D.A.Y.";
const BRAND_FRIDAY_TYPE: Type = Type::new(Face::Display, 13.0).spaced(3.9);
const BRAND_CYBER: &str = "NETWATCH//ERROL";
const BRAND_CYBER_TYPE: Type = Type::new(Face::Bold, 19.0).spaced(1.5);
const BRAND_HOGWARTS: &str = "La Carte du Maraudeur";
const BRAND_HOGWARTS_TYPE: Type = Type::new(Face::Italic, 17.0).spaced(0.5);

/// Where the bar's text sits.
const MIDDLE: f32 = 19.0;
const CHIP_LABEL: Type = Type::new(Face::Strong, 10.5).spaced(1.4);
const CHIP_VALUE: Type = Type::new(Face::Bold, 11.5);
const WEATHER: Type = Type::new(Face::Display, 12.5).spaced(0.5);
/// Where the chips end, left of the clock.
const CHIPS_END: f32 = 726.0;

/// A status chip: a label, its value, a light that is on when all is well,
/// and the play triangle of a projector that is on.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Chip {
    label: &'static str,
    value: String,
    good: bool,
    play: bool,
}

pub fn draw(gfx: &mut Gfx, view: &View) {
    draw_brand(gfx);
    let (brand, style) = brand(gfx.look);
    let brand_end = 16.0 + gfx.measure(brand, style) + 12.0;
    draw_mode(gfx, view, brand_end);
    gfx.text(
        784.0,
        MIDDLE,
        &text::clock(view.now),
        Type::new(Face::Display, 13.0).spaced(1.0),
        gfx.palette.text,
        Anchor::MIDDLE_RIGHT,
    );
    let Some(snapshot) = &view.snapshot else {
        return;
    };
    let mut x = CHIPS_END;
    if let Some(weather) = &snapshot.weather {
        x = draw_weather(gfx, x - 8.0, weather, view.now) - 4.0;
    }
    for chip in chips(snapshot).iter().rev() {
        let width = chip_width(gfx, chip);
        x -= width + 8.0;
        draw_chip(gfx, x, width, chip);
    }
}

/// The AI's name and how it is written in each look.
fn brand(look: Look) -> (&'static str, Type) {
    match look {
        Look::Friday => (BRAND_FRIDAY, BRAND_FRIDAY_TYPE),
        Look::Cyberpunk => (BRAND_CYBER, BRAND_CYBER_TYPE),
        Look::Hogwarts => (BRAND_HOGWARTS, BRAND_HOGWARTS_TYPE),
    }
}

/// The bar's background and the AI's name.
fn draw_brand(gfx: &mut Gfx) {
    let palette = gfx.palette;
    let (brand, style) = brand(gfx.look);
    match gfx.look {
        Look::Cyberpunk => {
            gfx.gradient_rect(
                0.0,
                0.0,
                800.0,
                38.0,
                fade(palette.accent, 0.16),
                fade(palette.accent, 0.0),
            );
            gfx.rect(0.0, 36.0, 800.0, 2.0, palette.accent);
            gfx.split_text(
                16.0,
                MIDDLE,
                brand,
                style,
                palette.hot,
                Anchor::MIDDLE_LEFT,
                0.0,
            );
        }
        Look::Hogwarts => {
            // the Map's title in its italic hand, a dashed rule under the line
            gfx.dashed_rule(16.0, 36.5, 768.0, fade(palette.accent, 0.55));
            gfx.text(
                16.0,
                MIDDLE,
                brand,
                style,
                palette.accent,
                Anchor::MIDDLE_LEFT,
            );
        }
        Look::Friday => {
            gfx.vertical_gradient(0.0, 0.0, 800.0, 38.0, palette.bg, fade(palette.bg, 0.0));
            gfx.rect(0.0, 37.0, 800.0, 1.0, fade(palette.accent, 0.12));
            gfx.glow_text(
                16.0,
                MIDDLE,
                brand,
                style,
                palette.accent,
                Anchor::MIDDLE_LEFT,
                1.0,
            );
        }
    }
}

/// What the screen is doing, in each look's words.
fn mode(scene: &Scene, look: Look) -> &'static str {
    match (scene, look) {
        (Scene::Idle, Look::Friday) => "VEILLE",
        (Scene::Idle, Look::Cyberpunk) => "STANDBY",
        (Scene::Idle, Look::Hogwarts) => "RIEN À SIGNALER",
        (Scene::Event { .. }, Look::Friday) => "ALERTE",
        (Scene::Event { .. }, Look::Cyberpunk) => "INCOMING",
        (Scene::Event { .. }, Look::Hogwarts) => "MÉFAIT EN COURS",
        (Scene::Cinema(_), Look::Friday) => "PROJECTION",
        (Scene::Cinema(_), Look::Cyberpunk) => "BRAINDANCE",
        (Scene::Cinema(_), Look::Hogwarts) => "PENSINE",
    }
}

/// The blinking dot (red while the feed has a warning) and the mode.
fn draw_mode(gfx: &mut Gfx, view: &View, brand_end: f32) {
    let palette = gfx.palette;
    let blink = 0.35 + 0.65 * (gfx.t * std::f32::consts::PI).sin().abs();
    let dot = if view.warning.is_some() {
        palette.hot
    } else {
        palette.ok
    };
    gfx.halo(brand_end + 3.0, MIDDLE, 7.0, fade(dot, 0.5 * blink));
    gfx.disc(brand_end + 3.0, MIDDLE, 3.0, fade(dot, blink));
    gfx.text(
        brand_end + 12.0,
        19.5,
        mode(&view.scene, gfx.look),
        Type::new(Face::Strong, 11.0).spaced(1.8),
        palette.muted,
        Anchor::MIDDLE_LEFT,
    );
}

/// The weather by the clock, its right edge at `right`: the sky's small
/// glyph and the temperature. Returns where it starts.
fn draw_weather(gfx: &mut Gfx, right: f32, weather: &Weather, now: Timestamp) -> f32 {
    let width = cards::degrees_width(gfx, weather.temperature, WEATHER);
    let colour = gfx.palette.text;
    let figure = (weather.temperature.round() as i64).to_string();
    let y = MIDDLE + 0.5;
    let drawn = gfx.text(
        right - width,
        y,
        &figure,
        WEATHER,
        colour,
        Anchor::MIDDLE_LEFT,
    );
    cards::degree_ring(gfx, (right - width + drawn, y), WEATHER, colour);
    let glyph = right - width - 14.0;
    let night = weather.daylight(now) == Some(false);
    cards::sky_icon(gfx, (glyph, MIDDLE, 10.0), weather.sky(), night);
    glyph - 12.0
}

/// The three chips, left to right: services up, downloads, lamps on.
fn chips(snapshot: &Snapshot) -> [Chip; 3] {
    let services = &snapshot.services;
    let lamps = snapshot
        .house
        .as_ref()
        .map(|house| house.outlets.iter().filter(|outlet| outlet.on).count());
    let projecting = snapshot
        .house
        .as_ref()
        .is_some_and(|house| house.projectors.iter().any(|projector| projector.on));
    [
        Chip {
            label: "SERVICES",
            value: if services.total > 0 {
                format!("{}/{}", services.up, services.total)
            } else {
                "–".into()
            },
            good: services.total > 0 && services.up == services.total,
            play: false,
        },
        Chip {
            label: "FLUX",
            value: snapshot.downloads.len().to_string(),
            good: !snapshot.downloads.is_empty(),
            play: false,
        },
        Chip {
            label: "MAISON",
            value: lamps.map_or("–".into(), |lamps| lamps.to_string()),
            good: lamps.is_some(),
            // the projector, when on, is a small play triangle after the lamps
            play: projecting,
        },
    ]
}

fn chip_width(gfx: &mut Gfx, chip: &Chip) -> f32 {
    22.0 + gfx.measure(chip.label, CHIP_LABEL)
        + 6.0
        + gfx.measure(&chip.value, CHIP_VALUE)
        + 10.0
        + if chip.play { 12.0 } else { 0.0 }
}

fn draw_chip(gfx: &mut Gfx, x: f32, width: f32, chip: &Chip) {
    let palette = gfx.palette;
    chip_frame(gfx, x, width);
    let light = if chip.good { palette.ok } else { palette.muted };
    gfx.rect(x + 9.0, 16.5, 5.0, 5.0, light);
    let label_width = gfx.text(
        x + 20.0,
        19.5,
        chip.label,
        CHIP_LABEL,
        palette.muted,
        Anchor::MIDDLE_LEFT,
    );
    let value_width = gfx.text(
        x + 26.0 + label_width,
        19.5,
        &chip.value,
        CHIP_VALUE,
        palette.text,
        Anchor::MIDDLE_LEFT,
    );
    if chip.play {
        let px = x + 32.0 + label_width + value_width;
        let mut triangle = femtovg::Path::new();
        triangle.move_to(px, 15.5);
        triangle.line_to(px + 7.0, 19.5);
        triangle.line_to(px, 23.5);
        triangle.close();
        gfx.fill(&triangle, palette.accent2);
    }
}

/// A chip's outline: a slanted plate in cyberpunk, a thin box elsewhere.
fn chip_frame(gfx: &mut Gfx, x: f32, width: f32) {
    let palette = gfx.palette;
    if gfx.cyber() {
        let mut path = femtovg::Path::new();
        path.move_to(x + 6.0, 10.0);
        path.line_to(x + width, 10.0);
        path.line_to(x + width - 6.0, 28.0);
        path.line_to(x, 28.0);
        path.close();
        gfx.fill(&path, fade(palette.accent, 0.12));
    } else {
        gfx.rect_line(x, 10.0, width, 18.0, fade(palette.accent, 0.14), 1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use errol_domain::entities::{Download, House, Outlet, Projector, Services};

    #[test]
    fn each_look_names_the_mode_its_way() {
        let event = Scene::Event {
            event: errol_domain::entities::Event {
                id: errol_domain::entities::EventId::new("e"),
                at: errol_domain::entities::Timestamp::from_secs(0),
                kind: errol_domain::entities::EventKind::Play {
                    title: "Dune".into(),
                },
            },
            started: errol_domain::entities::Timestamp::from_secs(0),
            ends: errol_domain::entities::Timestamp::from_secs(60),
        };
        let cinema = Scene::Cinema(errol_domain::entities::Playing {
            id: "p".into(),
            kind: errol_domain::entities::MediaKind::Movie,
            title: "Dune".into(),
            series: None,
            season: None,
            episode: None,
            year: None,
            overview: String::new(),
            genres: Vec::new(),
            runtime: 0.0,
            position: 0.0,
            paused: false,
            device: String::new(),
            client: String::new(),
            user: String::new(),
            poster: None,
            backdrop: None,
        });
        let modes = |scene: &Scene| {
            [Look::Friday, Look::Cyberpunk, Look::Hogwarts].map(|look| mode(scene, look))
        };
        assert_eq!(
            modes(&Scene::Idle),
            ["VEILLE", "STANDBY", "RIEN À SIGNALER"]
        );
        assert_eq!(modes(&event), ["ALERTE", "INCOMING", "MÉFAIT EN COURS"]);
        assert_eq!(modes(&cinema), ["PROJECTION", "BRAINDANCE", "PENSINE"]);
        assert_eq!(brand(Look::Cyberpunk).0, "NETWATCH//ERROL");
        assert_eq!(brand(Look::Hogwarts).0, "La Carte du Maraudeur");
        assert_eq!(brand(Look::Friday).0, "F.R.I.D.A.Y.");
    }

    #[test]
    fn the_chips_say_what_is_up() {
        let empty = chips(&Snapshot::default());
        assert_eq!(
            empty.map(|chip| (chip.value, chip.good, chip.play)),
            [
                ("–".to_owned(), false, false),
                ("0".to_owned(), false, false),
                ("–".to_owned(), false, false)
            ]
        );
        let snapshot = Snapshot {
            services: Services {
                up: 18,
                total: 18,
                down: vec![],
            },
            downloads: vec![Download {
                name: "Dune".into(),
                percent: 1.0,
                eta: None,
            }],
            house: Some(House {
                outlets: vec![
                    Outlet {
                        id: "a".into(),
                        name: "A".into(),
                        on: true,
                        next: None,
                    },
                    Outlet {
                        id: "b".into(),
                        name: "B".into(),
                        on: false,
                        next: None,
                    },
                ],
                covers: Vec::new(),
                projectors: vec![Projector {
                    id: "p".into(),
                    name: "P".into(),
                    on: true,
                    app: None,
                    closes_at: None,
                }],
            }),
            ..Snapshot::default()
        };
        let full = chips(&snapshot);
        assert_eq!(
            full.map(|chip| (chip.value, chip.good, chip.play)),
            [
                ("18/18".to_owned(), true, false),
                ("1".to_owned(), true, false),
                ("1".to_owned(), true, true)
            ]
        );
    }
}
