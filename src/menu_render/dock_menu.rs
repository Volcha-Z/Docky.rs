use super::*;

pub struct DockMenuArgs<'a> {
    pub category: crate::menu::MenuCategory,
    pub controls: &'a [Control],
    pub panel_w: f32,
    pub panel_h: f32,
    pub slide_offset: f32,
    pub dock: &'a Dock,
    pub hovered: Option<HitTarget>,
    pub render_scale: f32,
    pub dragging_widget: Option<(crate::config::WidgetKind, f32, f32)>,
    pub open_dropdown: crate::menu::OpenDropdown,
    pub dropdown_anim: f32,
    pub dropdown_scroll: usize,
    pub available_fonts: &'a [String],
    pub font_query: &'a str,
    pub dropdown_selected: usize,
    pub custom_hex: &'a [String; 5],
    pub custom_light: bool,
    pub custom_focus: Option<usize>,
    pub custom_name: &'a str,
    pub custom_name_focused: bool,
    pub custom_panel_blend: Option<u8>,
}
pub fn draw_dock_menu(pixmap: &mut Pixmap, icon_cache: &mut IconCache, text_cache: &mut TextCache, args: &DockMenuArgs) {
    use crate::menu::{DOCK_MENU_DIVIDER_W, DOCK_MENU_LEFT_COL_W, DOCK_MENU_TAB_H, MENU_CATEGORIES};
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

    let default_color = text_hex(settings);
    let selected_color = on_accent_hex(settings);
    for (i, category) in MENU_CATEGORIES.iter().enumerate() {
        let ty = MENU_PADDING + i as f32 * DOCK_MENU_TAB_H;
        let selected = *category == args.category;
        let hot = matches!(args.hovered, Some(HitTarget::Tab(c)) if c == *category);
        if selected || hot {
            let bg_color = if selected { accent(settings) } else { accent2_soft(settings) };
            fill_rrect(pixmap, 6.0 * s, ty * s, (DOCK_MENU_LEFT_COL_W - 12.0) * s, (DOCK_MENU_TAB_H - 4.0) * s, 6.0 * s, bg_color);
        }
        let color = if selected { &selected_color } else { &default_color };
        let tty = ty * s + centered_text_y(DOCK_MENU_TAB_H - 4.0, 10.0) * s;
        draw_text(pixmap, text_cache, category.label(), 14.0 * s, tty, 10.0 * s, color, if selected { 700 } else { 500 });
    }

    let mut divider = Paint::default();
    divider.set_color_rgba8(HAIRLINE.0, HAIRLINE.1, HAIRLINE.2, HAIRLINE.3);
    if let Some(r) = Rect::from_xywh(DOCK_MENU_LEFT_COL_W * s, 0.0, DOCK_MENU_DIVIDER_W * s, h) {
        pixmap.fill_rect(r, &divider, Transform::identity(), None);
    }

    let right_x = (DOCK_MENU_LEFT_COL_W + DOCK_MENU_DIVIDER_W) * s;
    let right_w = args.panel_w - DOCK_MENU_LEFT_COL_W - DOCK_MENU_DIVIDER_W;
    let mut right_pixmap = Pixmap::new((right_w * s).round().max(1.0) as u32, h.round().max(1.0) as u32).unwrap();
    let right_hovered = match args.hovered {
        Some(HitTarget::Tab(_)) => None,
        other => other,
    };
    let right_args = DrawArgs {
        screen: MenuScreen::AddApp,
        controls: args.controls,
        content_height: args.panel_h,
        panel_width: right_w,
        dock: args.dock,
        app_entries: &[],
        icon_choices: &[],
        search_query: "",
        wallpapers: &[],
        wallpaper_hovered: None,
        wallpaper_scroll_x: 0.0,
        hovered: right_hovered,
        render_scale: s,
        available_fonts: args.available_fonts,
        open_dropdown: args.open_dropdown,
        font_query: args.font_query,
        custom_hex: args.custom_hex,
        custom_light: args.custom_light,
        custom_focus: args.custom_focus,
        custom_name: args.custom_name,
        custom_name_focused: args.custom_name_focused,
        custom_panel_blend: args.custom_panel_blend,
        tray_items: &[],
    };
    draw_control_rows(&mut right_pixmap, icon_cache, text_cache, args.controls, &right_args);
    if args.dropdown_anim > 0.001 {
        use crate::menu::OpenDropdown;
        let anchor = match args.open_dropdown {
            OpenDropdown::Scheme => args.controls.iter().find(|c| matches!(c.kind, ControlKind::SchemeDropdown)),
            OpenDropdown::DockFont => args.controls.iter().find(|c| matches!(c.kind, ControlKind::DockFontDropdown)),
            OpenDropdown::SystemFont => args.controls.iter().find(|c| matches!(c.kind, ControlKind::SystemFontDropdown)),
            OpenDropdown::None => None,
        };
        if let Some(c) = anchor {
            let t = args.dropdown_anim.clamp(0.0, 1.0);
            let eased = 0.5 - 0.5 * (std::f32::consts::PI * t).cos();
            let mut overlay_pixmap = Pixmap::new(right_pixmap.width(), right_pixmap.height()).unwrap();
            let overlay_y = c.y + c.height + 4.0;
            match args.open_dropdown {
                OpenDropdown::Scheme => draw_scheme_overlay(&mut overlay_pixmap, text_cache, overlay_y, &right_args),
                OpenDropdown::DockFont => draw_font_overlay(
                    &mut overlay_pixmap,
                    text_cache,
                    overlay_y,
                    &right_args,
                    &args.dock.config.settings.dock_font,
                    args.dropdown_scroll,
                    args.font_query,
                    args.dropdown_selected,
                ),
                OpenDropdown::SystemFont => draw_font_overlay(
                    &mut overlay_pixmap,
                    text_cache,
                    overlay_y,
                    &right_args,
                    &args.dock.config.settings.system_font,
                    args.dropdown_scroll,
                    args.font_query,
                    args.dropdown_selected,
                ),
                OpenDropdown::None => {}
            }
            let slide = (1.0 - eased) * -6.0 * s;
            let paint = tiny_skia::PixmapPaint { opacity: eased, ..Default::default() };
            right_pixmap.draw_pixmap(0, 0, overlay_pixmap.as_ref(), &paint, Transform::from_translate(0.0, slide), None);
        }
    }
    pixmap.draw_pixmap(0, 0, right_pixmap.as_ref(), &tiny_skia::PixmapPaint::default(), Transform::from_translate(right_x + args.slide_offset, 0.0), None);

    if let Some((kind, dx, dy)) = args.dragging_widget {
        let chips_y = args.controls.iter().find_map(|c| matches!(c.kind, ControlKind::WidgetChips).then_some(c.y)).unwrap_or(MENU_PADDING);
        draw_widget_drag_overlay(pixmap, text_cache, settings, right_x, right_w, s, kind, dx, dy, chips_y);
    }
}

// ----- above columns -----
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_widget_drag_overlay(
    pixmap: &mut Pixmap,
    text_cache: &mut TextCache,
    settings: &crate::config::DockSettings,
    right_x: f32,
    right_w: f32,
    s: f32,
    kind: crate::config::WidgetKind,
    dx: f32,
    dy: f32,
    chips_y: f32,
) {
    use crate::config::WidgetSlot;
    use crate::menu::{widget_chip_layout, widget_drop_target, widget_label, MENU_PADDING, WIDGET_CHIP_H, WIDGET_CHIP_W, WIDGET_COL_GAP, WIDGET_SECTION_GAP};

    let layout = widget_chip_layout(settings, right_w, chips_y);
    let target = widget_drop_target(settings, chips_y, right_w, dx, dy);

    let content_w = right_w - MENU_PADDING * 2.0;
    let (hx, hy, hw, hh) = match target {
        None => (
            MENU_PADDING,
            layout.available_label_y,
            content_w,
            layout.columns_label_y - layout.available_label_y - WIDGET_SECTION_GAP,
        ),
        Some(slot) => {
            let ci = match slot {
                WidgetSlot::Left => 0,
                WidgetSlot::Middle => 1,
                WidgetSlot::Right => 2,
            };
            let col_x = MENU_PADDING + ci as f32 * (layout.col_w + WIDGET_COL_GAP);
            (col_x, layout.columns_label_y, layout.col_w, layout.total_h - layout.columns_label_y)
        }
    };
    let accent_c = accent(settings);
    fill_rrect(
        pixmap,
        right_x + hx * s,
        hy * s,
        hw * s,
        hh * s,
        8.0 * s,
        (accent_c.0, accent_c.1, accent_c.2, 45),
    );

    let label = widget_label(kind);
    let gw = WIDGET_CHIP_W * s;
    let gh = WIDGET_CHIP_H * s;
    let gx = right_x + dx * s - gw / 2.0;
    let gy = dy * s - gh / 2.0;
    fill_rrect(pixmap, gx, gy, gw, gh, gh / 2.0, (accent_c.0, accent_c.1, accent_c.2, 235));
    let on_accent = on_accent_hex(settings);
    let tx = gx + gw / 2.0 - text_width_estimate(label, 8.5) * s / 2.0;
    let ty = gy + centered_text_y(WIDGET_CHIP_H, 8.5) * s;
    draw_text(pixmap, text_cache, label, tx, ty, 8.5 * s, &on_accent, 600);
}
pub(crate) fn draw_edge_picker(pixmap: &mut Pixmap, text_cache: &mut TextCache, control: &Control, args: &DrawArgs, y: f32) {
    use crate::menu::{edge_label, edge_segment_rect, EDGE_OPTIONS, EDGE_PICKER_H};
    let s = args.render_scale;
    let settings = &args.dock.config.settings;
    let default_color = text_hex(settings);
    let selected_color = on_accent_hex(settings);
    for (i, &edge) in EDGE_OPTIONS.iter().enumerate() {
        let (seg_x, seg_w, _) = edge_segment_rect(i, control.y, args.panel_width);
        let selected = settings.dock_edge == edge;
        let hot = matches!(args.hovered, Some(HitTarget::Edge(h)) if h == edge);
        let pad = 2.0;
        if selected || hot {
            let color = if selected { accent(settings) } else { HOVER_SOFT };
            fill_rrect(pixmap, (seg_x + pad) * s, y + pad * s, (seg_w - pad * 2.0) * s, (EDGE_PICKER_H - pad * 2.0) * s, 6.0 * s, color);
        }
        let color = if selected { &selected_color } else { &default_color };
        let ty = y + centered_text_y(EDGE_PICKER_H, 9.0) * s;
        let label = edge_label(edge);
        let tx = (seg_x + seg_w / 2.0) * s - text_width_estimate(label, 9.0) * s / 2.0;
        draw_text(pixmap, text_cache, label, tx, ty, 9.0 * s, color, if selected { 700 } else { 500 });
    }
}
