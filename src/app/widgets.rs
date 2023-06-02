use std::io::Read;

use image::imageops::FilterType;
use image::io::Reader as ImageReader;
use image::DynamicImage;
use image::GenericImageView;
use image::Pixel;
use ratatui::{
    backend::{Backend, CrosstermBackend},
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::Span,
    widgets::Widget,
    Frame, Terminal,
};

pub struct Image<'a> {
    img: &'a DynamicImage,
}

impl<'a> Image<'a> {
    pub fn new(img: &'a DynamicImage) -> Image<'a> {
        Image { img }
    }
}

impl<'a> Widget for Image<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let img = self
            .img
            .resize_exact(area.width.into(), area.height.into(), FilterType::Triangle);

        assert!(area.width as u32 == img.width());
        assert!(area.height as u32 == img.height());

        for x in 0..img.width() {
            for y in 0..img.height() {
                let pixel = img.get_pixel(x, y);
                let rgb = pixel.to_rgb();

                let style = Style::default().fg(Color::Rgb(rgb.0[0], rgb.0[1], rgb.0[2]));
                let mut block_full = Span::raw(ratatui::symbols::block::FULL);
                block_full.patch_style(style);
                buf.set_span(area.x + x as u16, area.y + y as u16, &block_full, 1);
            }
        }
    }
}
