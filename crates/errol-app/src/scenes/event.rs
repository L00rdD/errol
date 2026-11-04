//! An event takes the screen: a picture of what happened on the left (a lamp,
//! the projection screen, the projector's beam, a poster, a check, a chapter
//! number, the room of a Claude Code session as Cerebro draws it, the slicer's
//! picture of a print), what it is on the right, and a bar draining until the
//! next scene.
//! On the Map it is a notice of the Gazette pinned over the castle, its tags
//! stamped in red ink.

use std::f32::consts::TAU;

use errol_domain::entities::{
    Cerebro, Chapter, Event, EventKind, Import, PrintNews, PrintPhase, Session, Snapshot,
};
use errol_domain::usecases::View;
use femtovg::{Color, Paint, Path};

use super::{BOTTOM, TOP, printer, text, xmansion};
use crate::anim::{decode, ease_back, ease_out, progress};
use crate::look::{Look, Palette, fade, rgb};
use crate::render::{Anchor, Face, Finish, Gfx, Textures, Type, ink};

const CX: f32 = 176.0;
const CY: f32 = f32::midpoint(TOP, BOTTOM);
const TEXT_X: f32 = 352.0;
const TEXT_W: f32 = 420.0;
/// The notice pinned to the Map: x, y, w, h.
const NOTICE: (f32, f32, f32, f32) = (30.0, TOP + 14.0, 740.0, BOTTOM - TOP - 40.0);

/// What an event says: (kicker, title, subtitle, tags).
type Words = (String, String, String, Vec<String>);

/// How the words of an event are written in a look.
#[derive(Debug, Clone, Copy)]
struct Lettering {
    kicker: Color,
    title: Type,
    title_colour: Color,
    /// Cyberpunk shouts its titles in capitals.
    capitals: bool,
    /// How wide the words run.
    width: f32,
}

pub fn draw(
    gfx: &mut Gfx,
    view: &View,
    event: &Event,
    age: f32,
    length: f32,
    textures: &mut Textures,
) {
    if gfx.hogwarts() {
        notice(gfx);
    }
    // the picture pops in, overshooting a little
    let pop = 0.4 + 0.6 * ease_back(progress(age, 0.9));
    gfx.canvas.save();
    gfx.canvas.translate(CX, CY);
    gfx.canvas.scale(pop, pop);
    gfx.canvas.translate(-CX, -CY);
    picture(gfx, view, event, age, textures);
    gfx.canvas.restore();
    draw_words(gfx, event, words(event), age);
    timer(gfx, age, length);
}

/// The Gazette's notice pinned over the Map, in a double rule.
fn notice(gfx: &mut Gfx) {
    let (nx, ny, nw, nh) = NOTICE;
    gfx.pinned(nx, ny, nw, nh, -0.35);
    gfx.double_rule(
        nx + 8.0,
        ny + 8.0,
        nw - 16.0,
        nh - 16.0,
        fade(ink::news_ink(), 0.8),
    );
}

/// How far right a word still is, `delay` seconds after the scene came: the
/// words slide in one after the other.
fn slide(age: f32, delay: f32) -> f32 {
    30.0 * (1.0 - ease_out(progress(age - delay, 0.7)))
}

fn lettering(look: Look, palette: &Palette) -> Lettering {
    match look {
        Look::Cyberpunk => Lettering {
            kicker: palette.accent2,
            title: Type::new(Face::Bold, 38.0),
            title_colour: palette.hot,
            capitals: true,
            width: TEXT_W,
        },
        Look::Hogwarts => Lettering {
            kicker: palette.accent,
            title: Type::new(Face::Display, 28.0),
            title_colour: palette.accent,
            capitals: false,
            width: 390.0,
        },
        Look::Friday => Lettering {
            kicker: palette.accent,
            title: Type::new(Face::Display, 28.0),
            title_colour: palette.text,
            capitals: false,
            width: TEXT_W,
        },
    }
}

/// The height of the words: the kicker, the title's lines, the subtitle when
/// there is one, the tags.
fn block_height(lines: usize, line_height: f32, has_sub: bool) -> f32 {
    26.0 + lines as f32 * line_height + 8.0 + if has_sub { 30.0 } else { 0.0 } + 36.0
}

/// The words on the right, centred on the picture: kicker, title, subtitle, tags.
fn draw_words(gfx: &mut Gfx, event: &Event, words: Words, age: f32) {
    let (kicker, title, sub, tags) = words;
    let glyphs = gfx.glyphs();
    let lettering = lettering(gfx.look, &gfx.palette);
    let title = cased(title, lettering.capitals);
    let lines = gfx.wrap(&title, lettering.width, lettering.title, 3);
    let line_height = lettering.title.size * 1.22;
    let mut y = CY - block_height(lines.len(), line_height, !sub.is_empty()) / 2.0;
    gfx.kicker(TEXT_X + slide(age, 0.0), y, &kicker, lettering.kicker);
    y += 26.0;
    for (index, line) in lines.iter().enumerate() {
        let shown = decode(line, age - 0.15, 1.1, glyphs);
        gfx.title(
            TEXT_X + slide(age, 0.15),
            y + index as f32 * line_height,
            &shown,
            lettering.title,
            lettering.title_colour,
            Anchor::TOP_LEFT,
        );
    }
    y += lines.len() as f32 * line_height + 8.0;
    if !sub.is_empty() {
        gfx.text(
            TEXT_X + slide(age, 0.3),
            y,
            &sub,
            Type::new(Face::Italic, 20.0),
            gfx.palette.muted,
            Anchor::TOP_LEFT,
        );
        y += 30.0;
    }
    let mut x = TEXT_X + slide(age, 0.45);
    for tag in tags.iter().chain(std::iter::once(&text::clock(event.at))) {
        x += gfx.tag(x, y + 6.0, tag) + 8.0;
    }
}

/// The title as the look writes it.
fn cased(title: String, capitals: bool) -> String {
    if capitals {
        title.to_uppercase()
    } else {
        title
    }
}

/// Where the bar of the time left runs: under the notice on the Map, along
/// the bottom elsewhere.
fn timer_box(hogwarts: bool) -> (f32, f32, f32) {
    let (nx, ny, nw, nh) = NOTICE;
    if hogwarts {
        (nx + 30.0, ny + nh - 18.0, nw - 60.0)
    } else {
        (26.0, BOTTOM - 10.0, 748.0)
    }
}

/// The time left to this scene, draining.
fn timer(gfx: &mut Gfx, age: f32, length: f32) {
    let left = 1.0 - progress(age, length);
    let (bx, by, bw) = timer_box(gfx.hogwarts());
    gfx.rect(bx, by, bw, 2.0, fade(gfx.palette.accent, 0.1));
    gfx.rect(bx, by, bw * left, 2.0, gfx.palette.accent);
}

/// Draws the picture of the event.
fn picture(gfx: &mut Gfx, view: &View, event: &Event, age: f32, textures: &mut Textures) {
    match &event.kind {
        EventKind::Light { .. } | EventKind::Cover { .. } | EventKind::Projector { .. } => {
            house_picture(gfx, view, &event.kind, age);
        }
        EventKind::Waiting { name, .. } => waiting_picture(gfx, view, name, age, textures),
        EventKind::Print(news) => print(gfx, news, age, textures),
        kind => media_picture(gfx, kind, age, textures),
    }
}

/// A lamp, the projection screen, the projector's beam.
fn house_picture(gfx: &mut Gfx, view: &View, kind: &EventKind, age: f32) {
    match kind {
        EventKind::Light { on, .. } => {
            halos(gfx, age);
            bulb(gfx, *on);
        }
        EventKind::Cover { id, .. } => {
            let (position, moving) = cover_state(view.snapshot.as_deref(), id);
            projection_screen(gfx, position, moving);
        }
        EventKind::Projector { on, app, .. } => projector(gfx, *on, app.as_deref(), age),
        _ => {}
    }
}

/// Where the screen of the event is and whether it moves: the cover with that
/// id, else the first one, else rolled up and still.
fn cover_state(snapshot: Option<&Snapshot>, id: &str) -> (f32, bool) {
    let cover = snapshot.and_then(|s| s.house.as_ref()).and_then(|house| {
        house
            .covers
            .iter()
            .find(|c| c.id == id)
            .or(house.covers.first())
    });
    cover.map_or((0.0, false), |c| (c.position as f32, c.moving))
}

/// A check for a download, a poster for an import, the chapter's number for
/// a scan; nothing for a play (the cinema shows it).
fn media_picture(gfx: &mut Gfx, kind: &EventKind, age: f32, textures: &mut Textures) {
    match kind {
        EventKind::Download { .. } => {
            halos(gfx, age);
            check(gfx, age);
        }
        EventKind::Import(import) => import_picture(gfx, import, age, textures),
        EventKind::Scan(chapter) => {
            halos(gfx, age);
            chapter_number(gfx, &chapter.number.to_string());
        }
        _ => {}
    }
}

/// The poster of what entered the library, or a check while it loads.
fn import_picture(gfx: &mut Gfx, import: &Import, age: f32, textures: &mut Textures) {
    let finish = if gfx.hogwarts() {
        Finish::Print
    } else {
        Finish::Plain
    };
    if let Some(image) = import
        .poster
        .as_ref()
        .and_then(|poster| textures.get(gfx.canvas, poster, 400, finish))
    {
        poster(gfx, image, CX - 100.0, CY - 150.0, 200.0, 300.0, age);
    } else {
        halos(gfx, age);
        check(gfx, age);
    }
}

/// The chapter's number, big: split in cyberpunk, in red ink as the Gazette
/// prints its headlines, glowing in friday.
fn chapter_number(gfx: &mut Gfx, number: &str) {
    if gfx.cyber() {
        gfx.split_text(
            CX,
            CY,
            number,
            Type::new(Face::Bold, 118.0),
            gfx.palette.hot,
            Anchor::MIDDLE_CENTER,
            1.0,
        );
    } else if gfx.hogwarts() {
        gfx.text(
            CX,
            CY,
            number,
            Type::new(Face::Display, 84.0),
            gfx.palette.accent2,
            Anchor::MIDDLE_CENTER,
        );
    } else {
        let colour = gfx.palette.accent2;
        gfx.halo(CX, CY, 150.0, fade(colour, 0.18));
        gfx.glow_text(
            CX,
            CY,
            number,
            Type::new(Face::Display, 78.0),
            colour,
            Anchor::MIDDLE_CENTER,
            2.5,
        );
    }
}

/// The session that waits, when Cerebro drew it.
fn waiting_session<'a>(
    snapshot: Option<&'a Snapshot>,
    name: &str,
) -> Option<(&'a Cerebro, &'a Session)> {
    let cerebro = snapshot.and_then(|s| s.cerebro.as_ref())?;
    cerebro
        .sessions
        .iter()
        .find(|session| session.name == name && session.scene.is_some())
        .map(|session| (cerebro, session))
}

/// The session's card as Cerebro shows it, its lead's "!" blinking; a
/// question mark when Cerebro has no picture of it.
fn waiting_picture(gfx: &mut Gfx, view: &View, name: &str, age: f32, textures: &mut Textures) {
    let Some((cerebro, session)) = waiting_session(view.snapshot.as_deref(), name) else {
        halos(gfx, age);
        question(gfx);
        return;
    };
    let scale = 1.4;
    let (w, h) = (
        xmansion::card_width(cerebro, scale),
        xmansion::card_height(cerebro, scale),
    );
    xmansion::draw_card(
        gfx,
        textures,
        cerebro,
        session,
        CX - w / 2.0,
        CY - h / 2.0,
        scale,
        view.now.0,
    );
}

/// What the event says: (kicker, title, subtitle, tags).
fn words(event: &Event) -> Words {
    match &event.kind {
        EventKind::Light { name, on } => light_words(name, *on),
        EventKind::Cover { name, down, .. } => cover_words(name, *down),
        EventKind::Projector { name, on, app } => projector_words(name, *on, app.as_deref()),
        EventKind::Download { name, size } => (
            "Téléchargement terminé".into(),
            text::release(name),
            String::new(),
            size.map(text::size).into_iter().collect(),
        ),
        EventKind::Import(import) => import_words(import),
        EventKind::Scan(chapter) => scan_words(chapter),
        EventKind::Waiting {
            name,
            project,
            room,
        } => waiting_words(name, project.as_deref(), room.as_deref()),
        EventKind::Play { title } => ("Lecture".into(), title.clone(), String::new(), vec![]),
        EventKind::Print(news) => print_words(event, news),
    }
}

fn light_words(name: &str, on: bool) -> Words {
    let sub = if on {
        "vient de s'allumer"
    } else {
        "vient de s'éteindre"
    };
    ("Maison · éclairage".into(), name.into(), sub.into(), vec![])
}

fn cover_words(name: &str, down: bool) -> Words {
    let sub = if down {
        "descend · séance imminente"
    } else {
        "remonte · fin de séance"
    };
    (
        "Salle de projection".into(),
        name.into(),
        sub.into(),
        vec![],
    )
}

fn projector_words(name: &str, on: bool, app: Option<&str>) -> Words {
    let title = format!("{name} {}", if on { "allumé" } else { "éteint" });
    let sub = match (on, app) {
        (true, Some(app)) => format!("sur {app}"),
        (true, None) => String::new(),
        (false, _) => "la salle se rallume".into(),
    };
    ("Salle de projection".into(), title, sub, vec![])
}

fn import_words(import: &Import) -> Words {
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

fn scan_words(chapter: &Chapter) -> Words {
    (
        "One Piece · nouveau scan".into(),
        chapter.name.clone(),
        chapter
            .arc
            .as_deref()
            .map(|arc| format!("arc {arc}"))
            .unwrap_or_default(),
        chapter
            .pages
            .map(|pages| format!("{pages} pages"))
            .into_iter()
            .collect(),
    )
}

fn waiting_words(name: &str, project: Option<&str>, room: Option<&str>) -> Words {
    let sub = match project {
        Some(project) => format!("attend ta réponse · {project}"),
        None => "attend ta réponse".into(),
    };
    (
        "Claude Code · Cerebro".into(),
        name.into(),
        sub,
        room.map(str::to_owned).into_iter().collect(),
    )
}

/// A print that started, finished or was cancelled: the slicer's picture, or
/// the vat filling (started), a check (done), a cross (cancelled).
fn print(gfx: &mut Gfx, news: &PrintNews, age: f32, textures: &mut Textures) {
    let image = news
        .image
        .as_ref()
        .and_then(|image| printer::load(gfx, image, textures));
    if let Some(image) = image {
        print_picture(gfx, image, age);
        return;
    }
    halos(gfx, age);
    match news.phase {
        PrintPhase::Started => {
            let rising = f64::from(ease_out(progress(age - 0.3, 4.0)));
            printer::vat(gfx, CX, CY, 1.5, 100.0 * rising);
        }
        PrintPhase::Done => check(gfx, age),
        PrintPhase::Cancelled => cross(gfx, age),
    }
}

/// The slicer's picture, in the look's frame.
fn print_picture(gfx: &mut Gfx, image: femtovg::ImageId, age: f32) {
    let (w, h) = (288.0, 216.0);
    let (x, y) = (CX - w / 2.0, CY - h / 2.0);
    if gfx.hogwarts() {
        printer::shot(gfx, image, (x, y, w, h), ease_out(progress(age, 0.6)));
        return;
    }
    gfx.halo(CX, CY, w, fade(gfx.palette.accent, 0.18));
    printer::shot(gfx, image, (x, y, w, h), ease_out(progress(age, 0.6)));
    let corner = if gfx.cyber() {
        gfx.palette.hot
    } else {
        gfx.palette.accent
    };
    gfx.brackets(x - 8.0, y - 8.0, w + 16.0, h + 16.0, 26.0, corner, 2.0);
}

/// What a print event says: (kicker, title, subtitle, tags).
fn print_words(event: &Event, news: &PrintNews) -> Words {
    // how long it took, from its start to the event
    let took = news
        .started
        .map(|started| text::hours(event.at.secs_since(started)));
    let (kicker, sub) = match news.phase {
        PrintPhase::Started => (
            "Impression 3D lancée",
            news.remaining
                .map(|left| format!("fin vers {}", text::clock(event.at.plus_secs(left))))
                .unwrap_or_default(),
        ),
        PrintPhase::Done => (
            "Impression 3D terminée",
            took.map(|took| format!("en {took}")).unwrap_or_default(),
        ),
        PrintPhase::Cancelled => (
            "Impression 3D annulée",
            took.map(|took| format!("après {took}")).unwrap_or_default(),
        ),
    };
    (
        kicker.into(),
        text::release(&news.file),
        sub,
        news.layers
            .map(|layers| format!("{layers} couches"))
            .into_iter()
            .collect(),
    )
}

/// Rings (friday, in ink on the Map) or diamonds (cyberpunk) widening from
/// the centre, one a second.
fn halos(gfx: &mut Gfx, age: f32) {
    let colour = if gfx.cyber() {
        gfx.palette.hot
    } else {
        gfx.palette.accent
    };
    let strength = if gfx.hogwarts() { 0.5 } else { 0.9 };
    for phase in 0..3 {
        let k = ((age + phase as f32) / 3.0).fract();
        let alpha = strength * (1.0 - k);
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

/// A question mark beating like a call: Claude asks something. Split in
/// cyberpunk, in red ink on the Map, glowing in friday.
fn question(gfx: &mut Gfx) {
    let beat = 1.0 + 0.06 * (gfx.t * 3.2).sin().max(0.0);
    gfx.canvas.save();
    gfx.canvas.translate(CX, CY);
    gfx.canvas.scale(beat, beat);
    gfx.canvas.translate(-CX, -CY);
    if gfx.cyber() {
        gfx.split_text(
            CX,
            CY,
            "?",
            Type::new(Face::Bold, 150.0),
            gfx.palette.hot,
            Anchor::MIDDLE_CENTER,
            1.0,
        );
    } else if gfx.hogwarts() {
        gfx.text(
            CX,
            CY,
            "?",
            Type::new(Face::Display, 120.0),
            gfx.palette.accent2,
            Anchor::MIDDLE_CENTER,
        );
    } else {
        let colour = gfx.palette.accent2;
        gfx.halo(CX, CY, 150.0, fade(colour, 0.18));
        gfx.glow_text(
            CX,
            CY,
            "?",
            Type::new(Face::Display, 110.0),
            colour,
            Anchor::MIDDLE_CENTER,
            2.5,
        );
    }
    gfx.canvas.restore();
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
        // on the Map the bulb is drawn in ink, lit from inside
        let line = if gfx.hogwarts() {
            gfx.palette.accent
        } else {
            rgb(0xffe3a3)
        };
        gfx.fill(&glass, fade(warm, 0.35));
        gfx.stroke(&glass, line, 4.0);
        gfx.stroke(&base, line, 4.0);
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
        // the beam is light; on paper, a wash of ink
        let (near, far) = if gfx.hogwarts() {
            (
                fade(palette.accent, 0.22 * beam * flutter),
                fade(palette.accent, 0.04 * beam),
            )
        } else {
            (
                fade(Color::white(), 0.8 * beam * flutter),
                fade(rgb(0xa0e6ff), 0.12 * beam),
            )
        };
        let paint = Paint::linear_gradient(lx, 0.0, CX + 150.0, 0.0, near, far);
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
        if gfx.hogwarts() {
            gfx.halo(lx, ly, 34.0, fade(rgb(0xd9a441), 0.8));
        } else {
            gfx.halo(lx, ly, 34.0, fade(palette.accent, 0.8));
        }
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

/// A circle drawing itself, then a cross: something stopped.
fn cross(gfx: &mut Gfx, age: f32) {
    let hot = gfx.palette.hot;
    let circle = ease_out(progress(age - 0.2, 1.2));
    gfx.ring(CX, CY, 78.0, 4.0, hot, &[], -TAU / 4.0, TAU * circle);
    let bars = ease_out(progress(age - 1.2, 0.6));
    if bars > 0.0 {
        let reach = 30.0 * bars;
        let mut path = Path::new();
        path.move_to(CX - 30.0, CY - 30.0);
        path.line_to(CX - 30.0 + 2.0 * reach, CY - 30.0 + 2.0 * reach);
        path.move_to(CX + 30.0, CY - 30.0);
        path.line_to(CX + 30.0 - 2.0 * reach, CY - 30.0 + 2.0 * reach);
        gfx.halo(CX, CY, 60.0, fade(hot, 0.18 * bars));
        let paint = Paint::color(hot)
            .with_line_width(9.0)
            .with_line_cap(femtovg::LineCap::Round);
        gfx.canvas.stroke_path(&path, &paint);
    }
}

/// A poster in a holographic frame: brackets, a slow sway, a light sweeping
/// down. On the Map, a photograph of the Gazette, pinned.
#[allow(clippy::many_single_char_names)] // a box: x, y, w, h
pub fn poster(gfx: &mut Gfx, image: femtovg::ImageId, x: f32, y: f32, w: f32, h: f32, age: f32) {
    let palette = gfx.palette;
    if gfx.hogwarts() {
        gfx.photo(image, x, y, w, h, ease_out(progress(age, 0.6)));
        gfx.pin(x + w / 2.0, y + 2.0);
        return;
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use errol_domain::entities::{EventId, Timestamp};

    fn news(phase: PrintPhase) -> (Event, PrintNews) {
        let news = PrintNews {
            phase,
            printer: "M7".into(),
            file: "04_riser.v2".into(),
            image: None,
            layers: Some(2978),
            remaining: Some(3_600.0),
            started: Some(Timestamp::from_secs(0)),
        };
        let event = Event {
            id: EventId::new("p"),
            at: Timestamp::from_secs(18_720),
            kind: EventKind::Print(news.clone()),
        };
        (event, news)
    }

    #[test]
    fn a_print_is_told_by_its_phase() {
        let (event, started) = news(PrintPhase::Started);
        let (kicker, title, sub, tags) = print_words(&event, &started);
        assert_eq!(
            (kicker.as_str(), title.as_str()),
            ("Impression 3D lancée", "04 riser v2")
        );
        assert!(sub.starts_with("fin vers "));
        assert_eq!(tags, ["2978 couches"]);
        let (event, done) = news(PrintPhase::Done);
        assert_eq!(print_words(&event, &done).2, "en 5 h 12");
        let (event, cancelled) = news(PrintPhase::Cancelled);
        let (kicker, _, sub, _) = print_words(&event, &cancelled);
        assert_eq!(
            (kicker.as_str(), sub.as_str()),
            ("Impression 3D annulée", "après 5 h 12")
        );
    }

    #[test]
    fn without_times_nothing_under_the_title() {
        let (event, mut done) = news(PrintPhase::Done);
        done.started = None;
        assert_eq!(print_words(&event, &done).2, "");
        let (event, mut started) = news(PrintPhase::Started);
        started.remaining = None;
        assert_eq!(print_words(&event, &started).2, "");
    }

    fn kind(kind: EventKind) -> Event {
        Event {
            id: EventId::new("e"),
            at: Timestamp::from_secs(0),
            kind,
        }
    }

    fn said(event_kind: EventKind) -> Words {
        words(&kind(event_kind))
    }

    #[test]
    fn the_house_says_what_moved() {
        let light = |on| {
            said(EventKind::Light {
                name: "Lampe".into(),
                on,
            })
        };
        assert_eq!(light(true).2, "vient de s'allumer");
        assert_eq!(light(false).2, "vient de s'éteindre");
        let cover = |down| {
            said(EventKind::Cover {
                id: "screen".into(),
                name: "Écran".into(),
                down,
            })
        };
        assert_eq!(cover(true).2, "descend · séance imminente");
        assert_eq!(cover(false).0, "Salle de projection");
        assert_eq!(cover(false).2, "remonte · fin de séance");
        let projector = |on, app: Option<&str>| {
            said(EventKind::Projector {
                name: "Projecteur".into(),
                on,
                app: app.map(Into::into),
            })
        };
        assert_eq!(projector(true, Some("Jellyfin")).1, "Projecteur allumé");
        assert_eq!(projector(true, Some("Jellyfin")).2, "sur Jellyfin");
        assert_eq!(projector(true, None).2, "");
        assert_eq!(projector(false, None).1, "Projecteur éteint");
        assert_eq!(projector(false, None).2, "la salle se rallume");
    }

    #[test]
    fn the_media_say_what_came() {
        let download = said(EventKind::Download {
            name: "Dune.Part.Two".into(),
            size: Some(6_400_000_000),
        });
        assert_eq!(download.0, "Téléchargement terminé");
        assert_eq!(download.1, "Dune Part Two");
        assert_eq!(download.3.len(), 1);
        let mut import = Import {
            kind: errol_domain::entities::MediaKind::Episode,
            title: "Arcane".into(),
            year: Some(2024),
            season: Some(2),
            episode: Some(4),
            episode_title: Some("Paris".into()),
            quality: Some("WEBDL-1080p".into()),
            poster: None,
        };
        let episode = said(EventKind::Import(import.clone()));
        assert_eq!(episode.0, "Nouvel épisode dans la bibliothèque");
        assert_eq!(episode.2, "Saison 2 · épisode 4 — Paris");
        assert_eq!(episode.3, ["WEBDL-1080p"]);
        import.season = None;
        let film = said(EventKind::Import(import));
        assert_eq!(film.0, "Nouveau film dans la bibliothèque");
        assert_eq!(film.2, "2024");
        let scan = said(EventKind::Scan(Chapter {
            number: 1197,
            name: "Loki".into(),
            arc: Some("Elbaf".into()),
            pages: Some(17),
        }));
        assert_eq!(
            scan,
            (
                "One Piece · nouveau scan".to_owned(),
                "Loki".to_owned(),
                "arc Elbaf".to_owned(),
                vec!["17 pages".to_owned()]
            )
        );
        let play = said(EventKind::Play {
            title: "Dune".into(),
        });
        assert_eq!((play.0.as_str(), play.1.as_str()), ("Lecture", "Dune"));
    }

    #[test]
    fn a_waiting_session_names_its_project_and_room() {
        let waiting = |project: Option<&str>| {
            said(EventKind::Waiting {
                name: "errol-8b".into(),
                project: project.map(Into::into),
                room: Some("Danger Room".into()),
            })
        };
        assert_eq!(waiting(Some("~/errol")).2, "attend ta réponse · ~/errol");
        assert_eq!(waiting(None).2, "attend ta réponse");
        assert_eq!(waiting(None).3, ["Danger Room"]);
        let (event, news) = news(PrintPhase::Done);
        assert_eq!(words(&event), print_words(&event, &news));
    }

    #[test]
    fn the_screen_of_the_event_is_found_or_the_first() {
        use errol_domain::entities::{Cover, House};
        let cover = |id: &str, position| Cover {
            id: id.into(),
            name: id.into(),
            position,
            moving: true,
        };
        let snapshot = Snapshot {
            house: Some(House {
                outlets: Vec::new(),
                covers: vec![cover("a", 30.0), cover("b", 100.0)],
                projectors: Vec::new(),
            }),
            ..Snapshot::default()
        };
        assert_eq!(cover_state(Some(&snapshot), "b"), (100.0, true));
        assert_eq!(cover_state(Some(&snapshot), "z"), (30.0, true));
        assert_eq!(cover_state(None, "b"), (0.0, false));
    }

    #[test]
    fn a_session_shows_only_when_cerebro_drew_it() {
        use errol_domain::entities::{PictureRef, SessionScene, SessionStatus};
        let session = |name: &str, drawn: bool| Session {
            name: name.into(),
            project: None,
            status: SessionStatus::Waiting,
            since: None,
            room: None,
            lead: None,
            agents: 0,
            scene: drawn.then(|| SessionScene {
                room: PictureRef::new("/room.png"),
                frames: 1,
                fps: 1,
                walk: (0, 0, 1, 1),
                actors: Vec::new(),
            }),
        };
        let snapshot = Snapshot {
            cerebro: Some(Cerebro {
                sessions: vec![session("plain", false), session("drawn", true)],
                room_size: (192, 120),
                character_size: (24, 24),
                emotes: None,
            }),
            ..Snapshot::default()
        };
        assert!(waiting_session(Some(&snapshot), "drawn").is_some());
        assert!(waiting_session(Some(&snapshot), "plain").is_none());
        assert!(waiting_session(None, "drawn").is_none());
    }

    #[test]
    fn the_words_are_laid_out_per_look() {
        let palette = Look::Cyberpunk.palette();
        let cyber = lettering(Look::Cyberpunk, &palette);
        assert!(cyber.capitals);
        assert_eq!(cyber.title_colour, palette.hot);
        let hogwarts = lettering(Look::Hogwarts, &palette);
        assert_eq!(hogwarts.title_colour, palette.accent);
        assert!((hogwarts.width - 390.0).abs() < f32::EPSILON);
        let friday = lettering(Look::Friday, &palette);
        assert!(!friday.capitals);
        assert_eq!(friday.title_colour, palette.text);
        assert_eq!(cased("Dune".into(), true), "DUNE");
        assert_eq!(cased("Dune".into(), false), "Dune");
        assert!((block_height(2, 10.0, true) - 120.0).abs() < 1e-4);
        assert!((block_height(1, 10.0, false) - 80.0).abs() < 1e-4);
        assert!((slide(10.0, 0.0)).abs() < 1e-4);
        assert!((slide(0.0, 0.3) - 30.0).abs() < 1e-4);
        assert_eq!(timer_box(false), (26.0, BOTTOM - 10.0, 748.0));
        assert!((timer_box(true).2 - 680.0).abs() < 1e-4);
    }
}
