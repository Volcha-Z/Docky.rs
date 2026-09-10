use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;
use tiny_skia::{Pixmap, Transform};

const CACHE_CAP: usize = 300;

pub struct IconCache {
    theme: String,
    cache: HashMap<(String, u32), Option<Rc<Pixmap>>>,
    rounded_cache: HashMap<(String, u32), Option<Rc<Pixmap>>>,
}

impl IconCache {
    pub fn new(theme: impl Into<String>) -> Self {
        Self {
            theme: theme.into(),
            cache: HashMap::new(),
            rounded_cache: HashMap::new(),
        }
    }

    pub fn get(&mut self, icon_name: &str, size: u32) -> Option<Rc<Pixmap>> {
        let key = (icon_name.to_string(), size);
        if let Some(hit) = self.cache.get(&key) {
            return hit.clone();
        }
        if self.cache.len() >= CACHE_CAP {
            self.cache.clear();
        }
        let pixmap = resolve(&self.theme, icon_name, size).map(Rc::new);
        self.cache.insert(key, pixmap.clone());
        pixmap
    }

    pub fn get_rounded(&mut self, icon_name: &str, size: u32) -> Option<Rc<Pixmap>> {
        let key = (icon_name.to_string(), size);
        if let Some(hit) = self.rounded_cache.get(&key) {
            return hit.clone();
        }
        if self.rounded_cache.len() >= CACHE_CAP {
            self.rounded_cache.clear();
        }
        let base = self.get(icon_name, size);
        let rounded = base.and_then(|base| round_corners(&base, size));
        self.rounded_cache.insert(key, rounded.clone());
        rounded
    }
}

fn round_corners(base: &Pixmap, size: u32) -> Option<Rc<Pixmap>> {
    let radius = size as f32 * 0.22;
    let path = rounded_rect_path(size as f32, size as f32, radius);
    let mut mask = tiny_skia::Mask::new(size, size)?;
    mask.fill_path(&path, tiny_skia::FillRule::Winding, true, Transform::identity());
    let mut clipped = Pixmap::new(size, size)?;
    clipped.draw_pixmap(0, 0, base.as_ref(), &tiny_skia::PixmapPaint::default(), Transform::identity(), Some(&mask));
    Some(Rc::new(clipped))
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

fn resolve(theme: &str, icon_name: &str, size: u32) -> Option<Pixmap> {
    if icon_name.is_empty() {
        return None;
    }
    let direct = Path::new(icon_name);
    let path = if direct.is_absolute() && direct.exists() {
        direct.to_path_buf()
    } else {
        freedesktop_icons::lookup(icon_name)
            .with_size(size as u16)
            .with_theme(theme)
            .with_cache()
            .find()
            .or_else(|| {
                freedesktop_icons::lookup(icon_name)
                    .with_size(size as u16)
                    .find()
            })?
    };
    load_from_path(&path, size)
}

fn load_from_path(path: &Path, size: u32) -> Option<Pixmap> {
    match path.extension().and_then(|e| e.to_str()) {
        Some(ext) if ext.eq_ignore_ascii_case("svg") => load_svg(path, size),
        _ => load_raster(path, size),
    }
}

fn load_svg(path: &Path, size: u32) -> Option<Pixmap> {
    let data = std::fs::read(path).ok()?;
    let opts = usvg::Options::default();
    let tree = usvg::Tree::from_data(&data, &opts).ok()?;
    let src_size = tree.size();
    let mut pixmap = Pixmap::new(size, size)?;
    let scale_x = size as f32 / src_size.width();
    let scale_y = size as f32 / src_size.height();
    let transform = tiny_skia::Transform::from_scale(scale_x, scale_y);
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    Some(pixmap)
}

fn load_raster(path: &Path, size: u32) -> Option<Pixmap> {
    let img = image::open(path).ok()?.into_rgba8();
    let resized = image::imageops::resize(&img, size, size, image::imageops::FilterType::Lanczos3);
    let mut pixmap = Pixmap::new(size, size)?;
    let dst = pixmap.data_mut();
    for (i, px) in resized.pixels().enumerate() {
        let [r, g, b, a] = px.0;
        let a16 = a as u16;
        let pr = ((r as u16 * a16) / 255) as u8;
        let pg = ((g as u16 * a16) / 255) as u8;
        let pb = ((b as u16 * a16) / 255) as u8;
        dst[i * 4] = pr;
        dst[i * 4 + 1] = pg;
        dst[i * 4 + 2] = pb;
        dst[i * 4 + 3] = a;
    }
    Some(pixmap)
}
