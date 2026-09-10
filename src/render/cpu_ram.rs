use super::*;

pub(super) fn draw_cpu_icon(pixmap: &mut Pixmap, render_scale: f32, cx: f32, cy: f32, colors: &WidgetColors) {
    let s = 5.0 * render_scale;
    let mut paint = Paint::default();
    paint.set_color_rgba8(colors.text_rgb.0, colors.text_rgb.1, colors.text_rgb.2, 210);
    paint.anti_alias = true;
    let stroke = tiny_skia::Stroke { width: 1.3 * render_scale, ..Default::default() };
    let path = rounded_rect_path(cx - s, cy - s, s * 2.0, s * 2.0, 1.5 * render_scale);
    pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);

    let pin = s * 0.55;
    let mut pb = tiny_skia::PathBuilder::new();
    pb.move_to(cx, cy - s - pin);
    pb.line_to(cx, cy - s);
    pb.move_to(cx, cy + s);
    pb.line_to(cx, cy + s + pin);
    pb.move_to(cx - s - pin, cy);
    pb.line_to(cx - s, cy);
    pb.move_to(cx + s, cy);
    pb.line_to(cx + s + pin, cy);
    if let Some(path) = pb.finish() {
        pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
    }
}

pub(super) fn draw_ram_icon(pixmap: &mut Pixmap, render_scale: f32, cx: f32, cy: f32, colors: &WidgetColors) {
    let w = 11.0 * render_scale;
    let h = 7.0 * render_scale;
    let mut paint = Paint::default();
    paint.set_color_rgba8(colors.text_rgb.0, colors.text_rgb.1, colors.text_rgb.2, 210);
    paint.anti_alias = true;
    let stroke = tiny_skia::Stroke { width: 1.3 * render_scale, ..Default::default() };
    let path = rounded_rect_path(cx - w / 2.0, cy - h / 2.0, w, h, 1.5 * render_scale);
    pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);

    let notch_h = h * 0.35;
    let by = cy + h / 2.0;
    let mut pb = tiny_skia::PathBuilder::new();
    for i in 0..3 {
        let nx = cx - w / 2.0 + w * (i as f32 + 1.0) / 4.0;
        pb.move_to(nx, by);
        pb.line_to(nx, by + notch_h);
    }
    if let Some(path) = pb.finish() {
        pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn draw_percentage_widget(
    pixmap: &mut Pixmap,
    text_cache: &mut TextCache,
    pct: Option<u8>,
    draw_icon: fn(&mut Pixmap, f32, f32, f32, &WidgetColors),
    zx: f32,
    zy: f32,
    zw: f32,
    zh: f32,
    render_scale: f32,
    colors: &WidgetColors,
    is_vertical: bool,
) {
    let Some(pct) = pct else { return };
    let label = format!("{pct}%");
    let icon_side = 13.0 * render_scale;
    if is_vertical {
        let label_len = text_width_estimate_render(&label, 8.5 * render_scale);
        let gap = 6.0 * render_scale;
        let total = icon_side + gap + label_len;
        let block_start = zy + (zh - total) / 2.0;
        let cx = zx + zw / 2.0;
        draw_icon(pixmap, render_scale, cx, block_start + icon_side / 2.0, colors);
        let ty = block_start + icon_side + gap + label_len / 2.0;
        draw_text_rotated(pixmap, text_cache, &label, cx, ty, 8.5 * render_scale, colors.text_color, 600);
    } else {
        let label_w = text_width_estimate_render(&label, 9.0 * render_scale);
        let gap = 8.0 * render_scale;
        let content_w = icon_side + gap + label_w;
        let bx = zx + (zw - content_w) / 2.0;
        let cy = zy + zh / 2.0;
        draw_icon(pixmap, render_scale, bx + icon_side / 2.0, cy, colors);
        if let Some(txt) = text_cache.get(&label, 9.0 * render_scale, colors.text_color, 600) {
            let tx = bx + icon_side + gap;
            let ty = cy - txt.height() as f32 / 2.0;
            pixmap.draw_pixmap(0, 0, txt.as_ref().as_ref(), &tiny_skia::PixmapPaint::default(), Transform::from_translate(tx, ty), None);
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn draw_cpu_widget(
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
    draw_percentage_widget(pixmap, text_cache, widgets.cpu, draw_cpu_icon, zx, zy, zw, zh, render_scale, colors, is_vertical);
}

#[allow(clippy::too_many_arguments)]
pub(super) fn draw_ram_widget(
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
    draw_percentage_widget(pixmap, text_cache, widgets.ram, draw_ram_icon, zx, zy, zw, zh, render_scale, colors, is_vertical);
}
