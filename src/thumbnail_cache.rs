use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;
use tiny_skia::{Pixmap, Transform};

pub struct ThumbnailCache {
    cache: HashMap<(String, u32, u32), Option<Rc<Pixmap>>>,
}

impl ThumbnailCache {
    pub fn new() -> Self {
        Self { cache: HashMap::new() }
    }

    pub fn peek(&self, path: &Path, w: u32, h: u32) -> Option<Rc<Pixmap>> {
        self.cache.get(&(path.to_string_lossy().to_string(), w, h)).and_then(|v| v.clone())
    }

    pub fn insert(&mut self, path: &Path, w: u32, h: u32, pixmap: Option<Pixmap>) {
        let key = (path.to_string_lossy().to_string(), w, h);
        self.cache.insert(key, pixmap.map(Rc::new));
    }

    pub fn clear(&mut self) {
        self.cache.clear();
    }
}

// ----- baked corners -----
pub fn load(path: &Path, w: u32, h: u32, radius: f32) -> Option<Pixmap> {
    let img = image::open(path).ok()?.into_rgba8();
    let (iw, ih) = (img.width(), img.height());
    let target_ratio = w as f32 / h as f32;
    let src_ratio = iw as f32 / ih as f32;
    let (cw, ch) = if src_ratio > target_ratio {
        (((ih as f32 * target_ratio).round() as u32).min(iw).max(1), ih)
    } else {
        (iw, ((iw as f32 / target_ratio).round() as u32).min(ih).max(1))
    };
    let cx = (iw - cw) / 2;
    let cy = (ih - ch) / 2;
    let cropped = image::imageops::crop_imm(&img, cx, cy, cw, ch).to_image();
    let resized = image::imageops::resize(&cropped, w, h, image::imageops::FilterType::Triangle);

    let mut pixmap = Pixmap::new(w, h)?;
    let dst = pixmap.data_mut();
    for (i, px) in resized.pixels().enumerate() {
        let [r, g, b, a] = px.0;
        let a16 = a as u16;
        dst[i * 4] = ((r as u16 * a16) / 255) as u8;
        dst[i * 4 + 1] = ((g as u16 * a16) / 255) as u8;
        dst[i * 4 + 2] = ((b as u16 * a16) / 255) as u8;
        dst[i * 4 + 3] = a;
    }

    let rect = rounded_rect_path(w as f32, h as f32, radius);
    let mut mask = tiny_skia::Mask::new(w, h)?;
    mask.fill_path(&rect, tiny_skia::FillRule::Winding, true, Transform::identity());
    let mut clipped = Pixmap::new(w, h)?;
    clipped.draw_pixmap(0, 0, pixmap.as_ref(), &tiny_skia::PixmapPaint::default(), Transform::identity(), Some(&mask));
    Some(clipped)
}

fn rounded_rect_path(w: f32, h: f32, r: f32) -> tiny_skia::Path {
    let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
    let mut pb = tiny_skia::PathBuilder::new();
    pb.move_to(r, 0.0);
    pb.line_to(w - r, 0.0);
    pb.quad_to(w, 0.0, w, r);
    pb.line_to(w, h - r);
    pb.quad_to(w, h, w - r, h);
    pb.line_to(r, h);
    pb.quad_to(0.0, h, 0.0, h - r);
    pb.line_to(0.0, r);
    pb.quad_to(0.0, 0.0, r, 0.0);
    pb.close();
    pb.finish().unwrap()
}
