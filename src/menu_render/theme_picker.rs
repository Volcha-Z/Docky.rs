use super::*;

pub(crate) fn draw_theme_picker(pixmap: &mut Pixmap, text_cache: &mut TextCache, control: &Control, args: &DrawArgs, y: f32) {
    use crate::menu::{palette_name, palette_swatches, theme_picker_layout, THEME_DOT_D};
    let s = args.render_scale;
    let settings = &args.dock.config.settings;
    let layout = theme_picker_layout(settings, args.panel_width, control.y);
    let accent_c = accent(settings);
    let on_accent = on_accent_hex(settings);
    let text_c = text_hex(settings);

    for r in &layout.rects {
        let (rx, ry, rw, rh) = (r.x * s, y + (r.y - control.y) * s, r.w * s, r.h * s);
        let hot = matches!(args.hovered, Some(HitTarget::ThemeOption(c)) if c == r.choice);
        let selected = match r.choice {
            None => settings.accent_from_wallpaper,
            Some(i) => {
                !settings.accent_from_wallpaper
                    && palette_swatches(settings, i).map(|sw| sw[0] == (settings.accent_r, settings.accent_g, settings.accent_b)).unwrap_or(false)
            }
        };
        if selected {
            fill_rrect(pixmap, rx, ry, rw, rh, 8.0 * s, accent_c);
        } else if hot {
            fill_rrect(pixmap, rx, ry, rw, rh, 8.0 * s, HOVER_SOFT);
        } else {
            fill_rrect(pixmap, rx, ry, rw, rh, 8.0 * s, (255, 255, 255, 14));
        }
        let color = if selected { &on_accent } else { &text_c };
        match r.choice {
            None => {
                let ty = ry + centered_text_y(r.h, 10.0) * s;
                draw_text(pixmap, text_cache, "Matugen", rx + 10.0 * s, ty, 10.0 * s, color, 700);
            }
            Some(i) => {
                let Some(name) = palette_name(settings, i) else { continue };
                let Some(swatches) = palette_swatches(settings, i) else { continue };
                let dot_r = THEME_DOT_D / 2.0 * s;
                let dot_gap = 3.0 * s;
                let dots_w = swatches.len() as f32 * (dot_r * 2.0) + (swatches.len() as f32 - 1.0) * dot_gap;
                // ----- label spacing -----
                let text_budget_px = (rw - 8.0 * s - dots_w - 10.0 * s) / s;
                let max_chars = (text_budget_px / (8.5 * 0.56)).floor().max(3.0) as usize;
                let label = truncate_label(name, max_chars);
                let ty = ry + centered_text_y(r.h, 8.5) * s;
                draw_text(pixmap, text_cache, &label, rx + 8.0 * s, ty, 8.5 * s, color, 600);
                let mut dx = rx + rw - 6.0 * s - dots_w + dot_r;
                let dy = ry + rh / 2.0;
                for &(dr, dg, db) in &swatches {
                    fill_rrect(pixmap, dx - dot_r, dy - dot_r, dot_r * 2.0, dot_r * 2.0, dot_r * 0.45, (dr, dg, db, 255));
                    dx += dot_r * 2.0 + dot_gap;
                }

                // ----- hover only -----
                let is_custom = i >= crate::menu::THEME_PRESETS.len();
                let custom_index = i.saturating_sub(crate::menu::THEME_PRESETS.len());
                let del_hot = matches!(args.hovered, Some(HitTarget::DeleteCustomPalette(h)) if h == custom_index);
                let tile_active = hot || del_hot;
                if is_custom && tile_active {
                    let (dbx, dby, dbw, dbh) = crate::menu::theme_delete_button_rect(r);
                    let (dbx, dby, dbw, dbh) = (dbx * s, y + (dby - control.y) * s, dbw * s, dbh * s);
                    let bg = if del_hot { accent(settings) } else { (0, 0, 0, 130) };
                    fill_rrect(pixmap, dbx, dby, dbw, dbh, 3.0 * s, bg);
                    let label = "Delete";
                    let label_color = if del_hot { on_accent_hex(settings) } else { "#ffffff".to_string() };
                    let label_ty = dby + centered_text_y(dbh / s.max(0.001), 7.0) * s;
                    let label_tx = dbx + dbw / 2.0 - text_width_estimate(label, 7.0) * s / 2.0;
                    draw_text(pixmap, text_cache, label, label_tx, label_ty, 7.0 * s, &label_color, 700);
                }
            }
        }
    }
}
pub(crate) fn draw_palette_mode_picker(pixmap: &mut Pixmap, text_cache: &mut TextCache, control: &Control, args: &DrawArgs, y: f32) {
    use crate::menu::{palette_mode_segment_rect, PALETTE_MODE_LABELS, PALETTE_MODE_PICKER_H};
    let s = args.render_scale;
    let settings = &args.dock.config.settings;
    let default_color = text_hex(settings);
    let selected_color = on_accent_hex(settings);
    for (i, &label) in PALETTE_MODE_LABELS.iter().enumerate() {
        let (seg_x, seg_w, _) = palette_mode_segment_rect(i, control.y, args.panel_width);
        let is_light_seg = i == 1;
        let selected = args.custom_light == is_light_seg;
        let hot = matches!(args.hovered, Some(HitTarget::PaletteMode(h)) if h == is_light_seg);
        let pad = 2.0;
        if selected || hot {
            let color = if selected { accent(settings) } else { HOVER_SOFT };
            fill_rrect(pixmap, (seg_x + pad) * s, y + pad * s, (seg_w - pad * 2.0) * s, (PALETTE_MODE_PICKER_H - pad * 2.0) * s, 6.0 * s, color);
        }
        let color = if selected { &selected_color } else { &default_color };
        let ty = y + centered_text_y(PALETTE_MODE_PICKER_H, 9.0) * s;
        let tx = (seg_x + seg_w / 2.0) * s - text_width_estimate(label, 9.0) * s / 2.0;
        draw_text(pixmap, text_cache, label, tx, ty, 9.0 * s, color, if selected { 700 } else { 500 });
    }
}
pub(crate) fn draw_align_picker(pixmap: &mut Pixmap, text_cache: &mut TextCache, control: &Control, args: &DrawArgs, y: f32) {
    use crate::menu::{align_segment_rect, ALIGN_LABELS, ALIGN_PICKER_H, ALIGN_VALUES};
    let s = args.render_scale;
    let settings = &args.dock.config.settings;
    let default_color = text_hex(settings);
    let selected_color = on_accent_hex(settings);
    for (i, &label) in ALIGN_LABELS.iter().enumerate() {
        let (seg_x, seg_w, _) = align_segment_rect(i, control.y, args.panel_width);
        let selected = settings.dock_align == ALIGN_VALUES[i];
        let hot = matches!(args.hovered, Some(HitTarget::Align(h)) if h == ALIGN_VALUES[i]);
        let pad = 2.0;
        if selected || hot {
            let color = if selected { accent(settings) } else { HOVER_SOFT };
            fill_rrect(pixmap, (seg_x + pad) * s, y + pad * s, (seg_w - pad * 2.0) * s, (ALIGN_PICKER_H - pad * 2.0) * s, 6.0 * s, color);
        }
        let color = if selected { &selected_color } else { &default_color };
        let ty = y + centered_text_y(ALIGN_PICKER_H, 9.0) * s;
        let tx = (seg_x + seg_w / 2.0) * s - text_width_estimate(label, 9.0) * s / 2.0;
        draw_text(pixmap, text_cache, label, tx, ty, 9.0 * s, color, if selected { 700 } else { 500 });
    }
}
pub(crate) fn draw_panel_blend_picker(pixmap: &mut Pixmap, text_cache: &mut TextCache, control: &Control, args: &DrawArgs, y: f32) {
    use crate::menu::{panel_blend_segment_rect, PANEL_BLEND_ACCENT_PCT, PANEL_BLEND_LABELS, PANEL_BLEND_PICKER_H};
    let s = args.render_scale;
    let settings = &args.dock.config.settings;
    let default_color = text_hex(settings);
    let selected_color = on_accent_hex(settings);
    for (i, &label) in PANEL_BLEND_LABELS.iter().enumerate() {
        let (seg_x, seg_w, _) = panel_blend_segment_rect(i, control.y, args.panel_width);
        let pct = PANEL_BLEND_ACCENT_PCT[i];
        let selected = args.custom_panel_blend == Some(pct);
        let hot = matches!(args.hovered, Some(HitTarget::PanelBlend(h)) if h == pct);
        let pad = 2.0;
        if selected || hot {
            let color = if selected { accent(settings) } else { HOVER_SOFT };
            fill_rrect(pixmap, (seg_x + pad) * s, y + pad * s, (seg_w - pad * 2.0) * s, (PANEL_BLEND_PICKER_H - pad * 2.0) * s, 6.0 * s, color);
        }
        let color = if selected { &selected_color } else { &default_color };
        let ty = y + centered_text_y(PANEL_BLEND_PICKER_H, 7.5) * s;
        let tx = (seg_x + seg_w / 2.0) * s - text_width_estimate(label, 7.5) * s / 2.0;
        draw_text(pixmap, text_cache, label, tx, ty, 7.5 * s, color, if selected { 700 } else { 500 });
    }
}
pub(crate) fn draw_name_field(pixmap: &mut Pixmap, text_cache: &mut TextCache, control: &Control, args: &DrawArgs, y: f32) {
    let s = args.render_scale;
    let settings = &args.dock.config.settings;
    let label = "Palette Name";
    let ty = y + centered_text_y(control.height, 9.0) * s;
    draw_text(pixmap, text_cache, label, MENU_PADDING * s, ty, 9.0 * s, &text_hex(settings), 600);

    let focused = args.custom_name_focused;
    let box_w = 120.0 * s;
    let box_h = (control.height - 6.0) * s;
    let box_x = (args.panel_width - MENU_PADDING) * s - box_w;
    let box_y = y + (control.height * s - box_h) / 2.0;
    let box_bg = if focused { accent2_soft(settings) } else { (255, 255, 255, 14) };
    fill_rrect(pixmap, box_x, box_y, box_w, box_h, 5.0 * s, box_bg);

    let text_x = box_x + 6.0 * s;
    let max_chars = ((box_w / s - 12.0) / (8.5 * 0.56)).floor().max(3.0) as usize;
    let (display, color_owned) = if args.custom_name.is_empty() {
        (truncate_label("My Palette", max_chars), text_dim_hex(settings))
    } else {
        (truncate_label(args.custom_name, max_chars), text_hex(settings))
    };
    let text_ty = box_y + centered_text_y(box_h / s.max(0.001), 8.5) * s;
    draw_text(pixmap, text_cache, &display, text_x, text_ty, 8.5 * s, &color_owned, 500);

    if focused {
        let shown = if args.custom_name.is_empty() { "" } else { &display };
        let cursor_x = text_x + text_width_estimate(shown, 8.5) * s + 1.0 * s;
        fill_rrect(pixmap, cursor_x, box_y + 4.0 * s, 1.2 * s, box_h - 8.0 * s, 0.6 * s, accent(settings));
    }
}
pub(crate) fn draw_hex_field(pixmap: &mut Pixmap, text_cache: &mut TextCache, control: &Control, args: &DrawArgs, y: f32, index: usize) {
    let s = args.render_scale;
    let settings = &args.dock.config.settings;
    let label = crate::menu::CUSTOM_HEX_LABELS.get(index).copied().unwrap_or("");
    let ty = y + centered_text_y(control.height, 9.0) * s;
    draw_text(pixmap, text_cache, label, MENU_PADDING * s, ty, 9.0 * s, &text_hex(settings), 600);

    let focused = args.custom_focus == Some(index);
    let hex = args.custom_hex.get(index).map(|v| v.as_str()).unwrap_or("");

    let box_w = 84.0 * s;
    let box_h = (control.height - 6.0) * s;
    let box_x = (args.panel_width - MENU_PADDING) * s - box_w;
    let box_y = y + (control.height * s - box_h) / 2.0;
    let box_bg = if focused { accent2_soft(settings) } else { (255, 255, 255, 14) };
    fill_rrect(pixmap, box_x, box_y, box_w, box_h, 5.0 * s, box_bg);

    let swatch_d = box_h - 8.0 * s;
    let swatch_x = box_x + 5.0 * s;
    let swatch_y = box_y + (box_h - swatch_d) / 2.0;
    let swatch_rgb = crate::menu::parse_hex(hex).unwrap_or((90, 90, 98));
    fill_rrect(pixmap, swatch_x, swatch_y, swatch_d, swatch_d, 3.0 * s, (swatch_rgb.0, swatch_rgb.1, swatch_rgb.2, 255));

    let text_x = swatch_x + swatch_d + 6.0 * s;
    let display = format!("#{hex}");
    let text_ty = box_y + centered_text_y(box_h / s.max(0.001), 8.5) * s;
    draw_text(pixmap, text_cache, &display, text_x, text_ty, 8.5 * s, &text_hex(settings), 500);

    if focused {
        let cursor_x = text_x + text_width_estimate(&display, 8.5) * s + 1.0 * s;
        fill_rrect(pixmap, cursor_x, box_y + 4.0 * s, 1.2 * s, box_h - 8.0 * s, 0.6 * s, accent(settings));
    }
}
