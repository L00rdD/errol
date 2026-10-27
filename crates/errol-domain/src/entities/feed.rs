use super::{Event, PictureRef, Timestamp};

/// Everything the screen knows at one moment, as written by the feed service.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Snapshot {
    /// When the feed service wrote it.
    pub updated_at: Timestamp,
    /// Videos playing on Jellyfin, one per session.
    pub playing: Vec<Playing>,
    /// The last changes the feed noticed, oldest first.
    pub events: Vec<Event>,
    pub house: Option<House>,
    /// Torrents still downloading.
    pub downloads: Vec<Download>,
    /// Latest films and episodes added to the library, one per title.
    pub recent: Vec<Import>,
    pub one_piece: Option<OnePiece>,
    pub portfolio: Option<Portfolio>,
    pub disk: Option<Disk>,
    pub services: Services,
    /// Sources the feed could not read, with the reason.
    pub warnings: Vec<String>,
    /// What the user asked of the screen, through the feed (its remote control).
    pub settings: Settings,
}

/// Settings the user sets from elsewhere (the home page) and the feed passes on.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Settings {
    /// The look asked for ("friday", "cyberpunk", "hogwarts"); None leaves the screen's own.
    pub look: Option<String>,
    /// The look follows the home page's (the home page sets `look` when it changes).
    pub follow_hub: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaKind {
    Movie,
    Episode,
}

/// A video playing on Jellyfin.
#[derive(Debug, Clone, PartialEq)]
pub struct Playing {
    pub id: String,
    pub kind: MediaKind,
    pub title: String,
    pub series: Option<String>,
    pub season: Option<u32>,
    pub episode: Option<u32>,
    pub year: Option<u32>,
    pub overview: String,
    pub genres: Vec<String>,
    /// Length of the video, in seconds (0 when unknown).
    pub runtime: f64,
    /// Where the playback was when the feed was written, in seconds.
    pub position: f64,
    pub paused: bool,
    pub device: String,
    pub client: String,
    pub user: String,
    pub poster: Option<PictureRef>,
    pub backdrop: Option<PictureRef>,
}

impl Playing {
    /// Where the playback is at `now`: the feed's position, moved on by the
    /// time elapsed since `written` unless paused, at most 10 s ahead (a feed
    /// that stopped answering must not run the bar to the end).
    pub fn position_at(&self, written: Timestamp, now: Timestamp) -> f64 {
        let ahead = if self.paused {
            0.0
        } else {
            now.secs_since(written).clamp(0.0, 10.0)
        };
        let position = self.position + ahead;
        if self.runtime > 0.0 {
            position.min(self.runtime)
        } else {
            position
        }
    }
}

/// The house, as a home hub reports it: switched outlets, covers, projectors.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct House {
    pub outlets: Vec<Outlet>,
    pub covers: Vec<Cover>,
    pub projectors: Vec<Projector>,
}

/// A switched outlet: a lamp, a neon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outlet {
    pub id: String,
    pub name: String,
    pub on: bool,
}

/// A motorised cover: the projection screen.
#[derive(Debug, Clone, PartialEq)]
pub struct Cover {
    pub id: String,
    pub name: String,
    /// 0 = rolled up, 100 = fully down.
    pub position: f64,
    pub moving: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Projector {
    pub id: String,
    pub name: String,
    pub on: bool,
    /// The app open on it (Netflix, Jellyfin…), when known.
    pub app: Option<String>,
}

/// A torrent still downloading.
#[derive(Debug, Clone, PartialEq)]
pub struct Download {
    pub name: String,
    pub percent: f64,
    /// Seconds left, when Transmission knows.
    pub eta: Option<f64>,
}

/// A film or an episode imported into the library by Radarr or Sonarr.
#[derive(Debug, Clone, PartialEq)]
pub struct Import {
    pub kind: MediaKind,
    pub title: String,
    pub year: Option<u32>,
    pub season: Option<u32>,
    pub episode: Option<u32>,
    pub episode_title: Option<String>,
    pub quality: Option<String>,
    pub poster: Option<PictureRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chapter {
    pub number: u32,
    pub name: String,
    pub arc: Option<String>,
    pub pages: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NextChapter {
    pub number: u32,
    pub at: Timestamp,
}

/// The latest One Piece scan and the next one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnePiece {
    pub latest: Chapter,
    pub next: Option<NextChapter>,
}

/// A trading bot's portfolio.
#[derive(Debug, Clone, PartialEq)]
pub struct Portfolio {
    /// What the screen calls it (the bot's name).
    pub name: String,
    /// "paper" or "live".
    pub mode: String,
    pub equity: f64,
    pub initial: f64,
    pub trades: u32,
    pub positions: Vec<Position>,
}

impl Portfolio {
    /// Change since the start, in percent.
    pub fn change(&self) -> f64 {
        if self.initial > 0.0 {
            (self.equity / self.initial - 1.0) * 100.0
        } else {
            0.0
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Position {
    pub pair: String,
    pub average_price: f64,
    pub price: Option<f64>,
}

impl Position {
    /// Change since the position was opened, in percent.
    pub fn change(&self) -> Option<f64> {
        self.price
            .filter(|_| self.average_price > 0.0)
            .map(|price| (price / self.average_price - 1.0) * 100.0)
    }
}

/// Space on the storage pool, in bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Disk {
    pub total: u64,
    pub used: u64,
    pub free: u64,
}

/// Containers running, out of all of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Services {
    pub up: u32,
    pub total: u32,
}
