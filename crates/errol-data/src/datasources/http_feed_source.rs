use std::io::{BufRead, BufReader};
use std::time::Duration;

use errol_domain::DomainError;
use errol_domain::entities::Snapshot;
use errol_domain::ports::FeedSource;

use crate::models::FeedFile;

/// Reads `screen.json` from the home page (served by Caddy), and follows
/// `screen/events` next to it: the same document, pushed by the feed service
/// as Server-Sent Events each time it changes.
pub struct HttpFeedSource {
    url: String,
    events: String,
    jellyfin: String,
    /// The home page's origin (`http://localhost`): Cerebro's sprites are on it.
    hub: String,
    agent: ureq::Agent,
    /// The stream's connection: it stays open for hours, so no global budget;
    /// the feed writes a comment every 15 s, and reading waits for it.
    streaming: ureq::Agent,
}

impl HttpFeedSource {
    /// `url`: the feed (`http://localhost/screen.json`); `jellyfin`: the
    /// Jellyfin server its picture paths belong to.
    pub fn new(url: impl Into<String>, jellyfin: impl Into<String>) -> Self {
        let url = url.into();
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(4)))
            .build()
            .into();
        let streaming = ureq::Agent::config_builder()
            .timeout_connect(Some(Duration::from_secs(4)))
            .timeout_recv_response(Some(Duration::from_secs(10)))
            .build()
            .into();
        Self {
            events: events_url(&url),
            hub: origin(&url),
            url,
            jellyfin: jellyfin.into(),
            agent,
            streaming,
        }
    }
}

/// The stream next to the feed: `…/screen.json` → `…/screen/events`.
fn events_url(feed: &str) -> String {
    match feed.rsplit_once('/') {
        Some((base, _)) => format!("{base}/screen/events"),
        None => format!("{feed}/screen/events"),
    }
}

/// The scheme and host of a URL: `http://localhost/screen.json` → `http://localhost`.
fn origin(url: &str) -> String {
    let start = url.find("://").map_or(0, |at| at + 3);
    match url[start..].find('/') {
        Some(end) => url[..start + end].to_owned(),
        None => url.to_owned(),
    }
}

fn unavailable(error: &dyn std::fmt::Display) -> DomainError {
    DomainError::FeedUnavailable(error.to_string())
}

/// Reads Server-Sent Events from `lines`, handing each event's data (the
/// `data:` lines joined) to `sink`; comments and other fields are skipped.
/// Returns when the lines end.
pub fn events(lines: impl BufRead, sink: &mut dyn FnMut(&str)) -> std::io::Result<()> {
    let mut data = String::new();
    for line in lines.lines() {
        let line = line?;
        if line.is_empty() {
            if !data.is_empty() {
                sink(&data);
                data.clear();
            }
        } else if let Some(payload) = line.strip_prefix("data:") {
            if !data.is_empty() {
                data.push('\n');
            }
            data.push_str(payload.strip_prefix(' ').unwrap_or(payload));
        }
    }
    if !data.is_empty() {
        sink(&data);
    }
    Ok(())
}

impl FeedSource for HttpFeedSource {
    fn fetch(&self) -> Result<Snapshot, DomainError> {
        let file: FeedFile = self
            .agent
            .get(&self.url)
            .call()
            .map_err(|error| unavailable(&error))?
            .body_mut()
            .read_json()
            .map_err(|error| unavailable(&error))?;
        Ok(file.into_snapshot(&self.jellyfin, &self.hub))
    }

    fn follow(&self, sink: &mut dyn FnMut(Snapshot)) -> Result<(), DomainError> {
        let mut response = self
            .streaming
            .get(&self.events)
            .header("Accept", "text/event-stream")
            .call()
            .map_err(|error| unavailable(&error))?;
        let kind = response
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_owned();
        if !kind.starts_with("text/event-stream") {
            return Err(unavailable(&format!("{} answers {kind:?}", self.events)));
        }
        let reader = BufReader::new(response.body_mut().with_config().limit(u64::MAX).reader());
        events(reader, &mut |data| {
            // a document that does not parse is skipped: the next one may
            if let Ok(file) = serde_json::from_str::<FeedFile>(data) {
                sink(file.into_snapshot(&self.jellyfin, &self.hub));
            }
        })
        .map_err(|error| unavailable(&error))?;
        Err(unavailable(&"the stream ended"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hub_is_the_origin_of_the_feed() {
        assert_eq!(origin("http://localhost/screen.json"), "http://localhost");
        assert_eq!(
            origin("http://dpi.local:8088/screen.json"),
            "http://dpi.local:8088"
        );
    }

    #[test]
    fn the_stream_sits_next_to_the_feed() {
        assert_eq!(
            events_url("http://server.local/screen.json"),
            "http://server.local/screen/events"
        );
    }

    #[test]
    fn events_are_the_data_lines_between_blank_lines() {
        let text =
            ": keepalive\n\ndata: {\"a\": 1}\n\nevent: x\ndata: first\ndata: second\n\ndata:last";
        let mut seen = Vec::new();
        events(text.as_bytes(), &mut |data| seen.push(data.to_owned())).unwrap();
        assert_eq!(seen, ["{\"a\": 1}", "first\nsecond", "last"]);
    }
}
