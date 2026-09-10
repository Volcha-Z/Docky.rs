use crate::dock::Dock;
use crate::icon_cache::IconCache;
use crate::text::TextCache;
use crate::widgets::WidgetSnapshot;
use std::time::Instant;
use tiny_skia::{Color, Paint, Pixmap, Rect, Transform};

mod layout;
pub use layout::*;
mod tray;
pub use tray::*;
mod clock_battery;
use clock_battery::*;
mod media;
pub use media::*;
mod power_bluetooth;
use power_bluetooth::*;
mod cpu_ram;
use cpu_ram::*;
mod workspaces;
pub use workspaces::*;

const ICON_OVERSAMPLE: f32 = 1.0;
const DATE_FONT_FAMILY: &str = "JetBrains Mono";

const MARQUEE_SPEED: f32 = 12.0;
const MARQUEE_HOLD_SECS: f32 = 2.0;
const MARQUEE_GAP_FRAC: f32 = 0.4;
pub const MARQUEE_TICK_MS: u64 = 33;

struct MarqueeLane {
    offset: f32,
    hold_secs: f32,
    last_tick: Option<Instant>,
}

impl Default for MarqueeLane {
    fn default() -> Self {
        Self { offset: 0.0, hold_secs: MARQUEE_HOLD_SECS, last_tick: None }
    }
}

#[derive(Default)]
pub struct MarqueeState {
    key: String,
    title: MarqueeLane,
    ws_initialized: bool,
    ws_current: f32,
    ws_target: f32,
    ws_from: f32,
    ws_t: f32,
    ws_last_tick: Option<Instant>,
}

impl MarqueeState {
    pub fn workspace_animating(&self) -> bool {
        self.ws_initialized && self.ws_t < 1.0
    }
}

const WS_ANIM_DURATION_MS: f32 = 220.0;

fn ease_out(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

fn marquee_step(lane: &mut MarqueeLane, text_w: f32, avail_w: f32, advance: bool, render_scale: f32, smooth_scroll: bool) -> (f32, bool) {
    if !smooth_scroll || text_w <= avail_w {
        *lane = MarqueeLane::default();
        return (0.0, false);
    }
    if advance {
        let now = Instant::now();
        // ----- long gap clamp -----
        let dt = lane.last_tick.map(|t| now.duration_since(t).as_secs_f32()).unwrap_or(0.0).min(0.5);
        lane.last_tick = Some(now);
        if lane.hold_secs > 0.0 {
            lane.hold_secs -= dt;
        } else {
            let period = text_w + avail_w * MARQUEE_GAP_FRAC;
            lane.offset += MARQUEE_SPEED * render_scale * dt;
            if lane.offset >= period {
                lane.offset -= period;
            }
        }
    }
    (lane.offset, true)
}

fn tile_text(clip: &mut Pixmap, glyphs: &Pixmap, avail_w: f32, offset: f32) {
    let text_w = glyphs.width() as f32;
    let paint = tiny_skia::PixmapPaint::default();
    let period = text_w + avail_w * MARQUEE_GAP_FRAC;
    for i in -1..=1 {
        let tx = offset - period + period * i as f32;
        if tx + text_w < 0.0 || tx > avail_w {
            continue;
        }
        clip.draw_pixmap(0, 0, glyphs.as_ref(), &paint, Transform::from_translate(tx, 0.0), None);
    }
}

fn rounded_rect_path(x: f32, y: f32, w: f32, h: f32, r: f32) -> tiny_skia::Path {
    let r = r.min(w / 2.0).min(h / 2.0);
    let mut pb = tiny_skia::PathBuilder::new();
    pb.move_to(x + r, y);
    pb.line_to(x + w - r, y);
    pb.quad_to(x + w, y, x + w, y + r);
    pb.line_to(x + w, y + h - r);
    pb.quad_to(x + w, y + h, x + w - r, y + h);
    pb.line_to(x + r, y + h);
    pb.quad_to(x, y + h, x, y + h - r);
    pb.line_to(x, y + r);
    pb.quad_to(x, y, x + r, y);
    pb.close();
    pb.finish().unwrap()
}

#[allow(clippy::too_many_arguments)]
pub fn draw(
    pixmap: &mut Pixmap,
    dock: &Dock,
    icon_cache: &mut IconCache,
    text_cache: &mut TextCache,
    widgets: &WidgetSnapshot,
    tray: &[crate::tray::TrayIcon],
    marquee: &mut MarqueeState,
    advance_marquee: bool,
    advance_ws: bool,
    render_scale: f32,
) -> bool {
    pixmap.fill(Color::TRANSPARENT);

    let s = &dock.config.settings;
    let (base_w, base_h) = dock.base_size();
    let w = base_w as f32 * render_scale;
    let h = base_h as f32 * render_scale;

    // ----- blur via layerrule -----
    let bg_margin = 0.0;
    let bg_path = rounded_rect_path(
        bg_margin,
        bg_margin,
        w - bg_margin * 2.0,
        h - bg_margin * 2.0,
        s.corner_radius * render_scale,
    );
    let (bg_r, bg_g, bg_b) = (s.panel_r, s.panel_g, s.panel_b);
    let mut bg_paint = Paint::default();
    bg_paint.set_color_rgba8(bg_r, bg_g, bg_b, (255.0 * s.transparency) as u8);
    bg_paint.anti_alias = true;
    pixmap.fill_path(
        &bg_path,
        &bg_paint,
        tiny_skia::FillRule::Winding,
        Transform::identity(),
        None,
    );

    if s.border_width > 0.0 {
        let mut border_paint = Paint::default();
        border_paint.set_color_rgba8(s.accent_r, s.accent_g, s.accent_b, 200);
        border_paint.anti_alias = true;
        let stroke = tiny_skia::Stroke {
            width: s.border_width * render_scale,
            ..Default::default()
        };
        pixmap.stroke_path(&bg_path, &border_paint, &stroke, Transform::identity(), None);
    }

    if dock.icons.is_empty() {
        return draw_widgets(pixmap, dock, icon_cache, text_cache, widgets, tray, marquee, advance_marquee, advance_ws, render_scale);
    }

    let cache_size = (s.icon_size * s.dock_scale * s.magnify_scale * render_scale * ICON_OVERSAMPLE).ceil() as u32;
    let dragging = dock.dragging_index;
    let layout = dock.layout();
    let (edx, edy) = dock.elevate_dir();

    let order = (0..dock.icons.len()).filter(|i| Some(*i) != dragging).chain(dragging);

    for i in order {
        let icon = &dock.icons[i];
        let (cx, cy, drawn_w) = layout[i];
        let elevated = Some(i) == dragging;
        let cx = if elevated { cx + edx * 10.0 } else { cx };
        let cy = if elevated { cy + edy * 10.0 } else { cy };

        let Some(icon_pixmap) = icon_cache.get(&icon.app.icon, cache_size.max(1)) else {
            draw_placeholder(pixmap, &icon.app.name, cx * render_scale, cy * render_scale, drawn_w * render_scale);
            continue;
        };

        let drawn_w = drawn_w * render_scale;
        let cx = cx * render_scale;
        let cy = cy * render_scale;

        if elevated {
            let mut shadow_paint = Paint::default();
            shadow_paint.set_color_rgba8(0, 0, 0, 90);
            shadow_paint.anti_alias = true;
            let shadow_r = drawn_w * 0.5;
            let back_x = cx - edx * drawn_w * 0.32;
            let back_y = cy - edy * drawn_w * 0.32;
            let (sw, sh) = if edx.abs() > 0.5 { (shadow_r, shadow_r * 2.0) } else { (shadow_r * 2.0, shadow_r * 0.5) };
            if let Some(rect) = Rect::from_xywh(back_x - sw / 2.0, back_y - sh / 2.0, sw, sh) {
                let path = rounded_rect_path(rect.x(), rect.y(), rect.width(), rect.height(), sw.min(sh) * 0.4);
                pixmap.fill_path(&path, &shadow_paint, tiny_skia::FillRule::Winding, Transform::identity(), None);
            }
        }

        let scale = drawn_w / icon_pixmap.width() as f32;
        let transform = Transform::from_translate(cx - drawn_w / 2.0, cy - drawn_w / 2.0)
            .pre_scale(scale, scale);
        let icon_paint = tiny_skia::PixmapPaint::default();
        pixmap.draw_pixmap(0, 0, icon_pixmap.as_ref().as_ref(), &icon_paint, transform, None);
    }
    false
}


struct WidgetColors<'a> {
    accent: (u8, u8, u8, u8),
    text_rgb: (u8, u8, u8),
    text_color: &'a str,
    text_dim_color: &'a str,
}


fn draw_text_rotated(pixmap: &mut Pixmap, text_cache: &mut TextCache, text: &str, cx: f32, cy: f32, size: f32, color: &str, weight: u16) {
    let Some(glyphs) = text_cache.get(text, size, color, weight) else {
        return;
    };
    let iw = glyphs.width() as f32;
    let ih = glyphs.height() as f32;
    let paint = tiny_skia::PixmapPaint::default();
    let transform = Transform::from_translate(-iw / 2.0, -ih / 2.0).post_rotate(-90.0).post_translate(cx, cy);
    pixmap.draw_pixmap(0, 0, glyphs.as_ref().as_ref(), &paint, transform, None);
}

fn draw_text_rotated_family(
    pixmap: &mut Pixmap, text_cache: &mut TextCache, text: &str, cx: f32, cy: f32, size: f32, color: &str, weight: u16, family: &'static str,
) {
    let Some(glyphs) = text_cache.get_with_family(text, size, color, weight, family) else {
        return;
    };
    let iw = glyphs.width() as f32;
    let ih = glyphs.height() as f32;
    let paint = tiny_skia::PixmapPaint::default();
    let transform = Transform::from_translate(-iw / 2.0, -ih / 2.0).post_rotate(-90.0).post_translate(cx, cy);
    pixmap.draw_pixmap(0, 0, glyphs.as_ref().as_ref(), &paint, transform, None);
}



fn text_width_estimate_render(text: &str, size: f32) -> f32 {
    text.chars().count() as f32 * size * 0.64
}

fn draw_placeholder(pixmap: &mut Pixmap, name: &str, cx: f32, cy: f32, size: f32) {
    let hash = name.bytes().fold(37u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));
    let hue_r = 90 + (hash % 120) as u8;
    let hue_g = 90 + ((hash >> 8) % 120) as u8;
    let hue_b = 90 + ((hash >> 16) % 120) as u8;

    let mut paint = Paint::default();
    paint.set_color_rgba8(hue_r, hue_g, hue_b, 230);
    paint.anti_alias = true;
    let path = rounded_rect_path(cx - size / 2.0, cy - size / 2.0, size, size, size * 0.22);
    pixmap.fill_path(&path, &paint, tiny_skia::FillRule::Winding, Transform::identity(), None);
}
