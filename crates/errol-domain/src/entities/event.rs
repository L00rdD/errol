use super::{Chapter, Import, Timestamp};

/// Identifier the feed gives to an event, stable across reads.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EventId(String);

impl EventId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Something that changed, worth taking the screen for.
#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    pub id: EventId,
    pub at: Timestamp,
    pub kind: EventKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EventKind {
    /// A lamp switched on or off.
    Light { name: String, on: bool },
    /// The projection screen started moving; `down` when it goes down.
    Cover {
        id: String,
        name: String,
        down: bool,
    },
    /// The projector turned on or off, or switched app.
    Projector {
        name: String,
        on: bool,
        app: Option<String>,
    },
    /// A torrent finished.
    Download { name: String, size: Option<u64> },
    /// A film or an episode entered the library.
    Import(Import),
    /// A new One Piece scan is out.
    Scan(Chapter),
    /// A video started on Jellyfin (the cinema scene shows it; no takeover).
    Play { title: String },
}
