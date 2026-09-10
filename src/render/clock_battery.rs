use super::*;

fn draw_battery_icon(pixmap: &mut Pixmap, render_scale: f32, bx: f32, by: f32, pct: u8, charging: bool, colors: &WidgetColors) -> (f32, f32) {
    let bw = 20.0 * render_scale;
    let bh = 10.0 * render_scale;

    let mut outline = Paint::default();
    outline.set_color_rgba8(colors.text_rgb.0, colors.text_rgb.1, colors.text_rgb.2, 200);
    outline.anti_alias = true;
    let path = rounded_rect_path(bx, by, bw, bh, 2.0 * render_scale);
    let stroke = tiny_skia::Stroke { width: 1.2 * render_scale, ..Default::default() };
    pixmap.stroke_path(&path, &outline, &stroke, Transform::identity(), None);
    let nub_w = 2.0 * render_scale;
    let nub_h = bh * 0.5;
    if let Some(rect) = Rect::from_xywh(bx + bw + 1.0 * render_scale, by + (bh - nub_h) / 2.0, nub_w, nub_h) {
        pixmap.fill_rect(rect, &outline, Transform::identity(), None);
    }

    let fill_color = if pct <= 20 {
        (255, 69, 58, 255)
    } else if charging {
        colors.accent
    } else {
        (colors.text_rgb.0, colors.text_rgb.1, colors.text_rgb.2, 200)
    };
    let inset = 2.0 * render_scale;
    let fill_w = ((bw - inset * 2.0) * (pct as f32 / 100.0)).max(0.0);
    let mut fill_paint = Paint::default();
    fill_paint.set_color_rgba8(fill_color.0, fill_color.1, fill_color.2, fill_color.3);
    fill_paint.anti_alias = true;
    if let Some(rect) = Rect::from_xywh(bx + inset, by + inset, fill_w, bh - inset * 2.0) {
        pixmap.fill_rect(rect, &fill_paint, Transform::identity(), None);
    }
    (bw, bh)
}

pub(super) fn draw_clock_widget(
    pixmap: &mut Pixmap,
    text_cache: &mut TextCache,
    widgets: &WidgetSnapshot,
    zx: f32,
    zy: f32,
    zw: f32,
    zh: f32,
    render_scale: f32,
    colors: &WidgetColors,
    is_vertical: bool,
) {
    if is_vertical {
        let time_size = 12.0 * render_scale;
        let date_size = 7.5 * render_scale;
        let clock_gap = 3.0 * render_scale;
        let time_px = text_cache.get(&widgets.time, time_size, colors.text_color, 700);
        let date_px = text_cache.get_with_family(&widgets.date, date_size, colors.text_color, 500, DATE_FONT_FAMILY);
        // ----- rotation swap -----
        let time_cross = time_px.as_ref().map(|p| p.height() as f32).unwrap_or(0.0);
        let date_cross = date_px.as_ref().map(|p| p.height() as f32).unwrap_or(0.0);
        let total_cross = time_cross + clock_gap + date_cross;
        let block_start = zx + (zw - total_cross) / 2.0;
        let cy = zy + zh / 2.0;
        let time_cx = block_start + time_cross / 2.0;
        let date_cx = block_start + time_cross + clock_gap + date_cross / 2.0;
        draw_text_rotated(pixmap, text_cache, &widgets.time, time_cx, cy, time_size, colors.text_color, 700);
        draw_text_rotated_family(pixmap, text_cache, &widgets.date, date_cx, cy, date_size, colors.text_color, 500, DATE_FONT_FAMILY);
    } else {
        let time_size = 13.0 * render_scale;
        let date_size = 8.0 * render_scale;
        let clock_gap = 3.5 * render_scale;
        let time_px = text_cache.get(&widgets.time, time_size, colors.text_color, 700);
        let date_px = text_cache.get_with_family(&widgets.date, date_size, colors.text_color, 500, DATE_FONT_FAMILY);
        let cx = zx + zw / 2.0;
        let cy = zy + zh / 2.0;
        if let Some(time_px) = time_px {
            let tx = cx - time_px.width() as f32 / 2.0;
            let ty = cy - time_px.height() as f32 * 0.55 - clock_gap / 2.0;
            pixmap.draw_pixmap(0, 0, time_px.as_ref().as_ref(), &tiny_skia::PixmapPaint::default(), Transform::from_translate(tx, ty), None);
        }
        if let Some(date_px) = date_px {
            let dx = cx - date_px.width() as f32 / 2.0;
            let dy = cy + date_px.height() as f32 * 0.05 + clock_gap / 2.0;
            pixmap.draw_pixmap(0, 0, date_px.as_ref().as_ref(), &tiny_skia::PixmapPaint::default(), Transform::from_translate(dx, dy), None);
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn draw_battery_widget(
    pixmap: &mut Pixmap,
    text_cache: &mut TextCache,
    widgets: &WidgetSnapshot,
    zx: f32,
    zy: f32,
    zw: f32,
    zh: f32,
    render_scale: f32,
    colors: &WidgetColors,
    is_vertical: bool,
) {
    let Some((pct, charging)) = widgets.battery else { return };
    let bw = 20.0 * render_scale;
    let bh = 10.0 * render_scale;
    let label = format!("{pct}%");
    if is_vertical {
        let label_len = text_width_estimate_render(&label, 8.5 * render_scale);
        let gap = 6.0 * render_scale;
        let total = bh + gap + label_len;
        let by = zy + (zh - total) / 2.0;
        let bx = zx + zw / 2.0 - bw / 2.0;
        draw_battery_icon(pixmap, render_scale, bx, by, pct, charging, colors);
        let ty = by + bh + gap + label_len / 2.0;
        draw_text_rotated(pixmap, text_cache, &label, zx + zw / 2.0, ty, 8.5 * render_scale, colors.text_color, 600);
    } else {
        let label_w = text_width_estimate_render(&label, 9.0 * render_scale);
        let gap = 8.0 * render_scale;
        let content_w = bw + gap + label_w;
        let bx = zx + (zw - content_w) / 2.0;
        let by = zy + zh / 2.0 - bh / 2.0;
        draw_battery_icon(pixmap, render_scale, bx, by, pct, charging, colors);
        if let Some(txt) = text_cache.get(&label, 9.0 * render_scale, colors.text_color, 600) {
            let tx = bx + bw + gap;
            let ty = zy + zh / 2.0 - txt.height() as f32 / 2.0;
            pixmap.draw_pixmap(0, 0, txt.as_ref().as_ref(), &tiny_skia::PixmapPaint::default(), Transform::from_translate(tx, ty), None);
        }
    }
}
