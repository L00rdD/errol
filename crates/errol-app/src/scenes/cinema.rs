//! A video plays on Jellyfin: its backdrop, blurred and drifting, behind the
//! poster, the title, the story, and where the playback is (a braindance
//! editor's tracks in cyberpunk). On the Map it is the Gazette's front page,
//! pinned over the castle, the poster its photograph.

use errol_domain::entities::{MediaKind, Playing};
use errol_domain::usecases::View;
use femtovg::{Color, Paint, Path};

use super::event::poster;
use super::{BOTTOM, HEIGHT, TOP, WIDTH, text};
use crate::anim::{decode, ease_in_out, ease_out, progress};
use crate::look::{fade, mix, rgb};
use crate::render::{Anchor, Face, Finish, Gfx, Textures, Type, ink};

const INFO_X: f32 = 266.0;
const INFO_W: f32 = 504.0;
/// The Gazette's page pinned to the Map: x, y, w, h.
const PAGE: (f32, f32, f32, f32) = (30.0, TOP + 12.0, 740.0, BOTTOM - TOP - 28.0);

pub fn draw(gfx: &mut Gfx, view: &View, playing: &Playing, age: f32, textures: &mut Textures) {
    if gfx.hogwarts() {
        gazette(gfx, view, playing, age, textures);
        return;
    }
    let palette = gfx.palette;
    if let Some(image) = playing
        .backdrop
        .as_ref()
        .and_then(|backdrop| textures.get(gfx.canvas, backdrop, 480, Finish::Blur))
    {
        // a slow drift, 40 s each way
        let k = ease_in_out(((gfx.t / 40.0) % 2.0 - 1.0).abs());
        let zoom = 1.06 + 0.08 * k;
        gfx.canvas.save();
        gfx.canvas
            .translate(WIDTH / 2.0 - 12.0 * k, HEIGHT / 2.0 + 6.0 * k);
        gfx.canvas.scale(zoom, zoom);
        gfx.canvas.translate(-WIDTH / 2.0, -HEIGHT / 2.0);
        gfx.picture(
            image,
            0.0,
            0.0,
            WIDTH,
            HEIGHT,
            0.4 * ease_out(progress(age, 1.5)),
        );
        gfx.canvas.restore();
    }
    let mut vignette = Path::new();
    vignette.rect(0.0, 0.0, WIDTH, HEIGHT);
    let paint = Paint::radial_gradient(
        WIDTH * 0.3,
        HEIGHT * 0.5,
        120.0,
        620.0,
        fade(palette.bg, 0.0),
        fade(palette.bg, 0.75),
    );
    gfx.canvas.fill_path(&vignette, &paint);

    let (px, py, pw, ph) = (30.0, TOP + 58.0, 200.0, 300.0);
    gfx.rect(px, py, pw, ph, fade(palette.accent, 0.06));
    if let Some(image) = playing
        .poster
        .as_ref()
        .and_then(|p| textures.get(gfx.canvas, p, 400, Finish::Plain))
    {
        poster(gfx, image, px, py, pw, ph, age);
    }

    let slide = |delay: f32| 30.0 * (1.0 - ease_out(progress(age - delay, 0.9)));
    let mut y = TOP + 34.0;
    let kicker = if gfx.cyber() {
        "Braindance · lecture"
    } else {
        "En projection"
    };
    gfx.kicker(INFO_X + slide(0.0), y, kicker, palette.accent);
    if gfx.cyber() {
        let blink = if (gfx.t * 2.0).fract() < 0.5 {
            1.0
        } else {
            0.25
        };
        gfx.disc(
            INFO_X + INFO_W - 44.0,
            y + 6.0,
            5.0,
            fade(palette.accent, blink),
        );
        gfx.text(
            INFO_X + INFO_W,
            y,
            "REC",
            Type::new(Face::Strong, 12.0).spaced(2.5),
            fade(palette.accent, blink),
            Anchor::TOP_RIGHT,
        );
    }
    y += 24.0;
    let name = playing
        .series
        .clone()
        .unwrap_or_else(|| playing.title.clone());
    let style = if gfx.cyber() {
        Type::new(Face::Bold, 36.0)
    } else {
        Type::new(Face::Display, 26.0)
    };
    let name = if gfx.cyber() {
        name.to_uppercase()
    } else {
        name
    };
    let glyphs = gfx.glyphs();
    for line in gfx.wrap(&name, INFO_W, style, 2) {
        let shown = decode(&line, age - 0.1, 1.2, glyphs);
        gfx.title(
            INFO_X + slide(0.1),
            y,
            &shown,
            style,
            if gfx.cyber() {
                palette.hot
            } else {
                palette.text
            },
            Anchor::TOP_LEFT,
        );
        y += style.size * 1.2;
    }
    y += 4.0;
    let sub = match playing.kind {
        MediaKind::Episode => format!(
            "{} · {}",
            text::episode(playing.season, playing.episode),
            playing.title
        ),
        MediaKind::Movie => playing
            .year
            .map(|year| year.to_string())
            .unwrap_or_default(),
    };
    gfx.text(
        INFO_X + slide(0.2),
        y,
        &sub,
        Type::new(Face::Strong, 19.0),
        palette.accent2,
        Anchor::TOP_LEFT,
    );
    y += 26.0;
    let mut meta: Vec<String> = Vec::new();
    if playing.kind == MediaKind::Episode
        && let Some(year) = playing.year
    {
        meta.push(year.to_string());
    }
    if !playing.genres.is_empty() {
        meta.push(playing.genres.join(" · "));
    }
    if playing.runtime > 0.0 {
        meta.push(format!("{} min", (playing.runtime / 60.0).round()));
    }
    gfx.text(
        INFO_X + slide(0.25),
        y,
        &meta.join("  ·  "),
        Type::new(Face::Strong, 13.0).spaced(1.4),
        palette.muted,
        Anchor::TOP_LEFT,
    );
    y += 26.0;
    y += gfx.paragraph(
        INFO_X + slide(0.3),
        y,
        &playing.overview,
        INFO_W,
        Type::new(Face::Body, 15.5),
        19.5,
        fade(palette.text, 0.85),
        3,
    );
    y = y.max(TOP + 250.0) + 10.0;

    let position = playing.position_at(view.read_at, view.now);
    let share = if playing.runtime > 0.0 {
        (position / playing.runtime) as f32
    } else {
        0.0
    };
    if gfx.cyber() {
        tracks(gfx, y, share);
        y += 46.0;
    }
    let blink = if playing.paused {
        0.4 + 0.6 * (gfx.t * 2.0).sin().abs()
    } else {
        1.0
    };
    gfx.rect(INFO_X, y, INFO_W, 6.0, fade(Color::white(), 0.1));
    if gfx.cyber() {
        gfx.rect(INFO_X, y, INFO_W * share, 6.0, fade(palette.hot, blink));
    } else {
        gfx.gradient_rect(
            INFO_X,
            y,
            INFO_W * share,
            6.0,
            fade(palette.accent, blink),
            fade(mix(palette.accent, palette.accent2, 0.9), blink),
        );
    }
    let head = INFO_X + INFO_W * share;
    gfx.halo(head, y + 3.0, 14.0, fade(Color::white(), 0.5));
    gfx.rect(head - 1.0, y - 5.0, 2.0, 16.0, Color::white());
    y += 16.0;
    let mono = Type::new(Face::Strong, 13.5).spaced(1.0);
    gfx.text(
        INFO_X,
        y,
        &text::duration(position),
        mono,
        palette.muted,
        Anchor::TOP_LEFT,
    );
    let state = if playing.paused {
        "EN PAUSE".to_owned()
    } else if playing.runtime > 0.0 {
        format!(
            "FIN À {}",
            text::clock(view.now.plus_secs(playing.runtime - position))
        )
    } else {
        String::new()
    };
    gfx.text(
        INFO_X + INFO_W / 2.0,
        y,
        &state,
        mono.spaced(3.0),
        palette.accent,
        Anchor::TOP_CENTER,
    );
    if playing.runtime > 0.0 {
        gfx.text(
            INFO_X + INFO_W,
            y,
            &format!("-{}", text::duration(playing.runtime - position)),
            mono,
            palette.muted,
            Anchor::TOP_RIGHT,
        );
    }
    let foot: Vec<&str> = [
        playing.device.as_str(),
        playing.client.as_str(),
        playing.user.as_str(),
    ]
    .into_iter()
    .filter(|part| !part.is_empty())
    .collect();
    gfx.text(
        INFO_X,
        (y + 30.0).min(BOTTOM - 22.0),
        &foot.join(" · ").to_uppercase(),
        Type::new(Face::Strong, 12.0).spaced(2.0),
        palette.muted,
        Anchor::TOP_LEFT,
    );
}

/// The braindance editor's three layers, striped, with a light running along.
fn tracks(gfx: &mut Gfx, y: f32, share: f32) {
    let palette = gfx.palette;
    let layers = [
        ("VISUEL", palette.hot, 14.0_f32),
        ("AUDIO", palette.accent2, 4.0),
        ("THERMIQUE", palette.accent, 22.0),
    ];
    for (index, &(label, colour, stripe)) in layers.iter().enumerate() {
        let ty = y + index as f32 * 14.0;
        gfx.text(
            INFO_X,
            ty + 5.0,
            label,
            Type::new(Face::Strong, 10.0).spaced(1.6),
            palette.muted,
            Anchor::MIDDLE_LEFT,
        );
        let (x, w) = (INFO_X + 80.0, INFO_W - 80.0);
        let mut path = Path::new();
        let mut sx = x;
        while sx < x + w {
            path.rect(sx, ty, stripe.min(x + w - sx), 10.0);
            sx += stripe + 3.0;
        }
        gfx.fill(&path, fade(colour, 0.5));
        let run = x + ((gfx.t / 3.0 + index as f32 * 0.3).fract()) * w;
        let glare = Paint::linear_gradient_stops(
            run - 40.0,
            0.0,
            run + 40.0,
            0.0,
            [
                (0.0, fade(Color::white(), 0.0)),
                (0.5, fade(Color::white(), 0.55)),
                (1.0, fade(Color::white(), 0.0)),
            ],
        );
        let mut band = Path::new();
        band.rect((run - 40.0).max(x), ty, 80.0, 10.0);
        gfx.canvas.fill_path(&band, &glare);
        gfx.rect(x + w * share - 1.0, ty - 2.0, 2.0, 14.0, rgb(0xffffff));
    }
}

/// The Gazette's front page: the poster as its photograph, the masthead, the
/// headline (what plays), the deck (when it ends), the story in two columns,
/// and where the playback is, as a rule filling with ink.
fn gazette(gfx: &mut Gfx, view: &View, playing: &Playing, age: f32, textures: &mut Textures) {
    let (px, py, pw, ph) = PAGE;
    gfx.pinned(px, py, pw, ph, -0.35);
    let news = ink::news_ink();
    let fade_in = ease_out(progress(age, 0.8));
    // the photograph
    let (fx, fy, fw, fh) = (px + 18.0, py + 16.0, 200.0, 300.0);
    if let Some(image) = playing
        .poster
        .as_ref()
        .and_then(|p| textures.get(gfx.canvas, p, 400, Finish::Print))
    {
        gfx.photo(image, fx, fy, fw, fh, fade_in);
    } else {
        gfx.rect(fx, fy, fw, fh, rgb(0xd8c9a6));
        gfx.rect_line(fx, fy, fw, fh, news, 1.0);
    }
    let small = Type::new(Face::Italic, 11.0);
    gfx.text(
        fx,
        fy + fh + 8.0,
        "Photographié sur le plateau.",
        small,
        news,
        Anchor::TOP_LEFT,
    );
    // the masthead
    let (x, right) = (fx + fw + 28.0, px + pw - 24.0);
    let w = right - x;
    gfx.rect(x, py + 18.0, w, 3.0, news);
    gfx.text(
        x,
        py + 28.0,
        "Édition spéciale",
        small,
        news,
        Anchor::TOP_LEFT,
    );
    gfx.text(
        f32::midpoint(x, right),
        py + 26.0,
        "La Gazette du Sorcier",
        Type::new(Face::Body, 21.0).spaced(1.2),
        news,
        Anchor::TOP_CENTER,
    );
    let day = text::date(view.now);
    let weekday = day.split(' ').next().unwrap_or_default();
    gfx.text(right, py + 28.0, weekday, small, news, Anchor::TOP_RIGHT);
    gfx.rect(x, py + 52.0, w, 1.0, news);
    gfx.rect(x, py + 55.0, w, 1.0, news);
    // the headline, written as the page comes
    let mut y = py + 62.0;
    let name = playing
        .series
        .clone()
        .unwrap_or_else(|| playing.title.clone());
    let head = Type::new(Face::Body, 28.0);
    for line in gfx.wrap(&name, w, head, 2) {
        let shown = decode(&line, age - 0.1, 1.2, "");
        gfx.text(x, y, &shown, head, news, Anchor::TOP_LEFT);
        y += 33.0;
    }
    y += 4.0;
    // the deck: what it is, and when it ends
    let position = playing.position_at(view.read_at, view.now);
    let what = match playing.kind {
        MediaKind::Episode => format!(
            "{} · {}",
            text::episode(playing.season, playing.episode),
            playing.title
        ),
        MediaKind::Movie => playing
            .year
            .map(|year| year.to_string())
            .unwrap_or_default(),
    };
    let end = if playing.paused {
        "la projection est en pause".to_owned()
    } else if playing.runtime > 0.0 {
        let at = view.now.plus_secs(playing.runtime - position);
        let local = text::local(at);
        format!(
            "fin annoncée vers {}",
            text::in_words(
                chrono::Timelike::hour(&local),
                chrono::Timelike::minute(&local)
            )
        )
    } else {
        String::new()
    };
    let deck: Vec<String> = [what, end]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect();
    let deck_style = Type::new(Face::Italic, 15.0);
    for line in gfx.wrap(&deck.join(" — "), w, deck_style, 1) {
        gfx.text(x, y, &line, deck_style, rgb(0x3a2e20), Anchor::TOP_LEFT);
    }
    y += 24.0;
    // the story, in two columns
    let mut facts: Vec<String> = Vec::new();
    if let Some(year) = playing.year.filter(|_| playing.kind == MediaKind::Episode) {
        facts.push(year.to_string());
    }
    if playing.runtime > 0.0 {
        facts.push(format!("{} minutes", (playing.runtime / 60.0).round()));
    }
    if let Some(genre) = playing.genres.first() {
        facts.push(genre.clone());
    }
    let story = if facts.is_empty() {
        playing.overview.clone()
    } else {
        format!("{}. {}", facts.join(" · "), playing.overview)
    };
    let body = Type::new(Face::Body, 12.5);
    let column = (w - 26.0) / 2.0;
    let lines = gfx.wrap(&story, column, body, 12);
    let rows = 6;
    for (index, line) in lines.iter().enumerate() {
        let (cx, row) = if index < rows {
            (x, index)
        } else {
            (x + column + 26.0, index - rows)
        };
        gfx.text(
            cx,
            y + row as f32 * 15.5,
            line,
            body,
            rgb(0x2a2218),
            Anchor::TOP_LEFT,
        );
    }
    if lines.len() > rows {
        gfx.rect(
            x + column + 12.0,
            y + 2.0,
            1.0,
            rows as f32 * 15.5 - 4.0,
            news,
        );
    }
    // where the playback is: a rule filling with ink
    let share = if playing.runtime > 0.0 {
        (position / playing.runtime) as f32
    } else {
        0.0
    };
    let blink = if playing.paused {
        0.4 + 0.6 * (gfx.t * 2.0).sin().abs()
    } else {
        1.0
    };
    let by = py + ph - 52.0;
    gfx.rect(x, by, w, 5.0, fade(news, 0.12));
    gfx.rect(x, by, w * share, 5.0, fade(news, 0.85 * blink));
    gfx.rect(x + w * share - 1.0, by - 4.0, 2.0, 13.0, ink::red());
    let mono = Type::new(Face::Body, 11.5).spaced(1.0);
    let ty = by + 12.0;
    gfx.text(
        x,
        ty,
        &text::duration(position),
        mono,
        news,
        Anchor::TOP_LEFT,
    );
    let state = if playing.paused {
        "EN PAUSE".to_owned()
    } else if playing.runtime > 0.0 {
        format!(
            "FIN À {}",
            text::clock(view.now.plus_secs(playing.runtime - position))
        )
    } else {
        String::new()
    };
    gfx.text(
        x + w / 2.0,
        ty,
        &state,
        mono.spaced(2.5),
        ink::red(),
        Anchor::TOP_CENTER,
    );
    if playing.runtime > 0.0 {
        gfx.text(
            right,
            ty,
            &format!("-{}", text::duration(playing.runtime - position)),
            mono,
            news,
            Anchor::TOP_RIGHT,
        );
    }
    let foot: Vec<&str> = [
        playing.device.as_str(),
        playing.client.as_str(),
        playing.user.as_str(),
    ]
    .into_iter()
    .filter(|part| !part.is_empty())
    .collect();
    gfx.text(
        x,
        ty + 18.0,
        &foot.join(" · ").to_uppercase(),
        Type::new(Face::Body, 10.5).spaced(1.8),
        fade(news, 0.8),
        Anchor::TOP_LEFT,
    );
}
