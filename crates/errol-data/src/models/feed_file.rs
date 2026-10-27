use std::collections::BTreeMap;

use chrono::DateTime;
use errol_domain::entities::{
    Chapter, Cover, Disk, Download, Event, EventId, EventKind, House, Import, MediaKind,
    NextChapter, OnePiece, Outlet, PictureRef, Playing, Portfolio, Position, Projector, Services,
    Snapshot, Timestamp,
};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct FeedFile {
    updated_at: Option<String>,
    playing: Vec<PlayingModel>,
    events: Vec<BTreeMap<String, Value>>,
    house: Option<HouseModel>,
    downloads: Vec<DownloadModel>,
    recent: Vec<ImportModel>,
    one_piece: Option<OnePieceModel>,
    portfolio: Option<PortfolioModel>,
    disk: Option<DiskModel>,
    services: Option<ServicesModel>,
    errors: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct PlayingModel {
    id: String,
    #[serde(rename = "type")]
    kind: String,
    title: Option<String>,
    series: Option<String>,
    season: Option<u32>,
    episode: Option<u32>,
    year: Option<u32>,
    overview: String,
    genres: Vec<String>,
    runtime: f64,
    position: f64,
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
    outlets: Vec<OutletModel>,
    covers: Vec<CoverModel>,
    projectors: Vec<ProjectorModel>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct OutletModel {
    id: String,
    name: String,
    on: bool,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct CoverModel {
    id: String,
    name: String,
    position: Option<f64>,
    moving: Option<Value>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct ProjectorModel {
    id: String,
    name: String,
    on: bool,
    app: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct DownloadModel {
    name: String,
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
    scan: u32,
    name: String,
    arc: Option<String>,
    pages: Option<u32>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct NextModel {
    scan: u32,
    at: String,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct PortfolioModel {
    name: Option<String>,
    mode: String,
    equity: f64,
    initial: f64,
    trades: u32,
    positions: Vec<PositionModel>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct PositionModel {
    pair: String,
    avg: f64,
    price: Option<f64>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct DiskModel {
    total: u64,
    used: u64,
    free: u64,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct ServicesModel {
    up: u32,
    total: u32,
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

impl FeedFile {
    /// Turns the file into a snapshot. Jellyfin pictures are paths on the
    /// Jellyfin server, made absolute with `jellyfin`.
    pub fn into_snapshot(self, jellyfin: &str) -> Snapshot {
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
            events: self.events.iter().filter_map(event).collect(),
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
            warnings: self
                .errors
                .into_iter()
                .map(|(source, error)| format!("{source}: {error}"))
                .collect(),
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
            .into_snapshot("http://media.local:8096")
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
        assert_eq!(kinds.len(), 3);
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
    fn an_unknown_eta_is_none() {
        assert_eq!(snapshot().downloads[0].eta, None);
    }

    #[test]
    fn the_feed_errors_become_warnings() {
        assert_eq!(snapshot().warnings, vec!["house: timeout".to_owned()]);
    }

    #[test]
    fn an_empty_file_is_an_empty_snapshot() {
        let snapshot = serde_json::from_str::<FeedFile>("{}")
            .unwrap()
            .into_snapshot("");
        assert_eq!(snapshot, Snapshot::default());
    }
}
