//! Which scene the screen shows, and for how long.
//!
//! News comes in two sizes. A full event takes the screen for its own time; a
//! newer one cuts it short and takes the screen at once, and those still
//! waiting follow it. A banner (a lamp, the projection screen, the projector
//! changing app, and anything that happens while a video plays) slides over
//! whatever is shown for a few seconds, and the newest cuts the one before.
//! A video that starts cuts the event on screen short: the cinema comes at
//! once, and what was waiting comes as banners. Between events, the cinema
//! while a video plays, otherwise the idle recap. The first batch of events
//! only marks them seen, so a restart does not replay the past.

use std::collections::{HashSet, VecDeque};

use crate::entities::{Event, EventKind, Playing, Snapshot, Timestamp};

/// Full events waiting beyond this are dropped, oldest first: after a burst
/// (ten episodes imported at once) the screen must not stay busy for an hour.
const MAX_QUEUE: usize = 4;
/// Banners waiting beyond this are dropped, oldest first.
const MAX_BANNERS: usize = 3;
/// How long a banner stays, in seconds.
pub const BANNER_SECS: f64 = 8.0;
/// A full event that waited longer than this for its turn is old news, in
/// seconds: it is dropped rather than shown late.
const FULL_FRESH: f64 = 240.0;
/// The same for a banner: a lamp switched a minute ago is no news.
const BANNER_FRESH: f64 = 40.0;

#[derive(Debug, Clone, PartialEq)]
pub enum Scene {
    Idle,
    /// An event has the screen from `started` to `ends`.
    Event {
        event: Event,
        started: Timestamp,
        ends: Timestamp,
    },
    Cinema(Playing),
}

/// A small piece of news over the scene, from `started` to `ends`.
#[derive(Debug, Clone, PartialEq)]
pub struct Banner {
    pub event: Event,
    pub started: Timestamp,
    pub ends: Timestamp,
}

/// How much of the screen a piece of news takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    /// The whole screen, for the event's own time.
    Full,
    /// A banner over the scene, for `BANNER_SECS`.
    Banner,
}

/// An event waiting for its turn, and since when.
type Waiting = (Event, Timestamp);

#[derive(Debug, Default)]
pub struct SceneDirector {
    /// The ids of the last batch; None until the first batch is seen. Only
    /// the last batch is kept: the feed keeps its latest events and an event
    /// that left it never comes back, so older ids would only pile up.
    seen: Option<HashSet<String>>,
    queue: VecDeque<Waiting>,
    current: Option<(Event, Timestamp, Timestamp)>,
    banners: VecDeque<Waiting>,
    banner: Option<Banner>,
    /// A projector was on in the last snapshot: one that says "on" again
    /// only changed app.
    projector_on: bool,
}

impl SceneDirector {
    /// How long an event keeps the whole screen, in seconds; None when it
    /// takes no turn.
    pub fn duration(kind: &EventKind) -> Option<f64> {
        match kind {
            EventKind::Light { .. } | EventKind::Cover { .. } => Some(BANNER_SECS),
            EventKind::Projector { .. } => Some(20.0),
            EventKind::Waiting { .. } => Some(30.0),
            EventKind::Print(_) => Some(45.0),
            EventKind::Download { .. } => Some(60.0),
            EventKind::Import(_) => Some(75.0),
            EventKind::Scan(_) => Some(90.0),
            EventKind::Play { .. } => None,
        }
    }

    /// How much of the screen an event takes: lamps and the projection
    /// screen are small news, the projector changing app too (it was on
    /// already), and nothing takes the screen from a video playing.
    pub fn tier(kind: &EventKind, playing: bool, projector_on: bool) -> Tier {
        let small = match kind {
            EventKind::Light { .. } | EventKind::Cover { .. } => true,
            EventKind::Projector { on, .. } => *on && projector_on,
            _ => false,
        };
        if small || playing {
            Tier::Banner
        } else {
            Tier::Full
        }
    }

    /// Takes a new snapshot at `now`. Its new events cut short the news on
    /// screen of their tier and go first; those that were waiting come after.
    pub fn absorb(&mut self, snapshot: &Snapshot, now: Timestamp) {
        let fresh = self.fresh(&snapshot.events);
        let playing = !snapshot.playing.is_empty();
        let projector_on = std::mem::replace(&mut self.projector_on, projector_is_on(snapshot));
        let Some(fresh) = fresh else {
            return;
        };
        let (mut full, mut small) = (Vec::new(), Vec::new());
        for event in fresh {
            if matches!(event.kind, EventKind::Play { .. }) {
                self.video_starts(now);
                continue;
            }
            match Self::tier(&event.kind, playing, projector_on) {
                Tier::Full => full.push((event, now)),
                Tier::Banner => small.push((event, now)),
            }
        }
        if !full.is_empty() {
            self.current = None;
            push_first(&mut self.queue, full, MAX_QUEUE);
        }
        if !small.is_empty() {
            self.banner = None;
            push_first(&mut self.banners, small, MAX_BANNERS);
        }
    }

    /// The events of a batch not seen in the previous one, that take a turn;
    /// None for the first batch, which is only marked seen.
    fn fresh(&mut self, events: &[Event]) -> Option<Vec<Event>> {
        let batch: HashSet<String> = events
            .iter()
            .map(|event| event.id.as_str().to_owned())
            .collect();
        let seen = self.seen.replace(batch)?;
        let mut fresh: Vec<Event> = Vec::new();
        for event in events {
            if seen.contains(event.id.as_str()) || self.same_series(&fresh, event) {
                continue;
            }
            if Self::duration(&event.kind).is_some() || matches!(event.kind, EventKind::Play { .. })
            {
                fresh.push(event.clone());
            }
        }
        Some(fresh)
    }

    /// Several episodes of one series imported together: one turn is enough.
    fn same_series(&self, fresh: &[Event], event: &Event) -> bool {
        let EventKind::Import(import) = &event.kind else {
            return false;
        };
        let same =
            |other: &Event| matches!(&other.kind, EventKind::Import(o) if o.title == import.title);
        fresh.iter().any(same)
            || self.queue.iter().any(|(other, _)| same(other))
            || self
                .current
                .as_ref()
                .is_some_and(|(current, _, _)| same(current))
    }

    /// A video starts: the event on screen gives way to the cinema, and those
    /// waiting will come as banners over it.
    fn video_starts(&mut self, now: Timestamp) {
        self.current = None;
        let waiting: Vec<Waiting> = self
            .queue
            .drain(..)
            .map(|(event, _)| (event, now))
            .collect();
        push_first(&mut self.banners, waiting, MAX_BANNERS);
    }

    /// The scene at `now`, given the latest snapshot.
    pub fn scene(&mut self, now: Timestamp, snapshot: Option<&Snapshot>) -> Scene {
        if self
            .current
            .as_ref()
            .is_some_and(|(_, _, ends)| now >= *ends)
        {
            self.current = None;
        }
        if self.current.is_none()
            && let Some(event) = next_fresh(&mut self.queue, now, FULL_FRESH)
        {
            let secs = Self::duration(&event.kind).unwrap_or_default();
            self.current = Some((event, now, now.plus_secs(secs)));
        }
        if let Some((event, started, ends)) = &self.current {
            return Scene::Event {
                event: event.clone(),
                started: *started,
                ends: *ends,
            };
        }
        match snapshot.and_then(|snapshot| snapshot.playing.first()) {
            Some(playing) => Scene::Cinema(playing.clone()),
            None => Scene::Idle,
        }
    }

    /// The banner at `now`, if any.
    pub fn banner(&mut self, now: Timestamp) -> Option<Banner> {
        if self
            .banner
            .as_ref()
            .is_some_and(|banner| now >= banner.ends)
        {
            self.banner = None;
        }
        if self.banner.is_none()
            && let Some(event) = next_fresh(&mut self.banners, now, BANNER_FRESH)
        {
            self.banner = Some(Banner {
                event,
                started: now,
                ends: now.plus_secs(BANNER_SECS),
            });
        }
        self.banner.clone()
    }
}

/// Puts `fresh` before what waits in `queue`, keeping the newest `max` of
/// the fresh ones and at most `max` in all.
fn push_first(queue: &mut VecDeque<Waiting>, fresh: Vec<Waiting>, max: usize) {
    let skip = fresh.len().saturating_sub(max);
    let waiting = std::mem::take(queue);
    *queue = fresh.into_iter().skip(skip).chain(waiting).collect();
    queue.truncate(max);
}

/// The next event of `queue` that has not waited longer than `fresh` seconds.
fn next_fresh(queue: &mut VecDeque<Waiting>, now: Timestamp, fresh: f64) -> Option<Event> {
    while let Some((event, since)) = queue.pop_front() {
        if now.secs_since(since) <= fresh {
            return Some(event);
        }
    }
    None
}

fn projector_is_on(snapshot: &Snapshot) -> bool {
    snapshot
        .house
        .as_ref()
        .is_some_and(|house| house.projectors.iter().any(|projector| projector.on))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::{
        Chapter, EventId, House, Import, MediaKind, PrintNews, PrintPhase, Projector,
    };

    fn event(id: &str, kind: EventKind) -> Event {
        Event {
            id: EventId::new(id),
            at: Timestamp(0),
            kind,
        }
    }

    fn light(id: &str) -> Event {
        event(
            id,
            EventKind::Light {
                name: "Néon".into(),
                on: true,
            },
        )
    }

    fn scan(id: &str) -> Event {
        event(
            id,
            EventKind::Scan(Chapter {
                number: 1197,
                name: String::new(),
                arc: None,
                pages: None,
            }),
        )
    }

    fn import(id: &str, title: &str) -> Event {
        event(
            id,
            EventKind::Import(Import {
                kind: MediaKind::Episode,
                title: title.into(),
                year: None,
                season: Some(1),
                episode: Some(1),
                episode_title: None,
                quality: None,
                poster: None,
            }),
        )
    }

    fn projector(id: &str, on: bool) -> Event {
        event(
            id,
            EventKind::Projector {
                name: "Projecteur".into(),
                on,
                app: Some("Netflix".into()),
            },
        )
    }

    fn playing() -> Playing {
        Playing {
            id: "x".into(),
            kind: MediaKind::Movie,
            title: "Dune".into(),
            series: None,
            season: None,
            episode: None,
            year: Some(2024),
            overview: String::new(),
            genres: vec![],
            runtime: 9000.0,
            position: 10.0,
            paused: false,
            device: String::new(),
            client: String::new(),
            user: String::new(),
            poster: None,
            backdrop: None,
        }
    }

    fn batch(events: &[Event]) -> Snapshot {
        Snapshot {
            events: events.to_vec(),
            ..Snapshot::default()
        }
    }

    /// A director that has seen a first, empty batch.
    fn director() -> SceneDirector {
        let mut director = SceneDirector::default();
        director.absorb(&batch(&[]), Timestamp(0));
        director
    }

    fn shown(director: &mut SceneDirector, secs: i64) -> Option<String> {
        match director.scene(Timestamp::from_secs(secs), None) {
            Scene::Event { event, .. } => Some(event.id.as_str().to_owned()),
            _ => None,
        }
    }

    fn banner(director: &mut SceneDirector, secs: i64) -> Option<String> {
        director
            .banner(Timestamp::from_secs(secs))
            .map(|banner| banner.event.id.as_str().to_owned())
    }

    #[test]
    fn the_first_batch_is_only_marked_seen() {
        let mut director = SceneDirector::default();
        director.absorb(&batch(&[scan("a")]), Timestamp(0));
        assert_eq!(director.scene(Timestamp(0), None), Scene::Idle);
        assert_eq!(banner(&mut director, 0), None);
    }

    #[test]
    fn a_new_event_takes_the_screen_for_its_duration_then_gives_it_back() {
        let mut director = director();
        director.absorb(&batch(&[scan("a")]), Timestamp(0));
        assert_eq!(shown(&mut director, 0).as_deref(), Some("a"));
        assert_eq!(shown(&mut director, 89).as_deref(), Some("a"));
        assert_eq!(director.scene(Timestamp::from_secs(90), None), Scene::Idle);
    }

    #[test]
    fn a_lamp_is_a_banner_over_the_scene() {
        let mut director = director();
        director.absorb(&batch(&[scan("a")]), Timestamp(0));
        director.absorb(&batch(&[scan("a"), light("b")]), Timestamp::from_secs(5));
        assert_eq!(shown(&mut director, 5).as_deref(), Some("a"));
        assert_eq!(banner(&mut director, 5).as_deref(), Some("b"));
        assert_eq!(banner(&mut director, 13), None);
    }

    #[test]
    fn a_new_banner_cuts_the_one_shown_short() {
        let mut director = director();
        director.absorb(&batch(&[light("a")]), Timestamp(0));
        assert_eq!(banner(&mut director, 0).as_deref(), Some("a"));
        director.absorb(&batch(&[light("a"), light("b")]), Timestamp::from_secs(2));
        let shown = director.banner(Timestamp::from_secs(2)).unwrap();
        assert_eq!(
            (shown.event.id.as_str(), shown.started),
            ("b", Timestamp::from_secs(2))
        );
        // the one cut short does not come back
        assert_eq!(banner(&mut director, 10), None);
    }

    #[test]
    fn a_projector_turning_on_takes_the_screen_and_changing_app_is_a_banner() {
        let mut director = director();
        director.absorb(&batch(&[projector("a", true)]), Timestamp(0));
        assert_eq!(shown(&mut director, 0).as_deref(), Some("a"));
        let on = Snapshot {
            house: Some(House {
                projectors: vec![Projector {
                    id: "p".into(),
                    name: "Projecteur".into(),
                    on: true,
                    app: None,
                    closes_at: None,
                }],
                ..House::default()
            }),
            ..batch(&[projector("a", true)])
        };
        director.absorb(&on, Timestamp::from_secs(1));
        let app = Snapshot {
            events: vec![projector("a", true), projector("b", true)],
            ..on
        };
        director.absorb(&app, Timestamp::from_secs(2));
        assert_eq!(shown(&mut director, 2).as_deref(), Some("a"));
        assert_eq!(banner(&mut director, 2).as_deref(), Some("b"));
    }

    #[test]
    fn nothing_takes_the_screen_from_a_video() {
        let mut director = director();
        let film = Snapshot {
            playing: vec![playing()],
            ..batch(&[scan("a")])
        };
        director.absorb(&film, Timestamp(0));
        assert_eq!(
            director.scene(Timestamp(0), Some(&film)),
            Scene::Cinema(playing())
        );
        assert_eq!(banner(&mut director, 0).as_deref(), Some("a"));
    }

    #[test]
    fn a_video_that_starts_cuts_the_event_short_and_what_waited_comes_as_banners() {
        let mut director = director();
        director.absorb(&batch(&[scan("a"), import("b", "Arcane")]), Timestamp(0));
        assert_eq!(shown(&mut director, 0).as_deref(), Some("a"));
        let play = event(
            "c",
            EventKind::Play {
                title: "Dune".into(),
            },
        );
        let film = Snapshot {
            playing: vec![playing()],
            ..batch(&[scan("a"), import("b", "Arcane"), play])
        };
        director.absorb(&film, Timestamp::from_secs(3));
        assert_eq!(
            director.scene(Timestamp::from_secs(3), Some(&film)),
            Scene::Cinema(playing())
        );
        assert_eq!(banner(&mut director, 3).as_deref(), Some("b"));
    }

    #[test]
    fn events_take_turns_in_order() {
        let mut director = director();
        director.absorb(&batch(&[scan("a"), import("b", "Arcane")]), Timestamp(0));
        assert_eq!(shown(&mut director, 0).as_deref(), Some("a"));
        assert_eq!(shown(&mut director, 90).as_deref(), Some("b"));
    }

    #[test]
    fn an_event_that_waited_too_long_is_dropped() {
        let mut director = director();
        let many: Vec<Event> = (0..4).map(|n| scan(&n.to_string())).collect();
        director.absorb(&batch(&many), Timestamp(0));
        assert_eq!(shown(&mut director, 0).as_deref(), Some("0"));
        assert_eq!(shown(&mut director, 90).as_deref(), Some("1"));
        assert_eq!(shown(&mut director, 180).as_deref(), Some("2"));
        // the fourth has waited 270 s: old news
        assert_eq!(shown(&mut director, 270), None);
    }

    #[test]
    fn an_event_seen_again_is_not_replayed() {
        let mut director = director();
        director.absorb(&batch(&[scan("a")]), Timestamp(0));
        director.scene(Timestamp(0), None);
        director.absorb(&batch(&[scan("a")]), Timestamp(1));
        assert_eq!(director.scene(Timestamp::from_secs(90), None), Scene::Idle);
    }

    #[test]
    fn the_seen_ids_are_only_those_of_the_last_batch() {
        let mut director = director();
        for round in 0..50 {
            let events = [light(&format!("{round}-a")), light(&format!("{round}-b"))];
            director.absorb(&batch(&events), Timestamp(0));
        }
        assert_eq!(director.seen.as_ref().map(HashSet::len), Some(2));
    }

    #[test]
    fn an_event_still_in_the_feed_is_not_replayed_once_it_had_its_turn() {
        let mut director = director();
        director.absorb(&batch(&[scan("a")]), Timestamp(0));
        director.scene(Timestamp(0), None);
        director.absorb(&batch(&[scan("a"), scan("b")]), Timestamp(0));
        director.absorb(&batch(&[scan("a"), scan("b")]), Timestamp(0));
        assert_eq!(shown(&mut director, 30).as_deref(), Some("b"));
        assert_eq!(director.scene(Timestamp::from_secs(120), None), Scene::Idle);
    }

    #[test]
    fn episodes_of_one_series_imported_together_take_a_single_turn() {
        let mut director = director();
        let events = [
            import("1", "Pennyworth"),
            import("2", "Pennyworth"),
            import("3", "Pennyworth"),
        ];
        director.absorb(&batch(&events), Timestamp(0));
        director.scene(Timestamp(0), None);
        assert_eq!(director.scene(Timestamp::from_secs(75), None), Scene::Idle);
    }

    #[test]
    fn a_new_event_cuts_the_one_on_screen_short() {
        let mut director = director();
        director.absorb(&batch(&[import("a", "Arcane")]), Timestamp(0));
        director.scene(Timestamp(0), None);
        director.absorb(
            &batch(&[import("a", "Arcane"), scan("b")]),
            Timestamp::from_secs(3),
        );
        let Scene::Event { event, started, .. } = director.scene(Timestamp::from_secs(3), None)
        else {
            panic!()
        };
        assert_eq!((event.id.as_str(), started), ("b", Timestamp::from_secs(3)));
        assert_eq!(director.scene(Timestamp::from_secs(93), None), Scene::Idle);
    }

    #[test]
    fn a_new_event_goes_before_those_still_waiting() {
        let mut director = director();
        director.absorb(&batch(&[scan("a"), scan("b")]), Timestamp(0));
        director.scene(Timestamp(0), None);
        director.absorb(&batch(&[scan("a"), scan("b"), scan("c")]), Timestamp(0));
        assert_eq!(shown(&mut director, 0).as_deref(), Some("c"));
        assert_eq!(shown(&mut director, 90).as_deref(), Some("b"));
        // the one cut short does not come back
        assert_eq!(shown(&mut director, 180), None);
    }

    #[test]
    fn a_print_takes_the_screen_for_45_seconds() {
        let print = event(
            "p",
            EventKind::Print(PrintNews {
                phase: PrintPhase::Done,
                printer: "M7".into(),
                file: "riser".into(),
                image: None,
                layers: Some(2000),
                remaining: None,
                started: None,
            }),
        );
        let mut director = director();
        director.absorb(&batch(&[print]), Timestamp(0));
        assert_eq!(shown(&mut director, 0).as_deref(), Some("p"));
        assert_eq!(shown(&mut director, 44).as_deref(), Some("p"));
        assert_eq!(shown(&mut director, 45), None);
    }

    #[test]
    fn a_burst_keeps_only_the_latest_events() {
        let mut director = director();
        let burst: Vec<Event> = (0..10).map(|n| scan(&n.to_string())).collect();
        director.absorb(&batch(&burst), Timestamp(0));
        assert_eq!(shown(&mut director, 0).as_deref(), Some("6"));
    }

    #[test]
    fn a_video_playing_shows_the_cinema_between_events() {
        let snapshot = Snapshot {
            playing: vec![playing()],
            ..Snapshot::default()
        };
        let mut director = director();
        assert_eq!(
            director.scene(Timestamp(0), Some(&snapshot)),
            Scene::Cinema(playing())
        );
    }
}
