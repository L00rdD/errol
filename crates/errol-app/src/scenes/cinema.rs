//! A video plays on Jellyfin: its backdrop, blurred and drifting, behind the
//! poster, the title, the story, and where the playback is (a braindance
//! editor's tracks in cyberpunk).

use errol_domain::entities::{MediaKind, Playing};
use errol_domain::usecases::View;
use femtovg::{Color, Paint, Path};

use super::event::poster;
use super::{BOTTOM, HEIGHT, TOP, WIDTH, text};
use crate::anim::{decode, ease_in_out, ease_out, progress};
use crate::look::{fade, mix, rgb};
use crate::render::{Anchor, Face, Gfx, Textures, Type};

const INFO_X: f32 = 266.0;
const INFO_W: f32 = 504.0;

pub fn draw(gfx: &mut Gfx, view: &View, playing: &Playing, age: f32, textures: &mut Textures) {
    let palette = gfx.palette;
    if let Some(image) = playing
        .backdrop
        .as_ref()
        .and_then(|backdrop| textures.get(gfx.canvas, backdrop, 480, true))
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
        .and_then(|p| textures.get(gfx.canvas, p, 400, false))
    {
        poster(gfx, image, px, py, pw, ph, age);
    }

    if gfx.hogwarts() {
        // the candles of the hall float behind the words: a shadow keeps them readable
        let mut shade = Path::new();
        shade.rect(INFO_X - 24.0, TOP, WIDTH - INFO_X + 24.0, BOTTOM - TOP);
        let dark = Paint::linear_gradient(
            INFO_X - 24.0,
            0.0,
            INFO_X + 40.0,
            0.0,
            fade(palette.bg, 0.0),
            fade(palette.bg, 0.72),
        );
        gfx.canvas.fill_path(&shade, &dark);
    }
    let slide = |delay: f32| 30.0 * (1.0 - ease_out(progress(age - delay, 0.9)));
    let mut y = TOP + 34.0;
    let kicker = if gfx.cyber() {
        "Braindance · lecture"
    } else if gfx.hogwarts() {
        "La Pensine"
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
            } else if gfx.hogwarts() {
                palette.accent
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
        if gfx.hogwarts() {
            Type::new(Face::Italic, 20.0)
        } else {
            Type::new(Face::Strong, 19.0)
        },
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
        if gfx.hogwarts() {
            Type::new(Face::Italic, 16.0)
        } else {
            Type::new(Face::Body, 15.5)
        },
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
