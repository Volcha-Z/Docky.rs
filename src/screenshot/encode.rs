use std::io::Cursor;

use image::{
    codecs::png::{CompressionType, FilterType, PngEncoder},
    imageops, ColorType, DynamicImage, ImageEncoder, RgbaImage,
};
use wayland_client::protocol::wl_shm;

use super::{CaptureImage, FrameSpec};

pub fn encode(data: &[u8], spec: FrameSpec, inverted: bool) -> Option<CaptureImage> {
    let length = spec.width.checked_mul(spec.height)?.checked_mul(4)? as usize;
    let mut rgba = vec![0; length];
    for y in 0..spec.height {
        let source_y = if inverted { spec.height - y - 1 } else { y };
        let source_row = source_y as usize * spec.stride as usize;
        let target_row = y as usize * spec.width as usize * 4;
        for x in 0..spec.width as usize {
            let source = source_row + x * 4;
            let target = target_row + x * 4;
            let pixel = u32::from_ne_bytes(data.get(source..source + 4)?.try_into().ok()?);
            rgba[target] = (pixel >> 16) as u8;
            rgba[target + 1] = (pixel >> 8) as u8;
            rgba[target + 2] = pixel as u8;
            rgba[target + 3] = if spec.format == wl_shm::Format::Argb8888 {
                (pixel >> 24) as u8
            } else {
                255
            };
        }
    }
    let image = RgbaImage::from_raw(spec.width, spec.height, rgba)?;
    let thumbnail = DynamicImage::ImageRgba8(image.clone())
        .resize_to_fill(1024, 96, imageops::FilterType::Triangle)
        .to_rgba8()
        .into_raw();
    let mut png = Cursor::new(Vec::new());
    PngEncoder::new_with_quality(&mut png, CompressionType::Fast, FilterType::Sub)
        .write_image(image.as_raw(), spec.width, spec.height, ColorType::Rgba8.into())
        .ok()?;
    Some(CaptureImage {
        png: png.into_inner(),
        width: spec.width,
        height: spec.height,
        thumbnail,
    })
}
