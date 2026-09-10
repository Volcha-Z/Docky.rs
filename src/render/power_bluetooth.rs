use super::*;

pub(super) fn draw_power_icon(pixmap: &mut Pixmap, render_scale: f32, cx: f32, cy: f32, colors: &WidgetColors) {
    let r = 7.0 * render_scale;
    let mut paint = Paint::default();
    paint.set_color_rgba8(colors.text_rgb.0, colors.text_rgb.1, colors.text_rgb.2, 230);
    paint.anti_alias = true;
    let stroke = tiny_skia::Stroke { width: 1.6 * render_scale, line_cap: tiny_skia::LineCap::Round, ..Default::default() };

    let gap = 27.0_f32.to_radians();
    let steps = 28;
    let mut ring = tiny_skia::PathBuilder::new();
    for i in 0..=steps {
        let t = gap + (std::f32::consts::TAU - 2.0 * gap) * (i as f32 / steps as f32);
        let x = cx + r * t.sin();
        let y = cy - r * t.cos();
        if i == 0 {
            ring.move_to(x, y);
        } else {
            ring.line_to(x, y);
        }
    }
    if let Some(path) = ring.finish() {
        pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
    }

    let mut nub = tiny_skia::PathBuilder::new();
    nub.move_to(cx, cy - r - 1.6 * render_scale);
    nub.line_to(cx, cy - r * 0.45);
    if let Some(path) = nub.finish() {
        pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
    }
}

pub(super) fn draw_power_widget(pixmap: &mut Pixmap, zx: f32, zy: f32, zw: f32, zh: f32, render_scale: f32, colors: &WidgetColors, _is_vertical: bool) {
    draw_power_icon(pixmap, render_scale, zx + zw / 2.0, zy + zh / 2.0, colors);
}

pub(super) fn draw_bluetooth_icon(pixmap: &mut Pixmap, render_scale: f32, cx: f32, cy: f32, active: bool, colors: &WidgetColors) {
    let s = 6.5 * render_scale;
    let mut paint = Paint::default();
    if active {
        paint.set_color_rgba8(colors.accent.0, colors.accent.1, colors.accent.2, 255);
    } else {
        paint.set_color_rgba8(colors.text_rgb.0, colors.text_rgb.1, colors.text_rgb.2, 130);
    }
    paint.anti_alias = true;
    let stroke = tiny_skia::Stroke { width: 1.4 * render_scale, ..Default::default() };
    let mut pb = tiny_skia::PathBuilder::new();
    pb.move_to(cx, cy - s);
    pb.line_to(cx + s * 0.6, cy - s * 0.4);
    pb.line_to(cx - s * 0.6, cy + s * 0.4);
    pb.line_to(cx, cy + s);
    pb.line_to(cx, cy - s);
    pb.move_to(cx - s * 0.6, cy - s * 0.4);
    pb.line_to(cx + s * 0.6, cy + s * 0.4);
    if let Some(path) = pb.finish() {
        pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn draw_bluetooth_widget(
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
    let (powered, label) = match &widgets.bluetooth {
        Some(bt) if !bt.powered => (false, "Off".to_string()),
        Some(bt) => (true, bt.connected.clone().unwrap_or_else(|| "On".to_string())),
        None => (false, String::new()),
    };
    let icon_r = 6.5 * render_scale;
    if is_vertical {
        let label_len = if label.is_empty() { 0.0 } else { text_width_estimate_render(&label, 8.0 * render_scale) };
        let gap = if label.is_empty() { 0.0 } else { 8.0 * render_scale };
        let total = icon_r * 2.0 + gap + label_len;
        let block_start = zy + (zh - total) / 2.0;
        let cx = zx + zw / 2.0;
        draw_bluetooth_icon(pixmap, render_scale, cx, block_start + icon_r, powered, colors);
        if !label.is_empty() {
            let ty = block_start + icon_r * 2.0 + gap + label_len / 2.0;
            draw_text_rotated(pixmap, text_cache, &label, cx, ty, 8.0 * render_scale, colors.text_dim_color, 500);
        }
    } else {
        let label_w = if label.is_empty() { 0.0 } else { text_width_estimate_render(&label, 9.0 * render_scale) };
        let gap = if label.is_empty() { 0.0 } else { 10.0 * render_scale };
        let content_w = icon_r * 2.0 + gap + label_w;
        let block_x = zx + (zw - content_w) / 2.0;
        let cy = zy + zh / 2.0;
        let icon_cx = block_x + icon_r;
        draw_bluetooth_icon(pixmap, render_scale, icon_cx, cy, powered, colors);
        if !label.is_empty() {
            if let Some(txt) = text_cache.get(&label, 9.0 * render_scale, colors.text_dim_color, 500) {
                let tx = icon_cx + icon_r + gap;
                let ty = cy - txt.height() as f32 / 2.0;
                pixmap.draw_pixmap(0, 0, txt.as_ref().as_ref(), &tiny_skia::PixmapPaint::default(), Transform::from_translate(tx, ty), None);
            }
        }
    }
}
