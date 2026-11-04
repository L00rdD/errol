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
        let bytes = self
            .agent
            .get(&sized_url(picture, max_width))
            .call()
            .map_err(|error| unavailable(&error))?
            .body_mut()
            .with_config()
            .limit(MAX_BYTES)
            .read_to_vec()
            .map_err(|error| unavailable(&error))?;
        decode(&bytes, max_width).map_err(|error| unavailable(&error))
    }
}

/// Jellyfin scales on its side when asked; other hosts send what they have.
fn sized_url(picture: &PictureRef, max_width: u32) -> String {
    if picture.as_str().contains("/Items/") {
        format!("{}?maxWidth={max_width}&quality=85", picture.as_str())
    } else {
        picture.as_str().to_owned()
    }
}

/// Decodes a picture, brought down to `max_width` when it is wider.
fn decode(bytes: &[u8], max_width: u32) -> Result<Picture, image::ImageError> {
    let mut decoded = image::load_from_memory(bytes)?;
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

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use image::{ImageFormat, RgbaImage};

    use super::*;

    fn png(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = Cursor::new(Vec::new());
        RgbaImage::new(width, height)
            .write_to(&mut bytes, ImageFormat::Png)
            .unwrap();
        bytes.into_inner()
    }

    #[test]
    fn jellyfin_is_asked_for_the_width_it_is_shown_at() {
        let poster = PictureRef::new("http://jf:8096/Items/abc/Images/Primary");
        assert_eq!(
            sized_url(&poster, 300),
            "http://jf:8096/Items/abc/Images/Primary?maxWidth=300&quality=85"
        );
        let other = PictureRef::new("https://image.tmdb.org/t/p/w342/x.jpg");
        assert_eq!(
            sized_url(&other, 300),
            "https://image.tmdb.org/t/p/w342/x.jpg"
        );
    }

    #[test]
    fn a_wide_picture_is_brought_down_to_the_width() {
        let picture = decode(&png(400, 200), 100).unwrap();
        assert_eq!((picture.width, picture.height), (100, 50));
        assert_eq!(picture.rgba.len(), 100 * 50 * 4);
    }

    #[test]
    fn a_narrow_picture_keeps_its_size_and_a_sliver_keeps_a_row() {
        let picture = decode(&png(80, 60), 100).unwrap();
        assert_eq!((picture.width, picture.height), (80, 60));
        let sliver = decode(&png(1000, 2), 100).unwrap();
        assert_eq!((sliver.width, sliver.height), (100, 1));
    }

    #[test]
    fn what_is_not_a_picture_is_refused() {
        assert!(decode(b"<html>", 100).is_err());
    }
}
