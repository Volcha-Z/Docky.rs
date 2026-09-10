use std::collections::HashMap;

use tiny_skia::{Pixmap, PixmapPaint, Transform};

use crate::clipboard::{ClipboardEntry, ScaledPreview};
use crate::config::DockSettings;
use crate::text::TextCache;

use super::{centered_text_y, draw_text, fill_rrect, panel_bg, rounded_rect_path, text_dim_hex, text_hex};

pub const CLIP_ROW_H: f32 = 54.0;
pub const CLIP_HEADER_H: f32 = 42.0;
pub const CLIP_PAD: f32 = 10.0;
pub const CLIP_VISIBLE_ROWS: usize = 7;
pub const CLIP_PANEL_W: f32 = 440.0;

pub fn clip_panel_h() -> f32 {
    CLIP_HEADER_H + CLIP_ROW_H * CLIP_VISIBLE_ROWS as f32 + CLIP_PAD
}

pub struct ClipArgs<'a> {
    pub settings: &'a DockSettings,
    pub entries: &'a [ClipboardEntry],
    pub filtered: &'a [usize],
    pub previews: &'a mut HashMap<usize, ScaledPreview>,
    pub query: &'a str,
    pub selected: usize,
    pub hovered: Option<usize>,
    pub scroll_y: f32,
    pub render_scale: f32,
    pub panel_w: f32,
    pub panel_h: f32,
}

#[allow(clippy::too_many_arguments)]
pub fn draw_clipboard(pixmap: &mut Pixmap, text_cache: &mut TextCache, args: ClipArgs) {
    let s = args.render_scale;
    let settings = args.settings;
    let w = args.panel_w * s;
    let h = args.panel_h * s;

    let bg = panel_bg(settings);
    let path = rounded_rect_path(0.0, 0.0, w, h, settings.corner_radius * s);
    let mut paint = tiny_skia::Paint::default();
    paint.set_color_rgba8(bg.0, bg.1, bg.2, bg.3);
    paint.anti_alias = true;
    pixmap.fill_path(&path, &paint, tiny_skia::FillRule::Winding, Transform::identity(), None);

    // ----- search box -----
    let box_x = CLIP_PAD * s;
    let box_w = (args.panel_w - CLIP_PAD * 2.0) * s;
    let box_h = (CLIP_HEADER_H - 10.0) * s;
    fill_rrect(pixmap, box_x, 5.0 * s, box_w, box_h, 6.0 * s, super::track_bg(settings));
    let ty = 5.0 * s + centered_text_y(box_h / s, 9.5) * s;
    let (query_text, qcolor): (&str, String) = if args.query.is_empty() {
        ("Search clipboard\u{2026}", text_dim_hex(settings))
    } else {
        (args.query, text_hex(settings))
    };
    draw_text(pixmap, text_cache, query_text, (CLIP_PAD + 6.0) * s, ty, 9.5 * s, &qcolor, if args.query.is_empty() { 400 } else { 500 });

    if args.filtered.is_empty() {
        let msg = if args.query.is_empty() { "Clipboard is empty" } else { "No matching clipboard items" };
        draw_text(pixmap, text_cache, msg, CLIP_PAD * s, (CLIP_HEADER_H + 24.0) * s, 10.0 * s, &text_dim_hex(settings), 400);
        return;
    }

    let accent = super::accent(settings);
    let title_c = text_hex(settings);
    let dim_c = text_dim_hex(settings);
    let on_accent = super::on_accent_hex(settings);

    let start = (args.scroll_y / CLIP_ROW_H).floor().max(0.0) as usize;
    let end = (start + CLIP_VISIBLE_ROWS + 1).min(args.filtered.len());
    for pos in start..end {
        let entry_index = args.filtered[pos];
        let Some(entry) = args.entries.get(entry_index) else {
            continue;
        };
        let row_y = (CLIP_HEADER_H + pos as f32 * CLIP_ROW_H - args.scroll_y) * s;
        if row_y + CLIP_ROW_H * s < CLIP_HEADER_H * s || row_y > h {
            continue;
        }
        let selected = pos == args.selected;
        let hovered = args.hovered == Some(pos);
        if selected || hovered {
            let fill = if selected { accent } else { (255, 255, 255, 22) };
            fill_rrect(pixmap, 4.0 * s, row_y + 3.0 * s, w - 8.0 * s, (CLIP_ROW_H - 6.0) * s, 7.0 * s, fill);
        }

        if let Some(thumbnail) = entry.thumbnail.clone() {
            let px = 12.0 * s;
            let py = row_y + 6.0 * s;
            let pw = (args.panel_w - 24.0) * s;
            let ph = (CLIP_ROW_H - 12.0) * s;
            let want_w = pw.round().max(1.0) as u32;
            let want_h = ph.round().max(1.0) as u32;
            let needs = args.previews.get(&entry_index).map(|p| p.width != want_w || p.height != want_h).unwrap_or(true);
            if needs {
                if let Some(scaled) = thumbnail.scale_to(want_w, want_h) {
                    args.previews.insert(entry_index, scaled);
                }
            }
            fill_rrect(pixmap, px, py, pw, ph, 4.0 * s, (0, 0, 0, 90));
            if let Some(preview) = args.previews.get(&entry_index) {
                blit_preview(pixmap, preview, px, py);
            }
        } else {
            let icon_x = 16.0 * s;
            fill_rrect(pixmap, icon_x, row_y + 10.0 * s, 34.0 * s, 34.0 * s, 8.0 * s, super::track_bg(settings));
            draw_text(pixmap, text_cache, "T", icon_x + 11.0 * s, row_y + 15.0 * s, 15.0 * s, if selected { &on_accent } else { &dim_c }, 700);

            let text_x = 62.0 * s;
            let title = fit(entry.title.trim(), &entry.title, 11.0, args.panel_w - 74.0);
            let desc = &entry.description;
            let tcol = if selected { &on_accent } else { &title_c };
            let dcol = if selected { &on_accent } else { &dim_c };
            draw_text(pixmap, text_cache, &title, text_x, row_y + 12.0 * s, 11.0 * s, tcol, 600);
            draw_text(pixmap, text_cache, desc, text_x, row_y + 30.0 * s, 8.0 * s, dcol, 400);
        }
    }

    // ----- scrollbar -----
    let total = args.filtered.len() as f32 * CLIP_ROW_H;
    let viewport = CLIP_ROW_H * CLIP_VISIBLE_ROWS as f32;
    if total > viewport {
        let track_x = w - 4.0 * s;
        let track_h = h - CLIP_HEADER_H * s - CLIP_PAD * s;
        let thumb_h = (track_h * (viewport / total)).max(20.0 * s);
        let max_scroll = total - viewport;
        let thumb_y = CLIP_HEADER_H * s + (track_h - thumb_h) * (args.scroll_y / max_scroll).clamp(0.0, 1.0);
        fill_rrect(pixmap, track_x, thumb_y, 3.0 * s, thumb_h, 1.5 * s, (accent.0, accent.1, accent.2, 150));
    }
}

fn blit_preview(pixmap: &mut Pixmap, preview: &ScaledPreview, x: f32, y: f32) {
    let mut src = match Pixmap::new(preview.width, preview.height) {
        Some(p) => p,
        None => return,
    };
    let data = src.data_mut();
    for (dst, chunk) in data.chunks_exact_mut(4).zip(preview.pixels.chunks_exact(4)) {
        let a = chunk[3] as u16;
        dst[0] = (chunk[0] as u16 * a / 255) as u8;
        dst[1] = (chunk[1] as u16 * a / 255) as u8;
        dst[2] = (chunk[2] as u16 * a / 255) as u8;
        dst[3] = chunk[3];
    }
    pixmap.draw_pixmap(0, 0, src.as_ref(), &PixmapPaint::default(), Transform::from_translate(x, y), None);
}

fn fit(display: &str, _full: &str, size: f32, max_w: f32) -> String {
    let approx = (max_w / (size * 0.55)) as usize;
    if display.chars().count() <= approx {
        return display.to_string();
    }
    let mut out: String = display.chars().take(approx.saturating_sub(1)).collect();
    out.push('\u{2026}');
    out
}
