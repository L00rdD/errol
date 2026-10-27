//! One struct per application action.

mod load_picture;
mod observe_feed;

pub use load_picture::LoadPicture;
pub use observe_feed::{ObserveFeed, ObserveFeedDeps, View};
