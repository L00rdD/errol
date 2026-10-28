//! The idle recap: a clock in its rings (the Time-Turner on the Map), and a
//! carousel of cards (One Piece, a trading bot, the house, the Claude Code
//! sessions of the Mac, downloads, the latest additions, the server), rooms
//! of the castle on the Map.

use std::f32::consts::TAU;

use errol_domain::entities::{MediaKind, Snapshot};
use errol_domain::usecases::View;
use femtovg::Color;

use super::{BOTTOM, TOP, text, turner, xmansion};
use crate::anim::{decode, ease_out, progress};
use crate::look::{fade, mix};
use crate::render::{Anchor, Face, Finish, Gfx, Textures, Type, ink};

/// Seconds a card stays.
const CARD_TIME: f32 = 7.0;
/// Seconds a card takes to slide in or out.
const SLIDE: f32 = 0.8;
const CARD_X: f32 = 286.0;
const CARD_Y: f32 = TOP + 26.0;
const CARD_W: f32 = 490.0;
const CARD_H: f32 = BOTTOM - 40.0 - CARD_Y;
/// A box: x, y, w, h.
pub type Box = (f32, f32, f32, f32);
/// The Time-Turner's centre and scale on the Map, and the box it stands in
/// (the footsteps leave from its door).
const TURNER: (f32, f32, f32) = (140.0, 216.0, 0.6);
const TURNER_BOX: Box = (31.0, 75.0, 218.0, 349.0);
const OATH: &str = "« Je jure solennellement que mes intentions sont mauvaises. »";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Card {
    OnePiece,
    Portfolio,
    House,
    Cerebro,
    Downloads,
    Recent,
    Server,
}

impl Card {
    /// The room of the castle the card is, on the Map.
    fn room(self) -> &'static str {
        match self {
            Self::OnePiece => "Bibliothèque",
            Self::Portfolio => "Cachots",
            Self::House => "Grande Salle",
            Self::Cerebro => "Bureau de Dumbledore",
            Self::Downloads => "Volière",
            Self::Recent => "Salle sur Demande",
            Self::Server => "Tour d'astronomie",
        }
    }
}

#[derive(Default)]
pub struct Carousel {
    /// Rotations done; the card shown is this modulo the cards available.
    turn: usize,
    since: f32,
    previous: Option<Card>,
    /// The carousel just turned: footsteps are due from the clock to the card.
    walk_due: bool,
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
    if snapshot
        .cerebro
        .as_ref()
        .is_some_and(|cerebro| !cerebro.sessions.is_empty())
    {
        cards.push(Card::Cerebro);
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
            self.walk_due = list.len() > 1;
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
        // one dash per card, the current one lit; on the Map, in ink
        let dim = if gfx.hogwarts() { 0.3 } else { 0.15 };
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
                    fade(gfx.palette.accent, dim)
                },
            );
        }
    }

    /// The boxes footsteps should walk between, once, after the carousel
    /// turned: from the clock's to the card's.
    pub fn take_walk(&mut self) -> Option<(Box, Box)> {
        if !std::mem::take(&mut self.walk_due) {
            return None;
        }
        Some((TURNER_BOX, (CARD_X, CARD_Y, CARD_W, CARD_H)))
    }
}

/// The clock in its turning rings; the Time-Turner on the Map, the time in
/// words under it, and the Map's oath.
fn core(gfx: &mut Gfx, view: &View) {
    let palette = gfx.palette;
    if gfx.hogwarts() {
        let local = text::local(view.now);
        let (hour, minute) = (
            chrono::Timelike::hour(&local),
            chrono::Timelike::minute(&local),
        );
        let gone = (minute * 60 + chrono::Timelike::second(&local)) as f32 / 3600.0;
        let (x, y, scale) = TURNER;
        turner::draw(gfx, x, y, scale, (hour % 12) as usize, gone);
        gfx.text(
            x,
            y + 126.0,
            &text::in_words(hour, minute),
            Type::new(Face::Italic, 19.0),
            palette.accent,
            Anchor::MIDDLE_CENTER,
        );
        gfx.text(
            x,
            y + 146.0,
            &text::clock(view.now),
            Type::new(Face::Body, 11.0).spaced(2.0),
            palette.accent2,
            Anchor::MIDDLE_CENTER,
        );
        gfx.text(
            x,
            y + 164.0,
            &text::date(view.now),
            Type::new(Face::Italic, 13.0),
            palette.muted,
            Anchor::MIDDLE_CENTER,
        );
        let oath = Type::new(Face::Italic, 11.5);
        for (index, line) in gfx.wrap(OATH, 250.0, oath, 2).iter().enumerate() {
            gfx.text(
                x,
                y + 186.0 + index as f32 * 14.0,
                line,
                oath,
                fade(palette.muted, 0.85),
                Anchor::MIDDLE_CENTER,
            );
        }
        return;
    }
    let (x, y) = (140.0, f32::midpoint(TOP, BOTTOM));
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

/// The frame of a card: corner brackets in friday, a cut panel in cyberpunk,
/// a room of the castle with its name on a ribbon in hogwarts.
fn frame(gfx: &mut Gfx, x: f32, card: Card) {
    let palette = gfx.palette;
    if gfx.hogwarts() {
        gfx.room(x, CARD_Y, CARD_W, CARD_H);
        gfx.ribbon(x + CARD_W / 2.0, CARD_Y - 9.0, card.room(), 1.0);
    } else if gfx.cyber() {
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
    if gfx.hogwarts() {
        gfx.title(
            x,
            y,
            value,
            Type::new(Face::Display, 38.0),
            colour,
            Anchor::TOP_LEFT,
        );
    } else if gfx.cyber() {
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
    let glyphs = gfx.glyphs();
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
    let rule = fade(palette.accent, if gfx.hogwarts() { 0.45 } else { 0.12 });
    gfx.dashed_rule(x, y + 24.0, CARD_W - 44.0, rule);
}

/// A bar filled to `percent`, with a light running along it; in ink on the
/// Map, a rule filled with sand.
pub fn gauge(gfx: &mut Gfx, x: f32, y: f32, w: f32, percent: f64) {
    let palette = gfx.palette;
    let filled = w * (percent as f32 / 100.0).clamp(0.0, 1.0);
    if gfx.hogwarts() {
        gfx.rect(x, y, filled, 12.0, turner::sand());
        gfx.rect_line(x, y, w, 12.0, palette.accent, 1.0);
        gfx.rect_line(
            x + 3.0,
            y + 3.0,
            w - 6.0,
            6.0,
            fade(palette.accent, 0.35),
            1.0,
        );
        return;
    }
    gfx.rect(x, y, w, 8.0, fade(palette.accent, 0.08));
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

/// A plate of a Quidditch scoreboard: ink, a gold rule inside, its figure in
/// gold; unlit, it fades into the parchment.
fn plate(gfx: &mut Gfx, x: f32, y: f32, w: f32, h: f32, value: &str, size: f32, lit: bool) {
    let (board, rule, figure) = if lit {
        (ink::news_ink(), fade(turner::sand(), 0.8), turner::sand())
    } else {
        (
            fade(ink::ink(), 0.28),
            fade(turner::sand(), 0.35),
            fade(ink::ribbon(), 0.75),
        )
    };
    gfx.rect(x, y, w, h, board);
    gfx.rect_line(x + 3.0, y + 3.0, w - 6.0, h - 6.0, rule, 1.0);
    let style = Type::new(Face::Body, size);
    let value = gfx.wrap(value, w - 14.0, style, 1);
    gfx.text(
        x + w / 2.0,
        y + h / 2.0 + 1.0,
        value.first().map_or("", String::as_str),
        style,
        figure,
        Anchor::MIDDLE_CENTER,
    );
}

/// The beam of a Quidditch scoreboard across `w`, with `plates` hung under
/// it on their rings (figure, name, lit), sized for `columns` of them; the
/// names in red under the plates, in muted ink when unlit. Returns the
/// height used.
fn scoreboard(
    gfx: &mut Gfx,
    x: f32,
    y: f32,
    w: f32,
    plates: &[(String, String, bool)],
    columns: usize,
) -> f32 {
    let ink = ink::ink();
    gfx.rect(x, y, w, 3.0, ink);
    gfx.rect(x, y + 5.0, w, 1.0, ink);
    let gap = 10.0;
    let columns = columns.max(plates.len()) as f32;
    let pw = (w - gap * (columns - 1.0)) / columns;
    let (top, h) = (y + 14.0, 48.0);
    for (index, (figure, name, lit)) in plates.iter().enumerate() {
        let px = x + index as f32 * (pw + gap);
        let mut rings = femtovg::Path::new();
        for rx in [px + 12.0, px + pw - 12.0] {
            rings.move_to(rx, y + 6.0);
            rings.line_to(rx, top);
            rings.circle(rx, y + 9.5, 2.2);
        }
        gfx.stroke(&rings, ink, 1.3);
        if *lit {
            gfx.halo(
                px + pw / 2.0,
                top + h / 2.0,
                pw * 0.7,
                fade(turner::sand(), 0.22),
            );
        }
        plate(gfx, px, top, pw, h, figure, 24.0, *lit);
        gfx.text(
            px + pw / 2.0,
            top + h + 7.0,
            &name.to_uppercase(),
            Type::new(Face::Body, 10.5).spaced(2.0),
            if *lit { ink::red() } else { gfx.palette.muted },
            Anchor::TOP_CENTER,
        );
    }
    14.0 + h + 7.0 + 14.0
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
    gfx.canvas.set_global_alpha(gfx.alpha * alpha);
    let x = CARD_X + shift;
    let map = gfx.hogwarts();
    frame(gfx, x, card);
    let palette = gfx.palette;
    let finish = if map { Finish::Print } else { Finish::Plain };
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
                    // on the Map, an hourglass counts the week down on the right
                    let room = if map {
                        CARD_W - 44.0 - 76.0
                    } else {
                        CARD_W - 44.0
                    };
                    let w = (room - 30.0) / 4.0;
                    if map {
                        let week = 7.0 * 86_400.0;
                        let fallen = 1.0 - (left_secs as f32 / week).min(1.0);
                        turner::hourglass(
                            gfx,
                            left + CARD_W - 44.0 - 62.0,
                            top + 118.0,
                            0.75,
                            fallen,
                        );
                    }
                    if map {
                        // the scoreboard counts the days down, every plate lit
                        let plates: Vec<(String, String, bool)> = parts
                            .iter()
                            .map(|(value, label)| {
                                (format!("{value:02}"), (*label).to_owned(), true)
                            })
                            .collect();
                        scoreboard(gfx, left, y, room, &plates, 4);
                    }
                    for (index, (value, label)) in parts.iter().enumerate().filter(|_| !map) {
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
                        y + if map { 90.0 } else { 70.0 },
                        &format!("avant le scan {}, {}", next.number, text::weekday(next.at)),
                        Type::new(if map { Face::Italic } else { Face::Body }, 15.0),
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
                let mut y = if map {
                    // the score of the match: the equity, its change, the trades
                    let plates = [
                        (
                            format!("{:.0} $", portfolio.equity),
                            "capital".to_owned(),
                            true,
                        ),
                        (
                            text::percent(change),
                            "depuis le départ".to_owned(),
                            change >= 0.0,
                        ),
                        (portfolio.trades.to_string(), "trades".to_owned(), true),
                    ];
                    top + 26.0 + scoreboard(gfx, left, top + 26.0, CARD_W - 44.0, &plates, 3) + 4.0
                } else {
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
                    y
                };
                for (index, position) in portfolio.positions.iter().take(4).enumerate() {
                    if map && index == 0 {
                        y -= 32.0;
                    }
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
                // name, state, on
                let mut devices: Vec<(String, String, bool)> = house
                    .outlets
                    .iter()
                    .map(|o| {
                        (
                            o.name.clone(),
                            if o.on { "allumé" } else { "éteint" }.to_owned(),
                            o.on,
                        )
                    })
                    .collect();
                devices.extend(house.covers.iter().map(|c| {
                    let down = c.position > 0.0;
                    (
                        c.name.clone(),
                        if down { "baissé" } else { "remonté" }.to_owned(),
                        down,
                    )
                }));
                devices.extend(house.projectors.iter().map(|p| {
                    (
                        p.name.clone(),
                        p.app.as_deref().filter(|_| p.on).map_or_else(
                            || if p.on { "allumé" } else { "éteint" }.to_owned(),
                            str::to_owned,
                        ),
                        p.on,
                    )
                }));
                if map {
                    // the scoreboard of the house: a beam of three plates a row
                    let mut y = top + 62.0;
                    for row in devices.chunks(3).take(2) {
                        y += scoreboard(gfx, left, y, CARD_W - 44.0, row, 3) + 6.0;
                    }
                }
                let w = (CARD_W - 44.0 - 12.0) / 2.0;
                for (index, (name, state, on)) in
                    devices.iter().enumerate().take(6).filter(|_| !map)
                {
                    let name = format!("{name} · {state}");
                    let name = name.as_str();
                    let (bx, by) = (
                        left + (index % 2) as f32 * (w + 12.0),
                        top + 70.0 + (index / 2) as f32 * 54.0,
                    );
                    let (lit, dim) = if map { (0.7, 0.3) } else { (0.35, 0.12) };
                    gfx.rect_line(
                        bx,
                        by,
                        w,
                        44.0,
                        fade(palette.accent, if *on { lit } else { dim }),
                        1.0,
                    );
                    let pulse = 0.75 + 0.25 * (gfx.t * 2.4 + index as f32).sin();
                    if *on {
                        // a lit room glows candle-gold on the Map
                        let glow = if map { turner::sand() } else { palette.accent2 };
                        gfx.halo(bx + 20.0, by + 22.0, 18.0 * pulse, fade(glow, 0.55));
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
        Card::Cerebro => {
            // Cerebro's own session cards, its rooms and X-Men, in the look's frame
            if let Some(cerebro) = &snapshot.cerebro {
                gfx.kicker(left, top, "Cerebro · Claude Code", kicker);
                let area = (
                    left,
                    top + 24.0,
                    CARD_W - 44.0,
                    CARD_Y + CARD_H - 14.0 - (top + 24.0),
                );
                xmansion::draw_sessions(gfx, textures, cerebro, area, view.now.0);
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
            let (w, h, gap) = (100.0, 150.0, 14.0);
            for import in snapshot
                .recent
                .iter()
                .filter(|import| import.poster.is_some())
                .take(4)
            {
                let by = top + 34.0;
                gfx.rect(bx, by, w, h, fade(palette.accent, 0.06));
                let image = import
                    .poster
                    .as_ref()
                    .and_then(|poster| textures.get(gfx.canvas, poster, 200, finish));
                // the Gazette prints its photographs in halftone
                match image {
                    Some(image) if map => gfx.photo(image, bx, by, w, h, 1.0),
                    Some(image) => gfx.picture(image, bx, by, w, h, 1.0),
                    None => {}
                }
                if !map {
                    gfx.rect_line(bx, by, w, h, fade(palette.accent, 0.3), 1.0);
                }
                let caption = match import.kind {
                    MediaKind::Episode => format!(
                        "{} · {}x{:02}",
                        import.title,
                        import.season.unwrap_or_default(),
                        import.episode.unwrap_or_default()
                    ),
                    MediaKind::Movie => import.title.clone(),
                };
                let caption_style = Type::new(if map { Face::Italic } else { Face::Body }, 13.0);
                let lines = gfx.wrap(&caption, w, caption_style, 2);
                for (index, line) in lines.iter().enumerate() {
                    gfx.text(
                        bx,
                        by + h + 8.0 + index as f32 * 15.0,
                        line,
                        caption_style,
                        palette.muted,
                        Anchor::TOP_LEFT,
                    );
                }
                bx += w + gap;
            }
        }
        Card::Server => {
            if let Some(disk) = snapshot.disk
                && map
            {
                // the score of the server: free, used, services up, and the
                // disk as a gauge of sand under the board
                gfx.kicker(left, top, "Serveur", kicker);
                let services = snapshot.services;
                let plates = [
                    (text::size(disk.free), "libres".to_owned(), true),
                    (text::size(disk.used), "occupés".to_owned(), true),
                    (
                        format!("{} / {}", services.up, services.total),
                        "services".to_owned(),
                        services.total > 0 && services.up == services.total,
                    ),
                ];
                let used = disk.used as f64 / disk.total.max(1) as f64;
                let y = top
                    + 44.0
                    + scoreboard(gfx, left, top + 44.0, CARD_W - 44.0, &plates, 3)
                    + 30.0;
                gfx.text(
                    left,
                    y - 24.0,
                    "LE DISQUE",
                    Type::new(Face::Body, 12.0).spaced(2.0),
                    palette.accent,
                    Anchor::TOP_LEFT,
                );
                gauge(gfx, left, y, CARD_W - 44.0, used * 100.0);
                gfx.text(
                    left,
                    y + 20.0,
                    &format!("{:.0} % du disque", used * 100.0),
                    Type::new(Face::Italic, 14.0),
                    palette.muted,
                    Anchor::TOP_LEFT,
                );
                gfx.text(
                    left + CARD_W - 44.0,
                    y + 20.0,
                    &format!("{} en tout", text::size(disk.total)),
                    Type::new(Face::Italic, 14.0),
                    palette.muted,
                    Anchor::TOP_RIGHT,
                );
                if let Some(warning) = view
                    .warning
                    .as_deref()
                    .or(snapshot.warnings.first().map(String::as_str))
                {
                    gfx.paragraph(
                        left,
                        y + 50.0,
                        warning,
                        CARD_W - 44.0,
                        Type::new(Face::Italic, 14.0),
                        18.0,
                        palette.hot,
                        3,
                    );
                }
            } else if let Some(disk) = snapshot.disk {
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
                let used = disk.used as f64 / disk.total.max(1) as f64;
                gauge(gfx, left, top + 116.0, CARD_W - 44.0, used * 100.0);
                let below = 0.0;
                let services = snapshot.services;
                row(
                    gfx,
                    left,
                    top + 146.0 + below,
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
                        top + 186.0 + below,
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
