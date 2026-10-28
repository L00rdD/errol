//! A made-up evening, to see every scene without waiting for the house:
//! a neon switches on, the projection screen comes down, the projector starts,
//! a film plays, a Claude Code session waits for an answer, an episode and a
//! download arrive, a new scan is out. Pictures
//! come from TMDB. `DemoScene` other than `All` holds a single scene.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use errol_domain::DomainError;
use errol_domain::entities::{
    Actor, ActorState, Cerebro, Chapter, Cover, Disk, Download, Event, EventId, EventKind, House,
    Import, MediaKind, NextChapter, OnePiece, Outlet, PictureRef, Playing, Portfolio, Position,
    Projector, Services, Session, SessionScene, SessionStatus, Snapshot, Timestamp,
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
    ]
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
        house: Some(House {
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
        }),
        downloads: vec![Download {
            name: "Dune.Part.Two.2024.1080p.BluRay.x265".into(),
            percent: 63.4,
            eta: Some(1260.0),
        }],
        recent: vec![
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
        ],
        one_piece: Some(OnePiece {
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
        }),
        portfolio: Some(Portfolio {
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
        }),
        disk: Some(Disk {
            total: 1_430_000_000_000,
            used: 980_000_000_000,
            free: 450_000_000_000,
        }),
        services: Services { up: 18, total: 18 },
        cerebro: Some(Cerebro {
            room_size: (192, 120),
            character_size: (24, 24),
            emotes: Some(cerebro_sprite("ui/emotes.png")),
            sessions: vec![
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
                },
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
                },
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
                },
            ],
        }),
        ..Snapshot::default()
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
        // the session that asked is waiting until the end of the evening
        if let Some(cerebro) = snapshot.cerebro.as_mut().filter(|_| at >= 122.0) {
            cerebro.sessions[0].status = SessionStatus::Waiting;
            if let Some(scene) = &mut cerebro.sessions[0].scene {
                scene.actors[0].state = ActorState::Alert;
            }
            cerebro
                .sessions
                .sort_by_key(|s| s.status != SessionStatus::Waiting);
        }
        if at >= 110.0 || self.scene == DemoScene::Cinema {
            snapshot
                .playing
                .push(film(5230.0 + elapsed, (140.0..146.0).contains(&at)));
        }
        Ok(snapshot)
    }
}
