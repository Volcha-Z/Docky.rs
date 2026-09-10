use super::*;

pub struct NotificationArgs<'a> {
    pub title: &'a str,
    pub body: &'a str,
    pub panel_w: f32,
    pub panel_h: f32,
    pub dock: &'a Dock,
    pub render_scale: f32,
}

pub fn draw_notification(pixmap: &mut Pixmap, text_cache: &mut TextCache, args: &NotificationArgs) {
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

    if args.dock.is_vertical() {
        draw_notification_vertical(pixmap, text_cache, args, settings, w, h, acc);
    } else {
        draw_notification_horizontal(pixmap, text_cache, args, settings, w, h, acc);
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_notification_horizontal(
    pixmap: &mut Pixmap,
    text_cache: &mut TextCache,
    args: &NotificationArgs,
    settings: &crate::config::DockSettings,
    w: f32,
    h: f32,
    icon_color: (u8, u8, u8, u8),
) {
    let s = args.render_scale;
    let pad = MENU_PADDING * s;
    let icon_r = 9.0 * s;
    let icon_cx = pad + icon_r;
    let icon_cy = h / 2.0;
    draw_bell_icon(pixmap, icon_cx, icon_cy, icon_r, icon_color, s);

    let text_x = icon_cx + icon_r + 14.0 * s;
    let title_size = 9.5 * s;
    let body_size = 8.5 * s;
    let gap = 3.0 * s;
    let line_h = body_size * 1.4;
    let max_w = (w - text_x - pad).max(0.0);
    let title = truncate_to_width(args.title, title_size, max_w);
    let lines = wrap_to_width(args.body, body_size, max_w, crate::menu::NOTIFICATION_BODY_MAX_LINES);

    let block_h = title_size * 1.4 + gap + lines.len().max(1) as f32 * line_h;
    let top_y = (h - block_h) / 2.0;
    draw_text(pixmap, text_cache, &title, text_x, top_y, title_size, &text_hex(settings), 700);
    let mut y = top_y + title_size * 1.4 + gap;
    for line in &lines {
        draw_text(pixmap, text_cache, line, text_x, y, body_size, &text_dim_hex(settings), 500);
        y += line_h;
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_notification_vertical(
    pixmap: &mut Pixmap,
    text_cache: &mut TextCache,
    args: &NotificationArgs,
    settings: &crate::config::DockSettings,
    w: f32,
    h: f32,
    icon_color: (u8, u8, u8, u8),
) {
    let s = args.render_scale;
    let pad = MENU_PADDING * s;
    let icon_r = 9.0 * s;
    let icon_cx = w / 2.0;
    let icon_cy = pad + icon_r;
    draw_bell_icon(pixmap, icon_cx, icon_cy, icon_r, icon_color, s);

    let title_size = 9.5 * s;
    let body_size = 8.5 * s;
    let gap = 10.0 * s;
    let start_y = icon_cy + icon_r + 14.0 * s;
    let max_len = (h - pad - start_y).max(0.0);
    let half_len = ((max_len - gap) / 2.0).max(0.0);
    let title = truncate_to_width(args.title, title_size, half_len);
    let body = truncate_to_width(args.body, body_size, half_len);
    let title_len = text_width_estimate(&title, title_size) * s;
    let body_len = text_width_estimate(&body, body_size) * s;

    let title_cy = start_y + title_len / 2.0;
    let body_cy = start_y + title_len + gap + body_len / 2.0;
    draw_text_rotated(pixmap, text_cache, &title, w / 2.0, title_cy, title_size, &text_hex(settings), 700);
    draw_text_rotated(pixmap, text_cache, &body, w / 2.0, body_cy, body_size, &text_dim_hex(settings), 500);
}

pub(crate) fn draw_bell_icon(pixmap: &mut Pixmap, cx: f32, cy: f32, r: f32, color: (u8, u8, u8, u8), _s: f32) {
    let mut paint = Paint::default();
    paint.set_color_rgba8(color.0, color.1, color.2, color.3);
    paint.anti_alias = true;
    let mut pb = tiny_skia::PathBuilder::new();
    pb.move_to(cx - r * 0.75, cy + r * 0.35);
    pb.quad_to(cx - r * 0.75, cy - r * 0.15, cx - r * 0.55, cy - r * 0.55);
    pb.quad_to(cx - r * 0.3, cy - r * 0.95, cx, cy - r * 0.95);
    pb.quad_to(cx + r * 0.3, cy - r * 0.95, cx + r * 0.55, cy - r * 0.55);
    pb.quad_to(cx + r * 0.75, cy - r * 0.15, cx + r * 0.75, cy + r * 0.35);
    pb.line_to(cx + r * 0.95, cy + r * 0.35);
    pb.line_to(cx - r * 0.95, cy + r * 0.35);
    pb.close();
    if let Some(path) = pb.finish() {
        pixmap.fill_path(&path, &paint, tiny_skia::FillRule::Winding, Transform::identity(), None);
    }
    fill_circle(pixmap, cx, cy + r * 0.55, r * 0.15, color);
    fill_circle(pixmap, cx, cy - r * 1.0, r * 0.1, color);
}
