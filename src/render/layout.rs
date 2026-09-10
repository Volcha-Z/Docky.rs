use super::*;

pub(super) struct WidgetRect {
    pub(super) kind: crate::config::WidgetKind,
    pub(super) slot: crate::config::WidgetSlot,
    pub(super) x: f32,
    pub(super) y: f32,
    pub(super) w: f32,
    pub(super) h: f32,
}

pub(super) const WIDGET_GAP: f32 = 10.0;

// ----- flexible zones -----
pub(super) fn layout_widgets(
    settings: &crate::config::DockSettings,
    widgets: &WidgetSnapshot,
    tray_count: usize,
    is_vertical: bool,
    w: f32,
    h: f32,
    render_scale: f32,
) -> Vec<WidgetRect> {
    use crate::config::WidgetSlot;
    let bar_len = if is_vertical { h } else { w };
    let cross_len = if is_vertical { w } else { h };
    let gap = WIDGET_GAP * render_scale;
    let inset = 14.0 * render_scale;

    let members_of = |slot: WidgetSlot| -> Vec<crate::config::WidgetKind> {
        settings.widgets.iter().filter(|p| p.slot == slot).map(|p| p.kind).collect()
    };
    let lens_of = |members: &[crate::config::WidgetKind]| -> Vec<f32> {
        members
            .iter()
            .map(|&kind| widget_natural_len(kind, widgets, tray_count, is_vertical, cross_len, render_scale, settings.media_width_scale).max(1.0))
            .collect()
    };
    let block_len = |lens: &[f32]| -> f32 { lens.iter().sum::<f32>() + gap * (lens.len() as f32 - 1.0).max(0.0) };

    let place = |slot: WidgetSlot, members: Vec<crate::config::WidgetKind>, lens: Vec<f32>, block_start: f32| -> Vec<WidgetRect> {
        let mut pos = block_start;
        members
            .into_iter()
            .zip(lens)
            .map(|(kind, len)| {
                let start = pos;
                pos += len + gap;
                if is_vertical {
                    WidgetRect { kind, slot, x: 0.0, y: start, w, h: len }
                } else {
                    WidgetRect { kind, slot, x: start, y: 0.0, w: len, h }
                }
            })
            .collect()
    };

    let left_members = members_of(WidgetSlot::Left);
    let left_lens = lens_of(&left_members);
    let left_len = block_len(&left_lens);
    let left_end = inset + left_len;

    let right_members = members_of(WidgetSlot::Right);
    let right_lens = lens_of(&right_members);
    let right_len = block_len(&right_lens);
    let right_start = bar_len - inset - right_len;

    let mid_members = members_of(WidgetSlot::Middle);
    let mid_lens = lens_of(&mid_members);
    let mid_len = block_len(&mid_lens);
    let segment_start = if left_len > 0.0 { left_end + gap } else { inset };
    let segment_end = if right_len > 0.0 { right_start - gap } else { bar_len - inset };
    let gap_center = segment_start + (segment_end - segment_start - mid_len) / 2.0;
    let true_center = (bar_len - mid_len) / 2.0;
    let blended = gap_center + (true_center - gap_center) * 0.8;
    let mid_start = blended.clamp(segment_start.min(segment_end - mid_len), segment_end - mid_len);

    let mut out = place(WidgetSlot::Left, left_members, left_lens, inset);
    out.extend(place(WidgetSlot::Middle, mid_members, mid_lens, mid_start));
    out.extend(place(WidgetSlot::Right, right_members, right_lens, right_start));
    out
}

// ----- mirrors layout -----
pub fn widget_bar_natural_len(
    settings: &crate::config::DockSettings,
    widgets: &WidgetSnapshot,
    tray_count: usize,
    is_vertical: bool,
    cross_len: f32,
    render_scale: f32,
) -> f32 {
    use crate::config::WidgetSlot;
    if settings.widgets.is_empty() {
        return 0.0;
    }
    let gap = WIDGET_GAP * render_scale;
    let inset = 14.0 * render_scale;

    let zone_len = |slot: WidgetSlot| -> f32 {
        let lens: Vec<f32> = settings
            .widgets
            .iter()
            .filter(|p| p.slot == slot)
            .map(|p| widget_natural_len(p.kind, widgets, tray_count, is_vertical, cross_len, render_scale, settings.media_width_scale).max(1.0))
            .collect();
        if lens.is_empty() {
            0.0
        } else {
            lens.iter().sum::<f32>() + gap * (lens.len() as f32 - 1.0)
        }
    };

    let zones = [zone_len(WidgetSlot::Left), zone_len(WidgetSlot::Middle), zone_len(WidgetSlot::Right)];
    let present: Vec<f32> = zones.into_iter().filter(|&len| len > 0.0).collect();
    present.iter().sum::<f32>() + gap * (present.len() as f32 - 1.0).max(0.0) + inset * 2.0
}

pub(super) fn widget_natural_len(
    kind: crate::config::WidgetKind,
    widgets: &WidgetSnapshot,
    tray_count: usize,
    is_vertical: bool,
    cross_len: f32,
    render_scale: f32,
    media_width_scale: f32,
) -> f32 {
    use crate::config::WidgetKind;
    match kind {
        WidgetKind::Clock => {
            if is_vertical {
                let time_w = text_width_estimate_render(&widgets.time, 12.0 * render_scale);
                let date_w = text_width_estimate_render(&widgets.date, 7.5 * render_scale);
                time_w.max(date_w)
            } else {
                let time_w = text_width_estimate_render(&widgets.time, 13.0 * render_scale);
                let date_w = text_width_estimate_render(&widgets.date, 8.0 * render_scale);
                time_w.max(date_w)
            }
        }
        WidgetKind::Battery => {
            let Some((pct, _)) = widgets.battery else { return 0.0 };
            let label = format!("{pct}%");
            if is_vertical {
                let label_len = text_width_estimate_render(&label, 8.5 * render_scale);
                10.0 * render_scale + 6.0 * render_scale + label_len
            } else {
                let label_w = text_width_estimate_render(&label, 9.0 * render_scale);
                20.0 * render_scale + 8.0 * render_scale + label_w
            }
        }
        WidgetKind::Media => media_ideal_len(is_vertical, render_scale, media_width_scale),
        WidgetKind::PowerMenu => 14.0 * render_scale,
        WidgetKind::Bluetooth => {
            let icon_r = 6.5 * render_scale;
            let label = match &widgets.bluetooth {
                Some(bt) if !bt.powered => "Off".to_string(),
                Some(bt) => bt.connected.clone().unwrap_or_else(|| "On".to_string()),
                None => String::new(),
            };
            if label.is_empty() {
                icon_r * 2.0
            } else if is_vertical {
                icon_r * 2.0 + 8.0 * render_scale + text_width_estimate_render(&label, 8.0 * render_scale)
            } else {
                icon_r * 2.0 + 10.0 * render_scale + text_width_estimate_render(&label, 9.0 * render_scale)
            }
        }
        WidgetKind::Tray => {
            if tray_count == 0 {
                0.0
            } else {
                tray_geometry(tray_count, is_vertical, cross_len, cross_len, render_scale).2
            }
        }
        WidgetKind::Workspaces => workspaces_geometry(&widgets.workspaces, render_scale),
        WidgetKind::Cpu => percentage_widget_len(widgets.cpu, is_vertical, render_scale),
        WidgetKind::Ram => percentage_widget_len(widgets.ram, is_vertical, render_scale),
    }
}

pub(super) fn percentage_widget_len(pct: Option<u8>, is_vertical: bool, render_scale: f32) -> f32 {
    let Some(pct) = pct else { return 0.0 };
    let label = format!("{pct}%");
    let icon_side = 13.0 * render_scale;
    if is_vertical {
        icon_side + 6.0 * render_scale + text_width_estimate_render(&label, 8.5 * render_scale)
    } else {
        icon_side + 8.0 * render_scale + text_width_estimate_render(&label, 9.0 * render_scale)
    }
}
pub(super) fn draw_widgets(
    pixmap: &mut Pixmap,
    dock: &Dock,
    icon_cache: &mut IconCache,
    text_cache: &mut TextCache,
    widgets: &WidgetSnapshot,
    tray: &[crate::tray::TrayIcon],
    marquee: &mut MarqueeState,
    advance: bool,
    advance_ws: bool,
    render_scale: f32,
) -> bool {
    use crate::config::WidgetKind;

    let s = &dock.config.settings;
    let (base_w, base_h) = dock.base_size();
    let w = base_w as f32 * render_scale;
    let h = base_h as f32 * render_scale;
    // ----- widget scale -----
    let render_scale = render_scale * s.widget_scale;
    let accent = (s.accent_r, s.accent_g, s.accent_b, 255);
    // ----- skip accent blend -----
    let blend = |base: u8, tint: u8, frac: f32| (base as f32 * (1.0 - frac) + tint as f32 * frac) as u8;
    let (text_rgb, text_dim_rgb) = if s.custom_theme {
        ((s.text_r, s.text_g, s.text_b), (s.text_dim_r, s.text_dim_g, s.text_dim_b))
    } else {
        (
            (blend(s.text_r, s.accent_r, 0.45), blend(s.text_g, s.accent_g, 0.45), blend(s.text_b, s.accent_b, 0.45)),
            (blend(s.text_r, s.accent2_r, 0.65), blend(s.text_g, s.accent2_g, 0.65), blend(s.text_b, s.accent2_b, 0.65)),
        )
    };
    let text_color = format!("#{:02x}{:02x}{:02x}", text_rgb.0, text_rgb.1, text_rgb.2);
    let text_dim_color = format!("#{:02x}{:02x}{:02x}", text_dim_rgb.0, text_dim_rgb.1, text_dim_rgb.2);
    let colors = WidgetColors { accent, text_rgb, text_color: &text_color, text_dim_color: &text_dim_color };

    let key = widgets.media.as_ref().map(|m| m.title.clone()).unwrap_or_else(|| "Nothing is playing".to_string());
    if marquee.key != key {
        marquee.key = key;
        marquee.title = MarqueeLane::default();
    }

    let is_vertical = dock.is_vertical();
    let tray_count = tray.len();
    let mut animating = false;
    for r in layout_widgets(s, widgets, tray_count, is_vertical, w, h, render_scale) {
        match r.kind {
            WidgetKind::Clock => draw_clock_widget(pixmap, text_cache, widgets, r.x, r.y, r.w, r.h, render_scale, &colors, is_vertical),
            WidgetKind::Battery => draw_battery_widget(pixmap, text_cache, widgets, r.x, r.y, r.w, r.h, render_scale, &colors, is_vertical),
            WidgetKind::Media => {
                let mirrored = r.slot == crate::config::WidgetSlot::Right;
                let bar_len = if is_vertical { h } else { w };
                animating |= draw_media_widget(
                    pixmap, icon_cache, text_cache, widgets, marquee, advance, r.x, r.y, r.w, r.h, render_scale, &colors, is_vertical, mirrored,
                    bar_len, s.media_smooth_scroll, s.media_width_scale,
                );
            }
            WidgetKind::PowerMenu => draw_power_widget(pixmap, r.x, r.y, r.w, r.h, render_scale, &colors, is_vertical),
            WidgetKind::Bluetooth => {
                draw_bluetooth_widget(pixmap, text_cache, widgets, r.x, r.y, r.w, r.h, render_scale, &colors, is_vertical)
            }
            WidgetKind::Tray => draw_tray_widget(pixmap, icon_cache, tray, r.x, r.y, r.w, r.h, render_scale, is_vertical),
            WidgetKind::Workspaces => {
                draw_workspaces_widget(pixmap, &widgets.workspaces, marquee, advance_ws, r.x, r.y, r.w, r.h, render_scale, &colors, is_vertical);
            }
            WidgetKind::Cpu => draw_cpu_widget(pixmap, text_cache, widgets, r.x, r.y, r.w, r.h, render_scale, &colors, is_vertical),
            WidgetKind::Ram => draw_ram_widget(pixmap, text_cache, widgets, r.x, r.y, r.w, r.h, render_scale, &colors, is_vertical),
        }
    }
    animating
}

pub fn widget_hit_test(dock: &Dock, widgets: &WidgetSnapshot, tray_count: usize, x: f64, y: f64) -> Option<crate::config::WidgetKind> {
    let (base_w, base_h) = dock.base_size();
    let is_vertical = dock.is_vertical();
    let rects = layout_widgets(&dock.config.settings, widgets, tray_count, is_vertical, base_w as f32, base_h as f32, 1.0);
    let (xf, yf) = (x as f32, y as f32);
    rects.into_iter().find(|r| xf >= r.x && xf < r.x + r.w && yf >= r.y && yf < r.y + r.h).map(|r| r.kind)
}

pub fn widget_center(dock: &Dock, widgets: &WidgetSnapshot, tray_count: usize, kind: crate::config::WidgetKind) -> Option<(f32, f32)> {
    let (base_w, base_h) = dock.base_size();
    let is_vertical = dock.is_vertical();
    let rects = layout_widgets(&dock.config.settings, widgets, tray_count, is_vertical, base_w as f32, base_h as f32, 1.0);
    let r = rects.into_iter().find(|r| r.kind == kind)?;
    Some((r.x + r.w / 2.0, r.y + r.h / 2.0))
}
