use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    io::Cursor,
    sync::Arc,
};

use image::{
    codecs::png::{CompressionType, FilterType as PngFilterType, PngEncoder},
    imageops::FilterType,
    ColorType, ImageEncoder, ImageReader, Limits,
};

const PREVIEW_WIDTH: u32 = 1024;
const PREVIEW_HEIGHT: u32 = 96;

#[derive(Clone)]
pub struct ClipboardEntry {
    pub mime: String,
    pub data: Arc<[u8]>,
    pub title: String,
    pub description: String,
    pub thumbnail: Option<Arc<Thumbnail>>,
    fingerprint: u64,
}

pub struct Thumbnail {
    png: Vec<u8>,
}

pub struct ScaledPreview {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

pub(crate) struct StoredEntry<'a> {
    pub mime: &'a str,
    pub data: &'a [u8],
    pub title: &'a str,
    pub description: &'a str,
    pub preview: Option<&'a [u8]>,
}

impl ClipboardEntry {
    pub fn from_data(mime: String, data: Vec<u8>) -> Option<Self> {
        if data.is_empty() {
            return None;
        }
        let fingerprint = fingerprint(&mime, &data);
        if mime.starts_with("image/") {
            image_entry(mime, data, fingerprint)
        } else {
            text_entry(mime, data, fingerprint)
        }
    }

    pub fn from_screenshot(data: Vec<u8>, width: u32, height: u32, thumbnail: Vec<u8>) -> Self {
        let mime = String::from("image/png");
        let fingerprint = fingerprint(&mime, &data);
        Self {
            mime,
            data: data.into(),
            title: String::from("Screenshot"),
            description: format!("PNG image  \u{b7}  {width} \u{d7} {height}"),
            thumbnail: Thumbnail::from_rgba(&thumbnail, PREVIEW_WIDTH, PREVIEW_HEIGHT),
            fingerprint,
        }
    }

    pub fn matches(&self, query: &str) -> bool {
        self.title.to_lowercase().contains(query) || self.description.to_lowercase().contains(query)
    }

    pub fn same_content(&self, other: &Self) -> bool {
        self.fingerprint == other.fingerprint && self.data == other.data
    }

    pub fn is_text(&self) -> bool {
        !self.mime.starts_with("image/")
    }

    pub fn size(&self) -> usize {
        self.data.len() + self.thumbnail.as_ref().map_or(0, |thumbnail| thumbnail.png.len())
    }

    pub(crate) fn stored(&self) -> StoredEntry<'_> {
        StoredEntry {
            mime: &self.mime,
            data: &self.data,
            title: &self.title,
            description: &self.description,
            preview: self.thumbnail.as_ref().map(|thumbnail| thumbnail.png.as_slice()),
        }
    }

    pub(crate) fn restore(mime: String, data: Vec<u8>, title: String, description: String, preview: Vec<u8>) -> Option<Self> {
        if data.is_empty() {
            return None;
        }
        let fingerprint = fingerprint(&mime, &data);
        let thumbnail = if preview.is_empty() {
            None
        } else if preview.starts_with(b"\x89PNG") {
            Some(Arc::new(Thumbnail { png: preview }))
        } else if preview.len() == (PREVIEW_WIDTH * PREVIEW_HEIGHT * 4) as usize {
            Thumbnail::from_rgba(&preview, PREVIEW_WIDTH, PREVIEW_HEIGHT)
        } else {
            None
        };
        Some(Self {
            mime,
            data: data.into(),
            title,
            description,
            thumbnail,
            fingerprint,
        })
    }
}

impl Thumbnail {
    fn from_rgba(rgba: &[u8], width: u32, height: u32) -> Option<Arc<Self>> {
        if rgba.len() != (width * height * 4) as usize {
            return None;
        }
        let mut png = Vec::new();
        PngEncoder::new_with_quality(&mut png, CompressionType::Fast, PngFilterType::Sub)
            .write_image(rgba, width, height, ColorType::Rgba8.into())
            .ok()?;
        Some(Arc::new(Self { png }))
    }

    // ----- decode and rescale -----
    pub fn scale_to(&self, width: u32, height: u32) -> Option<ScaledPreview> {
        if width == 0 || height == 0 {
            return None;
        }
        let source = ImageReader::new(Cursor::new(&self.png)).with_guessed_format().ok()?.decode().ok()?.to_rgba8();
        let (source_width, source_height) = (source.width(), source.height());
        let source = source.as_raw();
        let mut pixels = vec![0; (width * height * 4) as usize];
        let horizontal_step = ((source_width as u64) << 32) / width as u64;
        let vertical_step = ((source_height as u64) << 32) / height as u64;
        for y in 0..height {
            let source_y = ((y as u64 * vertical_step) >> 32) as u32;
            let mut horizontal = 0_u64;
            for x in 0..width {
                let source_x = (horizontal >> 32) as u32;
                horizontal += horizontal_step;
                let origin = ((source_y * source_width + source_x) * 4) as usize;
                let target = ((y * width + x) * 4) as usize;
                pixels[target] = source[origin];
                pixels[target + 1] = source[origin + 1];
                pixels[target + 2] = source[origin + 2];
                pixels[target + 3] = source[origin + 3];
            }
        }
        Some(ScaledPreview { pixels, width, height })
    }
}

fn text_entry(mime: String, data: Vec<u8>, fingerprint: u64) -> Option<ClipboardEntry> {
    let text = std::str::from_utf8(&data).ok()?.trim_matches(['\0', '\n', '\r', ' ']);
    if text.is_empty() {
        return None;
    }
    let title = text.split_whitespace().take(18).collect::<Vec<_>>().join(" ");
    let characters = text.chars().count();
    Some(ClipboardEntry {
        mime,
        data: data.into(),
        title,
        description: format!("Text  \u{b7}  {characters} characters"),
        thumbnail: None,
        fingerprint,
    })
}

fn image_entry(mime: String, data: Vec<u8>, fingerprint: u64) -> Option<ClipboardEntry> {
    let mut reader = ImageReader::new(Cursor::new(data.as_slice())).with_guessed_format().ok()?;
    let mut limits = Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    limits.max_alloc = Some(64 * 1024 * 1024);
    reader.limits(limits);
    let image = reader.decode().ok()?;
    let (width, height) = (image.width(), image.height());
    let thumbnail = image.resize_to_fill(PREVIEW_WIDTH, PREVIEW_HEIGHT, FilterType::Lanczos3).to_rgba8().into_raw();
    Some(ClipboardEntry {
        mime: mime.clone(),
        data: data.into(),
        title: "Image".into(),
        description: format!("{}  \u{b7}  {width} \u{d7} {height}", mime_name(&mime)),
        thumbnail: Thumbnail::from_rgba(&thumbnail, PREVIEW_WIDTH, PREVIEW_HEIGHT),
        fingerprint,
    })
}

fn mime_name(mime: &str) -> &str {
    match mime {
        "image/png" => "PNG image",
        "image/jpeg" => "JPEG image",
        "image/webp" => "WebP image",
        _ => "Image",
    }
}

fn fingerprint(mime: &str, data: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    mime.hash(&mut hasher);
    data.hash(&mut hasher);
    hasher.finish()
}
