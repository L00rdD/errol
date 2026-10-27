/// A moment, in milliseconds since the Unix epoch. Time comes from a `Clock`
/// port or from the feed, never read directly in the domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Timestamp(pub i64);

impl Timestamp {
    pub fn from_secs(secs: i64) -> Self {
        Self(secs * 1000)
    }

    /// Seconds from `earlier` to `self`, negative when `earlier` is later.
    pub fn secs_since(self, earlier: Timestamp) -> f64 {
        (self.0 - earlier.0) as f64 / 1000.0
    }

    #[must_use]
    pub fn plus_secs(self, secs: f64) -> Self {
        Self(self.0 + (secs * 1000.0) as i64)
    }
}
