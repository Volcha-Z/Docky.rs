use super::*;

pub fn draw_app_search(pixmap: &mut Pixmap, icon_cache: &mut IconCache, text_cache: &mut TextCache, args: &DrawArgs, highlight_x: f32, content_anim: f32) {
    let s = args.render_scale;
    let w = args.panel_width * s;
    let h = args.content_height * s;
    let settings = &args.dock.config.settings;

    let bg = panel_bg(settings);
    let path = rounded_rect_path(0.0, 0.0, w, h, settings.corner_radius * s);
    let mut paint = Paint::default();
    paint.set_color_rgba8(bg.0, bg.1, bg.2, bg.3);
    paint.anti_alias = true;
    pixmap.fill_path(&path, &paint, tiny_skia::FillRule::Winding, Transform::identity(), None);

    draw_control_rows(pixmap, icon_cache, text_cache, args.controls, args);
    draw_app_search_strip(pixmap, icon_cache, text_cache, args, highlight_x, content_anim);
}

pub(crate) fn draw_app_search_strip(
    pixmap: &mut Pixmap,
    icon_cache: &mut IconCache,
    text_cache: &mut TextCache,
    args: &DrawArgs,
    highlight_along: f32,
    content_anim: f32,
) {
    if args.dock.is_vertical() {
        draw_app_search_strip_vertical(pixmap, icon_cache, text_cache, args, highlight_along, content_anim);
    } else {
        draw_app_search_strip_horizontal(pixmap, icon_cache, text_cache, args, highlight_along, content_anim);
    }
}

pub(crate) fn draw_app_search_strip_horizontal(
    pixmap: &mut Pixmap,
    icon_cache: &mut IconCache,
    text_cache: &mut TextCache,
    args: &DrawArgs,
    highlight_along: f32,
    content_anim: f32,
) {
    use crate::menu::{app_search_card_along, app_search_strip_y, APP_CARD_ICON, APP_CARD_ROW_H, APP_CARD_W};
    let s = args.render_scale;
    let settings = &args.dock.config.settings;
    let row_y = app_search_strip_y() * s;
    let row_h = APP_CARD_ROW_H * s;
    let viewport_w = args.panel_width * s;
    let scroll = args.wallpaper_scroll_x * s;

    if args.app_entries.is_empty() {
        let ty = row_y + centered_text_y(APP_CARD_ROW_H, 9.0) * s;
        draw_text(pixmap, text_cache, "No matching apps", MENU_PADDING * s, ty, 9.0 * s, &text_dim_hex(settings), 500);
        return;
    }

    let mut strip = Pixmap::new(viewport_w.round().max(1.0) as u32, row_h.round().max(1.0) as u32).unwrap();
    let hl_x = highlight_along * s - scroll;
    fill_rrect(&mut strip, hl_x, 0.0, APP_CARD_W * s, row_h, 8.0 * s, accent(settings));

    let default_color = text_hex(settings);
    for (i, entry) in args.app_entries.iter().enumerate() {
        let cx0 = app_search_card_along(i) * s - scroll;
        if cx0 + APP_CARD_W * s < 0.0 || cx0 > viewport_w {
            continue;
        }
        let hot = matches!(args.hovered, Some(HitTarget::AppEntry(hi)) if hi == i);
        let icon_size = APP_CARD_ICON * s;
        let icon_x = cx0 + (APP_CARD_W * s - icon_size) / 2.0;
        let icon_y = 6.0 * s;
        if let Some(icon_pixmap) = icon_cache.get(&entry.icon, icon_size.max(1.0) as u32) {
            let scale = icon_size / icon_pixmap.width() as f32;
            let paint = tiny_skia::PixmapPaint::default();
            strip.draw_pixmap(0, 0, icon_pixmap.as_ref().as_ref(), &paint, Transform::from_translate(icon_x, icon_y).pre_scale(scale, scale), None);
        }
        let hot_color;
        let color: &str = if hot {
            hot_color = on_accent_hex(settings);
            &hot_color
        } else {
            &default_color
        };
        let label = truncate_label(&entry.name, 13);
        let label_w = text_width_estimate(&label, 7.5) * s;
        let label_x = cx0 + (APP_CARD_W * s - label_w) / 2.0;
        let label_y = icon_y + icon_size + 4.0 * s;
        draw_text(&mut strip, text_cache, &label, label_x, label_y, 7.5 * s, color, 500);
    }

    let strip_paint = tiny_skia::PixmapPaint { opacity: content_anim, ..Default::default() };
    pixmap.draw_pixmap(0, 0, strip.as_ref(), &strip_paint, Transform::from_translate(0.0, row_y), None);
}

pub(crate) fn draw_app_search_strip_vertical(
    pixmap: &mut Pixmap,
    icon_cache: &mut IconCache,
    text_cache: &mut TextCache,
    args: &DrawArgs,
    highlight_along: f32,
    content_anim: f32,
) {
    use crate::menu::{app_search_card_along, app_search_strip_y, APP_CARD_ICON, APP_CARD_W, MENU_PADDING};
    let s = args.render_scale;
    let settings = &args.dock.config.settings;
    let row_y0 = app_search_strip_y() * s;
    let panel_w_px = args.panel_width * s;
    let card_w = (panel_w_px - MENU_PADDING * 2.0 * s).max(1.0);
    let card_h = APP_CARD_W * s;
    let viewport_h = (args.content_height * s - row_y0).max(1.0);
    let scroll = args.wallpaper_scroll_x * s;

    if args.app_entries.is_empty() {
        let ty = row_y0 + 14.0 * s;
        draw_text(pixmap, text_cache, "No matches", MENU_PADDING * s, ty, 8.0 * s, &text_dim_hex(settings), 500);
        return;
    }

    let mut strip = Pixmap::new(panel_w_px.round().max(1.0) as u32, viewport_h.round().max(1.0) as u32).unwrap();
    let card_x = (panel_w_px - card_w) / 2.0;
    let hl_y = highlight_along * s - scroll;
    fill_rrect(&mut strip, card_x, hl_y, card_w, card_h, 8.0 * s, accent(settings));

    let icon_size = APP_CARD_ICON.min(card_w / s - 8.0).max(10.0) * s;
    let max_chars = (((card_w / s - 4.0) / (7.5 * 0.56)).floor().max(3.0)) as usize;
    let default_color = text_hex(settings);
    for (i, entry) in args.app_entries.iter().enumerate() {
        let cy0 = app_search_card_along(i) * s - scroll;
        if cy0 + card_h < 0.0 || cy0 > viewport_h {
            continue;
        }
        let hot = matches!(args.hovered, Some(HitTarget::AppEntry(hi)) if hi == i);
        let icon_x = card_x + (card_w - icon_size) / 2.0;
        let icon_y = cy0 + 6.0 * s;
        if let Some(icon_pixmap) = icon_cache.get(&entry.icon, icon_size.max(1.0) as u32) {
            let scale = icon_size / icon_pixmap.width() as f32;
            let paint = tiny_skia::PixmapPaint::default();
            strip.draw_pixmap(0, 0, icon_pixmap.as_ref().as_ref(), &paint, Transform::from_translate(icon_x, icon_y).pre_scale(scale, scale), None);
        }
        let hot_color;
        let color: &str = if hot {
            hot_color = on_accent_hex(settings);
            &hot_color
        } else {
            &default_color
        };
        let label = truncate_label(&entry.name, max_chars);
        let label_w = text_width_estimate(&label, 7.5) * s;
        let label_x = card_x + (card_w - label_w) / 2.0;
        let label_y = icon_y + icon_size + 4.0 * s;
        draw_text(&mut strip, text_cache, &label, label_x, label_y, 7.5 * s, color, 500);
    }

    let strip_paint = tiny_skia::PixmapPaint { opacity: content_anim, ..Default::default() };
    pixmap.draw_pixmap(0, 0, strip.as_ref(), &strip_paint, Transform::from_translate(0.0, row_y0), None);
}
