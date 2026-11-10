//! The line scrolling at the bottom: One Piece, a trading bot, downloads, the
//! print on the resin printer, the latest additions, the free space.

use errol_domain::entities::{MediaKind, Snapshot};
use errol_domain::usecases::View;

use super::text;
use femtovg::Color;

use crate::look::{Palette, fade};
use crate::render::{Anchor, Face, Gfx, Type};

/// Pixels per second.
const SPEED: f32 = 42.0;

#[derive(Default)]
pub struct Ticker {
    offset: f32,
}

/// How the line prints a label and its news.
#[derive(Debug, Clone, Copy)]
struct Styles {
    label: Type,
    value: Type,
    label_colour: Color,
}

/// Space between two items.
const GAP: f32 = 46.0;
/// Where the line's text sits.
const LINE_Y: f32 = 467.5;

impl Ticker {
    pub fn draw(&mut self, gfx: &mut Gfx, view: &View, dt: f32) {
        rule(gfx);
        let Some(snapshot) = &view.snapshot else {
            return;
        };
        let items = items(snapshot);
        let styles = styles(gfx.hogwarts(), &gfx.palette);
        let widths: Vec<f32> = items
            .iter()
            .map(|(label, value)| {
                gfx.measure(label, styles.label) + 10.0 + gfx.measure(value, styles.value) + GAP
            })
            .collect();
        let total: f32 = widths.iter().sum();
        if total <= 0.0 {
            return;
        }
        self.offset = (self.offset + SPEED * dt) % total;
        gfx.canvas.save();
        gfx.canvas.scissor(0.0, 455.0, 800.0, 25.0);
        for (index, x) in placements(&widths, 800.0 - self.offset) {
            let (label, value) = &items[index];
            draw_item(gfx, x, widths[index], label, value, styles);
        }
        gfx.canvas.restore();
    }
}

/// Where the items on the screen go, the first at `x`: (index, x). The items
/// run twice, so the line never runs out.
fn placements(widths: &[f32], mut x: f32) -> Vec<(usize, f32)> {
    let mut placed = Vec::new();
    for _ in 0..2 {
        for (index, width) in widths.iter().enumerate() {
            if visible(x, *width) {
                placed.push((index, x));
            }
            x += width;
        }
        if x > 800.0 {
            break;
        }
    }
    placed
}

/// The line's top edge: a dashed rule on the Map, a fade and a line elsewhere.
fn rule(gfx: &mut Gfx) {
    let palette = gfx.palette;
    if gfx.hogwarts() {
        // the news run along the bottom of the Map, under a dashed rule
        gfx.dashed_rule(16.0, 454.5, 768.0, fade(palette.accent, 0.55));
        return;
    }
    let line = if gfx.cyber() {
        palette.accent
    } else {
        fade(palette.accent, 0.1)
    };
    gfx.vertical_gradient(0.0, 454.0, 800.0, 26.0, fade(palette.bg, 0.0), palette.bg);
    gfx.rect(0.0, 454.0, 800.0, 1.0, line);
}

/// Hogwarts prints the labels in red and the news in an italic hand.
fn styles(hogwarts: bool, palette: &Palette) -> Styles {
    if hogwarts {
        Styles {
            label: Type::new(Face::Body, 12.0).spaced(1.6),
            value: Type::new(Face::Italic, 14.0),
            label_colour: palette.accent2,
        }
    } else {
        Styles {
            label: Type::new(Face::Strong, 11.0).spaced(1.6),
            value: Type::new(Face::Body, 12.5).spaced(0.4),
            label_colour: palette.accent,
        }
    }
}

/// Whether an item `width` wide at `x` shows on the screen.
fn visible(x: f32, width: f32) -> bool {
    x < 800.0 && x + width > 0.0
}

/// One item: its label, its news, and the mark that separates it from the next.
fn draw_item(gfx: &mut Gfx, x: f32, width: f32, label: &str, value: &str, styles: Styles) {
    let palette = gfx.palette;
    let label_width = gfx.text(
        x,
        LINE_Y,
        label,
        styles.label,
        styles.label_colour,
        Anchor::MIDDLE_LEFT,
    );
    gfx.text(
        x + label_width + 10.0,
        LINE_Y,
        value,
        styles.value,
        palette.muted,
        Anchor::MIDDLE_LEFT,
    );
    if gfx.hogwarts() {
        gfx.fleuron(
            x + width - GAP / 2.0,
            LINE_Y,
            10.0,
            fade(palette.accent, 0.7),
        );
    } else {
        gfx.disc(
            x + width - GAP / 2.0,
            LINE_Y,
            2.0,
            fade(palette.accent2, 0.7),
        );
    }
}

/// The news of the line: (label, value), in the order they run.
fn items(snapshot: &Snapshot) -> Vec<(String, String)> {
    let mut items: Vec<(String, String)> = Vec::new();
    items.extend(one_piece(snapshot));
    items.extend(portfolio(snapshot));
    items.extend(downloads(snapshot));
    items.extend(print(snapshot));
    items.extend(recent(snapshot));
    items.extend(disk(snapshot));
    items
}

fn one_piece(snapshot: &Snapshot) -> Option<(String, String)> {
    let one_piece = snapshot.one_piece.as_ref()?;
    let next = one_piece
        .next
        .as_ref()
        .map(|next| format!(" · {} {}", next.number, text::weekday(next.at)))
        .unwrap_or_default();
    Some((
        "ONE PIECE".into(),
        format!(
            "scan {} « {} »{next}",
            one_piece.latest.number, one_piece.latest.name
        ),
    ))
}

fn portfolio(snapshot: &Snapshot) -> Option<(String, String)> {
    let portfolio = snapshot.portfolio.as_ref()?;
    Some((
        portfolio.name.to_uppercase(),
        format!(
            "{:.2} $ ({}) · {} trades",
            portfolio.equity,
            text::percent(portfolio.change()),
            portfolio.trades
        ),
    ))
}

fn downloads(snapshot: &Snapshot) -> Vec<(String, String)> {
    snapshot
        .downloads
        .iter()
        .map(|download| {
            (
                "FLUX".into(),
                format!(
                    "{} {:.0} %",
                    text::release(&download.name),
                    download.percent
                ),
            )
        })
        .collect()
}

/// The print going on, while there is one.
fn print(snapshot: &Snapshot) -> Option<(String, String)> {
    let printer = snapshot.printer.as_ref().filter(|printer| printer.active)?;
    let left = printer
        .remaining
        .map(|left| format!(" · reste {}", text::hours(left)))
        .unwrap_or_default();
    Some((
        "IMPRESSION".into(),
        format!(
            "{} {:.0} %{left}",
            text::release(printer.file.as_deref().unwrap_or("")),
            printer.progress.unwrap_or_default()
        ),
    ))
}

/// The three latest additions to the library.
fn recent(snapshot: &Snapshot) -> Vec<(String, String)> {
    snapshot
        .recent
        .iter()
        .take(3)
        .map(|import| {
            let detail = match import.kind {
                MediaKind::Episode => format!(" {}", text::episode(import.season, import.episode)),
                MediaKind::Movie => import
                    .year
                    .map(|year| format!(" ({year})"))
                    .unwrap_or_default(),
            };
            ("AJOUTÉ".into(), format!("{}{detail}", import.title))
        })
        .collect()
}

fn disk(snapshot: &Snapshot) -> Option<(String, String)> {
    let disk = snapshot.disk?;
    Some((
        "STOCKAGE".into(),
        format!("{} libres", text::size(disk.free)),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use errol_domain::entities::{Disk, Download, Printer};

    #[test]
    fn a_print_going_on_runs_on_the_line() {
        let mut snapshot = Snapshot {
            downloads: vec![Download {
                name: "Dune".into(),
                percent: 50.0,
                eta: None,
            }],
            disk: Some(Disk {
                total: 2,
                used: 1,
                free: 1_000_000_000,
            }),
            printer: Some(Printer {
                name: "M7".into(),
                state: Some("printing".into()),
                active: true,
                file: Some("04_riser".into()),
                progress: Some(37.4),
                remaining: Some(3_660.0),
                ..Printer::default()
            }),
            ..Snapshot::default()
        };
        let labels: Vec<_> = items(&snapshot).into_iter().collect();
        assert_eq!(
            labels[1],
            ("IMPRESSION".into(), "04 riser 37 % · reste 1 h 01".into())
        );
        assert_eq!(labels.len(), 3);
        // a print over leaves the line
        snapshot.printer.as_mut().unwrap().active = false;
        assert_eq!(items(&snapshot).len(), 2);
    }

    #[test]
    fn every_source_has_its_item_in_order() {
        use errol_domain::entities::{
            Chapter, Import, NextChapter, OnePiece, Portfolio, Timestamp,
        };
        let import = |kind, title: &str| Import {
            kind,
            title: title.into(),
            year: Some(2024),
            season: Some(2),
            episode: Some(4),
            episode_title: None,
            quality: None,
            poster: None,
        };
        let snapshot = Snapshot {
            one_piece: Some(OnePiece {
                latest: Chapter {
                    number: 1196,
                    name: "Elbaf".into(),
                    arc: None,
                    pages: None,
                },
                next: Some(NextChapter {
                    number: 1197,
                    at: Timestamp::from_secs(0),
                }),
            }),
            portfolio: Some(Portfolio {
                name: "Bot".into(),
                mode: "paper".into(),
                equity: 1100.0,
                initial: 1000.0,
                trades: 12,
                positions: Vec::new(),
            }),
            recent: vec![
                import(MediaKind::Movie, "Dune"),
                import(MediaKind::Episode, "Arcane"),
                import(MediaKind::Movie, "Alien"),
                import(MediaKind::Movie, "Heat"),
            ],
            ..Snapshot::default()
        };
        let news = items(&snapshot);
        let labels: Vec<_> = news.iter().map(|(label, _)| label.as_str()).collect();
        assert_eq!(labels, ["ONE PIECE", "BOT", "AJOUTÉ", "AJOUTÉ", "AJOUTÉ"]);
        assert!(news[0].1.starts_with("scan 1196 « Elbaf » · 1197 "));
        assert!(news[1].1.starts_with("1100.00 $ ("));
        assert!(news[1].1.ends_with(") · 12 trades"));
        assert_eq!(news[2].1, "Dune (2024)");
        assert!(news[3].1.starts_with("Arcane "));
        assert_eq!(items(&Snapshot::default()), []);
    }

    #[test]
    fn the_line_runs_twice_so_it_never_runs_out() {
        assert_eq!(placements(&[300.0, 300.0], 500.0), [(0, 500.0)]);
        assert_eq!(
            placements(&[300.0, 300.0], -250.0),
            [(0, -250.0), (1, 50.0), (0, 350.0), (1, 650.0)]
        );
    }

    #[test]
    fn only_items_on_the_screen_are_drawn() {
        assert!(visible(10.0, 50.0));
        assert!(visible(-40.0, 50.0));
        assert!(!visible(-50.0, 50.0));
        assert!(!visible(800.0, 50.0));
    }

    #[test]
    fn hogwarts_writes_the_news_in_italics_and_the_labels_in_red() {
        let palette = crate::look::Look::Hogwarts.palette();
        let hogwarts = styles(true, &palette);
        assert_eq!(hogwarts.value.face, Face::Italic);
        assert_eq!(hogwarts.label_colour, palette.accent2);
        let friday = styles(false, &palette);
        assert_eq!(friday.label.face, Face::Strong);
        assert_eq!(friday.label_colour, palette.accent);
    }
}
