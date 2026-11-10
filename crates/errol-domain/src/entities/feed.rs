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
    /// The Claude Code sessions of the Mac, while Cerebro answers.
    pub cerebro: Option<Cerebro>,
    /// The resin printer and its print, while it is on.
    pub printer: Option<Printer>,
    /// Sources the feed could not read, with the reason.
    pub warnings: Vec<String>,
    /// What the user asked of the screen, through the feed (its remote control).
    pub settings: Settings,
    /// The weather outside, and the sun.
    pub weather: Option<Weather>,
    /// The next episodes to watch, Jellyfin's "next up".
    pub next_up: Vec<Episode>,
    /// Episodes that air today or tomorrow, from Sonarr's calendar.
    pub airing: Vec<Episode>,
    /// What happened since midnight.
    pub today: Option<Today>,
}

/// The weather outside, as Open-Meteo gives it.
#[derive(Debug, Clone, PartialEq)]
pub struct Weather {
    /// °C, now.
    pub temperature: f64,
    /// WMO weather code, now.
    pub code: u32,
    /// km/h, now.
    pub wind: Option<f64>,
    /// Today's highest and lowest, °C.
    pub high: Option<f64>,
    pub low: Option<f64>,
    /// Chance of rain in the coming hours, in percent.
    pub rain: Option<f64>,
    pub sunrise: Option<Timestamp>,
    pub sunset: Option<Timestamp>,
}

/// The sky, in a few words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sky {
    Clear,
    Cloudy,
    Overcast,
    Fog,
    Drizzle,
    Rain,
    Snow,
    Storm,
}

impl Weather {
    /// The sky of a WMO code.
    pub fn sky(&self) -> Sky {
        match self.code {
            0 | 1 => Sky::Clear,
            3 => Sky::Overcast,
            45 | 48 => Sky::Fog,
            51..=57 => Sky::Drizzle,
            61..=67 | 80..=82 => Sky::Rain,
            71..=77 | 85 | 86 => Sky::Snow,
            95..=99 => Sky::Storm,
            // 2, partly cloudy, and any code the screen does not know
            _ => Sky::Cloudy,
        }
    }

    /// The sun is up at `now`; None without sunrise and sunset.
    pub fn daylight(&self, now: Timestamp) -> Option<bool> {
        Some(now >= self.sunrise? && now < self.sunset?)
    }
}

/// An episode to watch or about to air.
#[derive(Debug, Clone, PartialEq)]
pub struct Episode {
    pub series: String,
    pub season: Option<u32>,
    pub episode: Option<u32>,
    pub title: Option<String>,
    pub poster: Option<PictureRef>,
    /// When it airs; None for one already in the library.
    pub at: Option<Timestamp>,
}

/// How many times each thing happened since midnight.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Today {
    /// Films and episodes that entered the library.
    pub imports: u32,
    pub downloads: u32,
    /// Videos started on Jellyfin.
    pub plays: u32,
    /// Lamps switched on.
    pub lights: u32,
    /// Prints finished.
    pub prints: u32,
    /// Times a Claude Code session waited for an answer.
    pub questions: u32,
}

impl Today {
    /// Nothing happened yet.
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
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
    /// Its next scheduled switch, when it has a schedule.
    pub next: Option<Switch>,
}

/// A scheduled switch: at `at`, on or off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Switch {
    pub at: Timestamp,
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
    /// When its projection screen goes back up, the projector being off.
    pub closes_at: Option<Timestamp>,
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
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Services {
    pub up: u32,
    pub total: u32,
    /// The names of those down.
    pub down: Vec<String>,
}

/// The Claude Code sessions running on the Mac, as Cerebro shows them: those
/// waiting for an answer first, then those working.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Cerebro {
    pub sessions: Vec<Session>,
    /// Size of a room's frame and of a character's, in Cerebro's pixels.
    pub room_size: (u32, u32),
    pub character_size: (u32, u32),
    /// The sheet of emotes: asleep (Zz), alert (!).
    pub emotes: Option<PictureRef>,
}

impl Cerebro {
    /// How many sessions are in `status`.
    pub fn count(&self, status: SessionStatus) -> usize {
        self.sessions
            .iter()
            .filter(|session| session.status == status)
            .count()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionStatus {
    /// Claude is working.
    Busy,
    /// Claude waits for an answer (a permission to give).
    Waiting,
    /// Claude has finished and waits for the next prompt.
    Idle,
}

/// A Claude Code session, in its room of the X-Mansion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    pub name: String,
    /// The folder it works in, `~/…`.
    pub project: Option<String>,
    pub status: SessionStatus,
    /// Since when it is in that status.
    pub since: Option<Timestamp>,
    /// Its room of the X-Mansion (Danger Room…).
    pub room: Option<String>,
    /// The X-Man who leads it.
    pub lead: Option<String>,
    /// Subagents it runs now.
    pub agents: u32,
    /// What Cerebro draws for it, when the feed has it.
    pub scene: Option<SessionScene>,
}

/// A session's room and the X-Men in it, as Cerebro's panel draws them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionScene {
    /// The room's sprite sheet: its frames side by side.
    pub room: PictureRef,
    pub frames: u32,
    pub fps: u32,
    /// Where the actors walk: x, y, width, height, in the room's pixels.
    pub walk: (u32, u32, u32, u32),
    pub actors: Vec<Actor>,
}

/// An X-Man of a scene: the session's lead, or a subagent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Actor {
    /// Seeds its wandering: the same id walks the same path on every screen.
    pub id: String,
    pub name: String,
    /// Its sheet: 4 frames by 3 rows (front, back, side).
    pub sprite: PictureRef,
    /// How it travels: walk, fly, teleport, phase, dash.
    pub movement: String,
    /// The power it uses when it pauses (beam, bolt, psychic…).
    pub power: String,
    /// `#RRGGBB` of its power, and of its aura when it has one.
    pub colour: String,
    pub aura: Option<String>,
    pub state: ActorState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorState {
    Walking,
    Asleep,
    /// Waiting for an answer: an exclamation mark blinks over it.
    Alert,
}

/// The resin printer, while it is on, and its latest print (the one going on,
/// or the last one, finished or cancelled).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Printer {
    pub name: String,
    /// The print's state as the cloud names it: printing, done, cancelled,
    /// preheating… None when the cloud does not say (no token).
    pub state: Option<String>,
    /// A print is going on.
    pub active: bool,
    pub file: Option<String>,
    /// Percent done.
    pub progress: Option<f64>,
    pub layer: Option<u32>,
    pub layers: Option<u32>,
    /// Seconds left, while it prints.
    pub remaining: Option<f64>,
    /// When it should end, while it prints.
    pub ends: Option<Timestamp>,
    /// The slicer's picture of the print.
    pub image: Option<PictureRef>,
    /// What the cloud said when it gave no print ("no token", "token
    /// rejected", "unreachable"…); None when it answers.
    pub cloud: Option<String>,
}

impl Printer {
    /// A print is on the plate or has just left it.
    pub fn has_print(&self) -> bool {
        self.state.is_some()
    }
}
