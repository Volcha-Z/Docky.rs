use super::*;

pub(super) const WS_DOT: f32 = 13.0;
pub(super) const WS_ACTIVE: f32 = 28.0;
pub(super) const WS_SLOT: f32 = 24.0;

fn slot_count(workspaces: &[crate::widgets::WorkspaceInfo]) -> usize {
    workspaces.len().max(1)
}

fn active_slot(workspaces: &[crate::widgets::WorkspaceInfo]) -> f32 {
    workspaces.iter().position(|w| w.active).map(|i| i as f32 + 1.0).unwrap_or(1.0)
}

pub(super) fn workspaces_geometry(workspaces: &[crate::widgets::WorkspaceInfo], render_scale: f32) -> f32 {
    let n = slot_count(workspaces) as f32;
    ((n - 1.0) * WS_SLOT + WS_ACTIVE) * render_scale
}

fn first_center(count: usize, bar_len: f32, bar_start: f32, render_scale: f32) -> f32 {
    let slot = WS_SLOT * render_scale;
    let content = (count.max(1) as f32 - 1.0) * slot + WS_ACTIVE * render_scale;
    bar_start + (bar_len - content) / 2.0 + WS_ACTIVE * render_scale / 2.0
}

#[allow(clippy::too_many_arguments)]
pub(super) fn draw_workspaces_widget(
    pixmap: &mut Pixmap,
    workspaces: &[crate::widgets::WorkspaceInfo],
    marquee: &mut MarqueeState,
    advance: bool,
    zx: f32,
    zy: f32,
    zw: f32,
    zh: f32,
    render_scale: f32,
    colors: &WidgetColors,
    is_vertical: bool,
) -> bool {
    if workspaces.is_empty() {
        return false;
    }
    let dot = WS_DOT * render_scale;
    let active = WS_ACTIVE * render_scale;
    let slot = WS_SLOT * render_scale;
    let count = slot_count(workspaces);
    let target = active_slot(workspaces);

    if !marquee.ws_initialized {
        marquee.ws_initialized = true;
        marquee.ws_current = target;
        marquee.ws_target = target;
        marquee.ws_t = 1.0;
    } else if (marquee.ws_target - target).abs() > 0.01 {
        marquee.ws_from = marquee.ws_current;
        marquee.ws_target = target;
        marquee.ws_t = 0.0;
        marquee.ws_last_tick = None;
    }
    if advance && marquee.ws_t < 1.0 {
        let now = std::time::Instant::now();
        let dt_ms = marquee.ws_last_tick.map(|prev| now.duration_since(prev).as_secs_f32() * 1000.0).unwrap_or(1000.0 / 60.0);
        marquee.ws_last_tick = Some(now);
        marquee.ws_t = (marquee.ws_t + dt_ms / WS_ANIM_DURATION_MS).min(1.0);
        marquee.ws_current = marquee.ws_from + (marquee.ws_target - marquee.ws_from) * ease_out(marquee.ws_t);
        if marquee.ws_t >= 1.0 {
            marquee.ws_current = marquee.ws_target;
        }
    }
    let animating = marquee.ws_t < 1.0;

    let bar_len = if is_vertical { zh } else { zw };
    let bar_start = if is_vertical { zy } else { zx };
    let cross_center = if is_vertical { zx + zw / 2.0 } else { zy + zh / 2.0 };
    let first = first_center(count, bar_len, bar_start, render_scale);

    let (ar, ag, ab, _) = colors.accent;
    for i in 0..count {
        let c = first + i as f32 * slot;
        let (cx, cy) = if is_vertical { (cross_center, c) } else { (c, cross_center) };
        draw_ws_shape(pixmap, cx, cy, dot, dot, (ar, ag, ab), 70);
    }

    let c = first + (marquee.ws_current - 1.0).max(0.0) * slot;
    let (cx, cy, w, h) = if is_vertical { (cross_center, c, dot, active) } else { (c, cross_center, active, dot) };
    draw_ws_shape(pixmap, cx, cy, w, h, (ar, ag, ab), 255);
    animating
}

fn draw_ws_shape(pixmap: &mut Pixmap, cx: f32, cy: f32, w: f32, h: f32, rgb: (u8, u8, u8), alpha: u8) {
    let mut paint = Paint::default();
    paint.set_color_rgba8(rgb.0, rgb.1, rgb.2, alpha);
    paint.anti_alias = true;
    let path = rounded_rect_path(cx - w / 2.0, cy - h / 2.0, w, h, h.min(w) / 2.0);
    pixmap.fill_path(&path, &paint, tiny_skia::FillRule::Winding, Transform::identity(), None);
}

// ----- slot hit test -----
pub fn workspace_dot_hit(dock: &Dock, widgets: &WidgetSnapshot, tray_count: usize, x: f64, y: f64) -> Option<i32> {
    let workspaces = &widgets.workspaces;
    if workspaces.is_empty() {
        return None;
    }
    let (base_w, base_h) = dock.base_size();
    let is_vertical = dock.is_vertical();
    let rects = layout_widgets(&dock.config.settings, widgets, tray_count, is_vertical, base_w as f32, base_h as f32, 1.0);
    let r = rects.into_iter().find(|r| r.kind == crate::config::WidgetKind::Workspaces)?;
    let (bar_len, bar_start, main) = if is_vertical { (r.h, r.y, y as f32) } else { (r.w, r.x, x as f32) };
    let count = slot_count(workspaces);
    let first = first_center(count, bar_len, bar_start, 1.0);
    let half = WS_SLOT / 2.0;
    for (i, ws) in workspaces.iter().enumerate() {
        if (main - (first + i as f32 * WS_SLOT)).abs() <= half {
            return Some(ws.id);
        }
    }
    None
}
