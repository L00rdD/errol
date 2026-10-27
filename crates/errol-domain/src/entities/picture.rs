/// Where a picture lives (an absolute URL: Jellyfin, TMDB).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PictureRef(String);

impl PictureRef {
    pub fn new(url: impl Into<String>) -> Self {
        Self(url.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A decoded picture, RGBA, 8 bits per channel, rows top to bottom.
#[derive(Clone, PartialEq, Eq)]
pub struct Picture {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl std::fmt::Debug for Picture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Picture({}x{})", self.width, self.height)
    }
}
