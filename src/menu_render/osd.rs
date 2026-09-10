use super::*;

pub struct OsdArgs<'a> {
    pub kind: OsdKind,
    pub level: u8,
    pub muted: bool,
    pub panel_w: f32,
    pub panel_h: f32,
    pub dock: &'a Dock,
    pub render_scale: f32,
}

pub fn draw_osd(pixmap: &mut Pixmap, text_cache: &mut TextCache, args: &OsdArgs) {
    let s = args.render_scale;
    let w = args.panel_w * s;
    let h = args.panel_h * s;
    let settings = &args.dock.config.settings;

    let bg = panel_bg(settings);
    let path = rounded_rect_path(0.0, 0.0, w, h, settings.corner_radius * s);
    let mut paint = Paint::default();
    paint.set_color_rgba8(bg.0, bg.1, bg.2, bg.3);
    paint.anti_alias = true;
    pixmap.fill_path(&path, &paint, tiny_skia::FillRule::Winding, Transform::identity(), None);

    let acc = accent(settings);
    let icon_color = if args.muted { (255, 69, 58, 255) } else { acc };
    let fill_color = if args.muted { (120, 120, 124, 255) } else { acc };
    let label = if args.muted { "Muted".to_string() } else { format!("{}%", args.level) };

    if args.dock.is_vertical() {
        draw_osd_vertical(pixmap, text_cache, args, settings, w, h, icon_color, fill_color, &label);
    } else {
        draw_osd_horizontal(pixmap, text_cache, args, settings, w, h, icon_color, fill_color, &label);
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_osd_horizontal(
    pixmap: &mut Pixmap,
    text_cache: &mut TextCache,
    args: &OsdArgs,
    settings: &crate::config::DockSettings,
    w: f32,
    h: f32,
    icon_color: (u8, u8, u8, u8),
    fill_color: (u8, u8, u8, u8),
    label: &str,
) {
    let s = args.render_scale;
    let pad = MENU_PADDING * s;
    let icon_r = 9.0 * s;
    let icon_cx = pad + icon_r;
    let icon_cy = h / 2.0;
    draw_osd_icon(pixmap, args, icon_cx, icon_cy, icon_r, icon_color, s);

    let label_w = text_width_estimate(label, 9.0) * s;
    let bar_x0 = icon_cx + icon_r + 14.0 * s;
    let bar_x1 = w - pad - label_w - 10.0 * s;
    let bar_h = 6.0 * s;
    let bar_y = h / 2.0 - bar_h / 2.0;
    let bar_w = (bar_x1 - bar_x0).max(0.0);
    fill_rrect(pixmap, bar_x0, bar_y, bar_w, bar_h, bar_h / 2.0, track_bg(settings));
    let fill_frac = (args.level as f32 / 100.0).clamp(0.0, 1.0);
    fill_rrect(pixmap, bar_x0, bar_y, bar_w * fill_frac, bar_h, bar_h / 2.0, fill_color);

    let ty = h / 2.0 - (9.0 * s) * 0.6;
    draw_text(pixmap, text_cache, label, w - pad - label_w, ty, 9.0 * s, &text_hex(settings), 600);
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_osd_vertical(
    pixmap: &mut Pixmap,
    text_cache: &mut TextCache,
    args: &OsdArgs,
    settings: &crate::config::DockSettings,
    w: f32,
    h: f32,
    icon_color: (u8, u8, u8, u8),
    fill_color: (u8, u8, u8, u8),
    label: &str,
) {
    let s = args.render_scale;
    let pad = MENU_PADDING * s;
    let icon_r = 9.0 * s;
    let icon_cx = w / 2.0;
    let icon_cy = pad + icon_r;
    draw_osd_icon(pixmap, args, icon_cx, icon_cy, icon_r, icon_color, s);

    let label_len = text_width_estimate(label, 9.0) * s;
    let bar_y0 = icon_cy + icon_r + 14.0 * s;
    let bar_y1 = h - pad - label_len - 10.0 * s;
    let bar_w = 6.0 * s;
    let bar_x = w / 2.0 - bar_w / 2.0;
    let bar_h = (bar_y1 - bar_y0).max(0.0);
    fill_rrect(pixmap, bar_x, bar_y0, bar_w, bar_h, bar_w / 2.0, track_bg(settings));
    let fill_frac = (args.level as f32 / 100.0).clamp(0.0, 1.0);
    let fill_h = bar_h * fill_frac;
    fill_rrect(pixmap, bar_x, bar_y0 + bar_h - fill_h, bar_w, fill_h, bar_w / 2.0, fill_color);

    let label_cy = h - pad - label_len / 2.0;
    draw_text_rotated(pixmap, text_cache, label, w / 2.0, label_cy, 9.0 * s, &text_hex(settings), 600);
}

pub(crate) fn draw_osd_icon(pixmap: &mut Pixmap, args: &OsdArgs, cx: f32, cy: f32, r: f32, icon_color: (u8, u8, u8, u8), s: f32) {
    let mut icon_paint = Paint::default();
    icon_paint.set_color_rgba8(icon_color.0, icon_color.1, icon_color.2, icon_color.3);
    icon_paint.anti_alias = true;
    match args.kind {
        OsdKind::Volume => draw_volume_icon(pixmap, cx, cy, r, &icon_paint, args.muted, s),
        OsdKind::Brightness => draw_brightness_icon(pixmap, cx, cy, r, icon_color, s),
    }
}

// ----- vertical rotation -----
pub(crate) fn draw_text_rotated(pixmap: &mut Pixmap, text_cache: &mut TextCache, text: &str, cx: f32, cy: f32, size: f32, color: &str, weight: u16) {
    let Some(glyphs) = text_cache.get(text, size, color, weight) else {
        return;
    };
    let iw = glyphs.width() as f32;
    let ih = glyphs.height() as f32;
    let paint = tiny_skia::PixmapPaint::default();
    let transform = Transform::from_translate(-iw / 2.0, -ih / 2.0).post_rotate(-90.0).post_translate(cx, cy);
    pixmap.draw_pixmap(0, 0, glyphs.as_ref().as_ref(), &paint, transform, None);
}

pub(crate) fn draw_volume_icon(pixmap: &mut Pixmap, cx: f32, cy: f32, r: f32, paint: &Paint, muted: bool, s: f32) {
    let mut pb = tiny_skia::PathBuilder::new();
    let body_w = r * 0.5;
    let body_h = r * 0.9;
    pb.move_to(cx - r, cy - body_h * 0.3);
    pb.line_to(cx - r + body_w, cy - body_h * 0.3);
    pb.line_to(cx - r * 0.1, cy - body_h);
    pb.line_to(cx - r * 0.1, cy + body_h);
    pb.line_to(cx - r + body_w, cy + body_h * 0.3);
    pb.line_to(cx - r, cy + body_h * 0.3);
    pb.close();
    if let Some(path) = pb.finish() {
        pixmap.fill_path(&path, paint, tiny_skia::FillRule::Winding, Transform::identity(), None);
    }
    if muted {
        let mut xpb = tiny_skia::PathBuilder::new();
        let x0 = cx + r * 0.15;
        let x1 = cx + r * 0.95;
        xpb.move_to(x0, cy - r * 0.4);
        xpb.line_to(x1, cy + r * 0.4);
        xpb.move_to(x1, cy - r * 0.4);
        xpb.line_to(x0, cy + r * 0.4);
        if let Some(path) = xpb.finish() {
            let stroke = tiny_skia::Stroke { width: 1.6 * s, line_cap: tiny_skia::LineCap::Round, ..Default::default() };
            pixmap.stroke_path(&path, paint, &stroke, Transform::identity(), None);
        }
    } else {
        let mut wpb = tiny_skia::PathBuilder::new();
        for wave_r in [r * 0.45, r * 0.85] {
            wpb.move_to(cx + r * 0.05, cy - wave_r);
            wpb.quad_to(cx + r * 0.05 + wave_r * 1.3, cy, cx + r * 0.05, cy + wave_r);
        }
        if let Some(path) = wpb.finish() {
            let stroke = tiny_skia::Stroke { width: 1.4 * s, line_cap: tiny_skia::LineCap::Round, ..Default::default() };
            pixmap.stroke_path(&path, paint, &stroke, Transform::identity(), None);
        }
    }
}

pub(crate) fn draw_brightness_icon(pixmap: &mut Pixmap, cx: f32, cy: f32, r: f32, color: (u8, u8, u8, u8), s: f32) {
    fill_circle(pixmap, cx, cy, r * 0.45, color);
    let mut paint = Paint::default();
    paint.set_color_rgba8(color.0, color.1, color.2, color.3);
    paint.anti_alias = true;
    let mut pb = tiny_skia::PathBuilder::new();
    for i in 0..8 {
        let angle = i as f32 * std::f32::consts::PI / 4.0;
        let (cos, sin) = (angle.cos(), angle.sin());
        pb.move_to(cx + cos * r * 0.68, cy + sin * r * 0.68);
        pb.line_to(cx + cos * r, cy + sin * r);
    }
    if let Some(path) = pb.finish() {
        let stroke = tiny_skia::Stroke { width: 1.4 * s, line_cap: tiny_skia::LineCap::Round, ..Default::default() };
        pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
    }
}

pub(crate) fn truncate_to_width(text: &str, size: f32, max_w: f32) -> String {
    if text_width_estimate(text, size) <= max_w {
        return text.to_string();
    }
    let budget = (max_w / (size * 0.56) - 1.0).max(0.0) as usize;
    let mut out: String = text.chars().take(budget).collect();
    out.push('…');
    out
}

// ----- word wrap -----
pub(crate) fn wrap_to_width(text: &str, size: f32, max_w: f32, max_lines: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut cur = String::new();
    for word in text.split_whitespace() {
        if lines.len() >= max_lines {
            break;
        }
        let trial = if cur.is_empty() { word.to_string() } else { format!("{cur} {word}") };
        if cur.is_empty() || text_width_estimate(&trial, size) <= max_w {
            cur = trial;
        } else {
            lines.push(std::mem::take(&mut cur));
            cur = word.to_string();
        }
    }
    if !cur.is_empty() && lines.len() < max_lines {
        lines.push(cur);
    }
    let shown: usize = lines.iter().map(|l| l.split_whitespace().count()).sum();
    if shown < text.split_whitespace().count() {
        if let Some(last) = lines.last_mut() {
            last.push('…');
        }
    }
    lines.iter().map(|l| truncate_to_width(l, size, max_w)).collect()
}
