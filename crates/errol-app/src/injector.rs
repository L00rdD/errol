//! Dependency wiring: the only place that knows which adapter implements which port.

use std::sync::Arc;

use errol_data::datasources::{DemoFeedSource, HttpFeedSource, HttpPictureSource};
use errol_data::services::SystemClock;
use errol_domain::ports::{Clock, FeedSource};
use errol_domain::usecases::{LoadPicture, ObserveFeed, ObserveFeedDeps};

use crate::Options;

pub struct Container {
    pub observe: Arc<ObserveFeed>,
    pub load_picture: LoadPicture,
}

pub fn build(options: &Options) -> Container {
    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    let source: Arc<dyn FeedSource> = match options.demo {
        Some(scene) => Arc::new(DemoFeedSource::new(scene, clock.clone())),
        None => Arc::new(HttpFeedSource::new(
            &options.feed_url,
            &options.jellyfin_url,
        )),
    };
    Container {
        observe: Arc::new(ObserveFeed::new(ObserveFeedDeps { source, clock })),
        load_picture: LoadPicture::new(Arc::new(HttpPictureSource::default())),
    }
}
