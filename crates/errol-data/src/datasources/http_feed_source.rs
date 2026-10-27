use std::time::Duration;

use errol_domain::DomainError;
use errol_domain::entities::Snapshot;
use errol_domain::ports::FeedSource;

use crate::models::FeedFile;

/// Reads `screen.json` from the home page (served by Caddy).
pub struct HttpFeedSource {
    url: String,
    jellyfin: String,
    agent: ureq::Agent,
}

impl HttpFeedSource {
    /// `url`: the feed (`http://localhost/screen.json`); `jellyfin`: the
    /// Jellyfin server its picture paths belong to.
    pub fn new(url: impl Into<String>, jellyfin: impl Into<String>) -> Self {
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(4)))
            .build()
            .into();
        Self {
            url: url.into(),
            jellyfin: jellyfin.into(),
            agent,
        }
    }
}

impl FeedSource for HttpFeedSource {
    fn fetch(&self) -> Result<Snapshot, DomainError> {
        let unavailable =
            |error: &dyn std::fmt::Display| DomainError::FeedUnavailable(error.to_string());
        let file: FeedFile = self
            .agent
            .get(&self.url)
            .call()
            .map_err(|error| unavailable(&error))?
            .body_mut()
            .read_json()
            .map_err(|error| unavailable(&error))?;
        Ok(file.into_snapshot(&self.jellyfin))
    }
}
