use std::sync::Arc;

use gpui_kit::{img, px, AnyElement, ImageSource, IntoElement, RenderImage, Styled, Window};
use image::Frame;
use smallvec::smallvec;

use crate::outputs::Image as ImageData;

/// Builds an `Arc<RenderImage>` from already-BGRA `ImageData` and returns
/// it, so the caller can later `window.drop_image` it once the outputs
/// that reference it are replaced.
pub fn build_render_image(data: &ImageData) -> Arc<RenderImage> {
    let frame = image::RgbaImage::from_raw(data.width, data.height, data.data.clone())
        .expect("Image width/height must match data length");
    Arc::new(RenderImage::new(smallvec![Frame::new(frame)]))
}

/// Renders the image at its native pixel size.
pub fn image_element(data: &ImageData, render_image: Arc<RenderImage>) -> AnyElement {
    img(ImageSource::Render(render_image))
        .w(px(data.width as f32))
        .h(px(data.height as f32))
        .into_any_element()
}

pub fn drop_images(window: &mut Window, images: Vec<Arc<RenderImage>>) {
    for image in images {
        let _ = window.drop_image(image);
    }
}
