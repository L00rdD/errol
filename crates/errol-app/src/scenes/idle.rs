//! The idle recap: a clock in its rings (the Time-Turner on the Map), and a
//! carousel of cards (One Piece, a trading bot, the house, the weather, the
//! episodes to watch, the Claude Code sessions of the Mac, downloads, the
//! print on the resin printer, the latest additions, the day's recap, the
//! server), rooms of the castle on the Map. The carousel weighs its cards:
//! what has news comes sooner and stays longer, what has nothing to say waits.

use std::cmp::Reverse;
use std::f32::consts::TAU;

use errol_domain::entities::{
    Disk, Download, House, Import, MediaKind, NextChapter, PictureRef, Portfolio, Printer,
    Services, Snapshot, Timestamp,
};
use errol_domain::usecases::View;
use femtovg::Color;

use super::{BOTTOM, TOP, cards, printer, text, turner, xmansion};
use crate::anim::{decode, ease_out, progress};
use crate::look::{Palette, fade, mix};
use crate::render::{Anchor, Face, Finish, Gfx, Textures, Type, ink};

/// Seconds an important card stays, and a routine one.
const IMPORTANT_TIME: f32 = 10.0;
const ROUTINE_TIME: f32 = 6.0;
/// Turns between two showings of a server with nothing to report.
const SERVER_EVERY: usize = 5;
/// The share of the disk that must stay free.
const DISK_FREE: f64 = 0.15;
/// Seconds before the end of a print, and after it, while the printer
/// comes every other turn.
const PRINT_CLOSE: f64 = 600.0;
/// How far the portfolio must have moved since it was last shown to be
/// favoured, in percent.
const PORTFOLIO_MOVE: f64 = 2.0;
/// Seconds before an episode airs when its card is favoured.
const AIRS_SOON: f64 = 3.0 * 3600.0;
/// The hour from which the day's recap is favoured.
const EVENING: u32 = 18;
/// Seconds a card takes to slide in or out.
const SLIDE: f32 = 0.8;
const CARD_X: f32 = 286.0;
pub const CARD_Y: f32 = TOP + 26.0;
pub const CARD_W: f32 = 490.0;
pub const CARD_H: f32 = BOTTOM - 40.0 - CARD_Y;
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
    Weather,
    Tonight,
    Cerebro,
    Downloads,
    Printer,
    Recent,
    Today,
    Server,
}

impl Card {
    /// Every card, in the order the carousel first shows them.
    const ALL: [Self; 11] = [
        Self::OnePiece,
        Self::Portfolio,
        Self::House,
        Self::Weather,
        Self::Tonight,
        Self::Cerebro,
        Self::Downloads,
        Self::Printer,
        Self::Recent,
        Self::Today,
        Self::Server,
    ];

    /// What draws the inside of the card.
    fn painter(self) -> Painter {
        match self {
            Self::OnePiece => one_piece_card,
            Self::Portfolio => portfolio_card,
            Self::House => house_card,
            Self::Weather => cards::weather_card,
            Self::Tonight => cards::tonight_card,
            Self::Cerebro => cerebro_card,
            Self::Downloads => downloads_card,
            Self::Printer => printer_card,
            Self::Recent => recent_card,
            Self::Today => cards::today_card,
            Self::Server => server_card,
        }
    }

    /// The room of the castle the card is, on the Map.
    fn room(self) -> &'static str {
        match self {
            Self::OnePiece => "Bibliothèque",
            Self::Portfolio => "Cachots",
            Self::House => "Grande Salle",
            Self::Weather => "Tour de divination",
            Self::Tonight => "Salle commune",
            Self::Cerebro => "Bureau de Dumbledore",
            Self::Downloads => "Volière",
            Self::Printer => "Classe de Potions",
            Self::Recent => "Salle sur Demande",
            Self::Today => "Salle des trophées",
            Self::Server => "Tour d'astronomie",
        }
    }

    /// The card has something to say about `snapshot` at `now`.
    fn says_something(self, snapshot: &Snapshot, now: Timestamp) -> bool {
        match self {
            Self::OnePiece => snapshot.one_piece.is_some(),
            Self::Portfolio => snapshot.portfolio.is_some(),
            Self::House => snapshot.house.is_some(),
            Self::Weather => snapshot.weather.is_some(),
            Self::Tonight => !cards::lineup(snapshot, now).is_empty(),
            Self::Cerebro => snapshot
                .cerebro
                .as_ref()
                .is_some_and(|cerebro| !cerebro.sessions.is_empty()),
            Self::Downloads => !snapshot.downloads.is_empty(),
            Self::Printer => snapshot.printer.is_some(),
            Self::Recent => snapshot.recent.iter().any(|import| import.poster.is_some()),
            Self::Today => snapshot.today.is_some_and(|today| !today.is_empty()),
            Self::Server => snapshot.disk.is_some(),
        }
    }
}

/// The cards with something to say, in the carousel's order.
fn cards(snapshot: &Snapshot, now: Timestamp) -> Vec<Card> {
    Card::ALL
        .into_iter()
        .filter(|card| card.says_something(snapshot, now))
        .collect()
}

/// How much a card wants the next turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Weight {
    /// Nothing new: it takes its turn, and stays a short while.
    Routine,
    /// Worth seeing: it comes back sooner, and stays longer.
    Favoured,
    /// It comes next, and stays longer.
    First,
}

impl Weight {
    /// The turns of waiting the weight is worth.
    fn bonus(self) -> usize {
        match self {
            Self::Routine => 0,
            Self::Favoured => 3,
            Self::First => 1000,
        }
    }

    /// Seconds a card of this weight stays.
    fn time(self) -> f32 {
        if self == Self::Routine {
            ROUTINE_TIME
        } else {
            IMPORTANT_TIME
        }
    }

    /// Favoured when `news`, else routine.
    fn favoured(news: bool) -> Self {
        if news { Self::Favoured } else { Self::Routine }
    }
}

/// What the carousel remembers to weigh the next card.
#[derive(Debug, Default)]
struct Memory {
    /// Turns taken.
    turn: usize,
    /// The turn each card was last shown at.
    shown: [Option<usize>; Card::ALL.len()],
    /// The portfolio's equity when it was last shown.
    equity: Option<f64>,
    /// The house's switches as last seen.
    house: Option<Vec<bool>>,
    /// They changed since the house was last shown.
    house_changed: bool,
    /// The printer was printing when last seen.
    printing: bool,
    /// When its last print ended.
    print_done: Option<Timestamp>,
}

/// The house's switches: each outlet on, each cover down, each projector on.
fn switches(house: &House) -> Vec<bool> {
    house
        .outlets
        .iter()
        .map(|outlet| outlet.on)
        .chain(house.covers.iter().map(|cover| cover.position > 0.0))
        .chain(house.projectors.iter().map(|projector| projector.on))
        .collect()
}

impl Memory {
    /// Notices what changed since the last frame: the house, the end of a print.
    fn observe(&mut self, snapshot: &Snapshot, now: Timestamp) {
        if let Some(house) = snapshot.house.as_ref().map(switches) {
            self.house_changed |= self.house.as_ref().is_some_and(|seen| *seen != house);
            self.house = Some(house);
        }
        let printing = snapshot.printer.as_ref().is_some_and(|p| p.active);
        if self.printing && !printing {
            self.print_done = Some(now);
        }
        self.printing = printing;
    }

    /// Turns waited since `card` was last shown; a card never shown waits
    /// longer than any other, so that each comes once before any comes back.
    fn waited(&self, card: Card) -> usize {
        self.shown[card as usize].map_or(self.turn + Card::ALL.len(), |turn| self.turn - turn)
    }

    /// `card` is shown: it starts waiting again, and what it showed is seen.
    fn remember(&mut self, card: Card, snapshot: &Snapshot) {
        self.shown[card as usize] = Some(self.turn);
        self.turn += 1;
        match card {
            Card::House => self.house_changed = false,
            Card::Portfolio => self.equity = snapshot.portfolio.as_ref().map(|p| p.equity),
            _ => {}
        }
    }

    /// How much `card` wants the next turn; None when it must wait.
    fn weight(&self, card: Card, snapshot: &Snapshot, now: Timestamp) -> Option<Weight> {
        Some(match card {
            Card::House if self.house_changed => Weight::First,
            Card::Printer => self.printer_weight(snapshot.printer.as_ref()?, now),
            Card::OnePiece => Weight::favoured(release_day(snapshot, now)),
            Card::Portfolio => Weight::favoured(self.portfolio_moved(snapshot)),
            Card::Tonight => Weight::favoured(airs_soon(snapshot, now)),
            Card::Today => Weight::favoured(chrono::Timelike::hour(&text::local(now)) >= EVENING),
            Card::Server => self.server_weight(snapshot)?,
            _ => Weight::Routine,
        })
    }

    /// The printer comes every other turn while its print nears its end, and
    /// just after.
    fn printer_weight(&self, printer: &Printer, now: Timestamp) -> Weight {
        let closing = printer.active && printer.remaining.is_some_and(|left| left < PRINT_CLOSE);
        let finished = self
            .print_done
            .is_some_and(|done| now.secs_since(done) < PRINT_CLOSE);
        if closing || finished {
            Weight::First
        } else {
            Weight::Routine
        }
    }

    /// The portfolio moved more than `PORTFOLIO_MOVE` since it was last shown.
    fn portfolio_moved(&self, snapshot: &Snapshot) -> bool {
        match (&snapshot.portfolio, self.equity) {
            (Some(portfolio), Some(seen)) if seen > 0.0 => {
                ((portfolio.equity / seen - 1.0) * 100.0).abs() > PORTFOLIO_MOVE
            }
            _ => false,
        }
    }

    /// The server is favoured when something is wrong; otherwise it waits
    /// `SERVER_EVERY` turns between two showings.
    fn server_weight(&self, snapshot: &Snapshot) -> Option<Weight> {
        if server_alarm(snapshot) {
            return Some(Weight::Favoured);
        }
        let due =
            self.shown[Card::Server as usize].map_or(SERVER_EVERY - 1, |turn| turn + SERVER_EVERY);
        (self.turn >= due).then_some(Weight::Routine)
    }
}

/// The next One Piece chapter is out within a day.
fn release_day(snapshot: &Snapshot, now: Timestamp) -> bool {
    snapshot
        .one_piece
        .as_ref()
        .and_then(|one_piece| one_piece.next.as_ref())
        .is_some_and(|next| (0.0..86_400.0).contains(&next.at.secs_since(now)))
}

/// An episode airs within `AIRS_SOON`.
fn airs_soon(snapshot: &Snapshot, now: Timestamp) -> bool {
    snapshot
        .airing
        .iter()
        .filter_map(|episode| episode.at)
        .any(|at| (0.0..AIRS_SOON).contains(&at.secs_since(now)))
}

/// The disk is nearly full, or a service is down.
fn server_alarm(snapshot: &Snapshot) -> bool {
    let services = &snapshot.services;
    let full = snapshot
        .disk
        .is_some_and(|disk| (disk.free as f64) < disk.total as f64 * DISK_FREE);
    full || !services.down.is_empty() || services.up < services.total
}

/// The card to show next and its weight: of those that may come, the one
/// that waited longest, its weight counted as turns of waiting; never the
/// one shown now while another can come; the first in the list on a tie.
fn pick(
    list: &[Card],
    current: Option<Card>,
    snapshot: &Snapshot,
    memory: &Memory,
    now: Timestamp,
) -> (Card, Weight) {
    list.iter()
        .enumerate()
        .filter(|(_, card)| Some(**card) != current)
        .filter_map(|(index, &card)| {
            let weight = memory.weight(card, snapshot, now)?;
            Some((card, weight, memory.waited(card) + weight.bonus(), index))
        })
        .max_by_key(|&(_, _, score, index)| (score, Reverse(index)))
        .map_or_else(
            || {
                // nothing else may come: the card shown stays, or the first
                let card = current
                    .filter(|card| list.contains(card))
                    .unwrap_or(list[0]);
                (
                    card,
                    memory
                        .weight(card, snapshot, now)
                        .unwrap_or(Weight::Routine),
                )
            },
            |(card, weight, _, _)| (card, weight),
        )
}

#[derive(Default)]
pub struct Carousel {
    memory: Memory,
    /// The card shown, since when, and for how long.
    current: Option<Card>,
    since: f32,
    stay: f32,
    previous: Option<Card>,
    /// The carousel just turned: footsteps are due from the clock to the card.
    walk_due: bool,
}

impl Carousel {
    pub fn draw(&mut self, gfx: &mut Gfx, view: &View, textures: &mut Textures, _age: f32) {
        core(gfx, view);
        let Some(snapshot) = &view.snapshot else {
            return;
        };
        let list = cards(snapshot, view.now);
        if list.is_empty() {
            return;
        }
        let card = self.turn_to(gfx.t, &list, snapshot, view.now);
        let age = gfx.t - self.since;
        let k = ease_out(progress(age, SLIDE));
        if let Some(previous) = self.previous.filter(|_| age < SLIDE) {
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
        let index = list.iter().position(|c| *c == card).unwrap_or_default();
        dashes(gfx, list.len(), index);
    }

    /// Turns to the next card once the current one has stayed its time, or
    /// has nothing more to say; returns the card to show.
    fn turn_to(&mut self, t: f32, list: &[Card], snapshot: &Snapshot, now: Timestamp) -> Card {
        self.memory.observe(snapshot, now);
        if let Some(card) = self.current.filter(|card| list.contains(card))
            && t - self.since < self.stay
        {
            return card;
        }
        let (card, weight) = pick(list, self.current, snapshot, &self.memory, now);
        self.memory.remember(card, snapshot);
        if self.current == Some(card) {
            // nothing else to show: it stays on, without sliding again
            self.stay = t - self.since + weight.time();
        } else {
            self.previous = self.current.replace(card);
            self.walk_due = self.previous.is_some();
            self.since = t;
            self.stay = weight.time();
        }
        card
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

/// One dash per card, the current one lit; on the Map, in ink.
fn dashes(gfx: &mut Gfx, count: usize, current: usize) {
    let dim = if gfx.hogwarts() { 0.3 } else { 0.15 };
    for index in 0..count {
        let dash_x = 776.0 - (count - index) as f32 * 24.0;
        let colour = if index == current {
            gfx.palette.accent
        } else {
            fade(gfx.palette.accent, dim)
        };
        gfx.rect(dash_x, BOTTOM - 20.0, 18.0, 3.0, colour);
    }
}

/// The clock in its turning rings; the Time-Turner on the Map, the time in
/// words under it, and the Map's oath.
fn core(gfx: &mut Gfx, view: &View) {
    if gfx.hogwarts() {
        core_map(gfx, view);
        return;
    }
    let (x, y) = (140.0, f32::midpoint(TOP, BOTTOM));
    rings(gfx, x, y, view);
    clock(gfx, x, y, view);
}

/// The Time-Turner, the time in words, the date and the Map's oath.
fn core_map(gfx: &mut Gfx, view: &View) {
    let palette = gfx.palette;
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
}

/// The turning rings around the clock, the inner one counting the seconds.
fn rings(gfx: &mut Gfx, x: f32, y: f32, view: &View) {
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
    gfx.ring(
        x,
        y,
        98.0,
        2.0,
        palette.accent2,
        &[],
        -TAU / 4.0,
        TAU * seconds(view) / 60.0,
    );
}

/// Seconds into the minute, with their fraction.
fn seconds(view: &View) -> f32 {
    let local = text::local(view.now);
    chrono::Timelike::second(&local) as f32 + (view.now.0.rem_euclid(1000)) as f32 / 1000.0
}

/// The time, split in cyberpunk, glowing in friday, and the date under it.
fn clock(gfx: &mut Gfx, x: f32, y: f32, view: &View) {
    let palette = gfx.palette;
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

pub(super) fn big(gfx: &mut Gfx, x: f32, y: f32, value: &str, colour: Color) {
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

pub(super) fn heading(gfx: &mut Gfx, x: f32, y: f32, value: &str, age: f32) -> f32 {
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

pub(super) fn row(gfx: &mut Gfx, x: f32, y: f32, label: &str, value: &str, colour: Color) {
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
pub(super) fn scoreboard(
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

/// What a card is drawn from, and where its text starts.
pub(super) struct Sheet<'a> {
    pub(super) snapshot: &'a Snapshot,
    pub(super) view: &'a View,
    pub(super) left: f32,
    pub(super) top: f32,
    /// The colour of the card's kicker.
    pub(super) kicker: Color,
    /// Seconds since the card came in.
    pub(super) age: f32,
}

/// Draws the inside of a card.
type Painter = fn(&mut Gfx, &Sheet, &mut Textures);

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
    frame(gfx, x, card);
    let kicker = if gfx.cyber() {
        gfx.palette.hot
    } else {
        gfx.palette.accent
    };
    let sheet = Sheet {
        snapshot,
        view,
        left: x + 22.0,
        top: CARD_Y + 20.0,
        kicker,
        age,
    };
    card.painter()(gfx, &sheet, textures);
    gfx.canvas.restore();
}

/// The next scan's number, its name, and the time left before the next one.
fn one_piece_card(gfx: &mut Gfx, sheet: &Sheet, _: &mut Textures) {
    let Some(one_piece) = &sheet.snapshot.one_piece else {
        return;
    };
    let (left, top) = (sheet.left, sheet.top);
    let arc = arc_suffix(one_piece.latest.arc.as_deref());
    gfx.kicker(left, top, &format!("One Piece{arc}"), sheet.kicker);
    let accent = gfx.palette.accent;
    big(
        gfx,
        left,
        top + 22.0,
        &one_piece.latest.number.to_string(),
        accent,
    );
    let used = heading(gfx, left, top + 82.0, &one_piece.latest.name, sheet.age);
    if let Some(next) = &one_piece.next {
        countdown(gfx, sheet, next, top + 96.0 + used);
    }
}

/// " · arc Elbaf", or nothing without an arc.
fn arc_suffix(arc: Option<&str>) -> String {
    arc.map(|arc| format!(" · arc {arc}")).unwrap_or_default()
}

/// Days, hours, minutes and seconds in `secs`, with their labels.
fn countdown_parts(secs: u64) -> [(u64, &'static str); 4] {
    [
        (secs / 86_400, "JOURS"),
        (secs / 3600 % 24, "HEURES"),
        (secs / 60 % 60, "MIN"),
        (secs % 60, "SEC"),
    ]
}

/// How much of a week's sand has fallen with `secs` left.
fn week_fallen(secs: u64) -> f32 {
    let week = 7.0 * 86_400.0;
    1.0 - (secs as f32 / week).min(1.0)
}

/// The time left before the next scan, in four boxes; on the Map, an
/// hourglass and a scoreboard.
fn countdown(gfx: &mut Gfx, sheet: &Sheet, next: &NextChapter, y: f32) {
    let left_secs = next.at.secs_since(sheet.view.now).max(0.0) as u64;
    let parts = countdown_parts(left_secs);
    let map = gfx.hogwarts();
    if map {
        countdown_map(gfx, sheet, left_secs, &parts, y);
    } else {
        countdown_boxes(gfx, sheet.left, y, &parts);
    }
    let muted = gfx.palette.muted;
    gfx.text(
        sheet.left,
        y + if map { 90.0 } else { 70.0 },
        &format!("avant le scan {}, {}", next.number, text::weekday(next.at)),
        Type::new(if map { Face::Italic } else { Face::Body }, 15.0),
        muted,
        Anchor::TOP_LEFT,
    );
}

/// On the Map, an hourglass counts the week down on the right, and the
/// scoreboard counts the days down, every plate lit.
fn countdown_map(gfx: &mut Gfx, sheet: &Sheet, left_secs: u64, parts: &[(u64, &str); 4], y: f32) {
    let (left, top) = (sheet.left, sheet.top);
    let room = CARD_W - 44.0 - 76.0;
    turner::hourglass(
        gfx,
        left + CARD_W - 44.0 - 62.0,
        top + 118.0,
        0.75,
        week_fallen(left_secs),
    );
    let plates: Vec<(String, String, bool)> = parts
        .iter()
        .map(|(value, label)| (format!("{value:02}"), (*label).to_owned(), true))
        .collect();
    scoreboard(gfx, left, y, room, &plates, 4);
}

/// Four boxes: days, hours, minutes, seconds.
fn countdown_boxes(gfx: &mut Gfx, left: f32, y: f32, parts: &[(u64, &str); 4]) {
    let palette = gfx.palette;
    let w = (CARD_W - 44.0 - 30.0) / 4.0;
    let style = if gfx.cyber() {
        Type::new(Face::Bold, 28.0)
    } else {
        Type::new(Face::Display, 22.0)
    };
    for (index, (value, label)) in parts.iter().enumerate() {
        let bx = left + index as f32 * (w + 10.0);
        gfx.rect_line(bx, y, w, 58.0, fade(palette.accent, 0.15), 1.0);
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
}

/// Green when it went up, red when down, muted when unknown.
fn change_colour(palette: Palette, change: Option<f64>) -> Color {
    match change {
        Some(c) if c >= 0.0 => palette.ok,
        Some(_) => palette.hot,
        None => palette.muted,
    }
}

/// The trading bot: its equity, its change, its trades and positions.
fn portfolio_card(gfx: &mut Gfx, sheet: &Sheet, _: &mut Textures) {
    let Some(portfolio) = &sheet.snapshot.portfolio else {
        return;
    };
    let (left, top) = (sheet.left, sheet.top);
    gfx.kicker(
        left,
        top,
        &format!("{} · {}", portfolio.name, portfolio.mode),
        sheet.kicker,
    );
    let map = gfx.hogwarts();
    let y = if map {
        // the score of the match: the equity, its change, the trades
        let plates = portfolio_plates(portfolio);
        top + 26.0 + scoreboard(gfx, left, top + 26.0, CARD_W - 44.0, &plates, 3) + 4.0
    } else {
        portfolio_rows(gfx, left, top, portfolio)
    };
    positions(gfx, left, y, portfolio, map);
}

/// The plates of the bot's scoreboard: capital, change, trades.
fn portfolio_plates(portfolio: &Portfolio) -> [(String, String, bool); 3] {
    let change = portfolio.change();
    [
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
    ]
}

/// The equity in big, its change and the trades in rows; returns the last
/// row's height.
fn portfolio_rows(gfx: &mut Gfx, left: f32, top: f32, portfolio: &Portfolio) -> f32 {
    let palette = gfx.palette;
    let change = portfolio.change();
    let colour = change_colour(palette, Some(change));
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
}

/// A row per position, four at most, under `y`.
fn positions(gfx: &mut Gfx, left: f32, mut y: f32, portfolio: &Portfolio, map: bool) {
    let palette = gfx.palette;
    for (index, position) in portfolio.positions.iter().take(4).enumerate() {
        if map && index == 0 {
            y -= 32.0;
        }
        y += 32.0;
        let change = position.change();
        row(
            gfx,
            left,
            y,
            position.pair.trim_end_matches("/USDT"),
            &change.map_or("–".into(), text::percent),
            change_colour(palette, change),
        );
    }
}

/// The house: what is on, as tiles; on the Map, as scoreboards.
fn house_card(gfx: &mut Gfx, sheet: &Sheet, _: &mut Textures) {
    let Some(house) = &sheet.snapshot.house else {
        return;
    };
    let (left, top) = (sheet.left, sheet.top);
    gfx.kicker(left, top, "Maison", sheet.kicker);
    heading(gfx, left, top + 24.0, house_heading(house), sheet.age);
    let devices = devices(house);
    let map = gfx.hogwarts();
    if map {
        house_boards(gfx, left, top, &devices);
    } else {
        house_tiles(gfx, left, top, &devices);
    }
    let lines = schedule(house, sheet.view.now);
    schedule_lines(gfx, left, schedule_top(map, devices.len(), top), &lines);
}

/// What the house will do next, three things at most: the outlets' next
/// switches, the projection screen going back up.
fn schedule(house: &House, now: Timestamp) -> Vec<String> {
    let switches = house.outlets.iter().filter_map(|outlet| {
        let next = outlet.next.filter(|next| next.at > now)?;
        let verb = if next.on { "s'allume" } else { "s'éteint" };
        Some(format!(
            "{} · {verb} {}",
            outlet.name,
            text::at_time(next.at, now)
        ))
    });
    let closes = house
        .projectors
        .iter()
        .filter_map(|projector| projector.closes_at.filter(|at| *at > now))
        .map(|at| format!("La toile remonte {}", text::within(at.secs_since(now))));
    switches.chain(closes).take(3).collect()
}

/// Where the schedule starts: under the tiles, two a row, or under the
/// Map's beams, three plates each.
fn schedule_top(map: bool, devices: usize, top: f32) -> f32 {
    let shown = devices.min(6);
    if map {
        top + 62.0 + shown.div_ceil(3) as f32 * 89.0 + 4.0
    } else {
        top + 70.0 + shown.div_ceil(2) as f32 * 54.0 + 8.0
    }
}

/// The schedule, a small clock before each line.
fn schedule_lines(gfx: &mut Gfx, left: f32, y: f32, lines: &[String]) {
    let palette = gfx.palette;
    let face = if gfx.hogwarts() {
        Face::Italic
    } else {
        Face::Body
    };
    for (index, line) in lines.iter().enumerate() {
        let ly = y + index as f32 * 24.0;
        small_clock(gfx, left + 7.0, ly + 9.0, palette.accent2);
        gfx.text(
            left + 22.0,
            ly,
            line,
            Type::new(face, 16.0),
            palette.text,
            Anchor::TOP_LEFT,
        );
    }
}

/// A clock face the size of a letter, at ten past.
fn small_clock(gfx: &mut Gfx, x: f32, y: f32, colour: Color) {
    let mut path = femtovg::Path::new();
    path.circle(x, y, 6.5);
    path.move_to(x, y);
    path.line_to(x, y - 4.0);
    path.move_to(x, y);
    path.line_to(x + 3.5, y - 1.5);
    gfx.stroke(&path, colour, 1.3);
}

fn house_heading(house: &House) -> &'static str {
    if house.outlets.iter().any(|outlet| outlet.on) {
        "Lumières allumées"
    } else {
        "Tout est éteint"
    }
}

/// Every device of the house: its name, its state, whether it is on.
fn devices(house: &House) -> Vec<(String, String, bool)> {
    let on_off = |on: bool| if on { "allumé" } else { "éteint" }.to_owned();
    let mut devices: Vec<(String, String, bool)> = house
        .outlets
        .iter()
        .map(|o| (o.name.clone(), on_off(o.on), o.on))
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
            p.app
                .as_deref()
                .filter(|_| p.on)
                .map_or_else(|| on_off(p.on), str::to_owned),
            p.on,
        )
    }));
    devices
}

/// The scoreboard of the house: a beam of three plates a row.
fn house_boards(gfx: &mut Gfx, left: f32, top: f32, devices: &[(String, String, bool)]) {
    let mut y = top + 62.0;
    for row in devices.chunks(3).take(2) {
        y += scoreboard(gfx, left, y, CARD_W - 44.0, row, 3) + 6.0;
    }
}

/// Two columns of tiles, six at most.
fn house_tiles(gfx: &mut Gfx, left: f32, top: f32, devices: &[(String, String, bool)]) {
    let w = (CARD_W - 44.0 - 12.0) / 2.0;
    for (index, (name, state, on)) in devices.iter().enumerate().take(6) {
        let (bx, by) = tile_spot(left, top, w, index);
        device_tile(gfx, (bx, by, w), index, &format!("{name} · {state}"), *on);
    }
}

/// Where the tile `index` stands, two a row.
fn tile_spot(left: f32, top: f32, w: f32, index: usize) -> (f32, f32) {
    (
        left + (index % 2) as f32 * (w + 12.0),
        top + 70.0 + (index / 2) as f32 * 54.0,
    )
}

/// A device's tile: a lamp, lit and pulsing when it is on, and its name.
fn device_tile(gfx: &mut Gfx, (bx, by, w): (f32, f32, f32), index: usize, name: &str, on: bool) {
    let palette = gfx.palette;
    gfx.rect_line(
        bx,
        by,
        w,
        44.0,
        fade(palette.accent, if on { 0.35 } else { 0.12 }),
        1.0,
    );
    let pulse = 0.75 + 0.25 * (gfx.t * 2.4 + index as f32).sin();
    if on {
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
        if on { palette.text } else { palette.muted },
        Anchor::MIDDLE_LEFT,
    );
}

/// Cerebro's own session cards, its rooms and X-Men, in the look's frame.
fn cerebro_card(gfx: &mut Gfx, sheet: &Sheet, textures: &mut Textures) {
    let Some(cerebro) = &sheet.snapshot.cerebro else {
        return;
    };
    let (left, top) = (sheet.left, sheet.top);
    gfx.kicker(left, top, "Cerebro · Claude Code", sheet.kicker);
    let area = (
        left,
        top + 24.0,
        CARD_W - 44.0,
        CARD_Y + CARD_H - 14.0 - (top + 24.0),
    );
    xmansion::draw_sessions(gfx, textures, cerebro, area, sheet.view.now.0);
}

/// Four downloads at most, each with its gauge.
fn downloads_card(gfx: &mut Gfx, sheet: &Sheet, _: &mut Textures) {
    let (left, top) = (sheet.left, sheet.top);
    gfx.kicker(left, top, "Téléchargements", sheet.kicker);
    let mut y = top + 30.0;
    for download in sheet.snapshot.downloads.iter().take(4) {
        download_row(gfx, left, y, download);
        y += 72.0;
    }
}

/// "63,4 %".
fn download_percent(percent: f64) -> String {
    format!("{percent:.1} %").replace('.', ",")
}

/// A download's name, its gauge, how far it is and how long is left.
fn download_row(gfx: &mut Gfx, left: f32, y: f32, download: &Download) {
    let palette = gfx.palette;
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
        &download_percent(download.percent),
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
}

/// The print on the resin printer.
fn printer_card(gfx: &mut Gfx, sheet: &Sheet, textures: &mut Textures) {
    if let Some(p) = &sheet.snapshot.printer {
        printer::card(
            gfx,
            p,
            sheet.view,
            (sheet.left, sheet.top),
            sheet.kicker,
            sheet.age,
            textures,
        );
    }
}

/// A poster's width, height, and the gap to the next.
pub(super) const POSTER: (f32, f32, f32) = (100.0, 150.0, 14.0);

/// The latest additions: four posters at most, with their captions.
fn recent_card(gfx: &mut Gfx, sheet: &Sheet, textures: &mut Textures) {
    let (left, top) = (sheet.left, sheet.top);
    gfx.kicker(left, top, "Ajouté récemment", sheet.kicker);
    let (w, _, gap) = POSTER;
    let mut bx = left;
    for import in sheet
        .snapshot
        .recent
        .iter()
        .filter(|import| import.poster.is_some())
        .take(4)
    {
        let by = top + 34.0;
        poster(gfx, textures, import.poster.as_ref(), bx, by);
        poster_caption(gfx, import, bx, by);
        bx += w + gap;
    }
}

/// A poster; the Gazette prints its photographs in halftone.
pub(super) fn poster(
    gfx: &mut Gfx,
    textures: &mut Textures,
    picture: Option<&PictureRef>,
    bx: f32,
    by: f32,
) {
    let (w, h, _) = POSTER;
    let palette = gfx.palette;
    let map = gfx.hogwarts();
    let finish = if map { Finish::Print } else { Finish::Plain };
    gfx.rect(bx, by, w, h, fade(palette.accent, 0.06));
    poster_image(gfx, textures, picture, (bx, by), finish);
    if !map {
        gfx.rect_line(bx, by, w, h, fade(palette.accent, 0.3), 1.0);
    }
}

/// The poster's picture, once loaded: printed on the Map, plain elsewhere.
fn poster_image(
    gfx: &mut Gfx,
    textures: &mut Textures,
    picture: Option<&PictureRef>,
    (bx, by): (f32, f32),
    finish: Finish,
) {
    let (w, h, _) = POSTER;
    let image = picture.and_then(|poster| textures.get(gfx.canvas, poster, 200, finish));
    match image {
        Some(image) if finish == Finish::Print => gfx.photo(image, bx, by, w, h, 1.0),
        Some(image) => gfx.picture(image, bx, by, w, h, 1.0),
        None => {}
    }
}

/// "Arcane · 2x04", or a film's title.
fn caption(import: &Import) -> String {
    match import.kind {
        MediaKind::Episode => format!(
            "{} · {}x{:02}",
            import.title,
            import.season.unwrap_or_default(),
            import.episode.unwrap_or_default()
        ),
        MediaKind::Movie => import.title.clone(),
    }
}

/// The caption under a poster, on two lines at most.
fn poster_caption(gfx: &mut Gfx, import: &Import, bx: f32, by: f32) {
    let (w, h, _) = POSTER;
    let face = if gfx.hogwarts() {
        Face::Italic
    } else {
        Face::Body
    };
    let style = Type::new(face, 13.0);
    let muted = gfx.palette.muted;
    for (index, line) in gfx.wrap(&caption(import), w, style, 2).iter().enumerate() {
        gfx.text(
            bx,
            by + h + 8.0 + index as f32 * 15.0,
            line,
            style,
            muted,
            Anchor::TOP_LEFT,
        );
    }
}

/// The server: its disk and its services.
fn server_card(gfx: &mut Gfx, sheet: &Sheet, _: &mut Textures) {
    let Some(disk) = sheet.snapshot.disk else {
        return;
    };
    gfx.kicker(sheet.left, sheet.top, "Serveur", sheet.kicker);
    if gfx.hogwarts() {
        server_map(gfx, sheet, disk);
    } else {
        server_plain(gfx, sheet, disk);
    }
}

/// The share of the disk used, 0 to 1.
fn used_share(disk: Disk) -> f64 {
    disk.used as f64 / disk.total.max(1) as f64
}

/// The plates of the server's scoreboard: free, used, services up (lit when
/// they all are).
fn server_plates(disk: Disk, services: &Services) -> [(String, String, bool); 3] {
    [
        (text::size(disk.free), "libres".to_owned(), true),
        (text::size(disk.used), "occupés".to_owned(), true),
        (
            format!("{} / {}", services.up, services.total),
            "services".to_owned(),
            services.total > 0 && services.up == services.total,
        ),
    ]
}

/// Why the feed is incomplete: the last read's failure, else the feed's own
/// first warning.
fn warning<'a>(read: Option<&'a str>, feed: &'a [String]) -> Option<&'a str> {
    read.or(feed.first().map(String::as_str))
}

/// The score of the server: free, used, services up, and the disk as a
/// gauge of sand under the board.
fn server_map(gfx: &mut Gfx, sheet: &Sheet, disk: Disk) {
    let (left, top) = (sheet.left, sheet.top);
    let palette = gfx.palette;
    let plates = server_plates(disk, &sheet.snapshot.services);
    let used = used_share(disk);
    let y = top + 44.0 + scoreboard(gfx, left, top + 44.0, CARD_W - 44.0, &plates, 3) + 30.0;
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
    server_notes(gfx, sheet, y + 50.0, Face::Italic);
}

/// The free space in big, the disk's gauge, the services in a row.
fn server_plain(gfx: &mut Gfx, sheet: &Sheet, disk: Disk) {
    let (left, top) = (sheet.left, sheet.top);
    let palette = gfx.palette;
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
        used_share(disk) * 100.0,
    );
    let services = &sheet.snapshot.services;
    row(
        gfx,
        left,
        top + 146.0,
        "Services en ligne",
        &format!("{} / {}", services.up, services.total),
        palette.text,
    );
    server_notes(gfx, sheet, top + 186.0, Face::Body);
}

/// The services down, as the server's card names them.
fn down_note(services: &Services) -> Option<String> {
    (!services.down.is_empty()).then(|| format!("Hors ligne : {}", services.down.join(", ")))
}

/// What is wrong, in red under the server's figures: the services down,
/// then why the feed is incomplete.
fn server_notes(gfx: &mut Gfx, sheet: &Sheet, mut y: f32, face: Face) {
    let snapshot = sheet.snapshot;
    let read = warning(sheet.view.warning.as_deref(), &snapshot.warnings);
    let notes = down_note(&snapshot.services)
        .into_iter()
        .chain(read.map(str::to_owned));
    let hot = gfx.palette.hot;
    for note in notes {
        let style = Type::new(face, 14.0);
        y += gfx.paragraph(sheet.left, y, &note, CARD_W - 44.0, style, 18.0, hot, 2) + 6.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::look::Look;
    use errol_domain::entities::{Cover, Episode, Outlet, Position, Projector, Today, Weather};

    fn portfolio(equity: f64) -> Portfolio {
        Portfolio {
            name: "Bot".into(),
            mode: "paper".into(),
            equity,
            initial: 1000.0,
            trades: 12,
            positions: vec![Position {
                pair: "BTC/USDT".into(),
                average_price: 100.0,
                price: Some(90.0),
            }],
        }
    }

    fn import(kind: MediaKind) -> Import {
        Import {
            kind,
            title: "Arcane".into(),
            year: None,
            season: Some(2),
            episode: Some(4),
            episode_title: None,
            quality: None,
            poster: None,
        }
    }

    fn disk(free: u64) -> Disk {
        Disk {
            total: 100,
            used: 100 - free,
            free,
        }
    }

    fn healthy() -> Snapshot {
        Snapshot {
            disk: Some(disk(50)),
            services: Services {
                up: 3,
                total: 3,
                down: vec![],
            },
            ..Snapshot::default()
        }
    }

    fn printing(remaining: f64) -> Printer {
        Printer {
            state: Some("printing".into()),
            active: true,
            remaining: Some(remaining),
            ..Printer::default()
        }
    }

    /// The cards `memory` picks over `turns` turns, each shown in its turn.
    fn sequence(
        list: &[Card],
        snapshot: &Snapshot,
        memory: &mut Memory,
        turns: usize,
    ) -> Vec<Card> {
        let mut current = None;
        (0..turns)
            .map(|_| {
                let (card, _) = pick(list, current, snapshot, memory, Timestamp(0));
                memory.remember(card, snapshot);
                current = Some(card);
                card
            })
            .collect()
    }

    #[test]
    fn only_the_cards_with_something_to_say_come() {
        let mut snapshot = healthy();
        assert_eq!(cards(&snapshot, Timestamp(0)), [Card::Server]);
        snapshot.printer = Some(Printer::default());
        snapshot.today = Some(Today::default());
        assert_eq!(
            cards(&snapshot, Timestamp(0)),
            [Card::Printer, Card::Server]
        );
        snapshot.today = Some(Today {
            lights: 2,
            ..Today::default()
        });
        snapshot.weather = Some(Weather {
            temperature: 9.0,
            code: 0,
            wind: None,
            high: None,
            low: None,
            rain: None,
            sunrise: None,
            sunset: None,
        });
        snapshot.next_up = vec![Episode {
            series: "Arcane".into(),
            season: None,
            episode: None,
            title: None,
            poster: None,
            at: None,
        }];
        assert_eq!(
            cards(&snapshot, Timestamp(0)),
            [
                Card::Weather,
                Card::Tonight,
                Card::Printer,
                Card::Today,
                Card::Server
            ]
        );
    }

    #[test]
    fn every_card_is_a_room_of_the_castle() {
        let rooms = Card::ALL.map(Card::room);
        let distinct: std::collections::HashSet<_> = rooms.iter().collect();
        assert_eq!(distinct.len(), rooms.len());
        assert_eq!(Card::Printer.room(), "Classe de Potions");
    }

    #[test]
    fn every_card_has_its_own_painter() {
        let painters: std::collections::HashSet<usize> = Card::ALL
            .iter()
            .map(|card| card.painter() as usize)
            .collect();
        assert_eq!(painters.len(), Card::ALL.len());
    }

    #[test]
    fn the_carousel_turns_once_a_card_has_stayed_its_time() {
        let list = [Card::House, Card::Weather];
        let snapshot = Snapshot::default();
        let now = Timestamp(0);
        let mut carousel = Carousel::default();
        assert_eq!(carousel.turn_to(1.0, &list, &snapshot, now), Card::House);
        assert_eq!(carousel.take_walk(), None);
        let turn = 1.0 + ROUTINE_TIME;
        assert_eq!(
            carousel.turn_to(turn - 0.1, &list, &snapshot, now),
            Card::House
        );
        assert_eq!(carousel.turn_to(turn, &list, &snapshot, now), Card::Weather);
        assert_eq!(carousel.previous, Some(Card::House));
        assert!((carousel.since - turn).abs() < 1e-6);
        assert!(carousel.take_walk().is_some());
        assert_eq!(carousel.take_walk(), None);
        let next = turn + ROUTINE_TIME;
        assert_eq!(carousel.turn_to(next, &list, &snapshot, now), Card::House);
    }

    #[test]
    fn a_single_card_stays_without_sliding_again() {
        let mut carousel = Carousel::default();
        let (snapshot, now) = (Snapshot::default(), Timestamp(0));
        assert_eq!(
            carousel.turn_to(0.0, &[Card::Weather], &snapshot, now),
            Card::Weather
        );
        assert_eq!(
            carousel.turn_to(ROUTINE_TIME, &[Card::Weather], &snapshot, now),
            Card::Weather
        );
        assert_eq!(carousel.take_walk(), None);
        assert!(carousel.since.abs() < 1e-6);
        assert!((carousel.stay - 2.0 * ROUTINE_TIME).abs() < 1e-6);
    }

    #[test]
    fn a_card_that_has_nothing_more_to_say_gives_its_turn_at_once() {
        let mut carousel = Carousel::default();
        let (snapshot, now) = (Snapshot::default(), Timestamp(0));
        carousel.turn_to(0.0, &[Card::Printer, Card::Weather], &snapshot, now);
        assert_eq!(
            carousel.turn_to(1.0, &[Card::Weather], &snapshot, now),
            Card::Weather
        );
    }

    #[test]
    fn a_healthy_server_comes_every_fifth_turn() {
        let snapshot = healthy();
        let list = [Card::Weather, Card::House, Card::Server];
        let shown = sequence(&list, &snapshot, &mut Memory::default(), 10);
        let servers: Vec<usize> = shown
            .iter()
            .enumerate()
            .filter(|(_, card)| **card == Card::Server)
            .map(|(turn, _)| turn)
            .collect();
        assert_eq!(servers, [4, 9]);
    }

    #[test]
    fn a_server_in_trouble_is_favoured_and_says_why() {
        let mut snapshot = healthy();
        let memory = Memory::default();
        assert_eq!(memory.weight(Card::Server, &snapshot, Timestamp(0)), None);
        snapshot.disk = Some(disk(10));
        assert!(server_alarm(&snapshot));
        assert_eq!(
            memory.weight(Card::Server, &snapshot, Timestamp(0)),
            Some(Weight::Favoured)
        );
        snapshot.disk = Some(disk(50));
        snapshot.services.down = vec!["prowlarr".into(), "radarr".into()];
        assert!(server_alarm(&snapshot));
        assert_eq!(
            down_note(&snapshot.services).as_deref(),
            Some("Hors ligne : prowlarr, radarr")
        );
        assert_eq!(down_note(&Services::default()), None);
    }

    #[test]
    fn the_house_comes_next_when_it_changed() {
        let mut snapshot = Snapshot {
            house: Some(house()),
            ..Snapshot::default()
        };
        let mut memory = Memory::default();
        memory.observe(&snapshot, Timestamp(0));
        assert!(!memory.house_changed);
        snapshot.house.as_mut().unwrap().outlets[0].on = true;
        memory.observe(&snapshot, Timestamp(0));
        assert!(memory.house_changed);
        let list = [Card::Weather, Card::House];
        memory.shown[Card::Weather as usize] = None;
        memory.shown[Card::House as usize] = Some(0);
        memory.turn = 1;
        let (card, weight) = pick(&list, None, &snapshot, &memory, Timestamp(0));
        assert_eq!((card, weight), (Card::House, Weight::First));
        memory.remember(Card::House, &snapshot);
        assert!(!memory.house_changed);
        assert_eq!(switches(&house()), [false, true, false, true, true, false]);
    }

    #[test]
    fn a_print_near_its_end_comes_every_other_turn() {
        let snapshot = Snapshot {
            printer: Some(printing(300.0)),
            ..Snapshot::default()
        };
        let list = [Card::House, Card::Weather, Card::Printer];
        let shown = sequence(&list, &snapshot, &mut Memory::default(), 5);
        assert_eq!(
            shown,
            [
                Card::Printer,
                Card::House,
                Card::Printer,
                Card::Weather,
                Card::Printer
            ]
        );
        let far = Snapshot {
            printer: Some(printing(5000.0)),
            ..Snapshot::default()
        };
        let memory = Memory::default();
        assert_eq!(
            memory.weight(Card::Printer, &far, Timestamp(0)),
            Some(Weight::Routine)
        );
        assert_eq!(
            memory.weight(Card::Printer, &Snapshot::default(), Timestamp(0)),
            None
        );
    }

    #[test]
    fn a_print_just_finished_comes_every_other_turn_for_a_while() {
        let mut snapshot = Snapshot {
            printer: Some(printing(5000.0)),
            ..Snapshot::default()
        };
        let mut memory = Memory::default();
        memory.observe(&snapshot, Timestamp(0));
        snapshot.printer = Some(Printer {
            state: Some("done".into()),
            ..Printer::default()
        });
        let done = Timestamp::from_secs(100);
        memory.observe(&snapshot, done);
        assert_eq!(memory.print_done, Some(done));
        let weight = |at: i64| memory.weight(Card::Printer, &snapshot, Timestamp::from_secs(at));
        assert_eq!(weight(400), Some(Weight::First));
        assert_eq!(weight(800), Some(Weight::Routine));
    }

    #[test]
    fn news_favours_its_card() {
        let now = Timestamp::from_secs(0);
        let mut snapshot = Snapshot {
            one_piece: Some(errol_domain::entities::OnePiece {
                latest: errol_domain::entities::Chapter {
                    number: 1,
                    name: String::new(),
                    arc: None,
                    pages: None,
                },
                next: Some(NextChapter {
                    number: 2,
                    at: Timestamp::from_secs(3600),
                }),
            }),
            portfolio: Some(portfolio(1000.0)),
            ..Snapshot::default()
        };
        let mut memory = Memory::default();
        assert_eq!(
            memory.weight(Card::OnePiece, &snapshot, now),
            Some(Weight::Favoured)
        );
        assert_eq!(
            memory.weight(Card::Portfolio, &snapshot, now),
            Some(Weight::Routine)
        );
        memory.remember(Card::Portfolio, &snapshot);
        snapshot.portfolio = Some(portfolio(1030.0));
        assert_eq!(
            memory.weight(Card::Portfolio, &snapshot, now),
            Some(Weight::Favoured)
        );
        snapshot.portfolio = Some(portfolio(1010.0));
        assert_eq!(
            memory.weight(Card::Portfolio, &snapshot, now),
            Some(Weight::Routine)
        );
        snapshot.airing = vec![Episode {
            series: "Arcane".into(),
            season: None,
            episode: None,
            title: None,
            poster: None,
            at: Some(Timestamp::from_secs(7200)),
        }];
        assert_eq!(
            memory.weight(Card::Tonight, &snapshot, now),
            Some(Weight::Favoured)
        );
        assert!(!airs_soon(&snapshot, Timestamp::from_secs(-20_000)));
    }

    #[test]
    fn the_days_recap_is_favoured_in_the_evening() {
        let at = |hour: u32| {
            use chrono::TimeZone;
            Timestamp(
                chrono::Local
                    .with_ymd_and_hms(2026, 11, 8, hour, 0, 0)
                    .single()
                    .unwrap()
                    .timestamp_millis(),
            )
        };
        let (snapshot, memory) = (Snapshot::default(), Memory::default());
        assert_eq!(
            memory.weight(Card::Today, &snapshot, at(20)),
            Some(Weight::Favoured)
        );
        assert_eq!(
            memory.weight(Card::Today, &snapshot, at(10)),
            Some(Weight::Routine)
        );
    }

    #[test]
    fn important_cards_stay_longer() {
        assert!((Weight::Routine.time() - ROUTINE_TIME).abs() < 1e-6);
        assert!((Weight::Favoured.time() - IMPORTANT_TIME).abs() < 1e-6);
        assert!((Weight::First.time() - IMPORTANT_TIME).abs() < 1e-6);
        assert!(Weight::First.bonus() > Weight::Favoured.bonus());
    }

    #[test]
    fn the_house_says_what_it_will_do_next() {
        let now = Timestamp::from_secs(1_000_000);
        let mut house = house();
        house.outlets[0].next = Some(errol_domain::entities::Switch {
            at: now.plus_secs(2400.0),
            on: true,
        });
        house.outlets.push(Outlet {
            id: "old".into(),
            name: "Passé".into(),
            on: true,
            next: Some(errol_domain::entities::Switch {
                at: now.plus_secs(-60.0),
                on: false,
            }),
        });
        house.projectors[2].closes_at = Some(now.plus_secs(240.0));
        let lines = schedule(&house, now);
        assert_eq!(lines.len(), 2);
        assert!(lines[0].starts_with("Lampe · s'allume "));
        assert_eq!(lines[1], "La toile remonte dans 4 min");
        assert!((schedule_top(false, 4, 0.0) - 186.0).abs() < 1e-4);
        assert!((schedule_top(true, 4, 0.0) - 244.0).abs() < 1e-4);
    }

    #[test]
    fn the_countdown_splits_the_time_left() {
        let secs = 2 * 86_400 + 3 * 3600 + 4 * 60 + 5;
        assert_eq!(
            countdown_parts(secs),
            [(2, "JOURS"), (3, "HEURES"), (4, "MIN"), (5, "SEC")]
        );
        assert!((week_fallen(0) - 1.0).abs() < 1e-6);
        assert!(week_fallen(30 * 86_400).abs() < 1e-6);
        assert!((week_fallen(7 * 86_400 / 2) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn the_arc_follows_the_title_when_known() {
        assert_eq!(arc_suffix(Some("Elbaf")), " · arc Elbaf");
        assert_eq!(arc_suffix(None), "");
    }

    #[test]
    fn a_change_is_green_up_red_down_muted_unknown() {
        let palette = Look::Friday.palette();
        assert_eq!(change_colour(palette, Some(0.0)), palette.ok);
        assert_eq!(change_colour(palette, Some(-1.0)), palette.hot);
        assert_eq!(change_colour(palette, None), palette.muted);
    }

    #[test]
    fn the_bots_scoreboard_lights_a_gain_only() {
        let up = portfolio_plates(&portfolio(1087.0));
        assert_eq!(up[0], ("1087 $".into(), "capital".into(), true));
        assert_eq!(up[1], ("+8,7 %".into(), "depuis le départ".into(), true));
        assert_eq!(up[2], ("12".into(), "trades".into(), true));
        assert!(!portfolio_plates(&portfolio(900.0))[1].2);
    }

    fn house() -> House {
        House {
            outlets: vec![Outlet {
                id: "lamp".into(),
                name: "Lampe".into(),
                on: false,
                next: None,
            }],
            covers: vec![
                Cover {
                    id: "screen".into(),
                    name: "Écran".into(),
                    position: 100.0,
                    moving: false,
                },
                Cover {
                    id: "blind".into(),
                    name: "Store".into(),
                    position: 0.0,
                    moving: false,
                },
            ],
            projectors: vec![
                Projector {
                    id: "p".into(),
                    name: "Projecteur".into(),
                    on: true,
                    app: Some("Jellyfin".into()),
                    closes_at: None,
                },
                Projector {
                    id: "q".into(),
                    name: "Vieux".into(),
                    on: true,
                    app: None,
                    closes_at: None,
                },
                Projector {
                    id: "r".into(),
                    name: "Éteint".into(),
                    on: false,
                    app: Some("Netflix".into()),
                    closes_at: None,
                },
            ],
        }
    }

    #[test]
    fn every_device_of_the_house_says_its_state() {
        let mut house = house();
        let states: Vec<(String, bool)> = devices(&house)
            .into_iter()
            .map(|(_, state, on)| (state, on))
            .collect();
        assert_eq!(
            states,
            [
                ("éteint".into(), false),
                ("baissé".into(), true),
                ("remonté".into(), false),
                ("Jellyfin".into(), true),
                ("allumé".into(), true),
                ("éteint".into(), false),
            ]
        );
        assert_eq!(house_heading(&house), "Tout est éteint");
        house.outlets[0].on = true;
        assert_eq!(house_heading(&house), "Lumières allumées");
    }

    #[test]
    fn the_tiles_stand_two_a_row() {
        assert_eq!(tile_spot(10.0, 20.0, 100.0, 0), (10.0, 90.0));
        assert_eq!(tile_spot(10.0, 20.0, 100.0, 3), (122.0, 144.0));
    }

    #[test]
    fn a_download_says_how_far_it_is_with_a_comma() {
        assert_eq!(download_percent(63.44), "63,4 %");
    }

    #[test]
    fn an_episode_is_captioned_with_its_number() {
        assert_eq!(caption(&import(MediaKind::Episode)), "Arcane · 2x04");
        assert_eq!(caption(&import(MediaKind::Movie)), "Arcane");
    }

    #[test]
    fn the_server_lights_its_services_when_all_are_up() {
        let disk = Disk {
            total: 4,
            used: 1,
            free: 3,
        };
        assert!((used_share(disk) - 0.25).abs() < 1e-9);
        let all = server_plates(
            disk,
            &Services {
                up: 3,
                total: 3,
                down: vec![],
            },
        );
        assert_eq!(all[2], ("3 / 3".into(), "services".into(), true));
        assert!(
            !server_plates(
                disk,
                &Services {
                    up: 2,
                    total: 3,
                    down: vec![]
                }
            )[2]
            .2
        );
        assert!(!server_plates(disk, &Services::default())[2].2);
        let empty = Disk {
            total: 0,
            used: 0,
            free: 0,
        };
        assert!(used_share(empty).abs() < 1e-9);
    }

    #[test]
    fn a_failed_read_is_told_before_the_feeds_warnings() {
        let feed = ["house: timeout".to_owned()];
        assert_eq!(warning(Some("offline"), &feed), Some("offline"));
        assert_eq!(warning(None, &feed), Some("house: timeout"));
        assert_eq!(warning(None, &[]), None);
    }

    #[test]
    fn the_clock_counts_the_seconds_with_their_fraction() {
        let at = Timestamp(1_793_814_671_250);
        let s = seconds(&View {
            snapshot: None,
            read_at: at,
            scene: errol_domain::strategies::Scene::Idle,
            banner: None,
            now: at,
            warning: None,
        });
        assert!((s - 11.25).abs() < 1e-4);
    }
}
