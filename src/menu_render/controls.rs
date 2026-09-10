use super::*;

pub(crate) fn draw_widget_chips(pixmap: &mut Pixmap, text_cache: &mut TextCache, control: &Control, args: &DrawArgs, _y: f32) {
    use crate::menu::{widget_chip_layout, widget_label, WIDGET_COL_GAP, WIDGET_GROUP_LABEL_H, WIDGET_KIND_ORDER};
    let s = args.render_scale;
    let settings = &args.dock.config.settings;
    let layout = widget_chip_layout(settings, args.panel_width, control.y);

    let label_color = text_dim_hex(settings);
    let avail_ty = layout.available_label_y * s + centered_text_y(WIDGET_GROUP_LABEL_H, 9.0) * s;
    draw_text(pixmap, text_cache, "Available", MENU_PADDING * s, avail_ty, 9.0 * s, &label_color, 600);
    let cols_ty = layout.columns_label_y * s + centered_text_y(WIDGET_GROUP_LABEL_H, 9.0) * s;
    for (i, name) in ["Left", "Middle", "Right"].iter().enumerate() {
        let cx = (MENU_PADDING + i as f32 * (layout.col_w + WIDGET_COL_GAP)) * s;
        draw_text(pixmap, text_cache, name, cx, cols_ty, 9.0 * s, &label_color, 600);
    }

    let accent_c = accent(settings);
    let on_accent = on_accent_hex(settings);
    let text_c = text_hex(settings);
    for kind in WIDGET_KIND_ORDER {
        let Some(r) = layout.rects.iter().find(|r| r.kind == kind) else { continue };
        let hot = matches!(args.hovered, Some(HitTarget::WidgetChip(hk)) if hk == kind);
        let (rx, ry, rw, rh) = (r.x * s, r.y * s, r.w * s, r.h * s);
        if r.assigned {
            let fill = if hot { (accent_c.0, accent_c.1, accent_c.2, 210) } else { accent_c };
            fill_rrect(pixmap, rx, ry, rw, rh, rh / 2.0, fill);
        } else {
            let bg = if hot { (255, 255, 255, 30) } else { (255, 255, 255, 14) };
            fill_rrect(pixmap, rx, ry, rw, rh, rh / 2.0, bg);
        }
        let label = widget_label(kind);
        let color = if r.assigned { &on_accent } else { &text_c };
        let tx = rx + rw / 2.0 - text_width_estimate(label, 8.5) * s / 2.0;
        let ty = ry + centered_text_y(r.h, 8.5) * s;
        draw_text(pixmap, text_cache, label, tx, ty, 8.5 * s, color, 600);
    }
}
pub(crate) fn draw_slider(pixmap: &mut Pixmap, text_cache: &mut TextCache, control: &Control, id: crate::menu::SettingId, args: &DrawArgs, y: f32) {
    let s = args.render_scale;
    let settings = &args.dock.config.settings;
    draw_text(pixmap, text_cache, id.label(), MENU_PADDING * s, y, 9.0 * s, &text_hex(settings), 600);
    let value_text = id.display_value(settings);
    draw_value_rtl(pixmap, text_cache, &value_text, (args.panel_width - MENU_PADDING) * s, y, 9.0 * s, &text_dim_hex(settings), 500);

    let (minus_x, track_x0, track_x1, plus_x, track_y) = crate::menu::slider_geometry(control.y);
    let minus_hot = matches!(args.hovered, Some(HitTarget::SliderMinus(h)) if h == id);
    let plus_hot = matches!(args.hovered, Some(HitTarget::SliderPlus(h)) if h == id);
    let track_hot = matches!(args.hovered, Some(HitTarget::SliderTrack(h)) if h == id);
    let btn_cy = (track_y + STEP_BTN_SIZE / 2.0) * s;

    if minus_hot {
        fill_circle(pixmap, (minus_x + STEP_BTN_SIZE / 2.0) * s, btn_cy, STEP_BTN_SIZE * 0.5 * s, HOVER_SOFT);
    }
    draw_text(pixmap, text_cache, "\u{2212}", (minus_x + 5.5) * s, (track_y + 11.5) * s, 11.5 * s, &text_dim_hex(settings), 600);
    if plus_hot {
        fill_circle(pixmap, (plus_x + STEP_BTN_SIZE / 2.0) * s, btn_cy, STEP_BTN_SIZE * 0.5 * s, HOVER_SOFT);
    }
    draw_text(pixmap, text_cache, "+", (plus_x + 4.0) * s, (track_y + 11.5) * s, 11.5 * s, &text_dim_hex(settings), 600);

    let track_cy = btn_cy;
    let track_h = 3.0 * s;
    fill_rrect(pixmap, track_x0 * s, track_cy - track_h / 2.0, (track_x1 - track_x0) * s, track_h, track_h / 2.0, track_bg(settings));

    let (min, max, _) = id.range();
    let t = ((id.get(settings) - min) / (max - min).max(0.0001)).clamp(0.0, 1.0);
    let fill_w = (track_x1 - track_x0) * s * t;
    fill_rrect(pixmap, track_x0 * s, track_cy - track_h / 2.0, fill_w, track_h, track_h / 2.0, accent(settings));

    let thumb_x = track_x0 * s + fill_w;
    let thumb_r = if track_hot { 6.5 * s } else { 5.5 * s };
    fill_circle(pixmap, thumb_x, track_cy, thumb_r + 1.0 * s, KNOB_BORDER);
    fill_circle(pixmap, thumb_x, track_cy, thumb_r, (255, 255, 255, 255));
}
pub(crate) fn draw_toggle(pixmap: &mut Pixmap, text_cache: &mut TextCache, control: &Control, id: crate::menu::SettingId, args: &DrawArgs, y: f32) {
    let s = args.render_scale;
    let settings = &args.dock.config.settings;
    let ty = y + centered_text_y(control.height, 9.5) * s;
    draw_text(pixmap, text_cache, id.label(), MENU_PADDING * s, ty, 9.5 * s, &text_hex(settings), 600);

    let on = id.get(settings) >= 0.5;
    let pill_w = 30.0 * s;
    let pill_h = 16.0 * s;
    let pill_x = (args.panel_width - MENU_PADDING) * s - pill_w;
    let pill_y = y + (control.height * s - pill_h) / 2.0;
    fill_rrect(pixmap, pill_x, pill_y, pill_w, pill_h, pill_h / 2.0, if on { accent(settings) } else { track_bg(settings) });

    let knob_r = (pill_h / 2.0) - 2.0 * s;
    let knob_x = if on { pill_x + pill_w - pill_h / 2.0 } else { pill_x + pill_h / 2.0 };
    let knob_y = pill_y + pill_h / 2.0;
    fill_circle(pixmap, knob_x, knob_y, knob_r + 0.75 * s, KNOB_BORDER);
    fill_circle(pixmap, knob_x, knob_y, knob_r, (255, 255, 255, 255));
}
pub(crate) fn draw_button(pixmap: &mut Pixmap, text_cache: &mut TextCache, control: &Control, kind: ButtonKind, args: &DrawArgs, y: f32) {
    let s = args.render_scale;
    let settings = &args.dock.config.settings;
    let hot = matches!(args.hovered, Some(HitTarget::Button(k)) if k == kind);
    if hot {
        fill_rrect(pixmap, MENU_PADDING * 0.5 * s, y, (args.panel_width - MENU_PADDING) * s, control.height * s, 6.0 * s, accent(settings));
    }
    let color = if hot {
        on_accent_hex(settings)
    } else if kind.is_destructive() {
        accent_hex(settings)
    } else {
        text_hex(settings)
    };
    let ty = y + centered_text_y(control.height, 10.5) * s;
    draw_text(pixmap, text_cache, kind.label(), (MENU_PADDING + 6.0) * s, ty, 10.5 * s, &color, 500);
}
pub(crate) fn draw_app_entry(
    pixmap: &mut Pixmap,
    icon_cache: &mut IconCache,
    text_cache: &mut TextCache,
    control: &Control,
    index: usize,
    args: &DrawArgs,
    y: f32,
) {
    let s = args.render_scale;
    let settings = &args.dock.config.settings;
    let Some(entry) = args.app_entries.get(index) else {
        return;
    };
    let hot = matches!(args.hovered, Some(HitTarget::AppEntry(i)) if i == index);
    if hot {
        fill_rrect(pixmap, MENU_PADDING * 0.5 * s, y, (args.panel_width - MENU_PADDING) * s, control.height * s, 6.0 * s, accent(settings));
    }

    let icon_size = (control.height - 8.0) * s;
    let icon_x = (MENU_PADDING + 2.0) * s;
    let icon_y = y + 4.0 * s;
    if let Some(icon_pixmap) = icon_cache.get(&entry.icon, icon_size.max(1.0) as u32) {
        let scale = icon_size / icon_pixmap.width() as f32;
        let paint = tiny_skia::PixmapPaint::default();
        pixmap.draw_pixmap(0, 0, icon_pixmap.as_ref().as_ref(), &paint, Transform::from_translate(icon_x, icon_y).pre_scale(scale, scale), None);
    }

    let color = if hot { on_accent_hex(settings) } else { text_hex(settings) };
    let ty = y + centered_text_y(control.height, 9.5) * s;
    draw_text(pixmap, text_cache, &entry.name, (MENU_PADDING + 2.0) * s + icon_size + 8.0, ty, 9.5 * s, &color, 500);
}
pub(crate) fn draw_search_box(pixmap: &mut Pixmap, text_cache: &mut TextCache, control: &Control, args: &DrawArgs, y: f32) {
    let s = args.render_scale;
    let settings = &args.dock.config.settings;
    let hot = matches!(args.hovered, Some(HitTarget::SearchBox));
    let box_color = if hot { (255, 255, 255, 30) } else { track_bg(settings) };
    let box_x = MENU_PADDING * s;
    let box_w = (args.panel_width - MENU_PADDING * 2.0) * s;
    fill_rrect(pixmap, box_x, y, box_w, control.height * s, 6.0 * s, box_color);

    let ty = y + centered_text_y(control.height, 9.5) * s;
    let (text, color, weight) = if args.search_query.is_empty() {
        let placeholder = match args.screen {
            MenuScreen::IconPicker(_) => "Search icons\u{2026}",
            MenuScreen::AddApp => "Search apps\u{2026}",
            _ => "Search\u{2026}",
        };
        (placeholder, text_dim_hex(settings), 400)
    } else {
        (args.search_query, text_hex(settings), 500)
    };
    if let Some(glyphs) = text_cache.get(text, 9.5 * s, &color, weight) {
        let text_w = glyphs.width() as f32;
        // ----- overflow guard -----
        let tx = (box_x + (box_w - text_w) / 2.0).max(box_x + 2.0 * s);
        let paint = tiny_skia::PixmapPaint::default();
        pixmap.draw_pixmap(0, 0, glyphs.as_ref().as_ref(), &paint, Transform::from_translate(tx, ty), None);
    }
}
pub(crate) fn draw_icon_choice(
    pixmap: &mut Pixmap,
    icon_cache: &mut IconCache,
    text_cache: &mut TextCache,
    control: &Control,
    index: usize,
    args: &DrawArgs,
    y: f32,
) {
    let s = args.render_scale;
    let settings = &args.dock.config.settings;
    let Some(name) = args.icon_choices.get(index) else {
        return;
    };
    let hot = matches!(args.hovered, Some(HitTarget::IconChoice(i)) if i == index);
    if hot {
        fill_rrect(pixmap, MENU_PADDING * 0.5 * s, y, (args.panel_width - MENU_PADDING) * s, control.height * s, 6.0 * s, accent(settings));
    }

    let icon_size = (control.height - 8.0) * s;
    let icon_x = (MENU_PADDING + 2.0) * s;
    let icon_y = y + 4.0 * s;
    if let Some(icon_pixmap) = icon_cache.get(name, icon_size.max(1.0) as u32) {
        let scale = icon_size / icon_pixmap.width() as f32;
        let paint = tiny_skia::PixmapPaint::default();
        pixmap.draw_pixmap(0, 0, icon_pixmap.as_ref().as_ref(), &paint, Transform::from_translate(icon_x, icon_y).pre_scale(scale, scale), None);
    }

    let color = if hot { on_accent_hex(settings) } else { text_hex(settings) };
    let ty = y + centered_text_y(control.height, 9.5) * s;
    draw_text(pixmap, text_cache, name, (MENU_PADDING + 2.0) * s + icon_size + 8.0, ty, 9.5 * s, &color, 500);
}
pub(crate) fn draw_tray_item(pixmap: &mut Pixmap, text_cache: &mut TextCache, control: &Control, index: usize, args: &DrawArgs, y: f32) {
    let s = args.render_scale;
    let settings = &args.dock.config.settings;
    let Some(item) = args.tray_items.get(index) else { return };
    let hot = item.enabled && matches!(args.hovered, Some(HitTarget::TrayItem(hi)) if hi == index);
    if hot {
        fill_rrect(pixmap, MENU_PADDING * 0.5 * s, y, (args.panel_width - MENU_PADDING) * s, control.height * s, 5.0 * s, HOVER_SOFT);
    }
    let color = if item.enabled { text_hex(settings) } else { text_dim_hex(settings) };
    let ty = y + centered_text_y(control.height, 9.0) * s;
    draw_text(pixmap, text_cache, &item.label, MENU_PADDING * s, ty, 9.0 * s, &color, 500);
    if item.has_submenu {
        let chevron_w = text_width_estimate("\u{203a}", 10.0) * s;
        let cx = (args.panel_width - MENU_PADDING) * s - chevron_w;
        draw_text(pixmap, text_cache, "\u{203a}", cx, ty, 10.0 * s, &color, 600);
    }
}
pub(crate) fn draw_tray_separator(pixmap: &mut Pixmap, control: &Control, args: &DrawArgs, y: f32) {
    let s = args.render_scale;
    let cy = y + control.height * 0.5 * s;
    let x0 = MENU_PADDING * 0.6 * s;
    let x1 = (args.panel_width - MENU_PADDING * 0.6) * s;
    let mut paint = Paint::default();
    paint.set_color_rgba8(255, 255, 255, 26);
    paint.anti_alias = true;
    let mut pb = tiny_skia::PathBuilder::new();
    pb.move_to(x0, cy);
    pb.line_to(x1, cy);
    if let Some(path) = pb.finish() {
        let stroke = tiny_skia::Stroke { width: 1.0 * s, ..Default::default() };
        pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
    }
}
