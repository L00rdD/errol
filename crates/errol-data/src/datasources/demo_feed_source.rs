//! A made-up evening, to see every scene without waiting for the house:
//! a neon switches on, the projection screen comes down, the projector starts,
//! a film plays, a Claude Code session waits for an answer, an episode and a
//! download arrive, a new scan is out, a resin print finishes. Pictures
//! come from TMDB. `DemoScene` other than `All` holds a single scene.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use errol_domain::DomainError;
use errol_domain::entities::{
    Actor, ActorState, Cerebro, Chapter, Cover, Disk, Download, Event, EventId, EventKind, House,
    Import, MediaKind, NextChapter, OnePiece, Outlet, PictureRef, Playing, Portfolio, Position,
    PrintNews, PrintPhase, Printer, Projector, Services, Session, SessionScene, SessionStatus,
    Snapshot, Timestamp,
};
use errol_domain::ports::{Clock, FeedSource};

/// Length of one evening of the demo, in seconds.
const CYCLE: f64 = 240.0;
const TMDB: &str = "https://image.tmdb.org/t/p/";
/// Cerebro's sprites for the demo: a Mac running it (`ERROL_DEMO_CEREBRO`
/// overrides it); without one, the rooms stay empty.
const CEREBRO: &str = "http://172.28.115.163:4243/";

fn cerebro_sprite(path: &str) -> PictureRef {
    let base = std::env::var("ERROL_DEMO_CEREBRO").unwrap_or_else(|_| CEREBRO.to_owned());
    PictureRef::new(format!("{}/{path}", base.trim_end_matches('/')))
}

/// An X-Man as Cerebro's manifest describes it.
fn actor(
    id: &str,
    character: &str,
    name: &str,
    movement: &str,
    power: &str,
    colour: &str,
    aura: Option<&str>,
    state: ActorState,
) -> Actor {
    Actor {
        id: id.into(),
        name: name.into(),
        sprite: cerebro_sprite(&format!("characters/{character}.png")),
        movement: movement.into(),
        power: power.into(),
        colour: colour.into(),
        aura: aura.map(str::to_owned),
        state,
    }
}

fn room(id: &str, frames: u32, walk: (u32, u32, u32, u32), actors: Vec<Actor>) -> SessionScene {
    SessionScene {
        room: cerebro_sprite(&format!("rooms/{id}.png")),
        frames,
        fps: 8,
        walk,
        actors,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DemoScene {
    All,
    Idle,
    Light,
    Cover,
    Projector,
    Cinema,
    Import,
    Download,
    Scan,
    Cerebro,
    Print,
}

impl DemoScene {
    pub fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "all" | "" => Self::All,
            "idle" => Self::Idle,
            "light" => Self::Light,
            "cover" => Self::Cover,
            "projector" => Self::Projector,
            "cinema" => Self::Cinema,
            "import" => Self::Import,
            "download" => Self::Download,
            "scan" => Self::Scan,
            "cerebro" => Self::Cerebro,
            "print" => Self::Print,
            _ => return None,
        })
    }
}

pub struct DemoFeedSource {
    scene: DemoScene,
    clock: Arc<dyn Clock>,
    start: Timestamp,
    first: AtomicBool,
}

impl DemoFeedSource {
    pub fn new(scene: DemoScene, clock: Arc<dyn Clock>) -> Self {
        let start = clock.now();
        Self {
            scene,
            clock,
            start,
            first: AtomicBool::new(true),
        }
    }
}

/// The timeline: when each event happens in a cycle.
fn timeline() -> Vec<(f64, DemoScene, EventKind)> {
    let mut timeline = house_timeline();
    timeline.extend(news_timeline());
    timeline
}

/// The house's part of the evening: the neon, the screen, the projector.
fn house_timeline() -> Vec<(f64, DemoScene, EventKind)> {
    vec![
        (
            6.0,
            DemoScene::Light,
            EventKind::Light {
                name: "Néon".into(),
                on: true,
            },
        ),
        (
            34.0,
            DemoScene::Cover,
            EventKind::Cover {
                id: "screen".into(),
                name: "Écran".into(),
                down: true,
            },
        ),
        (
            76.0,
            DemoScene::Projector,
            EventKind::Projector {
                name: "Projecteur".into(),
                on: true,
                app: Some("Jellyfin".into()),
            },
        ),
    ]
}

/// The news of the evening: the Mac, the media, One Piece, the printer.
fn news_timeline() -> Vec<(f64, DemoScene, EventKind)> {
    vec![
        (
            122.0,
            DemoScene::Cerebro,
            EventKind::Waiting {
                name: "errol-8b".into(),
                project: Some("~/personal/errol".into()),
                room: Some("Danger Room".into()),
            },
        ),
        (150.0, DemoScene::Import, EventKind::Import(arcane())),
        (
            176.0,
            DemoScene::Download,
            EventKind::Download {
                name: "Dune.Part.Two.2024.1080p.BluRay.x265".into(),
                size: Some(6_400_000_000),
            },
        ),
        (
            200.0,
            DemoScene::Scan,
            EventKind::Scan(Chapter {
                number: 1197,
                name: "Le roi des géants".into(),
                arc: Some("Elbaf".into()),
                pages: Some(17),
            }),
        ),
        (
            224.0,
            DemoScene::Print,
            EventKind::Print(PrintNews {
                phase: PrintPhase::Started,
                printer: "Anycubic Photon Mono M7".into(),
                file: "04_riser_lychee".into(),
                image: None,
                layers: Some(2978),
                remaining: Some(18_720.0),
                started: None,
            }),
        ),
    ]
}

/// The resin printer, `done` percent into a print of 2978 layers.
fn printer(now: Timestamp, done: f64) -> Printer {
    let left = 18_720.0 * (1.0 - done / 100.0);
    Printer {
        name: "Anycubic Photon Mono M7".into(),
        state: Some("printing".into()),
        active: true,
        file: Some("04_riser_lychee".into()),
        progress: Some(done),
        layer: Some((2978.0 * done / 100.0) as u32),
        layers: Some(2978),
        remaining: Some(left),
        ends: Some(now.plus_secs(left)),
        image: None,
        cloud: None,
    }
}

fn arcane() -> Import {
    Import {
        kind: MediaKind::Episode,
        title: "Arcane".into(),
        year: Some(2021),
        season: Some(2),
        episode: Some(4),
        episode_title: Some("Paint the Town Blue".into()),
        quality: Some("WEBDL-1080p".into()),
        poster: Some(PictureRef::new(format!(
            "{TMDB}w342/fqldf2t8ztc9aiwn3k6mlX3tvRT.jpg"
        ))),
    }
}

fn film(position: f64, paused: bool) -> Playing {
    Playing {
        id: "demo-film".into(),
        kind: MediaKind::Movie,
        title: "Mission: Impossible - Dead Reckoning Part One".into(),
        series: None,
        season: None,
        episode: None,
        year: Some(2023),
        overview: "Ethan Hunt et l'équipe IMF se lancent dans leur mission la plus périlleuse : retrouver une nouvelle arme \
                   terrifiante qui menace l'humanité entière avant qu'elle ne tombe entre de mauvaises mains."
            .into(),
        genres: vec!["Action".into(), "Thriller".into()],
        runtime: 9780.0,
        position,
        paused,
        device: "Projecteur".into(),
        client: "Jellyfin Android TV".into(),
        user: "d".into(),
        poster: Some(PictureRef::new(format!("{TMDB}w342/NNxYkU70HPurnNCSiCjYAmacwm.jpg"))),
        backdrop: Some(PictureRef::new(format!("{TMDB}w780/628Dep6AxEtDxjZoGP78TsOxYbK.jpg"))),
    }
}

fn base(now: Timestamp) -> Snapshot {
    Snapshot {
        updated_at: now,
        house: Some(demo_house()),
        downloads: vec![Download {
            name: "Dune.Part.Two.2024.1080p.BluRay.x265".into(),
            percent: 63.4,
            eta: Some(1260.0),
        }],
        recent: demo_recent(),
        one_piece: Some(demo_one_piece(now)),
        portfolio: Some(demo_portfolio()),
        disk: Some(Disk {
            total: 1_430_000_000_000,
            used: 980_000_000_000,
            free: 450_000_000_000,
        }),
        services: Services { up: 18, total: 18 },
        cerebro: Some(demo_cerebro(now)),
        ..Snapshot::default()
    }
}

/// The lamps on, the neon off, the screen up, the projector off.
fn demo_house() -> House {
    House {
        outlets: vec![
            Outlet {
                id: "floor-lamp".into(),
                name: "Lampadaire".into(),
                on: true,
            },
            Outlet {
                id: "neon".into(),
                name: "Néon".into(),
                on: false,
            },
        ],
        covers: vec![Cover {
            id: "screen".into(),
            name: "Écran".into(),
            position: 0.0,
            moving: false,
        }],
        projectors: vec![Projector {
            id: "projector".into(),
            name: "Projecteur".into(),
            on: false,
            app: None,
        }],
    }
}

/// What entered the library last.
fn demo_recent() -> Vec<Import> {
    vec![
        Import {
            kind: MediaKind::Movie,
            title: "Mission: Impossible - Dead Reckoning".into(),
            year: Some(2023),
            season: None,
            episode: None,
            episode_title: None,
            quality: None,
            poster: Some(PictureRef::new(format!(
                "{TMDB}w342/NNxYkU70HPurnNCSiCjYAmacwm.jpg"
            ))),
        },
        arcane(),
    ]
}

/// The latest chapter, the next in under three days.
fn demo_one_piece(now: Timestamp) -> OnePiece {
    OnePiece {
        latest: Chapter {
            number: 1196,
            name: "L'amour de ceux qui veulent mourir n'a aucune valeur".into(),
            arc: Some("Elbaf".into()),
            pages: Some(17),
        },
        next: Some(NextChapter {
            number: 1197,
            at: now.plus_secs(2.8 * 86_400.0),
        }),
    }
}

/// The trading bot, a little up.
fn demo_portfolio() -> Portfolio {
    Portfolio {
        name: "Bot".into(),
        mode: "paper".into(),
        equity: 1087.42,
        initial: 1000.0,
        trades: 12,
        positions: vec![
            Position {
                pair: "BTC/USDT".into(),
                average_price: 60_000.0,
                price: Some(63_150.0),
            },
            Position {
                pair: "ETH/USDT".into(),
                average_price: 3_000.0,
                price: Some(2_910.0),
            },
            Position {
                pair: "SOL/USDT".into(),
                average_price: 140.0,
                price: Some(151.2),
            },
        ],
    }
}

/// Three sessions on the Mac.
fn demo_cerebro(now: Timestamp) -> Cerebro {
    Cerebro {
        room_size: (192, 120),
        character_size: (24, 24),
        emotes: Some(cerebro_sprite("ui/emotes.png")),
        sessions: vec![errol_session(now), skylink_session(now), pensieve_session()],
    }
}

/// The session that asks for an answer later in the evening.
fn errol_session(now: Timestamp) -> Session {
    Session {
        name: "errol-8b".into(),
        project: Some("~/personal/errol".into()),
        status: SessionStatus::Busy,
        since: Some(now.plus_secs(-754.0)),
        room: Some("Danger Room".into()),
        lead: Some("Cyclops".into()),
        agents: 1,
        scene: Some(room(
            "danger-room",
            16,
            (14, 64, 164, 50),
            vec![
                actor(
                    "lead:demo-errol",
                    "cyclops",
                    "Cyclops",
                    "walk",
                    "beam",
                    "#FF2A2A",
                    None,
                    ActorState::Walking,
                ),
                actor(
                    "agent:demo-errol-1",
                    "wolverine",
                    "Wolverine",
                    "walk",
                    "claws",
                    "#E8E8F0",
                    None,
                    ActorState::Walking,
                ),
            ],
        )),
    }
}

/// A busy session with two agents.
fn skylink_session(now: Timestamp) -> Session {
    Session {
        name: "skylink-41".into(),
        project: Some("~/dronevolt/skylink".into()),
        status: SessionStatus::Busy,
        since: Some(now.plus_secs(-2_410.0)),
        room: Some("Rooftop Helipad".into()),
        lead: Some("Storm".into()),
        agents: 2,
        scene: Some(room(
            "rooftop",
            16,
            (14, 54, 164, 60),
            vec![
                actor(
                    "lead:demo-skylink",
                    "storm",
                    "Storm",
                    "fly",
                    "bolt",
                    "#F4F4FF",
                    Some("#BFE6FF"),
                    ActorState::Walking,
                ),
                actor(
                    "agent:demo-skylink-1",
                    "nightcrawler",
                    "Nightcrawler",
                    "teleport",
                    "leap",
                    "#8A6AE0",
                    None,
                    ActorState::Walking,
                ),
                actor(
                    "agent:demo-skylink-2",
                    "northstar",
                    "Northstar",
                    "dash",
                    "sparks",
                    "#F4F4F4",
                    Some("#F4F4F4"),
                    ActorState::Walking,
                ),
            ],
        )),
    }
}

/// A session asleep in the library.
fn pensieve_session() -> Session {
    Session {
        name: "pensieve-3c".into(),
        project: Some("~/personal/pensieve".into()),
        status: SessionStatus::Idle,
        since: None,
        room: Some("Library".into()),
        lead: Some("Beast".into()),
        agents: 0,
        scene: Some(room(
            "library",
            1,
            (12, 66, 168, 48),
            vec![actor(
                "lead:demo-pensieve",
                "beast",
                "Beast",
                "walk",
                "leap",
                "#FFFFFF",
                None,
                ActorState::Asleep,
            )],
        )),
    }
}

/// The house and the Mac `at` seconds into the evening: the neon, the screen
/// coming down, the projector, the session that asked waiting until the end.
fn evening(snapshot: &mut Snapshot, at: f64) {
    if let Some(house) = &mut snapshot.house {
        house.outlets[1].on = at >= 6.0;
        let cover = &mut house.covers[0];
        cover.position = ((at - 34.0) * 4.0).clamp(0.0, 100.0);
        cover.moving = cover.position > 0.0 && cover.position < 100.0;
        if at >= 76.0 {
            house.projectors[0].on = true;
            house.projectors[0].app = Some("Jellyfin".into());
        }
    }
    if let Some(cerebro) = snapshot.cerebro.as_mut().filter(|_| at >= 122.0) {
        cerebro.sessions[0].status = SessionStatus::Waiting;
        if let Some(scene) = &mut cerebro.sessions[0].scene {
            scene.actors[0].state = ActorState::Alert;
        }
        cerebro
            .sessions
            .sort_by_key(|s| s.status != SessionStatus::Waiting);
    }
}

impl DemoFeedSource {
    /// The print starts near the end of the evening; held, or on the idle
    /// screen, it is well under way.
    fn printer(&self, now: Timestamp, at: f64) -> Option<Printer> {
        match self.scene {
            DemoScene::All if at >= 224.0 => Some(printer(now, (at - 224.0) / 2.0)),
            DemoScene::Idle | DemoScene::Print => Some(printer(now, 37.0)),
            _ => None,
        }
    }
}

impl FeedSource for DemoFeedSource {
    fn fetch(&self) -> Result<Snapshot, DomainError> {
        let now = self.clock.now();
        let elapsed = now.secs_since(self.start);
        let mut snapshot = base(now);
        // in a held scene the time stands where that scene happens
        let at = match self.scene {
            DemoScene::All => elapsed % CYCLE,
            DemoScene::Idle => 0.0,
            DemoScene::Cinema => 120.0,
            held => timeline()
                .iter()
                .find(|(_, scene, _)| *scene == held)
                .map_or(0.0, |(at, _, _)| at + elapsed % 20.0),
        };
        let round = (elapsed
            / if self.scene == DemoScene::All {
                CYCLE
            } else {
                30.0
            }) as u64;
        if !self.first.swap(false, Ordering::Relaxed) {
            for (index, (when, scene, kind)) in timeline().into_iter().enumerate() {
                let wanted = if self.scene == DemoScene::All {
                    at >= when
                } else {
                    scene == self.scene
                };
                if wanted {
                    snapshot.events.push(Event {
                        id: EventId::new(format!("demo-{index}-{round}")),
                        at: now,
                        kind,
                    });
                }
            }
        }
        evening(&mut snapshot, at);
        snapshot.printer = self.printer(now, at);
        if at >= 110.0 || self.scene == DemoScene::Cinema {
            snapshot
                .playing
                .push(film(5230.0 + elapsed, (140.0..146.0).contains(&at)));
        }
        Ok(snapshot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct Ticking(Mutex<i64>);
    impl Clock for Ticking {
        fn now(&self) -> Timestamp {
            let mut secs = self.0.lock().unwrap();
            *secs += 1;
            Timestamp::from_secs(*secs)
        }
    }

    fn source(scene: DemoScene) -> DemoFeedSource {
        DemoFeedSource::new(scene, Arc::new(Ticking(Mutex::new(0))))
    }

    #[test]
    fn every_scene_has_its_name() {
        for name in [
            "all",
            "idle",
            "light",
            "cover",
            "projector",
            "cinema",
            "import",
            "download",
            "scan",
            "cerebro",
            "print",
        ] {
            assert!(DemoScene::parse(name).is_some(), "{name}");
        }
        assert_eq!(DemoScene::parse(""), Some(DemoScene::All));
        assert_eq!(DemoScene::parse("teleport"), None);
    }

    #[test]
    fn a_held_print_shows_its_event_and_the_printer() {
        let demo = source(DemoScene::Print);
        assert_eq!(demo.fetch().unwrap().events.len(), 0);
        let snapshot = demo.fetch().unwrap();
        assert!(matches!(snapshot.events[0].kind, EventKind::Print(_)));
        assert_eq!(snapshot.printer.unwrap().progress, Some(37.0));
    }

    #[test]
    fn the_whole_evening_starts_with_the_house_and_no_print() {
        let demo = source(DemoScene::All);
        demo.fetch().unwrap();
        let snapshot = demo.fetch().unwrap();
        assert!(snapshot.printer.is_none());
        assert_eq!(snapshot.playing.len(), 0);
        assert!(!snapshot.house.unwrap().outlets[1].on);
    }

    #[test]
    fn late_in_the_evening_everything_happened() {
        let mut snapshot = base(Timestamp(0));
        evening(&mut snapshot, 230.0);
        let house = snapshot.house.unwrap();
        assert!(house.outlets[1].on && house.projectors[0].on);
        assert_eq!(house.covers[0].position, 100.0);
        assert_eq!(
            snapshot.cerebro.unwrap().sessions[0].status,
            SessionStatus::Waiting
        );
        let demo = source(DemoScene::All);
        assert_eq!(
            demo.printer(Timestamp(0), 230.0).unwrap().progress,
            Some(3.0)
        );
        assert!(
            source(DemoScene::Light)
                .printer(Timestamp(0), 6.0)
                .is_none()
        );
    }
}
