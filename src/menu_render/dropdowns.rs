use super::*;

pub(crate) fn current_scheme_label(settings: &crate::config::DockSettings) -> &'static str {
    crate::menu::MATUGEN_SCHEMES.iter().find(|(id, _)| *id == settings.matugen_scheme).map(|(_, label)| *label).unwrap_or("Tonal Spot")
}

pub(crate) fn draw_scheme_dropdown(pixmap: &mut Pixmap, text_cache: &mut TextCache, control: &Control, args: &DrawArgs, y: f32) {
    let s = args.render_scale;
    let settings = &args.dock.config.settings;
    let hot = matches!(args.hovered, Some(HitTarget::SchemeDropdownToggle));
    let (rx, ry, rw, rh) = (MENU_PADDING * s, y, (args.panel_width - MENU_PADDING * 2.0) * s, control.height * s);
    fill_rrect(pixmap, rx, ry, rw, rh, 6.0 * s, if hot { HOVER_SOFT } else { (255, 255, 255, 14) });
    let label = current_scheme_label(settings);
    let color = text_hex(settings);
    let ty = ry + centered_text_y(control.height, 9.0) * s;
    draw_text(pixmap, text_cache, label, (rx as f32) + 8.0 * s, ty, 9.0 * s, &color, 600);
    let chevron_x = rx + rw - 16.0 * s;
    draw_text(pixmap, text_cache, "\u{25BE}", chevron_x, ty, 9.0 * s, &color, 600);
}

pub(crate) fn draw_scheme_overlay(pixmap: &mut Pixmap, text_cache: &mut TextCache, start_y: f32, args: &DrawArgs) {
    use crate::menu::{scheme_picker_layout, MATUGEN_SCHEMES};
    let s = args.render_scale;
    let settings = &args.dock.config.settings;
    let layout = scheme_picker_layout(args.panel_width, start_y);
    let accent_c = accent(settings);
    let on_accent = on_accent_hex(settings);
    let text_c = text_hex(settings);

    let panel = panel_bg(settings);
    let bg_x = MENU_PADDING * s;
    let bg_w = (args.panel_width - MENU_PADDING * 2.0) * s;
    let bg_h = (layout.total_h - start_y + 6.0) * s;
    fill_rrect(pixmap, bg_x, start_y * s - 4.0 * s, bg_w, bg_h, 8.0 * s, (panel.0, panel.1, panel.2, 250));

    for r in &layout.rects {
        let (rx, ry, rw, rh) = (r.x * s, r.y * s, r.w * s, r.h * s);
        let hot = matches!(args.hovered, Some(HitTarget::SchemeOption(i)) if i == r.index);
        let selected = MATUGEN_SCHEMES.get(r.index).map(|(id, _)| *id == settings.matugen_scheme).unwrap_or(false);
        if selected {
            fill_rrect(pixmap, rx, ry, rw, rh, 6.0 * s, accent_c);
        } else if hot {
            fill_rrect(pixmap, rx, ry, rw, rh, 6.0 * s, HOVER_SOFT);
        } else {
            fill_rrect(pixmap, rx, ry, rw, rh, 6.0 * s, (255, 255, 255, 14));
        }
        let Some((_, label)) = MATUGEN_SCHEMES.get(r.index) else { continue };
        let color = if selected { &on_accent } else { &text_c };
        let tx = rx + rw / 2.0 - text_width_estimate(label, 8.5) * s / 2.0;
        let ty = ry + centered_text_y(r.h, 8.5) * s;
        draw_text(pixmap, text_cache, label, tx, ty, 8.5 * s, color, if selected { 700 } else { 500 });
    }
}

pub(crate) fn draw_font_dropdown(pixmap: &mut Pixmap, text_cache: &mut TextCache, control: &Control, args: &DrawArgs, y: f32, hot: bool, current_family: &str) {
    let s = args.render_scale;
    let settings = &args.dock.config.settings;
    let (rx, ry, rw, rh) = (MENU_PADDING * s, y, (args.panel_width - MENU_PADDING * 2.0) * s, control.height * s);
    fill_rrect(pixmap, rx, ry, rw, rh, 6.0 * s, if hot { HOVER_SOFT } else { (255, 255, 255, 14) });
    let max_chars = (((args.panel_width - MENU_PADDING * 2.0 - 34.0) / (9.0 * 0.56)).floor().max(4.0)) as usize;
    let label = if current_family.is_empty() { "System Default".to_string() } else { truncate_label(current_family, max_chars) };
    let color = text_hex(settings);
    let ty = ry + centered_text_y(control.height, 9.0) * s;
    draw_text(pixmap, text_cache, &label, rx + 8.0 * s, ty, 9.0 * s, &color, 600);
    let chevron_x = rx + rw - 16.0 * s;
    draw_text(pixmap, text_cache, "\u{25BE}", chevron_x, ty, 9.0 * s, &color, 600);
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_font_overlay(pixmap: &mut Pixmap, text_cache: &mut TextCache, start_y: f32, args: &DrawArgs, current_family: &str, scroll_rows: usize, query: &str, kb_selected: usize) {
    use crate::menu::{filter_font_indices, font_list_window, FONT_ROW_H};
    let s = args.render_scale;
    let settings = &args.dock.config.settings;
    let accent_c = accent(settings);
    let on_accent = on_accent_hex(settings);
    let text_c = text_hex(settings);

    let matches = filter_font_indices(args.available_fonts, query);
    let (first, visible) = font_list_window(matches.len(), scroll_rows);
    let total_h = (visible.max(1) as f32) * FONT_ROW_H;

    let panel = panel_bg(settings);
    let bg_x = MENU_PADDING * s;
    let bg_w = (args.panel_width - MENU_PADDING * 2.0) * s;
    let bg_h = (total_h + 6.0) * s;
    fill_rrect(pixmap, bg_x, start_y * s - 4.0 * s, bg_w, bg_h, 8.0 * s, (panel.0, panel.1, panel.2, 250));

    if matches.is_empty() {
        let ty = start_y * s + centered_text_y(FONT_ROW_H, 8.5) * s;
        draw_text(pixmap, text_cache, "No matches", (MENU_PADDING + 8.0) * s, ty, 8.5 * s, &text_dim_hex(settings), 500);
        return;
    }

    let row_w = args.panel_width - MENU_PADDING * 2.0;
    let max_chars = ((row_w - 16.0) / (8.5 * 0.56)).floor().max(4.0) as usize;

    for row in 0..visible {
        let idx = first + row;
        let Some(&global_idx) = matches.get(idx) else { continue };
        let Some(family) = args.available_fonts.get(global_idx) else { continue };
        let rx = MENU_PADDING * s;
        let ry = (start_y + row as f32 * FONT_ROW_H) * s;
        let rw = row_w * s;
        let rh = FONT_ROW_H * s;
        let hot = matches!(args.hovered, Some(HitTarget::FontOption(i)) if i == idx);
        let kb_hot = idx == kb_selected;
        let selected = family.as_str() == current_family;
        if selected {
            fill_rrect(pixmap, rx, ry, rw, rh, 6.0 * s, accent_c);
        } else if hot || kb_hot {
            fill_rrect(pixmap, rx, ry, rw, rh, 6.0 * s, HOVER_SOFT);
        }
        let label = truncate_label(family, max_chars);
        let color = if selected { &on_accent } else { &text_c };
        let ty = ry + centered_text_y(FONT_ROW_H, 8.5) * s;
        draw_text(pixmap, text_cache, &label, rx + 8.0 * s, ty, 8.5 * s, color, if selected { 700 } else { 500 });
    }
}
