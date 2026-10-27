use std::time::Duration;

use errol_domain::DomainError;
use errol_domain::entities::{Picture, PictureRef};
use errol_domain::ports::PictureSource;
use image::imageops::FilterType;

/// Pictures larger than this are refused (a broken URL serving a video).
const MAX_BYTES: u64 = 12 * 1024 * 1024;

/// Fetches a picture over HTTP and decodes it (JPEG, PNG, WebP).
pub struct HttpPictureSource {
    agent: ureq::Agent,
}

impl Default for HttpPictureSource {
    fn default() -> Self {
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(15)))
            .build()
            .into();
        Self { agent }
    }
}

impl PictureSource for HttpPictureSource {
    fn fetch(&self, picture: &PictureRef, max_width: u32) -> Result<Picture, DomainError> {
        let unavailable = |error: &dyn std::fmt::Display| {
            DomainError::PictureUnavailable(format!("{}: {error}", picture.as_str()))
        };
        // Jellyfin scales on its side when asked; other hosts send what they have
        let url = if picture.as_str().contains("/Items/") {
            format!("{}?maxWidth={max_width}&quality=85", picture.as_str())
        } else {
            picture.as_str().to_owned()
        };
        let bytes = self
            .agent
            .get(&url)
            .call()
            .map_err(|error| unavailable(&error))?
            .body_mut()
            .with_config()
            .limit(MAX_BYTES)
            .read_to_vec()
            .map_err(|error| unavailable(&error))?;
        let mut decoded = image::load_from_memory(&bytes).map_err(|error| unavailable(&error))?;
        if decoded.width() > max_width {
            let height = (u64::from(decoded.height()) * u64::from(max_width)
                / u64::from(decoded.width())) as u32;
            decoded = decoded.resize_exact(max_width, height.max(1), FilterType::Triangle);
        }
        let rgba = decoded.into_rgba8();
        Ok(Picture {
            width: rgba.width(),
            height: rgba.height(),
            rgba: rgba.into_raw(),
        })
    }
}
