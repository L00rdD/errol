//! An event takes the screen: a picture of what happened on the left (a lamp,
//! the projection screen, the projector's beam, a poster, a check, a chapter
//! number), what it is on the right, and a bar draining until the next scene.

use std::f32::consts::TAU;

use errol_domain::entities::{Event, EventKind};
use errol_domain::usecases::View;
use femtovg::{Color, Paint, Path};

use super::{BOTTOM, TOP, text};
use crate::anim::{decode, ease_back, ease_out, progress};
use crate::look::{fade, rgb};
use crate::render::{Anchor, Face, Gfx, Textures, Type};

const CX: f32 = 176.0;
const CY: f32 = f32::midpoint(TOP, BOTTOM);
const TEXT_X: f32 = 352.0;
const TEXT_W: f32 = 420.0;

pub fn draw(
    gfx: &mut Gfx,
    view: &View,
    event: &Event,
    age: f32,
    length: f32,
    textures: &mut Textures,
) {
    let palette = gfx.palette;
    // the picture pops in, overshooting a little
    let pop = 0.4 + 0.6 * ease_back(progress(age, 0.9));
    gfx.canvas.save();
    gfx.canvas.translate(CX, CY);
    gfx.canvas.scale(pop, pop);
    gfx.canvas.translate(-CX, -CY);
    let (kicker, title, sub, tags) = visual(gfx, view, event, age, textures);
    gfx.canvas.restore();

    // the words slide in one after the other
    let slide = |delay: f32| 30.0 * (1.0 - ease_out(progress(age - delay, 0.7)));
    let glyphs = gfx.glyphs();
    let kicker_colour = if gfx.cyber() {
        palette.accent2
    } else {
        palette.accent
    };
    let title_style = if gfx.cyber() {
        Type::new(Face::Bold, 38.0)
    } else {
        Type::new(Face::Display, 28.0)
    };
    let title = if gfx.cyber() {
        title.to_uppercase()
    } else {
        title
    };
    let lines = gfx.wrap(&title, TEXT_W, title_style, 3);
    let line_height = title_style.size * 1.22;
    let block = 26.0
        + lines.len() as f32 * line_height
        + 8.0
        + if sub.is_empty() { 0.0 } else { 30.0 }
        + 36.0;
    let mut y = CY - block / 2.0;
    gfx.kicker(TEXT_X + slide(0.0), y, &kicker, kicker_colour);
    y += 26.0;
    for (index, line) in lines.iter().enumerate() {
        let shown = decode(line, age - 0.15, 1.1, glyphs);
        let colour = if gfx.cyber() {
            palette.hot
        } else if gfx.hogwarts() {
            palette.accent
        } else {
            palette.text
        };
        gfx.title(
            TEXT_X + slide(0.15),
            y + index as f32 * line_height,
            &shown,
            title_style,
            colour,
            Anchor::TOP_LEFT,
        );
    }
    y += lines.len() as f32 * line_height + 8.0;
    if !sub.is_empty() {
        gfx.text(
            TEXT_X + slide(0.3),
            y,
            &sub,
            Type::new(Face::Italic, 20.0),
            palette.muted,
            Anchor::TOP_LEFT,
        );
        y += 30.0;
    }
    let mut x = TEXT_X + slide(0.45);
    for tag in tags.iter().chain(std::iter::once(&text::clock(event.at))) {
        x += gfx.tag(x, y + 6.0, tag) + 8.0;
    }

    // the time left to this scene
    let left = 1.0 - progress(age, length);
    gfx.rect(26.0, BOTTOM - 10.0, 748.0, 2.0, fade(palette.accent, 0.1));
    gfx.rect(26.0, BOTTOM - 10.0, 748.0 * left, 2.0, palette.accent);
}

/// Draws the picture of the event; returns (kicker, title, subtitle, tags).
fn visual(
    gfx: &mut Gfx,
    view: &View,
    event: &Event,
    age: f32,
    textures: &mut Textures,
) -> (String, String, String, Vec<String>) {
    match &event.kind {
        EventKind::Light { name, on } => {
            halos(gfx, age);
            bulb(gfx, *on);
            (
                "Maison · éclairage".into(),
                name.clone(),
                if *on {
                    "vient de s'allumer".into()
                } else {
                    "vient de s'éteindre".into()
                },
                vec![],
            )
        }
        EventKind::Cover { id, name, down } => {
            let cover = view
                .snapshot
                .as_ref()
                .and_then(|s| s.house.as_ref())
                .and_then(|house| {
                    house
                        .covers
                        .iter()
                        .find(|c| &c.id == id)
                        .or(house.covers.first())
                });
            let (position, moving) = cover.map_or((0.0, false), |c| (c.position as f32, c.moving));
            projection_screen(gfx, position, moving);
            let sub = if *down {
                "descend · séance imminente"
            } else {
                "remonte · fin de séance"
            };
            (
                "Salle de projection".into(),
                name.clone(),
                sub.into(),
                vec![],
            )
        }
        EventKind::Projector { name, on, app } => {
            projector(gfx, *on, app.as_deref(), age);
            let title = format!("{name} {}", if *on { "allumé" } else { "éteint" });
            let sub = match (on, app) {
                (true, Some(app)) => format!("sur {app}"),
                (true, None) => String::new(),
                (false, _) => "la salle se rallume".into(),
            };
            ("Salle de projection".into(), title, sub, vec![])
        }
        EventKind::Download { name, size } => {
            halos(gfx, age);
            check(gfx, age);
            (
                "Téléchargement terminé".into(),
                text::release(name),
                String::new(),
                size.map(text::size).into_iter().collect(),
            )
        }
        EventKind::Import(import) => {
            if let Some(image) = import
                .poster
                .as_ref()
                .and_then(|poster| textures.get(gfx.canvas, poster, 400, false))
            {
                poster(gfx, image, CX - 100.0, CY - 150.0, 200.0, 300.0, age);
            } else {
                halos(gfx, age);
                check(gfx, age);
            }
            let episode = import.season.is_some();
            let kicker = if episode {
                "Nouvel épisode dans la bibliothèque"
            } else {
                "Nouveau film dans la bibliothèque"
            };
            let sub = if episode {
                let title = import
                    .episode_title
                    .as_deref()
                    .map(|t| format!(" — {t}"))
                    .unwrap_or_default();
                format!(
                    "Saison {} · épisode {}{title}",
                    import.season.unwrap_or_default(),
                    import.episode.unwrap_or_default()
                )
            } else {
                import.year.map(|year| year.to_string()).unwrap_or_default()
            };
            (
                kicker.into(),
                import.title.clone(),
                sub,
                import.quality.iter().cloned().collect(),
            )
        }
        EventKind::Scan(chapter) => {
            halos(gfx, age);
            let number = chapter.number.to_string();
            if gfx.cyber() {
                gfx.split_text(
                    CX,
                    CY,
                    &number,
                    Type::new(Face::Bold, 118.0),
                    gfx.palette.hot,
                    Anchor::MIDDLE_CENTER,
                    1.0,
                );
            } else {
                // gold in hogwarts (a house point total), amber in friday
                let colour = if gfx.hogwarts() {
                    gfx.palette.accent
                } else {
                    gfx.palette.accent2
                };
                gfx.halo(CX, CY, 150.0, fade(colour, 0.18));
                gfx.glow_text(
                    CX,
                    CY,
                    &number,
                    Type::new(Face::Display, 78.0),
                    colour,
                    Anchor::MIDDLE_CENTER,
                    2.5,
                );
            }
            let tags = chapter
                .pages
                .map(|pages| format!("{pages} pages"))
                .into_iter()
                .collect();
            (
                "One Piece · nouveau scan".into(),
                chapter.name.clone(),
                chapter
                    .arc
                    .as_deref()
                    .map(|arc| format!("arc {arc}"))
                    .unwrap_or_default(),
                tags,
            )
        }
        EventKind::Play { title } => ("Lecture".into(), title.clone(), String::new(), vec![]),
    }
}

/// Rings (friday) or diamonds (cyberpunk) widening from the centre, one a second.
fn halos(gfx: &mut Gfx, age: f32) {
    let colour = if gfx.cyber() {
        gfx.palette.hot
    } else {
        gfx.palette.accent
    };
    for phase in 0..3 {
        let k = ((age + phase as f32) / 3.0).fract();
        let alpha = 0.9 * (1.0 - k);
        let size = 150.0 * (0.35 + 0.65 * k);
        if gfx.cyber() {
            let mut path = Path::new();
            path.move_to(CX, CY - size);
            path.line_to(CX + size, CY);
            path.line_to(CX, CY + size);
            path.line_to(CX - size, CY);
            path.close();
            gfx.stroke(&path, fade(colour, alpha * 0.6), 1.0);
        } else {
            gfx.ring(CX, CY, size, 1.0, fade(colour, alpha * 0.6), &[], 0.0, TAU);
        }
    }
}

fn bulb(gfx: &mut Gfx, on: bool) {
    let warm = rgb(0xffd27a);
    let breathe = 1.0 + 0.08 * (gfx.t * 2.1).sin();
    if on {
        gfx.halo(CX, CY - 10.0, 90.0 * breathe, fade(warm, 0.55));
    }
    let mut glass = Path::new();
    glass.move_to(CX - 18.0, CY + 26.0);
    glass.bezier_to(
        CX - 20.0,
        CY + 10.0,
        CX - 38.0,
        CY - 2.0,
        CX - 38.0,
        CY - 22.0,
    );
    glass.bezier_to(CX - 38.0, CY - 46.0, CX - 20.0, CY - 62.0, CX, CY - 62.0);
    glass.bezier_to(
        CX + 20.0,
        CY - 62.0,
        CX + 38.0,
        CY - 46.0,
        CX + 38.0,
        CY - 22.0,
    );
    glass.bezier_to(
        CX + 38.0,
        CY - 2.0,
        CX + 20.0,
        CY + 10.0,
        CX + 18.0,
        CY + 26.0,
    );
    glass.close();
    let mut base = Path::new();
    base.move_to(CX - 16.0, CY + 36.0);
    base.line_to(CX + 16.0, CY + 36.0);
    base.move_to(CX - 12.0, CY + 46.0);
    base.line_to(CX + 12.0, CY + 46.0);
    if on {
        gfx.fill(&glass, fade(warm, 0.35));
        gfx.stroke(&glass, rgb(0xffe3a3), 4.0);
        gfx.stroke(&base, rgb(0xffe3a3), 4.0);
    } else {
        let paint = Paint::color(gfx.palette.muted)
            .with_line_width(3.0)
            .with_line_dash(&[5.0, 5.0]);
        gfx.canvas.stroke_path(&glass, &paint);
        gfx.stroke(&base, gfx.palette.muted, 3.0);
    }
}

/// The projection screen in its box, unrolled to `position` percent.
fn projection_screen(gfx: &mut Gfx, position: f32, moving: bool) {
    let hum = if moving {
        ((gfx.t * 40.0).sin() * 0.8).round()
    } else {
        0.0
    };
    let (left, top, width) = (CX - 130.0, CY - 120.0 + hum, 260.0);
    let drop = 210.0 * (position / 100.0).clamp(0.0, 1.0);
    let cloth = Paint::linear_gradient(0.0, top, 0.0, top + 220.0, rgb(0xeef6ff), rgb(0xc9d6e4));
    let mut path = Path::new();
    path.rect(left + 10.0, top + 14.0, width - 20.0, drop);
    gfx.halo(
        CX,
        top + 14.0 + drop / 2.0,
        170.0,
        fade(rgb(0xdcefff), 0.12 * position / 100.0),
    );
    gfx.canvas.fill_path(&path, &cloth);
    let mut seams = Path::new();
    let mut y = top + 30.0;
    while y < top + 14.0 + drop {
        seams.move_to(left + 10.0, y);
        seams.line_to(left + width - 10.0, y);
        y += 19.0;
    }
    gfx.stroke(&seams, Color::rgbaf(0.0, 0.0, 0.0, 0.05), 1.0);
    if drop > 6.0 {
        gfx.rect(
            left + 10.0,
            top + 8.0 + drop,
            width - 20.0,
            7.0,
            rgb(0x333840),
        );
    }
    let case = Paint::linear_gradient(0.0, top, 0.0, top + 18.0, gfx.palette.muted, rgb(0x222428));
    let mut box_path = Path::new();
    box_path.rounded_rect(left, top, width, 18.0, 4.0);
    gfx.canvas.fill_path(&box_path, &case);
    let style = if gfx.cyber() {
        Type::new(Face::Bold, 26.0)
    } else {
        Type::new(Face::Display, 20.0)
    };
    gfx.title(
        left + width + 6.0,
        top + 240.0,
        &format!("{position:.0} %"),
        style,
        gfx.palette.accent,
        Anchor::TOP_RIGHT,
    );
}

fn projector(gfx: &mut Gfx, on: bool, app: Option<&str>, age: f32) {
    let palette = gfx.palette;
    let (bx, by) = (CX - 140.0, CY + 30.0);
    let mut body = Path::new();
    body.rounded_rect(bx, by, 92.0, 56.0, 8.0);
    gfx.fill(&body, fade(palette.accent, 0.08));
    gfx.stroke(&body, palette.accent, 3.0);
    let (lx, ly) = (bx + 98.0, by + 26.0);
    if on {
        let beam = ease_out(progress(age - 0.4, 1.5));
        let flutter = 0.85 + 0.15 * (gfx.t * 3.0).sin();
        let mut cone = Path::new();
        cone.move_to(lx, ly - 6.0);
        cone.line_to(CX + 150.0, CY - 110.0);
        cone.line_to(CX + 150.0, CY + 20.0);
        cone.line_to(lx, ly + 6.0);
        cone.close();
        let paint = Paint::linear_gradient(
            lx,
            0.0,
            CX + 150.0,
            0.0,
            fade(Color::white(), 0.8 * beam * flutter),
            fade(rgb(0xa0e6ff), 0.12 * beam),
        );
        gfx.canvas.fill_path(&cone, &paint);
        let panel = ease_out(progress(age - 1.6, 1.0));
        gfx.rect(
            CX + 10.0,
            CY - 110.0,
            140.0,
            130.0,
            fade(Color::white(), 0.85 * panel),
        );
        if let Some(app) = app {
            gfx.text(
                CX + 80.0,
                CY - 45.0,
                app,
                Type::new(Face::Bold, 20.0).spaced(1.0),
                fade(Color::black(), panel),
                Anchor::MIDDLE_CENTER,
            );
        }
        gfx.halo(lx, ly, 34.0, fade(palette.accent, 0.8));
        gfx.disc(lx, ly, 13.0, Color::white());
    } else {
        gfx.disc(lx, ly, 13.0, palette.bg);
    }
    gfx.ring(lx, ly, 13.0, 3.0, palette.accent, &[], 0.0, TAU);
}

/// A circle drawing itself, then a check mark.
fn check(gfx: &mut Gfx, age: f32) {
    let palette = gfx.palette;
    let circle = ease_out(progress(age - 0.2, 1.2));
    gfx.ring(
        CX,
        CY,
        78.0,
        4.0,
        palette.accent,
        &[],
        -TAU / 4.0,
        TAU * circle,
    );
    let tick = ease_out(progress(age - 1.2, 0.6));
    if tick > 0.0 {
        let points = [
            (CX - 35.0, CY + 3.0),
            (CX - 11.0, CY + 27.0),
            (CX + 37.0, CY - 25.0),
        ];
        let first = (
            (points[1].0 - points[0].0).hypot(points[1].1 - points[0].1),
            (points[2].0 - points[1].0).hypot(points[2].1 - points[1].1),
        );
        let mut reach = tick * (first.0 + first.1);
        let mut path = Path::new();
        path.move_to(points[0].0, points[0].1);
        for (index, segment) in [first.0, first.1].iter().enumerate() {
            let k = (reach / segment).min(1.0);
            let (a, b) = (points[index], points[index + 1]);
            path.line_to(a.0 + (b.0 - a.0) * k, a.1 + (b.1 - a.1) * k);
            reach -= segment;
            if reach <= 0.0 {
                break;
            }
        }
        gfx.halo(CX, CY, 60.0, fade(palette.ok, 0.18 * tick));
        let paint = Paint::color(palette.ok)
            .with_line_width(9.0)
            .with_line_cap(femtovg::LineCap::Round)
            .with_line_join(femtovg::LineJoin::Round);
        gfx.canvas.stroke_path(&path, &paint);
    }
}

/// A poster in a holographic frame: brackets, a slow sway, a light sweeping down.
#[allow(clippy::many_single_char_names)] // a box: x, y, w, h
pub fn poster(gfx: &mut Gfx, image: femtovg::ImageId, x: f32, y: f32, w: f32, h: f32, age: f32) {
    let palette = gfx.palette;
    let sway = 1.0 - 0.05 * (gfx.t * TAU / 6.0).sin().abs();
    gfx.canvas.save();
    gfx.canvas.translate(x + w / 2.0, 0.0);
    gfx.canvas.scale(sway, 1.0);
    gfx.canvas.translate(-(x + w / 2.0), 0.0);
    gfx.halo(x + w / 2.0, y + h / 2.0, w, fade(palette.accent, 0.18));
    gfx.picture(image, x, y, w, h, ease_out(progress(age, 0.6)));
    gfx.rect_line(x, y, w, h, fade(palette.accent, 0.45), 1.0);
    // the sweep runs down in 3.5 s, then waits
    let k = (gfx.t / 3.5).fract() / 0.6;
    if k <= 1.0 {
        let sy = y + (h - 40.0) * k;
        let glare = Paint::linear_gradient_stops(
            0.0,
            sy,
            0.0,
            sy + 40.0,
            [
                (0.0, fade(rgb(0xa0f0ff), 0.0)),
                (0.5, fade(rgb(0xa0f0ff), 0.35)),
                (1.0, fade(rgb(0xa0f0ff), 0.0)),
            ],
        );
        let mut band = Path::new();
        band.rect(x, sy, w, 40.0);
        gfx.canvas.fill_path(&band, &glare);
    }
    gfx.canvas.restore();
    let corner = if gfx.cyber() {
        palette.hot
    } else {
        palette.accent
    };
    gfx.brackets(x - 8.0, y - 8.0, w + 16.0, h + 16.0, 26.0, corner, 2.0);
}
