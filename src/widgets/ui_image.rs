use std::ops::DerefMut;

use cushy::animation::ZeroToOne;
use cushy::figures::units::UPx;
use cushy::figures::{Point, Rect, Size};
use cushy::kludgine::Texture;
use cushy::widget::{MakeWidget, Widget};
use cushy::widgets::Canvas;
use kludgine::wgpu;

use crate::outputs::Image;

pub fn image_widget(image: &Image) -> impl Widget {
    let image = image.clone();

    let size = Size::new(image.width, image.height);

    Canvas::new({
        move |context| {
            let rgba = &image.data;

            let texture = Texture::new_with_data(
                context.gfx.deref_mut().inner_graphics(),
                size.map(UPx::from),
                wgpu::TextureFormat::Rgba8UnormSrgb,
                wgpu::TextureUsages::TEXTURE_BINDING,
                wgpu::FilterMode::Linear,
                rgba.as_slice(),
            );
            context.gfx.draw_texture(
                &texture,
                Rect::new(Point::new(0, 0), size).map(UPx::from),
                ZeroToOne::ONE,
            );
        }
    })
    .width(UPx::new(size.width)..)
    .height(UPx::new(size.height)..)
}
