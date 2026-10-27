//! Contracts the domain needs from the outside world. The data layer
//! implements them; tests implement them with in-memory fakes.

use crate::DomainError;
use crate::entities::{Picture, PictureRef, Snapshot, Timestamp};

/// Where the snapshots come from: the feed service, or the demo.
pub trait FeedSource: Send + Sync {
    fn fetch(&self) -> Result<Snapshot, DomainError>;
}

/// Fetches and decodes a picture, scaled down to at most `max_width` pixels wide.
pub trait PictureSource: Send + Sync {
    fn fetch(&self, picture: &PictureRef, max_width: u32) -> Result<Picture, DomainError>;
}

pub trait Clock: Send + Sync {
    fn now(&self) -> Timestamp;
}
