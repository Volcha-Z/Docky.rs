use super::*;

pub fn tray_icon_hit(dock: &Dock, widgets: &WidgetSnapshot, tray_count: usize, x: f64, y: f64) -> Option<usize> {
    if tray_count == 0 {
        return None;
    }
    let (base_w, base_h) = dock.base_size();
    let is_vertical = dock.is_vertical();
    let rects = layout_widgets(&dock.config.settings, widgets, tray_count, is_vertical, base_w as f32, base_h as f32, 1.0);
    let r = rects.into_iter().find(|r| r.kind == crate::config::WidgetKind::Tray)?;
    let (icon_side, per, content_len) = tray_geometry(tray_count, is_vertical, r.w, r.h, 1.0);
    let (xf, yf) = (x as f32, y as f32);
    if is_vertical {
        if xf < r.x || xf >= r.x + r.w {
            return None;
        }
        let block_start = r.y + (r.h - content_len) / 2.0;
        let rel = yf - block_start;
        let idx = (rel / per).floor();
        (idx >= 0.0 && rel - idx * per <= icon_side).then(|| idx as usize).filter(|&i| i < tray_count)
    } else {
        if yf < r.y || yf >= r.y + r.h {
            return None;
        }
        let block_start = r.x + (r.w - content_len) / 2.0;
        let rel = xf - block_start;
        let idx = (rel / per).floor();
        (idx >= 0.0 && rel - idx * per <= icon_side).then(|| idx as usize).filter(|&i| i < tray_count)
    }
}

pub fn tray_icon_center(dock: &Dock, widgets: &WidgetSnapshot, tray_count: usize, idx: usize) -> Option<(f32, f32)> {
    if idx >= tray_count {
        return None;
    }
    let (base_w, base_h) = dock.base_size();
    let is_vertical = dock.is_vertical();
    let rects = layout_widgets(&dock.config.settings, widgets, tray_count, is_vertical, base_w as f32, base_h as f32, 1.0);
    let r = rects.into_iter().find(|r| r.kind == crate::config::WidgetKind::Tray)?;
    let (icon_side, per, content_len) = tray_geometry(tray_count, is_vertical, r.w, r.h, 1.0);
    if is_vertical {
        let block_start = r.y + (r.h - content_len) / 2.0;
        Some((r.x + r.w / 2.0, block_start + idx as f32 * per + icon_side / 2.0))
    } else {
        let block_start = r.x + (r.w - content_len) / 2.0;
        Some((block_start + idx as f32 * per + icon_side / 2.0, r.y + r.h / 2.0))
    }
}
pub(super) fn tray_geometry(tray_count: usize, is_vertical: bool, zw: f32, zh: f32, render_scale: f32) -> (f32, f32, f32) {
    let icon_side = (16.0 * render_scale).min(if is_vertical { zw } else { zh } * 0.7);
    let gap = 10.0 * render_scale;
    let per = icon_side + gap;
    let content_len = per * tray_count as f32 - gap;
    (icon_side, per, content_len)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn draw_tray_widget(
    pixmap: &mut Pixmap,
    icon_cache: &mut IconCache,
    tray: &[crate::tray::TrayIcon],
    zx: f32,
    zy: f32,
    zw: f32,
    zh: f32,
    render_scale: f32,
    is_vertical: bool,
) {
    if tray.is_empty() {
        return;
    }
    let (icon_side, per, content_len) = tray_geometry(tray.len(), is_vertical, zw, zh, render_scale);
    if is_vertical {
        let block_start = zy + (zh - content_len) / 2.0;
        let cx = zx + zw / 2.0;
        for (i, item) in tray.iter().enumerate() {
            let cy = block_start + i as f32 * per + icon_side / 2.0;
            draw_tray_icon(pixmap, icon_cache, item, cx, cy, icon_side);
        }
    } else {
        let block_start = zx + (zw - content_len) / 2.0;
        let cy = zy + zh / 2.0;
        for (i, item) in tray.iter().enumerate() {
            let cx = block_start + i as f32 * per + icon_side / 2.0;
            draw_tray_icon(pixmap, icon_cache, item, cx, cy, icon_side);
        }
    }
}

pub(super) fn draw_tray_icon(pixmap: &mut Pixmap, icon_cache: &mut IconCache, item: &crate::tray::TrayIcon, cx: f32, cy: f32, icon_side: f32) {
    let side_i = icon_side.round().max(1.0) as u32;
    if !item.icon_name.is_empty() {
        if let Some(icon) = icon_cache.get(&item.icon_name, side_i) {
            let x = cx - side_i as f32 / 2.0;
            let y = cy - side_i as f32 / 2.0;
            pixmap.draw_pixmap(0, 0, icon.as_ref().as_ref(), &tiny_skia::PixmapPaint::default(), Transform::from_translate(x, y), None);
            return;
        }
    }
    if let Some(raw) = &item.icon_pixmap {
        let scale = icon_side / raw.width().max(1) as f32;
        let x = cx - icon_side / 2.0;
        let y = cy - icon_side / 2.0;
        pixmap.draw_pixmap(0, 0, raw.as_ref().as_ref(), &tiny_skia::PixmapPaint::default(), Transform::from_translate(x, y).pre_scale(scale, scale), None);
        return;
    }
    draw_placeholder(pixmap, &item.service, cx, cy, icon_side);
}

