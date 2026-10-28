use std::collections::BTreeMap;

use chrono::DateTime;
use errol_domain::entities::{
    Actor, ActorState, Cerebro, Chapter, Cover, Disk, Download, Event, EventId, EventKind, House,
    Import, MediaKind, NextChapter, OnePiece, Outlet, PictureRef, Playing, Portfolio, Position,
    Projector, Services, Session, SessionScene, SessionStatus, Settings, Snapshot, Timestamp,
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
    #[serde(deserialize_with = "nullable")]
    errors: BTreeMap<String, Option<String>>,
    settings: Option<SettingsModel>,
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

/// RFC 3339 to a timestamp; None when absent or malformed.
fn parse_time(text: Option<&str>) -> Option<Timestamp> {
    text.and_then(|text| DateTime::parse_from_rfc3339(text).ok())
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

/// A status Cerebro does not name reads as idle: nothing to show for it.
fn status(text: Option<&str>) -> SessionStatus {
    match text {
        Some("busy") => SessionStatus::Busy,
        Some("waiting") => SessionStatus::Waiting,
        _ => SessionStatus::Idle,
    }
}

impl FeedFile {
    /// Turns the file into a snapshot. Jellyfin pictures are paths on the
    /// Jellyfin server, made absolute with `jellyfin`; Cerebro's sprites are
    /// paths on the home page (`hub`, its origin), which passes them on.
    pub fn into_snapshot(self, jellyfin: &str, hub: &str) -> Snapshot {
        let sprite = |path: Option<String>| {
            path.filter(|path| !path.is_empty()).map(|path| {
                if path.starts_with("http") {
                    PictureRef::new(path)
                } else {
                    PictureRef::new(format!("{hub}{path}"))
                }
            })
        };
        let picture = |path: Option<String>| {
            path.filter(|path| !path.is_empty()).map(|path| {
                if path.starts_with("http") {
                    PictureRef::new(path)
                } else {
                    PictureRef::new(format!("{jellyfin}{path}"))
                }
            })
        };
        Snapshot {
            updated_at: parse_time(self.updated_at.as_deref()).unwrap_or_default(),
            playing: self
                .playing
                .into_iter()
                .map(|item| Playing {
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
                })
                .collect(),
            events: self
                .events
                .iter()
                .filter_map(Value::as_object)
                .map(|fields| fields.clone().into_iter().collect::<BTreeMap<_, _>>())
                .filter_map(|fields| event(&fields))
                .collect(),
            house: self.house.map(|house| House {
                outlets: house
                    .outlets
                    .into_iter()
                    .map(|o| Outlet {
                        id: o.id,
                        name: o.name,
                        on: o.on,
                    })
                    .collect(),
                covers: house
                    .covers
                    .into_iter()
                    .map(|c| Cover {
                        id: c.id,
                        name: c.name,
                        position: c.position.unwrap_or_default(),
                        moving: c.moving.is_some_and(|moving| {
                            !moving.is_null() && moving != Value::Bool(false)
                        }),
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
                    })
                    .collect(),
            }),
            downloads: self
                .downloads
                .into_iter()
                .map(|d| Download {
                    name: d.name,
                    percent: d.percent,
                    eta: d.eta.filter(|eta| *eta >= 0.0),
                })
                .collect(),
            recent: self
                .recent
                .into_iter()
                .map(|i| import(i, &picture))
                .collect(),
            one_piece: self.one_piece.and_then(|k| {
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
            }),
            portfolio: self.portfolio.map(|d| Portfolio {
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
            }),
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
                })
                .unwrap_or_default(),
            cerebro: self.cerebro.map(|cerebro| Cerebro {
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
                        scene: s.scene.and_then(|scene| scene_of(scene, &sprite)),
                    })
                    .collect(),
            }),
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
        }
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

/// One event of the file; None for a kind this version does not know.
fn event(fields: &BTreeMap<String, Value>) -> Option<Event> {
    let text = |key: &str| fields.get(key).and_then(Value::as_str).map(str::to_owned);
    let flag = |key: &str| fields.get(key).and_then(Value::as_bool).unwrap_or(false);
    let number = |key: &str| fields.get(key).and_then(Value::as_u64);
    let kind = match text("kind")?.as_str() {
        "light" => EventKind::Light {
            name: text("name")?,
            on: flag("on"),
        },
        "cover" => EventKind::Cover {
            id: text("cover").unwrap_or_default(),
            name: text("name").unwrap_or_default(),
            down: match fields.get("moving") {
                Some(Value::String(direction)) => !direction.contains("up"),
                _ => fields
                    .get("target")
                    .and_then(Value::as_f64)
                    .is_none_or(|target| target > 0.0),
            },
        },
        "projector" => EventKind::Projector {
            name: text("name").unwrap_or_default(),
            on: flag("on"),
            app: text("app"),
        },
        "download" => EventKind::Download {
            name: text("name").unwrap_or_default(),
            size: number("size"),
        },
        "import" => {
            let model: ImportModel =
                serde_json::from_value(Value::Object(fields.clone().into_iter().collect())).ok()?;
            EventKind::Import(import(model, &|path: Option<String>| {
                path.map(PictureRef::new)
            }))
        }
        "scan" => EventKind::Scan(Chapter {
            number: number("scan")? as u32,
            name: text("name").unwrap_or_default(),
            arc: text("arc"),
            pages: number("pages").map(|pages| pages as u32),
        }),
        "cerebro" => EventKind::Waiting {
            name: text("name").unwrap_or_default(),
            project: text("project"),
            room: text("room"),
        },
        "play" => EventKind::Play {
            title: text("title").unwrap_or_default(),
        },
        _ => return None,
    };
    // the feed's ids carry their own counter; a missing one would make the event replay forever
    let id = text("id")?;
    Some(Event {
        id: EventId::new(id),
        at: parse_time(text("at").as_deref()).unwrap_or_default(),
        kind,
    })
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
        assert_eq!(kinds.len(), 4);
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
    fn an_empty_file_is_an_empty_snapshot() {
        let snapshot = serde_json::from_str::<FeedFile>("{}")
            .unwrap()
            .into_snapshot("", "");
        assert_eq!(snapshot, Snapshot::default());
    }
}
