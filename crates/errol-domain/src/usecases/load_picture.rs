use std::sync::Arc;

use crate::DomainError;
use crate::entities::{Picture, PictureRef};
use crate::ports::PictureSource;

/// Fetches a picture for the screen (a poster, a backdrop).
pub struct LoadPicture {
    source: Arc<dyn PictureSource>,
}

impl LoadPicture {
    pub fn new(source: Arc<dyn PictureSource>) -> Self {
        Self { source }
    }

    pub fn execute(&self, picture: &PictureRef, max_width: u32) -> Result<Picture, DomainError> {
        self.source.fetch(picture, max_width)
    }
}
