use super::{Chapter, Import, PictureRef, Timestamp};

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
    /// A Claude Code session on the Mac waits for an answer (Cerebro).
    Waiting {
        name: String,
        project: Option<String>,
        room: Option<String>,
    },
    /// A video started on Jellyfin (the cinema scene shows it; no takeover).
    Play { title: String },
    /// A print on the resin printer started, finished or was cancelled.
    Print(PrintNews),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrintPhase {
    /// Its first layers are being cured.
    Started,
    Done,
    Cancelled,
}

/// What a print event tells.
#[derive(Debug, Clone, PartialEq)]
pub struct PrintNews {
    pub phase: PrintPhase,
    /// The printer's name.
    pub printer: String,
    pub file: String,
    pub image: Option<PictureRef>,
    pub layers: Option<u32>,
    /// Seconds left when it started.
    pub remaining: Option<f64>,
    pub started: Option<Timestamp>,
}
