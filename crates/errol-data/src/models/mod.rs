//! Serde mirrors of `screen.json` (written by `screen/feed.py`). Every field
//! is optional or defaulted: a source the feed could not read must not make
//! the whole file unreadable.

mod feed_file;

pub use feed_file::FeedFile;
