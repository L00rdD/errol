//! Pictures on the GPU. A picture is uploaded once, the first frame it is
//! there; a backdrop is blurred once, on the GPU, when uploaded. Beyond `KEEP`
//! the oldest is dropped, and fetched again if it is wanted later.

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

pub struct Textures<'a> {
    pictures: &'a Pictures,
    images: HashMap<(PictureRef, u32, bool), ImageId>,
    order: Vec<(PictureRef, u32, bool)>,
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

    /// The picture at `width` pixels, blurred or not; None while it loads.
    pub fn get(
        &mut self,
        canvas: &mut Canvas<OpenGl>,
        picture: &PictureRef,
        width: u32,
        blur: bool,
    ) -> Option<ImageId> {
        let key = (picture.clone(), width, blur);
        if let Some(image) = self.images.get(&key) {
            return Some(*image);
        }
        let decoded = self.pictures.take(picture, width)?;
        let source = femtovg::imgref::Img::new(
            decoded.rgba.as_rgba(),
            decoded.width as usize,
            decoded.height as usize,
        );
        let mut image = canvas.create_image(source, ImageFlags::empty()).ok()?;
        if blur {
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
