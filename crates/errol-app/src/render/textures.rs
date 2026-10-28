//! Pictures on the GPU. A picture is uploaded once, the first frame it is
//! there; a backdrop is blurred once, on the GPU, when uploaded; a photograph
//! for the Gazette is turned sepia before. Beyond `KEEP` the oldest is
//! dropped, and fetched again if it is wanted later.

use std::collections::HashMap;

use errol_domain::entities::PictureRef;
use femtovg::renderer::OpenGl;
use femtovg::rgb::FromSlice;
use femtovg::{Canvas, ImageFilter, ImageFlags, ImageId, PixelFormat};

use crate::pictures::Pictures;

/// How soft a blurred backdrop is.
const BLUR: f32 = 5.0;
/// Pictures kept on the GPU; beyond, the oldest go.
const KEEP: usize = 24;

/// What is done to a picture once decoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Finish {
    Plain,
    /// Softened, for a backdrop.
    Blur,
    /// Sepia, as the Gazette prints its photographs.
    Print,
}

pub struct Textures<'a> {
    pictures: &'a Pictures,
    images: HashMap<(PictureRef, u32, Finish), ImageId>,
    order: Vec<(PictureRef, u32, Finish)>,
    /// Sources of blurs: femtovg blurs at `flush`, so they go after it.
    trash: Vec<ImageId>,
}

impl<'a> Textures<'a> {
    pub fn new(pictures: &'a Pictures) -> Self {
        Self {
            pictures,
            images: HashMap::new(),
            order: Vec::new(),
            trash: Vec::new(),
        }
    }

    /// Frees what the last frame no longer needs; call after `flush`.
    pub fn after_flush(&mut self, canvas: &mut Canvas<OpenGl>) {
        for image in self.trash.drain(..) {
            canvas.delete_image(image);
        }
    }

    /// The picture at `width` pixels, with its finish; None while it loads.
    pub fn get(
        &mut self,
        canvas: &mut Canvas<OpenGl>,
        picture: &PictureRef,
        width: u32,
        finish: Finish,
    ) -> Option<ImageId> {
        let key = (picture.clone(), width, finish);
        if let Some(image) = self.images.get(&key) {
            return Some(*image);
        }
        let mut decoded = self.pictures.take(picture, width)?;
        if finish == Finish::Print {
            sepia(&mut decoded.rgba);
        }
        let source = femtovg::imgref::Img::new(
            decoded.rgba.as_rgba(),
            decoded.width as usize,
            decoded.height as usize,
        );
        let mut image = canvas.create_image(source, ImageFlags::empty()).ok()?;
        if finish == Finish::Blur {
            let soft = canvas
                .create_image_empty(
                    decoded.width as usize,
                    decoded.height as usize,
                    PixelFormat::Rgba8,
                    ImageFlags::empty(),
                )
                .ok()?;
            canvas.filter_image(soft, ImageFilter::GaussianBlur { sigma: BLUR }, image);
            self.trash.push(image);
            image = soft;
        }
        self.images.insert(key.clone(), image);
        self.order.push(key);
        if self.order.len() > KEEP {
            let old = self.order.remove(0);
            if let Some(image) = self.images.remove(&old) {
                canvas.delete_image(image);
            }
            // without this the picture could never come back to the screen
            self.pictures.forget(&old.0, old.1);
        }
        Some(image)
    }
}

/// Greys the picture and warms it, as an old print: the matrix of Pensieve's
/// Gazette.
fn sepia(rgba: &mut [u8]) {
    for pixel in rgba.as_chunks_mut::<4>().0 {
        let grey =
            0.3 * f32::from(pixel[0]) + 0.59 * f32::from(pixel[1]) + 0.11 * f32::from(pixel[2]);
        pixel[0] = (grey * 1.1 + 18.0).clamp(0.0, 255.0) as u8;
        pixel[1] = (grey + 6.0).clamp(0.0, 255.0) as u8;
        pixel[2] = (grey * 0.85 - 8.0).clamp(0.0, 255.0) as u8;
    }
}
