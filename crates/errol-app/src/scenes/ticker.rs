//! The line scrolling at the bottom: One Piece, a trading bot, downloads, the
//! latest additions, the free space.

use errol_domain::entities::MediaKind;
use errol_domain::usecases::View;

use super::text;
use crate::look::fade;
use crate::render::{Anchor, Face, Gfx, Type};

/// Pixels per second.
const SPEED: f32 = 42.0;

#[derive(Default)]
pub struct Ticker {
    offset: f32,
}

impl Ticker {
    pub fn draw(&mut self, gfx: &mut Gfx, view: &View, dt: f32) {
        let palette = gfx.palette;
        if gfx.hogwarts() {
            // the news run along the bottom of the Map, under a dashed rule
            gfx.dashed_rule(16.0, 454.5, 768.0, fade(palette.accent, 0.55));
        } else {
            let line = if gfx.cyber() {
                palette.accent
            } else {
                fade(palette.accent, 0.1)
            };
            gfx.vertical_gradient(0.0, 454.0, 800.0, 26.0, fade(palette.bg, 0.0), palette.bg);
            gfx.rect(0.0, 454.0, 800.0, 1.0, line);
        }
        let Some(snapshot) = &view.snapshot else {
            return;
        };
        let mut items: Vec<(String, String)> = Vec::new();
        if let Some(one_piece) = &snapshot.one_piece {
            let next = one_piece
                .next
                .as_ref()
                .map(|next| format!(" · {} {}", next.number, text::weekday(next.at)))
                .unwrap_or_default();
            items.push((
                "ONE PIECE".into(),
                format!(
                    "scan {} « {} »{next}",
                    one_piece.latest.number, one_piece.latest.name
                ),
            ));
        }
        if let Some(portfolio) = &snapshot.portfolio {
            items.push((
                portfolio.name.to_uppercase(),
                format!(
                    "{:.2} $ ({}) · {} trades",
                    portfolio.equity,
                    text::percent(portfolio.change()),
                    portfolio.trades
                ),
            ));
        }
        for download in &snapshot.downloads {
            items.push((
                "FLUX".into(),
                format!(
                    "{} {:.0} %",
                    text::release(&download.name),
                    download.percent
                ),
            ));
        }
        for import in snapshot.recent.iter().take(3) {
            let detail = match import.kind {
                MediaKind::Episode => format!(" {}", text::episode(import.season, import.episode)),
                MediaKind::Movie => import
                    .year
                    .map(|year| format!(" ({year})"))
                    .unwrap_or_default(),
            };
            items.push(("AJOUTÉ".into(), format!("{}{detail}", import.title)));
        }
        if let Some(disk) = snapshot.disk {
            items.push((
                "STOCKAGE".into(),
                format!("{} libres", text::size(disk.free)),
            ));
        }
        // hogwarts prints the labels in red and the news in an italic hand
        let (label_style, value_style, label_colour) = if gfx.hogwarts() {
            (
                Type::new(Face::Body, 12.0).spaced(1.6),
                Type::new(Face::Italic, 14.0),
                palette.accent2,
            )
        } else {
            (
                Type::new(Face::Strong, 11.0).spaced(1.6),
                Type::new(Face::Body, 12.5).spaced(0.4),
                palette.accent,
            )
        };
        let gap = 46.0;
        let widths: Vec<f32> = items
            .iter()
            .map(|(label, value)| {
                gfx.measure(label, label_style) + 10.0 + gfx.measure(value, value_style) + gap
            })
            .collect();
        let total: f32 = widths.iter().sum();
        if total <= 0.0 {
            return;
        }
        self.offset = (self.offset + SPEED * dt) % total;
        gfx.canvas.save();
        gfx.canvas.scissor(0.0, 455.0, 800.0, 25.0);
        let mut x = 800.0 - self.offset;
        // two passes so the line never runs out
        for _ in 0..2 {
            for ((label, value), width) in items.iter().zip(&widths) {
                if x < 800.0 && x + width > 0.0 {
                    let label_width = gfx.text(
                        x,
                        467.5,
                        label,
                        label_style,
                        label_colour,
                        Anchor::MIDDLE_LEFT,
                    );
                    gfx.text(
                        x + label_width + 10.0,
                        467.5,
                        value,
                        value_style,
                        palette.muted,
                        Anchor::MIDDLE_LEFT,
                    );
                    if gfx.hogwarts() {
                        gfx.fleuron(
                            x + width - gap / 2.0,
                            467.5,
                            10.0,
                            fade(palette.accent, 0.7),
                        );
                    } else {
                        gfx.disc(
                            x + width - gap / 2.0,
                            467.5,
                            2.0,
                            fade(palette.accent2, 0.7),
                        );
                    }
                }
                x += width;
            }
            if x > 800.0 {
                break;
            }
        }
        gfx.canvas.restore();
    }
}
