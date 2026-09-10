use std::collections::HashMap;
use std::rc::Rc;
use std::sync::OnceLock;
use tiny_skia::Pixmap;

const SANS_CANDIDATES: &[&str] = &["Noto Sans", "DejaVu Sans", "Liberation Sans", "Cantarell", "Ubuntu", "Roboto", "Arial"];

fn options() -> &'static usvg::Options<'static> {
    static OPTS: OnceLock<usvg::Options> = OnceLock::new();
    OPTS.get_or_init(|| {
        let mut opts = usvg::Options::default();
        let fontdb = opts.fontdb_mut();
        fontdb.load_system_fonts();
        for candidate in SANS_CANDIDATES {
            if fontdb.faces().any(|f| f.families.iter().any(|(name, _)| name == candidate)) {
                fontdb.set_sans_serif_family(*candidate);
                break;
            }
        }
        opts
    })
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn escape_attr(s: &str) -> String {
    escape(s).replace('"', "&quot;")
}

// ----- enumerated once -----
pub fn list_font_families() -> Vec<String> {
    let mut names: Vec<String> = options().fontdb.faces().flat_map(|f| f.families.iter().map(|(name, _)| name.clone())).collect();
    names.sort_unstable();
    names.dedup();
    names
}

fn rasterize(text: &str, size_px: f32, color: &str, weight: u16, family: &str) -> Option<Pixmap> {
    if text.is_empty() {
        return None;
    }
    let canvas_w = (text.chars().count() as f32 * size_px * 1.3 + size_px * 2.0).ceil().max(1.0);
    let h = (size_px * 1.5).ceil();
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}"><text x="0" y="{baseline}" font-family="{family}" font-weight="{weight}" font-size="{size}" fill="{color}">{text}</text></svg>"#,
        w = canvas_w,
        h = h,
        baseline = size_px * 1.05,
        size = size_px,
        color = escape_attr(color),
        weight = weight,
        family = escape_attr(family),
        text = escape(text)
    );
    let tree = usvg::Tree::from_str(&svg, options()).ok()?;
    let mut full = Pixmap::new(canvas_w as u32, h as u32)?;
    resvg::render(&tree, tiny_skia::Transform::identity(), &mut full.as_mut());
    let true_w = (tree.root().abs_bounding_box().right().ceil().max(1.0) as u32).min(canvas_w as u32);
    full.clone_rect(tiny_skia::IntRect::from_xywh(0, 0, true_w, h as u32)?)
}

const CACHE_CAP: usize = 400;

pub struct TextCache {
    cache: HashMap<String, Option<Rc<Pixmap>>>,
    default_family: String,
    key_buf: String,
}

impl TextCache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            default_family: "sans-serif".to_string(),
            key_buf: String::new(),
        }
    }

    pub fn set_default_family(&mut self, family: &str) {
        let family = if family.is_empty() { "sans-serif" } else { family };
        if family != self.default_family {
            self.default_family = family.to_string();
            self.cache.clear();
        }
    }

    pub fn get(&mut self, text: &str, size_px: f32, color: &str, weight: u16) -> Option<Rc<Pixmap>> {
        let family = std::mem::take(&mut self.default_family);
        let result = self.lookup(text, size_px, color, weight, &family);
        self.default_family = family;
        result
    }

    pub fn get_with_family(&mut self, text: &str, size_px: f32, color: &str, weight: u16, family: &str) -> Option<Rc<Pixmap>> {
        self.lookup(text, size_px, color, weight, family)
    }

    // ----- reused key buffer -----
    fn lookup(&mut self, text: &str, size_px: f32, color: &str, weight: u16, family: &str) -> Option<Rc<Pixmap>> {
        use std::fmt::Write;
        self.key_buf.clear();
        let _ = write!(self.key_buf, "{}\u{1f}{weight}\u{1f}{color}\u{1f}{family}\u{1f}{text}", size_px as u32);
        if let Some(hit) = self.cache.get(&self.key_buf) {
            return hit.clone();
        }
        if self.cache.len() >= CACHE_CAP {
            self.cache.clear();
        }
        let pixmap = rasterize(text, size_px, color, weight, family).map(Rc::new);
        self.cache.insert(self.key_buf.clone(), pixmap.clone());
        pixmap
    }
}
