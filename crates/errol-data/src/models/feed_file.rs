use std::collections::BTreeMap;

use chrono::DateTime;
use errol_domain::entities::{
    Actor, ActorState, Cerebro, Chapter, Cover, Disk, Download, Episode, Event, EventId, EventKind,
    House, Import, MediaKind, NextChapter, OnePiece, Outlet, PictureRef, Playing, Portfolio,
    Position, PrintNews, PrintPhase, Printer, Projector, Services, Session, SessionScene,
    SessionStatus, Settings, Snapshot, Switch, Timestamp, Today, Weather,
};
use serde::{Deserialize, Deserializer};
use serde_json::Value;

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct FeedFile {
    updated_at: Option<String>,
    #[serde(deserialize_with = "nullable")]
    playing: Vec<PlayingModel>,
    #[serde(deserialize_with = "nullable")]
    /// Kept as raw values and read one by one: a bad event is skipped, not the file.
    events: Vec<Value>,
    house: Option<HouseModel>,
    #[serde(deserialize_with = "nullable")]
    downloads: Vec<DownloadModel>,
    #[serde(deserialize_with = "nullable")]
    recent: Vec<ImportModel>,
    one_piece: Option<OnePieceModel>,
    portfolio: Option<PortfolioModel>,
    disk: Option<DiskModel>,
    services: Option<ServicesModel>,
    cerebro: Option<CerebroModel>,
    printer: Option<PrinterModel>,
    #[serde(deserialize_with = "nullable")]
    errors: BTreeMap<String, Option<String>>,
    settings: Option<SettingsModel>,
    weather: Option<WeatherModel>,
    #[serde(deserialize_with = "nullable")]
    next_up: Vec<EpisodeModel>,
    #[serde(deserialize_with = "nullable")]
    airing: Vec<EpisodeModel>,
    today: Option<TodayModel>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct WeatherModel {
    temperature: Option<f64>,
    code: Option<u32>,
    wind: Option<f64>,
    high: Option<f64>,
    low: Option<f64>,
    rain: Option<f64>,
    sunrise: Option<String>,
    sunset: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct EpisodeModel {
    series: Option<String>,
    season: Option<u32>,
    episode: Option<u32>,
    title: Option<String>,
    poster: Option<String>,
    at: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct TodayModel {
    #[serde(deserialize_with = "nullable")]
    imports: u32,
    #[serde(deserialize_with = "nullable")]
    downloads: u32,
    #[serde(deserialize_with = "nullable")]
    plays: u32,
    #[serde(deserialize_with = "nullable")]
    lights: u32,
    #[serde(deserialize_with = "nullable")]
    prints: u32,
    #[serde(deserialize_with = "nullable")]
    questions: u32,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct SwitchModel {
    at: Option<String>,
    on: Option<bool>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct PlayingModel {
    #[serde(deserialize_with = "nullable")]
    id: String,
    #[serde(rename = "type", deserialize_with = "nullable")]
    kind: String,
    title: Option<String>,
    series: Option<String>,
    season: Option<u32>,
    episode: Option<u32>,
    year: Option<u32>,
    #[serde(deserialize_with = "nullable")]
    overview: String,
    #[serde(deserialize_with = "nullable")]
    genres: Vec<String>,
    #[serde(deserialize_with = "nullable")]
    runtime: f64,
    #[serde(deserialize_with = "nullable")]
    position: f64,
    #[serde(deserialize_with = "nullable")]
    paused: bool,
    device: Option<String>,
    client: Option<String>,
    user: Option<String>,
    poster: Option<String>,
    backdrop: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct HouseModel {
    #[serde(deserialize_with = "nullable")]
    outlets: Vec<OutletModel>,
    #[serde(deserialize_with = "nullable")]
    covers: Vec<CoverModel>,
    #[serde(deserialize_with = "nullable")]
    projectors: Vec<ProjectorModel>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct OutletModel {
    #[serde(deserialize_with = "nullable")]
    id: String,
    #[serde(deserialize_with = "nullable")]
    name: String,
    #[serde(deserialize_with = "nullable")]
    on: bool,
    next: Option<SwitchModel>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct CoverModel {
    #[serde(deserialize_with = "nullable")]
    id: String,
    #[serde(deserialize_with = "nullable")]
    name: String,
    position: Option<f64>,
    moving: Option<Value>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct ProjectorModel {
    #[serde(deserialize_with = "nullable")]
    id: String,
    #[serde(deserialize_with = "nullable")]
    name: String,
    #[serde(deserialize_with = "nullable")]
    on: bool,
    app: Option<String>,
    closes_at: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct DownloadModel {
    #[serde(deserialize_with = "nullable")]
    name: String,
    #[serde(deserialize_with = "nullable")]
    percent: f64,
    eta: Option<f64>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct ImportModel {
    media: Option<String>,
    title: Option<String>,
    year: Option<u32>,
    season: Option<u32>,
    episode: Option<u32>,
    episode_title: Option<String>,
    quality: Option<String>,
    poster: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct OnePieceModel {
    latest: Option<ChapterModel>,
    next: Option<NextModel>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct ChapterModel {
    #[serde(deserialize_with = "nullable")]
    scan: u32,
    #[serde(deserialize_with = "nullable")]
    name: String,
    arc: Option<String>,
    pages: Option<u32>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct NextModel {
    #[serde(deserialize_with = "nullable")]
    scan: u32,
    #[serde(deserialize_with = "nullable")]
    at: String,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct PortfolioModel {
    name: Option<String>,
    #[serde(deserialize_with = "nullable")]
    mode: String,
    #[serde(deserialize_with = "nullable")]
    equity: f64,
    #[serde(deserialize_with = "nullable")]
    initial: f64,
    #[serde(deserialize_with = "nullable")]
    trades: u32,
    #[serde(deserialize_with = "nullable")]
    positions: Vec<PositionModel>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct PositionModel {
    #[serde(deserialize_with = "nullable")]
    pair: String,
    #[serde(deserialize_with = "nullable")]
    avg: f64,
    price: Option<f64>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct DiskModel {
    #[serde(deserialize_with = "nullable")]
    total: u64,
    #[serde(deserialize_with = "nullable")]
    used: u64,
    #[serde(deserialize_with = "nullable")]
    free: u64,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct SettingsModel {
    look: Option<String>,
    #[serde(deserialize_with = "nullable")]
    follow_hub: bool,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct ServicesModel {
    #[serde(deserialize_with = "nullable")]
    up: u32,
    #[serde(deserialize_with = "nullable")]
    total: u32,
    #[serde(deserialize_with = "nullable")]
    down: Vec<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct CerebroModel {
    #[serde(deserialize_with = "nullable")]
    sessions: Vec<SessionModel>,
    room_size: Option<(u32, u32)>,
    character_size: Option<(u32, u32)>,
    emotes: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct SceneModel {
    room: Option<RoomModel>,
    #[serde(deserialize_with = "nullable")]
    actors: Vec<ActorModel>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct RoomModel {
    sprite: Option<String>,
    frames: Option<u32>,
    fps: Option<u32>,
    walk: Option<RectModel>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct RectModel {
    #[serde(deserialize_with = "nullable")]
    x: u32,
    #[serde(deserialize_with = "nullable")]
    y: u32,
    #[serde(deserialize_with = "nullable")]
    width: u32,
    #[serde(deserialize_with = "nullable")]
    height: u32,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct ActorModel {
    #[serde(deserialize_with = "nullable")]
    id: String,
    #[serde(deserialize_with = "nullable")]
    name: String,
    sprite: Option<String>,
    #[serde(rename = "move")]
    movement: Option<String>,
    power: Option<String>,
    color: Option<String>,
    aura: Option<String>,
    state: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct SessionModel {
    #[serde(deserialize_with = "nullable")]
    name: String,
    project: Option<String>,
    status: Option<String>,
    since: Option<String>,
    room: Option<String>,
    lead: Option<String>,
    #[serde(deserialize_with = "nullable")]
    agents: u32,
    scene: Option<SceneModel>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct PrinterModel {
    name: Option<String>,
    cloud: Option<String>,
    state: Option<String>,
    #[serde(deserialize_with = "nullable")]
    active: bool,
    file: Option<String>,
    progress: Option<f64>,
    layer: Option<f64>,
    layers: Option<f64>,
    remaining: Option<f64>,
    ends_at: Option<String>,
    image: Option<String>,
}

/// A JSON null read as an absent field: the field's default. A source that
/// answers `"on": null` (a projector just after a reboot) must not make the
/// whole file unreadable.
fn nullable<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Default + Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Option::unwrap_or_default)
}

/// RFC 3339 to a timestamp, seconds optional (the hub writes its schedules
/// to the minute); None when absent or malformed.
fn parse_time(text: Option<&str>) -> Option<Timestamp> {
    text.and_then(|text| {
        DateTime::parse_from_rfc3339(text)
            .or_else(|_| DateTime::parse_from_str(text, "%Y-%m-%dT%H:%M%:z"))
            .ok()
    })
    .map(|time| Timestamp(time.timestamp_millis()))
}

fn media(kind: Option<&str>) -> MediaKind {
    if kind == Some("episode") {
        MediaKind::Episode
    } else {
        MediaKind::Movie
    }
}

/// A session's scene; None without a room to draw it in. An actor without a
/// sprite is left out.
fn scene_of(
    model: SceneModel,
    sprite: &impl Fn(Option<String>) -> Option<PictureRef>,
) -> Option<SessionScene> {
    let room = model.room?;
    let walk = room.walk.unwrap_or_default();
    Some(SessionScene {
        room: sprite(room.sprite)?,
        frames: room.frames.unwrap_or(1).max(1),
        fps: room.fps.unwrap_or(8),
        walk: (walk.x, walk.y, walk.width, walk.height),
        actors: model
            .actors
            .into_iter()
            .filter_map(|a| {
                Some(Actor {
                    sprite: sprite(a.sprite)?,
                    id: a.id,
                    name: a.name,
                    movement: a.movement.unwrap_or_else(|| "walk".into()),
                    power: a.power.unwrap_or_else(|| "glow".into()),
                    colour: a.color.unwrap_or_else(|| "#FFFFFF".into()),
                    aura: a.aura.filter(|aura| !aura.is_empty()),
                    state: match a.state.as_deref() {
                        Some("asleep") => ActorState::Asleep,
                        Some("alert") => ActorState::Alert,
                        _ => ActorState::Walking,
                    },
                })
            })
            .collect(),
    })
}

/// The printer as the feed writes it; its picture is a link of the slicer's
/// cloud or a path on the home page, which keeps a copy of it.
fn printer_of(
    model: PrinterModel,
    picture: &impl Fn(Option<String>) -> Option<PictureRef>,
) -> Printer {
    let whole = |value: Option<f64>| value.filter(|v| *v >= 0.0).map(|v| v as u32);
    Printer {
        name: model
            .name
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| "Anycubic".into()),
        state: model.state.filter(|state| !state.is_empty()),
        active: model.active,
        file: model.file.filter(|file| !file.is_empty()),
        progress: model.progress,
        layer: whole(model.layer),
        layers: whole(model.layers).filter(|layers| *layers > 0),
        remaining: model.remaining.filter(|left| *left >= 0.0),
        ends: parse_time(model.ends_at.as_deref()),
        image: picture(model.image),
        cloud: model.cloud.filter(|cloud| cloud != "ok"),
    }
}

/// A status Cerebro does not name reads as idle: nothing to show for it.
fn status(text: Option<&str>) -> SessionStatus {
    match text {
        Some("busy") => SessionStatus::Busy,
        Some("waiting") => SessionStatus::Waiting,
        _ => SessionStatus::Idle,
    }
}

/// A picture path made absolute on `origin`, unless it is a URL already.
fn absolute(origin: &str, path: Option<String>) -> Option<PictureRef> {
    path.filter(|path| !path.is_empty()).map(|path| {
        if path.starts_with("http") {
            PictureRef::new(path)
        } else {
            PictureRef::new(format!("{origin}{path}"))
        }
    })
}

impl FeedFile {
    /// Turns the file into a snapshot. Jellyfin pictures are paths on the
    /// Jellyfin server, made absolute with `jellyfin`; Cerebro's sprites are
    /// paths on the home page (`hub`, its origin), which passes them on.
    pub fn into_snapshot(mut self, jellyfin: &str, hub: &str) -> Snapshot {
        let picture = |path: Option<String>| absolute(jellyfin, path);
        let extras = Snapshot {
            weather: self.weather.take().as_ref().and_then(weather_of),
            next_up: episodes(std::mem::take(&mut self.next_up), &picture),
            airing: episodes(std::mem::take(&mut self.airing), &picture),
            today: self.today.take().as_ref().map(today_of),
            ..Snapshot::default()
        };
        self.into_base(jellyfin, hub, extras)
    }

    /// The snapshot of what the first versions of the feed already said,
    /// over `extras`.
    fn into_base(self, jellyfin: &str, hub: &str, extras: Snapshot) -> Snapshot {
        let sprite = |path: Option<String>| absolute(hub, path);
        let picture = |path: Option<String>| absolute(jellyfin, path);
        Snapshot {
            updated_at: parse_time(self.updated_at.as_deref()).unwrap_or_default(),
            playing: self
                .playing
                .into_iter()
                .map(|item| playing_of(item, &picture))
                .collect(),
            events: self
                .events
                .iter()
                .filter_map(Value::as_object)
                .map(|fields| fields.clone().into_iter().collect::<BTreeMap<_, _>>())
                .filter_map(|fields| event(&fields, &sprite))
                .collect(),
            house: self.house.map(house_of),
            downloads: self.downloads.into_iter().map(download_of).collect(),
            recent: self
                .recent
                .into_iter()
                .map(|i| import(i, &picture))
                .collect(),
            one_piece: self.one_piece.and_then(one_piece_of),
            portfolio: self.portfolio.map(portfolio_of),
            disk: self.disk.map(|d| Disk {
                total: d.total,
                used: d.used,
                free: d.free,
            }),
            services: self
                .services
                .map(|s| Services {
                    up: s.up,
                    total: s.total,
                    down: s.down,
                })
                .unwrap_or_default(),
            cerebro: self.cerebro.map(|cerebro| cerebro_of(cerebro, &sprite)),
            printer: self.printer.map(|printer| printer_of(printer, &sprite)),
            warnings: self
                .errors
                .into_iter()
                .map(|(source, error)| format!("{source}: {}", error.unwrap_or_default()))
                .collect(),
            settings: self
                .settings
                .map(|settings| Settings {
                    look: settings.look.filter(|look| !look.is_empty()),
                    follow_hub: settings.follow_hub,
                })
                .unwrap_or_default(),
            ..extras
        }
    }
}

/// The weather needs its temperature and its code.
fn weather_of(w: &WeatherModel) -> Option<Weather> {
    Some(Weather {
        temperature: w.temperature?,
        code: w.code?,
        wind: w.wind,
        high: w.high,
        low: w.low,
        rain: w.rain,
        sunrise: parse_time(w.sunrise.as_deref()),
        sunset: parse_time(w.sunset.as_deref()),
    })
}

/// Episodes with a series; their posters, Jellyfin paths or URLs.
fn episodes(
    list: Vec<EpisodeModel>,
    picture: &impl Fn(Option<String>) -> Option<PictureRef>,
) -> Vec<Episode> {
    list.into_iter()
        .filter_map(|e| {
            Some(Episode {
                series: e.series.filter(|series| !series.is_empty())?,
                season: e.season,
                episode: e.episode,
                title: e.title,
                poster: picture(e.poster),
                at: parse_time(e.at.as_deref()),
            })
        })
        .collect()
}

fn today_of(t: &TodayModel) -> Today {
    Today {
        imports: t.imports,
        downloads: t.downloads,
        plays: t.plays,
        lights: t.lights,
        prints: t.prints,
        questions: t.questions,
    }
}

fn playing_of(
    item: PlayingModel,
    picture: &impl Fn(Option<String>) -> Option<PictureRef>,
) -> Playing {
    Playing {
        kind: if item.kind == "episode" {
            MediaKind::Episode
        } else {
            MediaKind::Movie
        },
        title: item.title.unwrap_or_default(),
        series: item.series,
        season: item.season,
        episode: item.episode,
        year: item.year,
        overview: item.overview,
        genres: item.genres,
        runtime: item.runtime,
        position: item.position,
        paused: item.paused,
        device: item.device.unwrap_or_default(),
        client: item.client.unwrap_or_default(),
        user: item.user.unwrap_or_default(),
        poster: picture(item.poster),
        backdrop: picture(item.backdrop),
        id: item.id,
    }
}

fn house_of(house: HouseModel) -> House {
    House {
        outlets: house
            .outlets
            .into_iter()
            .map(|o| Outlet {
                id: o.id,
                name: o.name,
                on: o.on,
                next: o.next.and_then(|next| {
                    Some(Switch {
                        at: parse_time(next.at.as_deref())?,
                        on: next.on?,
                    })
                }),
            })
            .collect(),
        covers: house
            .covers
            .into_iter()
            .map(|c| Cover {
                id: c.id,
                name: c.name,
                position: c.position.unwrap_or_default(),
                moving: c
                    .moving
                    .is_some_and(|moving| !moving.is_null() && moving != Value::Bool(false)),
            })
            .collect(),
        projectors: house
            .projectors
            .into_iter()
            .map(|p| Projector {
                id: p.id,
                name: p.name,
                on: p.on,
                app: p.app,
                closes_at: parse_time(p.closes_at.as_deref()),
            })
            .collect(),
    }
}

fn download_of(d: DownloadModel) -> Download {
    Download {
        name: d.name,
        percent: d.percent,
        eta: d.eta.filter(|eta| *eta >= 0.0),
    }
}

/// One Piece needs its latest chapter; the next one is shown when it has a date.
fn one_piece_of(k: OnePieceModel) -> Option<OnePiece> {
    let latest = k.latest?;
    Some(OnePiece {
        latest: Chapter {
            number: latest.scan,
            name: latest.name,
            arc: latest.arc,
            pages: latest.pages,
        },
        next: k.next.and_then(|n| {
            Some(NextChapter {
                number: n.scan,
                at: parse_time(Some(&n.at))?,
            })
        }),
    })
}

fn portfolio_of(d: PortfolioModel) -> Portfolio {
    Portfolio {
        name: d
            .name
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| "Portfolio".into()),
        mode: d.mode,
        equity: d.equity,
        initial: d.initial,
        trades: d.trades,
        positions: d
            .positions
            .into_iter()
            .map(|p| Position {
                pair: p.pair,
                average_price: p.avg,
                price: p.price,
            })
            .collect(),
    }
}

fn cerebro_of(
    cerebro: CerebroModel,
    sprite: &impl Fn(Option<String>) -> Option<PictureRef>,
) -> Cerebro {
    Cerebro {
        room_size: cerebro.room_size.unwrap_or((192, 120)),
        character_size: cerebro.character_size.unwrap_or((24, 24)),
        emotes: sprite(cerebro.emotes),
        sessions: cerebro
            .sessions
            .into_iter()
            .map(|s| Session {
                status: status(s.status.as_deref()),
                since: parse_time(s.since.as_deref()),
                name: s.name,
                project: s.project.filter(|project| !project.is_empty()),
                room: s.room,
                lead: s.lead,
                agents: s.agents,
                scene: s.scene.and_then(|scene| scene_of(scene, sprite)),
            })
            .collect(),
    }
}

fn import(model: ImportModel, picture: &impl Fn(Option<String>) -> Option<PictureRef>) -> Import {
    Import {
        kind: media(model.media.as_deref()),
        title: model.title.unwrap_or_default(),
        year: model.year,
        season: model.season,
        episode: model.episode,
        episode_title: model.episode_title,
        quality: model.quality,
        poster: picture(model.poster),
    }
}

/// An event's fields, read by type: a field of another type reads as missing.
struct Fields<'a>(&'a BTreeMap<String, Value>);

impl Fields<'_> {
    fn text(&self, key: &str) -> Option<String> {
        self.0.get(key).and_then(Value::as_str).map(str::to_owned)
    }

    fn flag(&self, key: &str) -> Option<bool> {
        self.0.get(key).and_then(Value::as_bool)
    }

    fn number(&self, key: &str) -> Option<u64> {
        self.0.get(key).and_then(Value::as_u64)
    }
}

/// One event of the file; None for a kind this version does not know.
/// `hub` makes a path on the home page absolute (a print's picture).
fn event(
    fields: &BTreeMap<String, Value>,
    hub: &impl Fn(Option<String>) -> Option<PictureRef>,
) -> Option<Event> {
    let fields = Fields(fields);
    let kind = event_kind(&fields, hub)?;
    // the feed's ids carry their own counter; a missing one would make the event replay forever
    let id = fields.text("id")?;
    Some(Event {
        id: EventId::new(id),
        at: parse_time(fields.text("at").as_deref()).unwrap_or_default(),
        kind,
    })
}

fn event_kind(
    fields: &Fields,
    hub: &impl Fn(Option<String>) -> Option<PictureRef>,
) -> Option<EventKind> {
    let f = fields;
    Some(match f.text("kind")?.as_str() {
        kind @ ("light" | "cover" | "projector") => house_event(f, kind)?,
        "download" => EventKind::Download {
            name: f.text("name").unwrap_or_default(),
            size: f.number("size"),
        },
        "import" => EventKind::Import(import_event(f.0)?),
        "scan" => EventKind::Scan(Chapter {
            number: f.number("scan")? as u32,
            name: f.text("name").unwrap_or_default(),
            arc: f.text("arc"),
            pages: f.number("pages").map(|pages| pages as u32),
        }),
        "cerebro" => EventKind::Waiting {
            name: f.text("name").unwrap_or_default(),
            project: f.text("project"),
            room: f.text("room"),
        },
        "play" => EventKind::Play {
            title: f.text("title").unwrap_or_default(),
        },
        "print" => EventKind::Print(print_news(f, hub)?),
        _ => return None,
    })
}

/// A lamp, a cover or a projector. A switch the hub could not read is no
/// news, neither on nor off: None.
fn house_event(f: &Fields, kind: &str) -> Option<EventKind> {
    match kind {
        "light" => f
            .text("name")
            .zip(f.flag("on"))
            .map(|(name, on)| EventKind::Light { name, on }),
        "cover" => Some(EventKind::Cover {
            id: f.text("cover").unwrap_or_default(),
            name: f.text("name").unwrap_or_default(),
            down: cover_goes_down(f.0),
        }),
        _ => f.flag("on").map(|on| EventKind::Projector {
            name: f.text("name").unwrap_or_default(),
            on,
            app: f.text("app"),
        }),
    }
}

/// A cover goes down unless it moves up, or, standing still, rests at 0.
fn cover_goes_down(fields: &BTreeMap<String, Value>) -> bool {
    match fields.get("moving") {
        Some(Value::String(direction)) => !direction.contains("up"),
        _ => fields
            .get("target")
            .and_then(Value::as_f64)
            .is_none_or(|target| target > 0.0),
    }
}

/// An import event carries the same fields as the file's recent imports;
/// its poster is a URL already.
fn import_event(fields: &BTreeMap<String, Value>) -> Option<Import> {
    let model: ImportModel =
        serde_json::from_value(Value::Object(fields.clone().into_iter().collect())).ok()?;
    Some(import(model, &|path: Option<String>| {
        path.map(PictureRef::new)
    }))
}

fn print_news(
    f: &Fields,
    hub: &impl Fn(Option<String>) -> Option<PictureRef>,
) -> Option<PrintNews> {
    Some(PrintNews {
        phase: print_phase(&f.text("phase")?)?,
        printer: f.text("printer").unwrap_or_default(),
        file: f.text("file").unwrap_or_default(),
        image: hub(f.text("image")),
        layers: f.number("layers").map(|layers| layers as u32),
        remaining: f.0.get("remaining").and_then(Value::as_f64),
        started: parse_time(f.text("started_at").as_deref()),
    })
}

fn print_phase(phase: &str) -> Option<PrintPhase> {
    match phase {
        "started" => Some(PrintPhase::Started),
        "done" => Some(PrintPhase::Done),
        "cancelled" => Some(PrintPhase::Cancelled),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = include_str!("../../tests/fixtures/screen.json");

    fn snapshot() -> Snapshot {
        serde_json::from_str::<FeedFile>(SAMPLE)
            .unwrap()
            .into_snapshot("http://media.local:8096", "http://media.local")
    }

    #[test]
    fn a_jellyfin_picture_path_becomes_an_absolute_url() {
        let poster = snapshot().playing[0].poster.clone().unwrap();
        assert_eq!(
            poster.as_str(),
            "http://media.local:8096/Items/abc/Images/Primary"
        );
    }

    #[test]
    fn every_known_event_kind_is_read_and_an_unknown_one_is_skipped() {
        let kinds: Vec<_> = snapshot()
            .events
            .iter()
            .map(|e| std::mem::discriminant(&e.kind))
            .collect();
        assert_eq!(kinds.len(), 5);
    }

    #[test]
    fn the_printer_is_read_with_its_print() {
        let printer = snapshot().printer.unwrap();
        assert_eq!(printer.name, "Anycubic Photon Mono M7");
        assert_eq!(printer.state.as_deref(), Some("printing"));
        assert!(printer.active && printer.has_print());
        assert_eq!(printer.file.as_deref(), Some("01_Cafetière"));
        assert_eq!(
            (printer.progress, printer.layer, printer.layers),
            (Some(5.0), Some(166), Some(2978))
        );
        assert_eq!(printer.remaining, Some(15840.0));
        assert!(printer.ends.is_some());
        assert_eq!(printer.cloud, None);
        // the slicer's own link is kept as it is
        assert!(
            printer
                .image
                .unwrap()
                .as_str()
                .starts_with("https://cloud-slice")
        );
    }

    #[test]
    fn a_printer_without_the_cloud_has_no_print() {
        let file = r#"{"printer": {"name": null, "cloud": "no token", "state": null, "active": false, "file": null,
                        "progress": null, "layer": null, "layers": 0, "remaining": null, "image": null}}"#;
        let printer = serde_json::from_str::<FeedFile>(file)
            .unwrap()
            .into_snapshot("", "")
            .printer
            .unwrap();
        assert_eq!(printer.name, "Anycubic");
        assert_eq!(printer.cloud.as_deref(), Some("no token"));
        assert!(!printer.has_print());
        assert_eq!((printer.layers, printer.image), (None, None));
    }

    #[test]
    fn a_print_event_takes_its_picture_from_the_hub() {
        let news = snapshot().events.into_iter().find_map(|e| match e.kind {
            EventKind::Print(news) => Some(news),
            _ => None,
        });
        let news = news.unwrap();
        assert_eq!(news.phase, PrintPhase::Started);
        assert_eq!(news.file, "01_Cafetière");
        assert_eq!(
            news.image.unwrap().as_str(),
            "http://media.local/printer.jpg?v=1"
        );
        assert_eq!((news.layers, news.remaining), (Some(2978), Some(18720.0)));
        assert!(news.started.is_some());
    }

    #[test]
    fn a_cover_moving_up_is_not_going_down() {
        let up = snapshot().events.into_iter().find_map(|e| match e.kind {
            EventKind::Cover { down, .. } => Some(down),
            _ => None,
        });
        assert_eq!(up, Some(false));
    }

    #[test]
    fn the_cerebro_sessions_are_read_with_their_status() {
        let cerebro = snapshot().cerebro.unwrap();
        let statuses: Vec<_> = cerebro.sessions.iter().map(|s| s.status).collect();
        assert_eq!(
            statuses,
            [
                SessionStatus::Waiting,
                SessionStatus::Busy,
                SessionStatus::Idle
            ]
        );
        assert_eq!(cerebro.sessions[0].room.as_deref(), Some("Danger Room"));
        assert_eq!(cerebro.sessions[1].agents, 2);
    }

    #[test]
    fn a_scene_takes_its_sprites_from_the_hub() {
        let cerebro = snapshot().cerebro.unwrap();
        let scene = cerebro.sessions[0].scene.clone().unwrap();
        assert_eq!(
            scene.room.as_str(),
            "http://media.local/cerebro/rooms/danger-room.png"
        );
        assert_eq!(
            (scene.frames, scene.fps, scene.walk),
            (16, 8, (10, 76, 172, 40))
        );
        assert_eq!(scene.actors[0].state, ActorState::Alert);
        assert_eq!(scene.actors[0].power, "beam");
        assert_eq!(
            cerebro.emotes.unwrap().as_str(),
            "http://media.local/cerebro/ui/emotes.png"
        );
        assert_eq!(cerebro.sessions[2].scene, None);
    }

    #[test]
    fn a_session_waiting_is_an_event() {
        let waiting = snapshot().events.into_iter().find_map(|e| match e.kind {
            EventKind::Waiting { name, .. } => Some(name),
            _ => None,
        });
        assert_eq!(waiting.as_deref(), Some("errol-8b"));
    }

    #[test]
    fn without_cerebro_there_is_no_cerebro() {
        let file = r#"{"cerebro": null}"#;
        let snapshot = serde_json::from_str::<FeedFile>(file)
            .unwrap()
            .into_snapshot("", "");
        assert_eq!(snapshot.cerebro, None);
    }

    #[test]
    fn an_unknown_eta_is_none() {
        assert_eq!(snapshot().downloads[0].eta, None);
    }

    #[test]
    fn the_feed_errors_become_warnings() {
        assert_eq!(snapshot().warnings, vec!["house: timeout".to_owned()]);
    }

    #[test]
    fn the_settings_name_the_look_asked_for() {
        assert_eq!(
            snapshot().settings,
            Settings {
                look: Some("hogwarts".into()),
                follow_hub: true
            }
        );
    }

    #[test]
    fn an_empty_look_is_no_look() {
        let file = r#"{"settings": {"look": "", "follow_hub": false}}"#;
        let snapshot = serde_json::from_str::<FeedFile>(file)
            .unwrap()
            .into_snapshot("", "");
        assert_eq!(snapshot.settings.look, None);
    }

    #[test]
    fn nulls_read_like_absent_fields() {
        let file = r#"{"playing": [{"id": "x", "type": null, "overview": null, "genres": null, "runtime": null,
                        "position": null, "paused": null}],
                       "events": null, "downloads": null, "recent": null, "errors": null,
                       "house": {"outlets": [{"id": "lamp", "name": null, "on": null}], "covers": null,
                                 "projectors": [{"id": "projector", "name": "Projector", "on": null, "app": null}]},
                       "portfolio": {"name": null, "mode": null, "equity": null, "initial": null, "trades": null,
                                     "positions": [{"pair": null, "avg": null, "price": null}]},
                       "disk": {"total": null, "used": null, "free": null},
                       "services": {"up": null, "total": null},
                       "settings": {"look": null, "follow_hub": null}}"#;
        let snapshot = serde_json::from_str::<FeedFile>(file)
            .unwrap()
            .into_snapshot("", "");
        let house = snapshot.house.unwrap();
        assert!(!house.projectors[0].on);
        assert!(!snapshot.playing[0].paused);
        assert_eq!(snapshot.portfolio.unwrap().trades, 0);
    }

    #[test]
    fn a_broken_event_is_skipped_and_the_rest_is_read() {
        let file = r#"{"events": [null, 3, {"id": "1", "kind": "light", "name": "Lamp", "on": true}],
                       "errors": {"house": null}}"#;
        let snapshot = serde_json::from_str::<FeedFile>(file)
            .unwrap()
            .into_snapshot("", "");
        assert_eq!(snapshot.events.len(), 1);
        assert_eq!(snapshot.warnings, vec!["house: ".to_owned()]);
    }

    #[test]
    fn a_switch_the_hub_could_not_read_is_no_event() {
        let file = r#"{"events": [{"id": "1", "kind": "projector", "name": "Projector", "on": null},
                                  {"id": "2", "kind": "light", "name": "Lamp"},
                                  {"id": "3", "kind": "projector", "name": "Projector", "on": false}]}"#;
        let snapshot = serde_json::from_str::<FeedFile>(file)
            .unwrap()
            .into_snapshot("", "");
        let ids: Vec<_> = snapshot.events.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, ["3"]);
    }

    #[test]
    fn the_weather_the_episodes_the_day_and_the_hub_schedules_are_read() {
        let file = r#"{"weather": {"temperature": 9.4, "code": 61, "wind": 18, "high": 12, "low": 6, "rain": 70,
                                   "sunrise": "2026-11-08T07:58:00+01:00", "sunset": "2026-11-08T17:21:00+01:00"},
                       "next_up": [{"series": "Arcane", "season": 2, "episode": 5, "poster": "/Items/a/Images/Primary"},
                                   {"series": null}],
                       "airing": [{"series": "Arcane", "season": 2, "episode": 6, "at": "2026-11-08T21:00:00+01:00",
                                   "poster": "https://artworks.thetvdb.com/a.jpg"}],
                       "today": {"imports": 3, "lights": null},
                       "services": {"up": 17, "total": 18, "down": ["prowlarr"]},
                       "house": {"outlets": [{"id": "neon", "name": "Néon", "on": false,
                                              "next": {"at": "2026-11-08T19:00+01:00", "on": true}}],
                                 "projectors": [{"id": "p", "name": "P", "on": false,
                                                 "closes_at": "2026-11-08T23:05:00+01:00"}]}}"#;
        let snapshot = serde_json::from_str::<FeedFile>(file)
            .unwrap()
            .into_snapshot("http://jf", "");
        let weather = snapshot.weather.unwrap();
        assert_eq!(weather.sky(), errol_domain::entities::Sky::Rain);
        assert!(weather.sunset.is_some());
        assert_eq!(snapshot.next_up.len(), 1);
        assert_eq!(
            snapshot.next_up[0].poster.as_ref().unwrap().as_str(),
            "http://jf/Items/a/Images/Primary"
        );
        assert!(snapshot.airing[0].at.is_some());
        assert_eq!(snapshot.today.unwrap().imports, 3);
        assert_eq!(snapshot.services.down, ["prowlarr"]);
        let house = snapshot.house.unwrap();
        assert!(house.outlets[0].next.is_some_and(|next| next.on));
        assert!(house.projectors[0].closes_at.is_some());
    }

    #[test]
    fn an_empty_file_is_an_empty_snapshot() {
        let snapshot = serde_json::from_str::<FeedFile>("{}")
            .unwrap()
            .into_snapshot("", "");
        assert_eq!(snapshot, Snapshot::default());
    }
}
