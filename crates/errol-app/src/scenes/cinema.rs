//! A video plays on Jellyfin: its backdrop, blurred and drifting, behind the
//! poster, the title, the story, and where the playback is (a braindance
//! editor's tracks in cyberpunk). On the Map it is the Gazette's front page,
//! pinned over the castle, the poster its photograph.

use errol_domain::entities::{MediaKind, Playing, Timestamp};
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
    backdrop(gfx, playing, age, textures);
    vignette(gfx);
    poster_frame(gfx, playing, age, textures);
    let mut y = TOP + 34.0;
    marquee(gfx, y, age);
    y += 24.0;
    y = name_lines(gfx, playing, y, age);
    y += 4.0;
    y = details(gfx, playing, y, age);
    y = y.max(TOP + 250.0) + 10.0;
    let position = playing.position_at(view.read_at, view.now);
    let share = share(playing, position);
    if gfx.cyber() {
        tracks(gfx, y, share);
        y += 46.0;
    }
    playback_bar(gfx, playing, y, share);
    y += 16.0;
    playback_times(gfx, view, playing, position, y);
    gfx.text(
        INFO_X,
        (y + 30.0).min(BOTTOM - 22.0),
        &foot(playing),
        Type::new(Face::Strong, 12.0).spaced(2.0),
        gfx.palette.muted,
        Anchor::TOP_LEFT,
    );
}

/// How far right a line still is, `delay` seconds after the scene came.
fn slide(age: f32, delay: f32) -> f32 {
    30.0 * (1.0 - ease_out(progress(age - delay, 0.9)))
}

/// The backdrop, blurred, drifting slowly: 40 s each way.
fn backdrop(gfx: &mut Gfx, playing: &Playing, age: f32, textures: &mut Textures) {
    let Some(image) = playing
        .backdrop
        .as_ref()
        .and_then(|backdrop| textures.get(gfx.canvas, backdrop, 480, Finish::Blur))
    else {
        return;
    };
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

/// Darkens the edges, the poster's side less.
fn vignette(gfx: &mut Gfx) {
    let palette = gfx.palette;
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
}

/// The poster's place, and the poster once it is loaded.
fn poster_frame(gfx: &mut Gfx, playing: &Playing, age: f32, textures: &mut Textures) {
    let (px, py, pw, ph) = (30.0, TOP + 58.0, 200.0, 300.0);
    gfx.rect(px, py, pw, ph, fade(gfx.palette.accent, 0.06));
    if let Some(image) = playing
        .poster
        .as_ref()
        .and_then(|p| textures.get(gfx.canvas, p, 400, Finish::Plain))
    {
        poster(gfx, image, px, py, pw, ph, age);
    }
}

/// The kicker, and in cyberpunk a blinking REC on the right.
fn marquee(gfx: &mut Gfx, y: f32, age: f32) {
    let palette = gfx.palette;
    let kicker = if gfx.cyber() {
        "Braindance · lecture"
    } else {
        "En projection"
    };
    gfx.kicker(INFO_X + slide(age, 0.0), y, kicker, palette.accent);
    if !gfx.cyber() {
        return;
    }
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

/// The series' or the film's name, decoding, on up to two lines; returns
/// where the next line goes.
fn name_lines(gfx: &mut Gfx, playing: &Playing, mut y: f32, age: f32) -> f32 {
    let cyber = gfx.cyber();
    let (style, colour) = if cyber {
        (Type::new(Face::Bold, 36.0), gfx.palette.hot)
    } else {
        (Type::new(Face::Display, 26.0), gfx.palette.text)
    };
    let name = if cyber {
        name(playing).to_uppercase()
    } else {
        name(playing)
    };
    let glyphs = gfx.glyphs();
    for line in gfx.wrap(&name, INFO_W, style, 2) {
        let shown = decode(&line, age - 0.1, 1.2, glyphs);
        gfx.title(
            INFO_X + slide(age, 0.1),
            y,
            &shown,
            style,
            colour,
            Anchor::TOP_LEFT,
        );
        y += style.size * 1.2;
    }
    y
}

/// The episode or the year, the facts, the story; returns where the next
/// line goes.
fn details(gfx: &mut Gfx, playing: &Playing, mut y: f32, age: f32) -> f32 {
    let palette = gfx.palette;
    gfx.text(
        INFO_X + slide(age, 0.2),
        y,
        &subtitle(playing),
        Type::new(Face::Strong, 19.0),
        palette.accent2,
        Anchor::TOP_LEFT,
    );
    y += 26.0;
    gfx.text(
        INFO_X + slide(age, 0.25),
        y,
        &meta(playing),
        Type::new(Face::Strong, 13.0).spaced(1.4),
        palette.muted,
        Anchor::TOP_LEFT,
    );
    y += 26.0;
    y + gfx.paragraph(
        INFO_X + slide(age, 0.3),
        y,
        &playing.overview,
        INFO_W,
        Type::new(Face::Body, 15.5),
        19.5,
        fade(palette.text, 0.85),
        3,
    )
}

/// Where the playback is: a bar filling, blinking while paused, its head lit.
fn playback_bar(gfx: &mut Gfx, playing: &Playing, y: f32, share: f32) {
    let palette = gfx.palette;
    let blink = pause_blink(playing.paused, gfx.t);
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
}

/// Under the bar: the time played, when it ends (or that it is paused), the
/// time left.
fn playback_times(gfx: &mut Gfx, view: &View, playing: &Playing, position: f64, y: f32) {
    let palette = gfx.palette;
    let mono = Type::new(Face::Strong, 13.5).spaced(1.0);
    gfx.text(
        INFO_X,
        y,
        &text::duration(position),
        mono,
        palette.muted,
        Anchor::TOP_LEFT,
    );
    gfx.text(
        INFO_X + INFO_W / 2.0,
        y,
        &state(playing, view.now, position),
        mono.spaced(3.0),
        palette.accent,
        Anchor::TOP_CENTER,
    );
    if let Some(left) = time_left(playing, position) {
        gfx.text(
            INFO_X + INFO_W,
            y,
            &left,
            mono,
            palette.muted,
            Anchor::TOP_RIGHT,
        );
    }
}

/// What plays: the series, or the film.
fn name(playing: &Playing) -> String {
    playing
        .series
        .clone()
        .unwrap_or_else(|| playing.title.clone())
}

/// The episode and its title, or the film's year.
fn subtitle(playing: &Playing) -> String {
    match playing.kind {
        MediaKind::Episode => format!(
            "{} · {}",
            text::episode(playing.season, playing.episode),
            playing.title
        ),
        MediaKind::Movie => playing
            .year
            .map(|year| year.to_string())
            .unwrap_or_default(),
    }
}

/// The facts under the subtitle: an episode's year, the genres, the length.
fn meta(playing: &Playing) -> String {
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
    meta.join("  ·  ")
}

/// The share of the video played, 0 when its length is unknown.
fn share(playing: &Playing, position: f64) -> f32 {
    if playing.runtime > 0.0 {
        (position / playing.runtime) as f32
    } else {
        0.0
    }
}

/// The bar blinks while the playback is paused.
fn pause_blink(paused: bool, t: f32) -> f32 {
    if paused {
        0.4 + 0.6 * (t * 2.0).sin().abs()
    } else {
        1.0
    }
}

/// Under the middle of the bar: paused, or when it ends.
fn state(playing: &Playing, now: Timestamp, position: f64) -> String {
    if playing.paused {
        "EN PAUSE".to_owned()
    } else if playing.runtime > 0.0 {
        format!(
            "FIN À {}",
            text::clock(now.plus_secs(playing.runtime - position))
        )
    } else {
        String::new()
    }
}

/// The time left, when the length is known.
fn time_left(playing: &Playing, position: f64) -> Option<String> {
    (playing.runtime > 0.0).then(|| format!("-{}", text::duration(playing.runtime - position)))
}

/// Where it plays and for whom, in capitals.
fn foot(playing: &Playing) -> String {
    let foot: Vec<&str> = [
        playing.device.as_str(),
        playing.client.as_str(),
        playing.user.as_str(),
    ]
    .into_iter()
    .filter(|part| !part.is_empty())
    .collect();
    foot.join(" · ").to_uppercase()
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
    photograph(gfx, playing, age, textures);
    // the text, right of the photograph
    let (x, right) = (px + 18.0 + 200.0 + 28.0, px + pw - 24.0);
    let w = right - x;
    masthead(gfx, view, x, right);
    // the headline, written as the page comes
    let mut y = py + 62.0;
    let head = Type::new(Face::Body, 28.0);
    for line in gfx.wrap(&name(playing), w, head, 2) {
        let shown = decode(&line, age - 0.1, 1.2, "");
        gfx.text(x, y, &shown, head, ink::news_ink(), Anchor::TOP_LEFT);
        y += 33.0;
    }
    y += 4.0;
    // the deck: what it is, and when it ends
    let position = playing.position_at(view.read_at, view.now);
    let deck_style = Type::new(Face::Italic, 15.0);
    for line in gfx.wrap(&deck(playing, view.now, position), w, deck_style, 1) {
        gfx.text(x, y, &line, deck_style, rgb(0x3a2e20), Anchor::TOP_LEFT);
    }
    y += 24.0;
    columns(gfx, &story(playing), x, w, y);
    ink_rule(gfx, view, playing, position, (x, right, py + ph - 52.0));
}

/// The poster as the page's photograph, or its empty place, and its caption.
fn photograph(gfx: &mut Gfx, playing: &Playing, age: f32, textures: &mut Textures) {
    let (px, py, ..) = PAGE;
    let news = ink::news_ink();
    let (fx, fy, fw, fh) = (px + 18.0, py + 16.0, 200.0, 300.0);
    if let Some(image) = playing
        .poster
        .as_ref()
        .and_then(|p| textures.get(gfx.canvas, p, 400, Finish::Print))
    {
        gfx.photo(image, fx, fy, fw, fh, ease_out(progress(age, 0.8)));
    } else {
        gfx.rect(fx, fy, fw, fh, rgb(0xd8c9a6));
        gfx.rect_line(fx, fy, fw, fh, news, 1.0);
    }
    gfx.text(
        fx,
        fy + fh + 8.0,
        "Photographié sur le plateau.",
        Type::new(Face::Italic, 11.0),
        news,
        Anchor::TOP_LEFT,
    );
}

/// The masthead: the edition, the paper's name, the weekday, double rules.
fn masthead(gfx: &mut Gfx, view: &View, x: f32, right: f32) {
    let (_, py, ..) = PAGE;
    let news = ink::news_ink();
    let small = Type::new(Face::Italic, 11.0);
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
}

/// The deck: what it is, and when it ends, in words.
fn deck(playing: &Playing, now: Timestamp, position: f64) -> String {
    let end = if playing.paused {
        "la projection est en pause".to_owned()
    } else if playing.runtime > 0.0 {
        let at = now.plus_secs(playing.runtime - position);
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
    let deck: Vec<String> = [subtitle(playing), end]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect();
    deck.join(" — ")
}

/// The story, after an episode's year, the length and the first genre.
fn story(playing: &Playing) -> String {
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
    if facts.is_empty() {
        playing.overview.clone()
    } else {
        format!("{}. {}", facts.join(" · "), playing.overview)
    }
}

/// Lines of the story per column.
const ROWS: usize = 6;
/// Space between the two columns.
const GUTTER: f32 = 26.0;

/// Where the `index`th line of the story goes: its column's x and its row.
fn slot(index: usize, x: f32, column: f32) -> (f32, usize) {
    if index < ROWS {
        (x, index)
    } else {
        (x + column + GUTTER, index - ROWS)
    }
}

/// The story in two columns, a rule between them when the second is used.
fn columns(gfx: &mut Gfx, story: &str, x: f32, w: f32, y: f32) {
    let body = Type::new(Face::Body, 12.5);
    let column = (w - GUTTER) / 2.0;
    let lines = gfx.wrap(story, column, body, 12);
    for (index, line) in lines.iter().enumerate() {
        let (cx, row) = slot(index, x, column);
        gfx.text(
            cx,
            y + row as f32 * 15.5,
            line,
            body,
            rgb(0x2a2218),
            Anchor::TOP_LEFT,
        );
    }
    if lines.len() > ROWS {
        gfx.rect(
            x + column + 12.0,
            y + 2.0,
            1.0,
            ROWS as f32 * 15.5 - 4.0,
            ink::news_ink(),
        );
    }
}

/// Where the playback is: a rule filling with ink at `(x, right, y)`, the
/// times under it, where it plays.
fn ink_rule(
    gfx: &mut Gfx,
    view: &View,
    playing: &Playing,
    position: f64,
    (x, right, by): (f32, f32, f32),
) {
    let news = ink::news_ink();
    let w = right - x;
    let share = share(playing, position);
    let blink = pause_blink(playing.paused, gfx.t);
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
    gfx.text(
        x + w / 2.0,
        ty,
        &state(playing, view.now, position),
        mono.spaced(2.5),
        ink::red(),
        Anchor::TOP_CENTER,
    );
    if let Some(left) = time_left(playing, position) {
        gfx.text(right, ty, &left, mono, news, Anchor::TOP_RIGHT);
    }
    gfx.text(
        x,
        ty + 18.0,
        &foot(playing),
        Type::new(Face::Body, 10.5).spaced(1.8),
        fade(news, 0.8),
        Anchor::TOP_LEFT,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn playing(kind: MediaKind) -> Playing {
        Playing {
            id: "p".into(),
            kind,
            title: "Paris".into(),
            series: (kind == MediaKind::Episode).then(|| "Arcane".into()),
            season: Some(2),
            episode: Some(4),
            year: Some(2024),
            overview: "Story.".into(),
            genres: vec!["Animation".into(), "Action".into()],
            runtime: 2_400.0,
            position: 600.0,
            paused: false,
            device: "Projector".into(),
            client: String::new(),
            user: "d".into(),
            poster: None,
            backdrop: None,
        }
    }

    #[test]
    fn an_episode_is_named_by_its_series() {
        let episode = playing(MediaKind::Episode);
        assert_eq!(name(&episode), "Arcane");
        assert!(subtitle(&episode).ends_with(" · Paris"));
        assert_eq!(meta(&episode), "2024  ·  Animation · Action  ·  40 min");
        assert_eq!(story(&episode), "2024 · 40 minutes · Animation. Story.");
        let film = playing(MediaKind::Movie);
        assert_eq!(name(&film), "Paris");
        assert_eq!(subtitle(&film), "2024");
        assert_eq!(meta(&film), "Animation · Action  ·  40 min");
        assert_eq!(story(&film), "40 minutes · Animation. Story.");
    }

    #[test]
    fn an_unknown_length_hides_the_end() {
        let mut film = playing(MediaKind::Movie);
        film.runtime = 0.0;
        film.genres.clear();
        film.year = None;
        assert!(share(&film, 600.0).abs() < f32::EPSILON);
        assert_eq!(state(&film, Timestamp::from_secs(0), 600.0), "");
        assert_eq!(time_left(&film, 600.0), None);
        assert_eq!(meta(&film), "");
        assert_eq!(story(&film), "Story.");
        assert_eq!(deck(&film, Timestamp::from_secs(0), 600.0), "");
    }

    #[test]
    fn the_playback_says_where_it_is() {
        let mut film = playing(MediaKind::Movie);
        assert!((share(&film, 600.0) - 0.25).abs() < 1e-6);
        assert!(state(&film, Timestamp::from_secs(0), 600.0).starts_with("FIN À "));
        assert_eq!(
            time_left(&film, 600.0),
            Some(format!("-{}", text::duration(1_800.0)))
        );
        assert!(
            deck(&film, Timestamp::from_secs(0), 600.0).starts_with("2024 — fin annoncée vers ")
        );
        assert_eq!(foot(&film), "PROJECTOR · D");
        assert!((pause_blink(false, 1.0) - 1.0).abs() < f32::EPSILON);
        film.paused = true;
        assert_eq!(state(&film, Timestamp::from_secs(0), 600.0), "EN PAUSE");
        assert_eq!(
            deck(&film, Timestamp::from_secs(0), 600.0),
            "2024 — la projection est en pause"
        );
        assert!((pause_blink(true, 0.0) - 0.4).abs() < 1e-6);
    }

    #[test]
    fn the_story_runs_into_a_second_column() {
        assert_eq!(slot(0, 10.0, 100.0), (10.0, 0));
        assert_eq!(slot(7, 10.0, 100.0), (136.0, 1));
        assert!((slide(0.0, 0.2) - 30.0).abs() < 1e-4);
        assert!(slide(5.0, 0.0).abs() < 1e-4);
    }
}
