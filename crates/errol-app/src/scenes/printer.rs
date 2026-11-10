//! The resin printer's print: its card in the idle carousel (the slicer's
//! picture, the progress, the layers, the time left), and the vat drawn when
//! that picture is not there. A resin printer prints upside down: the part
//! hangs under the build plate, over the vat of resin lit from below.

use errol_domain::entities::{PictureRef, Printer};
use errol_domain::usecases::View;
use femtovg::{Color, ImageId, Path};

use super::idle::{CARD_W, big, gauge, row, scoreboard};
use super::text;
use crate::look::{Look, fade};
use crate::render::{Anchor, Face, Finish, Gfx, Textures, Type, ink};

/// The slicer's picture in the card: it renders 4:3. The Map's board needs
/// more room under it.
const PICTURE: (f32, f32) = (200.0, 150.0);
const MAP_PICTURE: (f32, f32) = (168.0, 126.0);
/// How much closer the slicer's picture is shown.
const ZOOM: f32 = 1.35;
/// The width pictures are fetched at, the card and the event sharing it.
pub const PICTURE_WIDTH: u32 = 400;

/// The card's content, from (`left`, `top`) inside the frame.
pub fn card(
    gfx: &mut Gfx,
    printer: &Printer,
    view: &View,
    (left, top): (f32, f32),
    kicker: Color,
    age: f32,
    textures: &mut Textures,
) {
    let state = printer.state.as_deref().map(text::print_state);
    gfx.kicker(
        left,
        top,
        &state.map_or_else(
            || "Impression 3D".to_owned(),
            |s| format!("Impression 3D · {s}"),
        ),
        kicker,
    );
    let (pw, ph) = if gfx.hogwarts() { MAP_PICTURE } else { PICTURE };
    let y = top + 28.0;
    let done = printer.progress.unwrap_or_default();
    picture(
        gfx,
        printer.image.as_ref(),
        (left, y, pw, ph),
        done,
        textures,
    );
    let (x, w) = (left + pw + 18.0, CARD_W - 44.0 - pw - 18.0);
    if !printer.has_print() {
        // on, but the cloud says nothing of a print
        let note = printer.cloud.as_deref().map_or("", text::print_cloud);
        side(gfx, (x, y, w), "Imprimante allumée", note, age);
        return;
    }
    let file = text::release(printer.file.as_deref().unwrap_or("–"));
    if gfx.hogwarts() {
        side(gfx, (x, y, w), &file, &printer.name, age);
        ledger(gfx, printer, view, (left, y + ph + 14.0));
    } else {
        big(gfx, x, y - 4.0, &format!("{done:.0} %"), gfx.palette.accent);
        side(gfx, (x, y + 56.0, w), &file, &printer.name, age);
        rows(gfx, printer, (left, y + ph + 18.0));
    }
}

/// The file's name (or what stands for it) and a note under it, in the
/// column right of the picture.
/// How the column right of the picture is written, by look: its title's
/// type, in capitals or not, and the face of the note.
fn side_type(look: Look) -> (Type, bool, Face) {
    match look {
        Look::Cyberpunk => (Type::new(Face::Bold, 22.0), true, Face::Body),
        Look::Hogwarts => (Type::new(Face::Display, 24.0), false, Face::Italic),
        Look::Friday => (Type::new(Face::Display, 17.0), false, Face::Body),
    }
}

fn side(gfx: &mut Gfx, (x, y, w): (f32, f32, f32), title: &str, note: &str, age: f32) {
    let palette = gfx.palette;
    let (style, capitals, face) = side_type(gfx.look);
    let title = if capitals {
        title.to_uppercase()
    } else {
        title.to_owned()
    };
    // ink on the Map, the text colour elsewhere
    let colour = [palette.text, palette.accent][usize::from(gfx.hogwarts())];
    let glyphs = gfx.glyphs();
    let lines = gfx.wrap(&title, w, style, 2);
    let line = style.size * 1.25;
    for (index, row) in lines.iter().enumerate() {
        let shown = crate::anim::decode(row, age, 0.7, glyphs);
        gfx.title(
            x,
            y + index as f32 * line,
            &shown,
            style,
            colour,
            Anchor::TOP_LEFT,
        );
    }
    gfx.paragraph(
        x,
        y + lines.len() as f32 * line + 6.0,
        note,
        w,
        Type::new(face, 15.0),
        19.0,
        palette.muted,
        3,
    );
}

/// The layers and the time left: a gauge and two rows (friday, cyberpunk).
fn rows(gfx: &mut Gfx, printer: &Printer, (left, y): (f32, f32)) {
    let text_colour = gfx.palette.text;
    let width = CARD_W - 44.0;
    gauge(gfx, left, y, width, printer.progress.unwrap_or_default());
    row(gfx, left, y + 20.0, "Couche", &layers(printer), text_colour);
    let (label, value) = if printer.active {
        ("Reste", left_and_end(printer))
    } else {
        (
            "Dernière impression",
            printer
                .state
                .as_deref()
                .map_or("–", text::print_state)
                .to_owned(),
        )
    };
    row(gfx, left, y + 52.0, label, &value, text_colour);
}

/// The score of the print on a Quidditch board, and the sand of the gauge
/// under it (the Map).
fn ledger(gfx: &mut Gfx, printer: &Printer, view: &View, (left, y): (f32, f32)) {
    let width = CARD_W - 44.0;
    let done = printer.progress.unwrap_or_default();
    let plates = [
        (format!("{done:.0} %"), "avancement".to_owned(), true),
        (layers(printer), "couches".to_owned(), true),
        (
            printer
                .remaining
                .filter(|_| printer.active)
                .map_or_else(|| "–".to_owned(), text::hours),
            "restant".to_owned(),
            printer.active,
        ),
    ];
    let used = scoreboard(gfx, left, y, width, &plates, 3);
    gauge(gfx, left, y + used + 2.0, width, done);
    let note = match printer.ends.filter(|_| printer.active) {
        Some(ends) if ends > view.now => format!("achevée vers {}", text::clock(ends)),
        _ => printer
            .state
            .as_deref()
            .map_or("", text::print_state)
            .to_owned(),
    };
    gfx.text(
        left,
        y + used + 19.0,
        &note,
        Type::new(Face::Italic, 14.0),
        gfx.palette.muted,
        Anchor::TOP_LEFT,
    );
}

/// 166 / 2978
fn layers(printer: &Printer) -> String {
    match (printer.layer, printer.layers) {
        (Some(layer), Some(layers)) => format!("{layer} / {layers}"),
        (None, Some(layers)) => layers.to_string(),
        _ => "–".into(),
    }
}

/// 4 h 24 · fin à 23:10
fn left_and_end(printer: &Printer) -> String {
    let left = printer
        .remaining
        .map_or_else(|| "–".to_owned(), text::hours);
    match printer.ends {
        Some(ends) => format!("{left} · fin à {}", text::clock(ends)),
        None => left,
    }
}

/// The slicer's picture in a box, framed as the look frames its pictures (a
/// photograph of the Gazette on the Map); the vat while it loads or without one.
fn picture(
    gfx: &mut Gfx,
    image: Option<&PictureRef>,
    (x, y, w, h): (f32, f32, f32, f32),
    done: f64,
    textures: &mut Textures,
) {
    let map = gfx.hogwarts();
    let accent = gfx.palette.accent;
    if let Some(image) = image.and_then(|image| load(gfx, image, textures)) {
        shot(gfx, image, (x, y, w, h), 1.0);
        return;
    }
    if !map {
        gfx.rect(x, y, w, h, fade(accent, 0.06));
        gfx.rect_line(x, y, w, h, fade(accent, 0.3), 1.0);
    }
    vat(gfx, x + w / 2.0, y + h / 2.0, h / 130.0, done);
}

/// The slicer's picture on the GPU, sepia on the Map; None while it loads.
pub fn load(gfx: &mut Gfx, image: &PictureRef, textures: &mut Textures) -> Option<ImageId> {
    let finish = if gfx.hogwarts() {
        Finish::Print
    } else {
        Finish::Plain
    };
    textures.get(gfx.canvas, image, PICTURE_WIDTH, finish)
}

/// The slicer's picture closer in (it leaves wide margins around the part),
/// cut to its box and framed: pinned and printed in halftone on the Map.
pub fn shot(gfx: &mut Gfx, image: ImageId, (x, y, w, h): (f32, f32, f32, f32), alpha: f32) {
    let (zw, zh) = (w * ZOOM, h * ZOOM);
    let (zx, zy) = (x - (zw - w) / 2.0, y - (zh - h) / 2.0);
    gfx.canvas.save();
    gfx.canvas.intersect_scissor(x, y, w, h);
    if gfx.hogwarts() {
        gfx.photo(image, zx, zy, zw, zh, alpha);
    } else {
        gfx.picture(image, zx, zy, zw, zh, alpha);
    }
    gfx.canvas.restore();
    if gfx.hogwarts() {
        gfx.rect_line(x, y, w, h, fade(ink::news_ink(), alpha), 1.0);
        gfx.pin(x + w / 2.0, y + 2.0);
    } else {
        gfx.rect_line(x, y, w, h, fade(gfx.palette.accent, 0.3 * alpha), 1.0);
    }
}

/// The printer at work, about 130 × 120 at `scale` 1 around (`cx`, `cy`):
/// the arm, the build plate, the part hanging under it, `done` percent of
/// its height, dipping into the vat lit from below.
pub fn vat(gfx: &mut Gfx, cx: f32, cy: f32, scale: f32, done: f64) {
    let palette = gfx.palette;
    let line = palette.accent;
    let s = scale;
    let plate_y = cy - 40.0 * s;
    let part = 52.0 * s * (done as f32 / 100.0).clamp(0.0, 1.0);
    // the UV light under the vat, flickering as each layer is cured
    let flicker = 0.75 + 0.25 * (gfx.t * 5.0).sin().abs();
    let glow = if gfx.hogwarts() {
        palette.accent2
    } else {
        Color::rgb(0x8a, 0x5c, 0xff)
    };
    // no light on paper
    if !gfx.hogwarts() {
        gfx.halo(cx, cy + 46.0 * s, 46.0 * s * flicker, fade(glow, 0.35));
    }
    // the resin in its vat
    gfx.rect(
        cx - 50.0 * s,
        cy + 22.0 * s,
        100.0 * s,
        20.0 * s,
        fade(palette.accent2, 0.3),
    );
    let mut tank = Path::new();
    tank.move_to(cx - 54.0 * s, cy + 12.0 * s);
    tank.line_to(cx - 54.0 * s, cy + 44.0 * s);
    tank.line_to(cx + 54.0 * s, cy + 44.0 * s);
    tank.line_to(cx + 54.0 * s, cy + 12.0 * s);
    gfx.stroke(&tank, line, 2.0);
    // the arm and the plate, the part under it
    gfx.rect(cx + 44.0 * s, cy - 62.0 * s, 4.0 * s, 26.0 * s, line);
    gfx.rect(cx - 40.0 * s, plate_y, 88.0 * s, 5.0 * s, line);
    if part > 0.0 {
        gfx.rect(
            cx - 24.0 * s,
            plate_y + 5.0 * s,
            48.0 * s,
            part,
            fade(line, 0.55),
        );
        // the layer being cured, bright
        gfx.rect(
            cx - 24.0 * s,
            plate_y + 5.0 * s + part - 2.0,
            48.0 * s,
            2.0,
            fade(glow, flicker),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn printing() -> Printer {
        Printer {
            name: "M7".into(),
            state: Some("printing".into()),
            active: true,
            layer: Some(166),
            layers: Some(2978),
            remaining: Some(15_840.0),
            ..Printer::default()
        }
    }

    #[test]
    fn layers_done_out_of_all() {
        assert_eq!(layers(&printing()), "166 / 2978");
        let unknown = Printer {
            layer: None,
            ..printing()
        };
        assert_eq!(layers(&unknown), "2978");
        assert_eq!(layers(&Printer::default()), "–");
    }

    #[test]
    fn time_left_and_end() {
        assert_eq!(left_and_end(&printing()), "4 h 24");
        assert_eq!(left_and_end(&Printer::default()), "–");
        let ends = Printer {
            ends: Some(errol_domain::entities::Timestamp(0)),
            ..printing()
        };
        assert!(left_and_end(&ends).starts_with("4 h 24 · fin à "));
    }

    #[test]
    fn cyberpunk_writes_its_titles_in_capitals() {
        assert!(side_type(Look::Cyberpunk).1);
        assert!(!side_type(Look::Friday).1);
        assert_eq!(side_type(Look::Hogwarts).2, Face::Italic);
    }
}
