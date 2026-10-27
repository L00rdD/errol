//! Domain layer of Errol, the living-room screen of the media server.
//!
//! Pure: no I/O, no serde, no knowledge of HTTP or of the GPU. The data layer
//! implements the ports, the app wires them in its injector and draws.
//!
//! ```text
//! entities   what the screen knows: a feed snapshot (what plays, the house,
//!            downloads, imports, One Piece, a trading bot, the disks) and events
//! ports      what the domain needs from outside (FeedSource, PictureSource, Clock)
//! strategies reusable policies (SceneDirector: which scene, for how long)
//! usecases   one struct per action (ObserveFeed, LoadPicture)
//! ```

mod error;

pub mod entities;
pub mod ports;
pub mod strategies;
pub mod usecases;

pub use error::DomainError;
