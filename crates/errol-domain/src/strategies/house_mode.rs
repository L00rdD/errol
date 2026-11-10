//! What the house is doing, from what the feed says and the hour: the screen
//! calms down for a film and goes to sleep at night.

use crate::entities::Snapshot;

/// The hours of the night, local time: from 1 h to 7 h.
const NIGHT: std::ops::Range<u32> = 1..7;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Life as usual.
    Day,
    /// The projector is on or a video plays: the screen keeps quiet.
    Cinema,
    /// Late at night with nothing on: the screen dims to a faint clock.
    Night,
}

impl Mode {
    /// The mode at `hour` (0 to 23, local time) given the latest snapshot.
    /// A lamp, the projector or a video keeps the night away.
    pub fn of(snapshot: Option<&Snapshot>, hour: u32) -> Self {
        let Some(snapshot) = snapshot else {
            return Self::Day;
        };
        let house = snapshot.house.as_ref();
        let projector = house.is_some_and(|h| h.projectors.iter().any(|p| p.on));
        if projector || !snapshot.playing.is_empty() {
            return Self::Cinema;
        }
        let lamp = house.is_some_and(|h| h.outlets.iter().any(|o| o.on));
        if NIGHT.contains(&hour) && !lamp {
            Self::Night
        } else {
            Self::Day
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::{House, Outlet, Projector};

    fn house(lamp: bool, projector: bool) -> Snapshot {
        Snapshot {
            house: Some(House {
                outlets: vec![Outlet {
                    id: "l".into(),
                    name: "Lampe".into(),
                    on: lamp,
                    next: None,
                }],
                covers: vec![],
                projectors: vec![Projector {
                    id: "p".into(),
                    name: "Projecteur".into(),
                    on: projector,
                    app: None,
                    closes_at: None,
                }],
            }),
            ..Snapshot::default()
        }
    }

    #[test]
    fn the_projector_on_is_the_cinema_whatever_the_hour() {
        assert_eq!(Mode::of(Some(&house(false, true)), 3), Mode::Cinema);
    }

    #[test]
    fn the_night_comes_late_with_nothing_on() {
        assert_eq!(Mode::of(Some(&house(false, false)), 3), Mode::Night);
        assert_eq!(Mode::of(Some(&house(true, false)), 3), Mode::Day);
        assert_eq!(Mode::of(Some(&house(false, false)), 7), Mode::Day);
        assert_eq!(Mode::of(Some(&house(false, false)), 0), Mode::Day);
        assert_eq!(Mode::of(None, 3), Mode::Day);
    }
}
