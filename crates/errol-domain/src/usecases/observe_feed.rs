use std::sync::{Arc, Mutex, PoisonError};

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

/// Reads the feed (`refresh`, from the poller thread) and tells the renderer
/// what to show (`view`, every frame). A failed read keeps the last snapshot.
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
        let result = self.deps.source.fetch();
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
    use crate::DomainError;
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
