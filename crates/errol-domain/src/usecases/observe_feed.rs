use std::sync::{Arc, Mutex, PoisonError};

use crate::DomainError;
use crate::entities::{Snapshot, Timestamp};
use crate::ports::{Clock, FeedSource};
use crate::strategies::{Scene, SceneDirector};

pub struct ObserveFeedDeps {
    pub source: Arc<dyn FeedSource>,
    pub clock: Arc<dyn Clock>,
}

/// What the renderer draws: the latest snapshot and the scene of the moment.
#[derive(Debug, Clone, PartialEq)]
pub struct View {
    /// Shared, not copied: the snapshot changes every second, the view 30 times a second.
    pub snapshot: Option<Arc<Snapshot>>,
    /// When the snapshot was read; playback positions move on from there.
    pub read_at: Timestamp,
    pub scene: Scene,
    pub now: Timestamp,
    /// Why the last read failed, when it did (the previous snapshot is kept).
    pub warning: Option<String>,
}

struct State {
    director: SceneDirector,
    snapshot: Option<Arc<Snapshot>>,
    read_at: Timestamp,
    warning: Option<String>,
}

/// Reads the feed (`refresh` from the poller thread, or `follow` as the feed
/// pushes) and tells the renderer what to show (`view`, every frame). A
/// failed read keeps the last snapshot.
pub struct ObserveFeed {
    deps: ObserveFeedDeps,
    state: Mutex<State>,
}

impl ObserveFeed {
    pub fn new(deps: ObserveFeedDeps) -> Self {
        let state = State {
            director: SceneDirector::default(),
            snapshot: None,
            read_at: Timestamp(0),
            warning: None,
        };
        Self {
            deps,
            state: Mutex::new(state),
        }
    }

    pub fn refresh(&self) {
        self.absorb(self.deps.source.fetch());
    }

    /// Takes every snapshot the feed pushes, calling `on_snapshot` for each,
    /// until the stream ends: then says why, and `refresh` is the way.
    pub fn follow(&self, on_snapshot: &mut dyn FnMut()) -> Result<(), DomainError> {
        self.deps.source.follow(&mut |snapshot| {
            self.absorb(Ok(snapshot));
            on_snapshot();
        })
    }

    fn absorb(&self, result: Result<Snapshot, DomainError>) {
        let now = self.deps.clock.now();
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        match result {
            Ok(snapshot) => {
                state.director.absorb(&snapshot.events);
                state.snapshot = Some(Arc::new(snapshot));
                state.read_at = now;
                state.warning = None;
            }
            Err(error) => state.warning = Some(error.to_string()),
        }
    }

    pub fn view(&self) -> View {
        let now = self.deps.clock.now();
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        let state = &mut *state;
        let scene = state.director.scene(now, state.snapshot.as_deref());
        View {
            snapshot: state.snapshot.clone(),
            read_at: state.read_at,
            scene,
            now,
            warning: state.warning.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::{Event, EventId, EventKind};

    struct FixedClock(i64);
    impl Clock for FixedClock {
        fn now(&self) -> Timestamp {
            Timestamp(self.0)
        }
    }

    struct ScriptedSource(Mutex<Vec<Result<Snapshot, DomainError>>>);
    impl FeedSource for ScriptedSource {
        fn fetch(&self) -> Result<Snapshot, DomainError> {
            self.0.lock().unwrap().remove(0)
        }
    }

    fn use_case(script: Vec<Result<Snapshot, DomainError>>) -> ObserveFeed {
        ObserveFeed::new(ObserveFeedDeps {
            source: Arc::new(ScriptedSource(Mutex::new(script))),
            clock: Arc::new(FixedClock(0)),
        })
    }

    struct PushingSource(Vec<Snapshot>);
    impl FeedSource for PushingSource {
        fn fetch(&self) -> Result<Snapshot, DomainError> {
            Err(DomainError::FeedUnavailable("not asked".into()))
        }
        fn follow(&self, sink: &mut dyn FnMut(Snapshot)) -> Result<(), DomainError> {
            for snapshot in &self.0 {
                sink(snapshot.clone());
            }
            Err(DomainError::FeedUnavailable("stream closed".into()))
        }
    }

    #[test]
    fn what_the_feed_pushes_is_shown_and_counted() {
        let observe = ObserveFeed::new(ObserveFeedDeps {
            source: Arc::new(PushingSource(vec![
                Snapshot::default(),
                Snapshot::default(),
            ])),
            clock: Arc::new(FixedClock(7)),
        });
        let mut pushed = 0;
        let ended = observe.follow(&mut || pushed += 1);
        assert_eq!(pushed, 2);
        assert!(ended.is_err());
        let view = observe.view();
        assert!(view.snapshot.is_some());
        assert_eq!(view.read_at, Timestamp(7));
        assert_eq!(view.warning, None);
    }

    #[test]
    fn a_source_that_cannot_push_says_so_at_once() {
        let observe = use_case(vec![]);
        assert!(observe.follow(&mut || {}).is_err());
    }

    #[test]
    fn a_failed_read_keeps_the_last_snapshot_and_says_why() {
        let observe = use_case(vec![
            Ok(Snapshot::default()),
            Err(DomainError::FeedUnavailable("down".into())),
        ]);
        observe.refresh();
        observe.refresh();
        let view = observe.view();
        assert!(view.snapshot.is_some());
        assert_eq!(view.warning.as_deref(), Some("feed unavailable: down"));
    }

    #[test]
    fn an_event_in_a_later_read_takes_the_screen() {
        let light = Event {
            id: EventId::new("1"),
            at: Timestamp(0),
            kind: EventKind::Light {
                name: "Néon".into(),
                on: true,
            },
        };
        let observe = use_case(vec![
            Ok(Snapshot::default()),
            Ok(Snapshot {
                events: vec![light],
                ..Snapshot::default()
            }),
        ]);
        observe.refresh();
        observe.refresh();
        assert!(matches!(observe.view().scene, Scene::Event { .. }));
    }
}
