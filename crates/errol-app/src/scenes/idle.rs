//! The idle recap: a clock in its rings, and a carousel of cards (One Piece,
//! a trading bot, the house, downloads, the latest additions, the server).

use std::f32::consts::TAU;

use errol_domain::entities::{MediaKind, Snapshot};
use errol_domain::usecases::View;
use femtovg::Color;

use super::text;
use super::{BOTTOM, TOP};
use crate::anim::{decode, ease_out, progress};
use crate::look::{fade, mix};
use crate::render::{Anchor, Face, Gfx, Textures, Type};

/// Seconds a card stays.
const CARD_TIME: f32 = 12.0;
/// Seconds a card takes to slide in or out.
const SLIDE: f32 = 0.8;
const CARD_X: f32 = 286.0;
const CARD_Y: f32 = TOP + 26.0;
const CARD_W: f32 = 490.0;
const CARD_H: f32 = BOTTOM - 40.0 - CARD_Y;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Card {
    OnePiece,
    Portfolio,
    House,
    Downloads,
    Recent,
    Server,
}

#[derive(Default)]
pub struct Carousel {
    /// Rotations done; the card shown is this modulo the cards available.
    turn: usize,
    since: f32,
    previous: Option<Card>,
}

fn cards(snapshot: &Snapshot) -> Vec<Card> {
    let mut cards = Vec::new();
    if snapshot.one_piece.is_some() {
        cards.push(Card::OnePiece);
    }
    if snapshot.portfolio.is_some() {
        cards.push(Card::Portfolio);
    }
    if snapshot.house.is_some() {
        cards.push(Card::House);
    }
    if !snapshot.downloads.is_empty() {
        cards.push(Card::Downloads);
    }
    if snapshot.recent.iter().any(|import| import.poster.is_some()) {
        cards.push(Card::Recent);
    }
    if snapshot.disk.is_some() {
        cards.push(Card::Server);
    }
    cards
}

impl Carousel {
    pub fn draw(&mut self, gfx: &mut Gfx, view: &View, textures: &mut Textures, _age: f32) {
        core(gfx, view);
        let Some(snapshot) = &view.snapshot else {
            return;
        };
        let list = cards(snapshot);
        if list.is_empty() {
            return;
        }
        if gfx.t - self.since >= CARD_TIME {
            self.previous = list.get(self.turn % list.len()).copied();
            self.turn += 1;
            self.since = gfx.t;
        }
        let card = list[self.turn % list.len()];
        let age = gfx.t - self.since;
        if let Some(previous) = self.previous.filter(|_| age < SLIDE) {
            let k = ease_out(progress(age, SLIDE));
            draw_card(
                gfx,
                snapshot,
                view,
                previous,
                -40.0 * k,
                1.0 - k,
                10.0,
                textures,
            );
        }
        let k = ease_out(progress(age, SLIDE));
        draw_card(
            gfx,
            snapshot,
            view,
            card,
            40.0 * (1.0 - k),
            k,
            age,
            textures,
        );
        // one dash per card, the current one lit
        for index in 0..list.len() {
            let dash_x = 776.0 - (list.len() - index) as f32 * 24.0;
            let current = index == self.turn % list.len();
            gfx.rect(
                dash_x,
                BOTTOM - 20.0,
                18.0,
                3.0,
                if current {
                    gfx.palette.accent
                } else {
                    fade(gfx.palette.accent, 0.15)
                },
            );
        }
    }
}

/// The clock in its turning rings.
fn core(gfx: &mut Gfx, view: &View) {
    let (x, y) = (140.0, f32::midpoint(TOP, BOTTOM));
    let palette = gfx.palette;
    let t = gfx.t;
    let ring = if gfx.cyber() {
        palette.hot
    } else {
        palette.accent
    };
    gfx.ring(
        x,
        y,
        104.0,
        1.5,
        fade(palette.accent, 0.5),
        &[2.0, 6.0],
        t * TAU / 40.0,
        TAU,
    );
    gfx.ring(
        x,
        y,
        92.0,
        3.0,
        fade(ring, 0.85),
        &[120.0, 40.0, 20.0, 40.0, 60.0, 300.0],
        -t * TAU / 18.0,
        TAU,
    );
    gfx.ring(
        x,
        y,
        80.0,
        10.0,
        fade(palette.accent, 0.28),
        &[1.0, 3.0],
        t * TAU / 90.0,
        TAU,
    );
    let local = text::local(view.now);
    let seconds =
        chrono::Timelike::second(&local) as f32 + (view.now.0.rem_euclid(1000)) as f32 / 1000.0;
    gfx.ring(
        x,
        y,
        98.0,
        2.0,
        palette.accent2,
        &[],
        -TAU / 4.0,
        TAU * seconds / 60.0,
    );
    let clock = text::clock(view.now);
    if gfx.cyber() {
        gfx.split_text(
            x,
            y - 6.0,
            &clock,
            Type::new(Face::Bold, 62.0),
            palette.hot,
            Anchor::MIDDLE_CENTER,
            0.0,
        );
    } else {
        gfx.halo(x, y - 6.0, 70.0, fade(palette.accent, 0.12));
        gfx.glow_text(
            x,
            y - 6.0,
            &clock,
            Type::new(Face::Display, 42.0).spaced(1.0),
            palette.text,
            Anchor::MIDDLE_CENTER,
            1.5,
        );
    }
    gfx.text(
        x,
        y + 30.0,
        &text::date(view.now).to_uppercase(),
        Type::new(Face::Strong, 12.0).spaced(3.0),
        palette.muted,
        Anchor::MIDDLE_CENTER,
    );
}

/// The frame of a card: corner brackets in friday, a cut panel in cyberpunk.
fn frame(gfx: &mut Gfx, x: f32) {
    let palette = gfx.palette;
    if gfx.cyber() {
        let path = Gfx::cut_box(x, CARD_Y, CARD_W, CARD_H, 22.0);
        let paint = femtovg::Paint::linear_gradient(
            x,
            0.0,
            x + CARD_W,
            0.0,
            fade(palette.accent, 0.13),
            fade(palette.bg, 0.6),
        );
        gfx.canvas.fill_path(&path, &paint);
        gfx.rect(x, CARD_Y, 3.0, CARD_H, palette.accent);
        gfx.brackets(
            x - 1.0,
            CARD_Y - 1.0,
            CARD_W,
            CARD_H,
            16.0,
            palette.accent2,
            2.0,
        );
    } else {
        let mut path = femtovg::Path::new();
        path.rect(x, CARD_Y, CARD_W, CARD_H);
        let paint = femtovg::Paint::linear_gradient(
            x,
            CARD_Y,
            x + CARD_W * 0.6,
            CARD_Y + CARD_H * 0.6,
            fade(palette.accent, 0.07),
            fade(palette.accent, 0.0),
        );
        gfx.canvas.fill_path(&path, &paint);
        gfx.rect_line(x, CARD_Y, CARD_W, CARD_H, fade(palette.accent, 0.08), 1.0);
        gfx.brackets(
            x - 1.0,
            CARD_Y - 1.0,
            CARD_W + 2.0,
            CARD_H + 2.0,
            16.0,
            palette.accent,
            2.0,
        );
    }
}

fn big(gfx: &mut Gfx, x: f32, y: f32, value: &str, colour: Color) {
    if gfx.cyber() {
        gfx.split_text(
            x,
            y,
            value,
            Type::new(Face::Bold, 50.0),
            colour,
            Anchor::TOP_LEFT,
            0.0,
        );
    } else {
        gfx.glow_text(
            x,
            y,
            value,
            Type::new(Face::Display, 36.0),
            colour,
            Anchor::TOP_LEFT,
            1.4,
        );
    }
}

fn heading(gfx: &mut Gfx, x: f32, y: f32, value: &str, age: f32) -> f32 {
    let style = if gfx.cyber() {
        Type::new(Face::Bold, 25.0)
    } else {
        Type::new(Face::Display, 18.0)
    };
    let value = if gfx.cyber() {
        value.to_uppercase()
    } else {
        value.to_owned()
    };
    let glyphs = if gfx.cyber() {
        "#$%&@!?/<>=+01"
    } else {
        "0123456789ABCDEF·:/"
    };
    let lines = gfx.wrap(&value, CARD_W - 44.0, style, 2);
    let line = style.size * 1.3;
    for (index, row) in lines.iter().enumerate() {
        let shown = decode(row, age, 0.7, glyphs);
        gfx.text(
            x,
            y + index as f32 * line,
            &shown,
            style,
            gfx.palette.text,
            Anchor::TOP_LEFT,
        );
    }
    lines.len() as f32 * line
}

fn row(gfx: &mut Gfx, x: f32, y: f32, label: &str, value: &str, colour: Color) {
    let palette = gfx.palette;
    let style = Type::new(Face::Body, 16.0);
    gfx.text(x, y, label, style, palette.muted, Anchor::TOP_LEFT);
    gfx.text(
        x + CARD_W - 44.0,
        y,
        value,
        Type::new(Face::Strong, 16.0),
        colour,
        Anchor::TOP_RIGHT,
    );
    let mut path = femtovg::Path::new();
    path.move_to(x, y + 24.0);
    path.line_to(x + CARD_W - 44.0, y + 24.0);
    let paint = femtovg::Paint::color(fade(palette.accent, 0.12))
        .with_line_width(1.0)
        .with_line_dash(&[3.0, 3.0]);
    gfx.canvas.stroke_path(&path, &paint);
}

/// A bar filled to `percent`, with a light running along it.
pub fn gauge(gfx: &mut Gfx, x: f32, y: f32, w: f32, percent: f64) {
    let palette = gfx.palette;
    gfx.rect(x, y, w, 8.0, fade(palette.accent, 0.08));
    let filled = w * (percent as f32 / 100.0).clamp(0.0, 1.0);
    gfx.gradient_rect(
        x,
        y,
        filled,
        8.0,
        palette.accent,
        mix(palette.accent, palette.accent2, 0.8),
    );
    let sweep = (gfx.t / 2.4).fract() * (filled + 60.0) - 60.0;
    let mut path = femtovg::Path::new();
    path.rect(
        x + sweep.max(0.0),
        y,
        (60.0_f32).min(filled - sweep.max(0.0)).max(0.0),
        8.0,
    );
    let glare = femtovg::Paint::linear_gradient_stops(
        x + sweep,
        0.0,
        x + sweep + 60.0,
        0.0,
        [
            (0.0, fade(Color::white(), 0.0)),
            (0.5, fade(Color::white(), 0.5)),
            (1.0, fade(Color::white(), 0.0)),
        ],
    );
    gfx.canvas.fill_path(&path, &glare);
}

#[allow(clippy::too_many_arguments)]
fn draw_card(
    gfx: &mut Gfx,
    snapshot: &Snapshot,
    view: &View,
    card: Card,
    shift: f32,
    alpha: f32,
    age: f32,
    textures: &mut Textures,
) {
    if alpha <= 0.01 {
        return;
    }
    gfx.canvas.save();
    gfx.canvas.set_global_alpha(alpha);
    let x = CARD_X + shift;
    frame(gfx, x);
    let palette = gfx.palette;
    let kicker = if gfx.cyber() {
        palette.hot
    } else {
        palette.accent
    };
    let (left, top) = (x + 22.0, CARD_Y + 20.0);
    match card {
        Card::OnePiece => {
            if let Some(one_piece) = &snapshot.one_piece {
                let arc = one_piece
                    .latest
                    .arc
                    .as_deref()
                    .map(|arc| format!(" · arc {arc}"))
                    .unwrap_or_default();
                gfx.kicker(left, top, &format!("One Piece{arc}"), kicker);
                big(
                    gfx,
                    left,
                    top + 22.0,
                    &one_piece.latest.number.to_string(),
                    palette.accent,
                );
                let used = heading(gfx, left, top + 82.0, &one_piece.latest.name, age);
                if let Some(next) = &one_piece.next {
                    let left_secs = next.at.secs_since(view.now).max(0.0) as u64;
                    let parts = [
                        (left_secs / 86_400, "JOURS"),
                        (left_secs / 3600 % 24, "HEURES"),
                        (left_secs / 60 % 60, "MIN"),
                        (left_secs % 60, "SEC"),
                    ];
                    let y = top + 96.0 + used;
                    let w = (CARD_W - 44.0 - 30.0) / 4.0;
                    for (index, (value, label)) in parts.iter().enumerate() {
                        let bx = left + index as f32 * (w + 10.0);
                        gfx.rect_line(bx, y, w, 58.0, fade(palette.accent, 0.15), 1.0);
                        let style = if gfx.cyber() {
                            Type::new(Face::Bold, 28.0)
                        } else {
                            Type::new(Face::Display, 22.0)
                        };
                        gfx.text(
                            bx + w / 2.0,
                            y + 22.0,
                            &format!("{value:02}"),
                            style,
                            palette.text,
                            Anchor::MIDDLE_CENTER,
                        );
                        gfx.text(
                            bx + w / 2.0,
                            y + 46.0,
                            label,
                            Type::new(Face::Strong, 10.0).spaced(2.0),
                            palette.muted,
                            Anchor::MIDDLE_CENTER,
                        );
                    }
                    gfx.text(
                        left,
                        y + 70.0,
                        &format!("avant le scan {}, {}", next.number, text::weekday(next.at)),
                        Type::new(Face::Body, 15.0),
                        palette.muted,
                        Anchor::TOP_LEFT,
                    );
                }
            }
        }
        Card::Portfolio => {
            if let Some(portfolio) = &snapshot.portfolio {
                gfx.kicker(
                    left,
                    top,
                    &format!("{} · {}", portfolio.name, portfolio.mode),
                    kicker,
                );
                let change = portfolio.change();
                let colour = if change >= 0.0 {
                    palette.ok
                } else {
                    palette.hot
                };
                big(
                    gfx,
                    left,
                    top + 22.0,
                    &format!("{:.0} $", portfolio.equity),
                    colour,
                );
                let mut y = top + 92.0;
                row(
                    gfx,
                    left,
                    y,
                    "Depuis le départ",
                    &text::percent(change),
                    colour,
                );
                y += 32.0;
                row(
                    gfx,
                    left,
                    y,
                    "Trades",
                    &portfolio.trades.to_string(),
                    palette.text,
                );
                for position in portfolio.positions.iter().take(4) {
                    y += 32.0;
                    let change = position.change();
                    let colour = match change {
                        Some(c) if c >= 0.0 => palette.ok,
                        Some(_) => palette.hot,
                        None => palette.muted,
                    };
                    row(
                        gfx,
                        left,
                        y,
                        position.pair.trim_end_matches("/USDT"),
                        &change.map_or("–".into(), text::percent),
                        colour,
                    );
                }
            }
        }
        Card::House => {
            if let Some(house) = &snapshot.house {
                gfx.kicker(left, top, "Maison", kicker);
                let lit = house.outlets.iter().any(|outlet| outlet.on);
                heading(
                    gfx,
                    left,
                    top + 24.0,
                    if lit {
                        "Lumières allumées"
                    } else {
                        "Tout est éteint"
                    },
                    age,
                );
                let mut devices: Vec<(String, bool)> = house
                    .outlets
                    .iter()
                    .map(|o| (o.name.clone(), o.on))
                    .collect();
                devices.extend(house.covers.iter().map(|c| {
                    (
                        format!(
                            "{} · {}",
                            c.name,
                            if c.position > 0.0 {
                                "baissé"
                            } else {
                                "remonté"
                            }
                        ),
                        c.position > 0.0,
                    )
                }));
                devices.extend(house.projectors.iter().map(|p| {
                    (
                        format!(
                            "{}{}",
                            p.name,
                            p.app
                                .as_deref()
                                .filter(|_| p.on)
                                .map(|app| format!(" · {app}"))
                                .unwrap_or_default()
                        ),
                        p.on,
                    )
                }));
                let w = (CARD_W - 44.0 - 12.0) / 2.0;
                for (index, (name, on)) in devices.iter().enumerate().take(6) {
                    let (bx, by) = (
                        left + (index % 2) as f32 * (w + 12.0),
                        top + 70.0 + (index / 2) as f32 * 54.0,
                    );
                    gfx.rect_line(
                        bx,
                        by,
                        w,
                        44.0,
                        fade(palette.accent, if *on { 0.35 } else { 0.12 }),
                        1.0,
                    );
                    let pulse = 0.75 + 0.25 * (gfx.t * 2.4 + index as f32).sin();
                    if *on {
                        gfx.halo(
                            bx + 20.0,
                            by + 22.0,
                            18.0 * pulse,
                            fade(palette.accent2, 0.55),
                        );
                        gfx.disc(bx + 20.0, by + 22.0, 6.0, palette.accent2);
                    } else {
                        gfx.disc(bx + 20.0, by + 22.0, 5.0, fade(palette.muted, 0.4));
                    }
                    gfx.text(
                        bx + 38.0,
                        by + 22.5,
                        name,
                        Type::new(Face::Body, 16.0),
                        if *on { palette.text } else { palette.muted },
                        Anchor::MIDDLE_LEFT,
                    );
                }
            }
        }
        Card::Downloads => {
            gfx.kicker(left, top, "Téléchargements", kicker);
            let mut y = top + 30.0;
            for download in snapshot.downloads.iter().take(4) {
                let name = gfx.wrap(
                    &text::release(&download.name),
                    CARD_W - 44.0,
                    Type::new(Face::Strong, 16.0),
                    1,
                );
                gfx.text(
                    left,
                    y,
                    name.first().map_or("", String::as_str),
                    Type::new(Face::Strong, 16.0),
                    palette.text,
                    Anchor::TOP_LEFT,
                );
                gauge(gfx, left, y + 26.0, CARD_W - 44.0, download.percent);
                gfx.text(
                    left,
                    y + 40.0,
                    &format!("{:.1} %", download.percent).replace('.', ","),
                    Type::new(Face::Body, 13.0),
                    palette.muted,
                    Anchor::TOP_LEFT,
                );
                if let Some(eta) = download.eta {
                    gfx.text(
                        left + CARD_W - 44.0,
                        y + 40.0,
                        &format!("reste {}", text::duration(eta)),
                        Type::new(Face::Body, 13.0),
                        palette.muted,
                        Anchor::TOP_RIGHT,
                    );
                }
                y += 72.0;
            }
        }
        Card::Recent => {
            gfx.kicker(left, top, "Ajouté récemment", kicker);
            let mut bx = left;
            for import in snapshot
                .recent
                .iter()
                .filter(|import| import.poster.is_some())
                .take(4)
            {
                let (w, h) = (100.0, 150.0);
                let by = top + 34.0;
                gfx.rect(bx, by, w, h, fade(palette.accent, 0.06));
                if let Some(image) = import
                    .poster
                    .as_ref()
                    .and_then(|poster| textures.get(gfx.canvas, poster, 200, false))
                {
                    gfx.picture(image, bx, by, w, h, 1.0);
                }
                gfx.rect_line(bx, by, w, h, fade(palette.accent, 0.3), 1.0);
                let caption = match import.kind {
                    MediaKind::Episode => format!(
                        "{} · {}x{:02}",
                        import.title,
                        import.season.unwrap_or_default(),
                        import.episode.unwrap_or_default()
                    ),
                    MediaKind::Movie => import.title.clone(),
                };
                let lines = gfx.wrap(&caption, w, Type::new(Face::Body, 13.0), 2);
                for (index, line) in lines.iter().enumerate() {
                    gfx.text(
                        bx,
                        by + h + 8.0 + index as f32 * 15.0,
                        line,
                        Type::new(Face::Body, 13.0),
                        palette.muted,
                        Anchor::TOP_LEFT,
                    );
                }
                bx += w + 14.0;
            }
        }
        Card::Server => {
            if let Some(disk) = snapshot.disk {
                gfx.kicker(left, top, "Serveur", kicker);
                big(
                    gfx,
                    left,
                    top + 22.0,
                    &text::size(disk.free),
                    palette.accent,
                );
                gfx.text(
                    left,
                    top + 82.0,
                    &format!("libres sur {}", text::size(disk.total)),
                    Type::new(Face::Body, 18.0),
                    palette.muted,
                    Anchor::TOP_LEFT,
                );
                gauge(
                    gfx,
                    left,
                    top + 116.0,
                    CARD_W - 44.0,
                    disk.used as f64 / disk.total.max(1) as f64 * 100.0,
                );
                let services = snapshot.services;
                row(
                    gfx,
                    left,
                    top + 146.0,
                    "Services en ligne",
                    &format!("{} / {}", services.up, services.total),
                    palette.text,
                );
                if let Some(warning) = view
                    .warning
                    .as_deref()
                    .or(snapshot.warnings.first().map(String::as_str))
                {
                    gfx.paragraph(
                        left,
                        top + 186.0,
                        warning,
                        CARD_W - 44.0,
                        Type::new(Face::Body, 14.0),
                        18.0,
                        palette.hot,
                        3,
                    );
                }
            }
        }
    }
    gfx.canvas.restore();
}
