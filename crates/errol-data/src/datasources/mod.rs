//! Where snapshots and pictures come from.

mod demo_feed_source;
mod http_feed_source;
mod http_picture_source;

pub use demo_feed_source::{DemoFeedSource, DemoScene};
pub use http_feed_source::HttpFeedSource;
pub use http_picture_source::HttpPictureSource;
