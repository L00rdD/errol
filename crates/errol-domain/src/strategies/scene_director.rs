//! Which scene the screen shows, and for how long.
//!
//! An event takes the screen for its own time, one after another; between
//! events, the cinema while a video plays, otherwise the idle recap. The first
//! batch of events only marks them seen, so a restart does not replay the past.

use std::collections::{HashSet, VecDeque};

use crate::entities::{Event, EventKind, Playing, Snapshot, Timestamp};

/// Events waiting beyond this are dropped, oldest first: after a burst
/// (ten episodes imported at once) the screen must not stay busy for an hour.
const MAX_QUEUE: usize = 4;

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

#[derive(Debug, Default)]
pub struct SceneDirector {
    /// The ids of the last batch; None until the first batch is seen. Only
    /// the last batch is kept: the feed keeps its latest events and an event
    /// that left it never comes back, so older ids would only pile up.
    seen: Option<HashSet<String>>,
    queue: VecDeque<Event>,
    current: Option<(Event, Timestamp, Timestamp)>,
}

impl SceneDirector {
    /// How long an event keeps the screen, in seconds; None when it takes no turn.
    pub fn duration(kind: &EventKind) -> Option<f64> {
        match kind {
            EventKind::Light { .. } => Some(25.0),
            EventKind::Cover { .. } | EventKind::Projector { .. } => Some(40.0),
            EventKind::Download { .. } | EventKind::Import(_) => Some(90.0),
            EventKind::Scan(_) => Some(120.0),
            EventKind::Play { .. } => None,
        }
    }

    /// Takes the events of a new snapshot; the new ones join the queue.
    pub fn absorb(&mut self, events: &[Event]) {
        let batch: HashSet<String> = events
            .iter()
            .map(|event| event.id.as_str().to_owned())
            .collect();
        let Some(seen) = self.seen.replace(batch) else {
            return;
        };
        for event in events {
            if seen.contains(event.id.as_str()) || Self::duration(&event.kind).is_none() {
                continue;
            }
            // several episodes of one series imported together: one turn is enough
            if let EventKind::Import(import) = &event.kind {
                let same = |other: &Event| matches!(&other.kind, EventKind::Import(o) if o.title == import.title);
                if self.queue.iter().any(same)
                    || self
                        .current
                        .as_ref()
                        .is_some_and(|(current, _, _)| same(current))
                {
                    continue;
                }
            }
            self.queue.push_back(event.clone());
            while self.queue.len() > MAX_QUEUE {
                self.queue.pop_front();
            }
        }
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
            && let Some(event) = self.queue.pop_front()
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::{EventId, Import, MediaKind};

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

    #[test]
    fn the_first_batch_is_only_marked_seen() {
        let mut director = SceneDirector::default();
        director.absorb(&[light("a")]);
        assert_eq!(director.scene(Timestamp(0), None), Scene::Idle);
    }

    #[test]
    fn a_new_event_takes_the_screen_for_its_duration_then_gives_it_back() {
        let mut director = SceneDirector::default();
        director.absorb(&[]);
        director.absorb(&[light("a")]);
        assert!(matches!(
            director.scene(Timestamp(0), None),
            Scene::Event { .. }
        ));
        assert!(matches!(
            director.scene(Timestamp::from_secs(24), None),
            Scene::Event { .. }
        ));
        assert_eq!(director.scene(Timestamp::from_secs(25), None), Scene::Idle);
    }

    #[test]
    fn events_take_turns_in_order() {
        let mut director = SceneDirector::default();
        director.absorb(&[]);
        director.absorb(&[light("a"), import("b", "Arcane")]);
        let Scene::Event { event: first, .. } = director.scene(Timestamp(0), None) else {
            panic!()
        };
        let Scene::Event { event: second, .. } = director.scene(Timestamp::from_secs(25), None)
        else {
            panic!()
        };
        assert_eq!((first.id.as_str(), second.id.as_str()), ("a", "b"));
    }

    #[test]
    fn an_event_seen_again_is_not_replayed() {
        let mut director = SceneDirector::default();
        director.absorb(&[]);
        director.absorb(&[light("a")]);
        director.scene(Timestamp(0), None);
        director.absorb(&[light("a")]);
        assert_eq!(director.scene(Timestamp::from_secs(30), None), Scene::Idle);
    }

    #[test]
    fn the_seen_ids_are_only_those_of_the_last_batch() {
        let mut director = SceneDirector::default();
        director.absorb(&[]);
        for round in 0..50 {
            director.absorb(&[light(&format!("{round}-a")), light(&format!("{round}-b"))]);
        }
        assert_eq!(director.seen.as_ref().map(HashSet::len), Some(2));
    }

    #[test]
    fn an_event_still_in_the_feed_is_not_replayed_once_it_had_its_turn() {
        let mut director = SceneDirector::default();
        director.absorb(&[]);
        director.absorb(&[light("a")]);
        director.scene(Timestamp(0), None);
        director.absorb(&[light("a"), light("b")]);
        director.absorb(&[light("a"), light("b")]);
        let Scene::Event { event, .. } = director.scene(Timestamp::from_secs(30), None) else {
            panic!()
        };
        assert_eq!(event.id.as_str(), "b");
        assert_eq!(director.scene(Timestamp::from_secs(60), None), Scene::Idle);
    }

    #[test]
    fn episodes_of_one_series_imported_together_take_a_single_turn() {
        let mut director = SceneDirector::default();
        director.absorb(&[]);
        director.absorb(&[
            import("1", "Pennyworth"),
            import("2", "Pennyworth"),
            import("3", "Pennyworth"),
        ]);
        director.scene(Timestamp(0), None);
        assert_eq!(director.scene(Timestamp::from_secs(90), None), Scene::Idle);
    }

    #[test]
    fn a_burst_keeps_only_the_latest_events() {
        let mut director = SceneDirector::default();
        director.absorb(&[]);
        director.absorb(&(0..10).map(|n| light(&n.to_string())).collect::<Vec<_>>());
        let Scene::Event { event, .. } = director.scene(Timestamp(0), None) else {
            panic!()
        };
        assert_eq!(event.id.as_str(), "6");
    }

    #[test]
    fn a_video_playing_shows_the_cinema_between_events() {
        let playing = Playing {
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
        };
        let snapshot = Snapshot {
            playing: vec![playing.clone()],
            ..Snapshot::default()
        };
        let mut director = SceneDirector::default();
        director.absorb(&[]);
        assert_eq!(
            director.scene(Timestamp(0), Some(&snapshot)),
            Scene::Cinema(playing)
        );
    }
}
