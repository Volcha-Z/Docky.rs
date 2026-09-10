use super::*;

pub(crate) fn draw_wallpaper_filmstrip(pixmap: &mut Pixmap, thumb_cache: &ThumbnailCache, text_cache: &mut TextCache, args: &DrawArgs) {
    if args.dock.is_vertical() {
        draw_wallpaper_filmstrip_vertical(pixmap, thumb_cache, text_cache, args);
    } else {
        draw_wallpaper_filmstrip_horizontal(pixmap, thumb_cache, text_cache, args);
    }
}

pub(crate) fn draw_wallpaper_filmstrip_horizontal(pixmap: &mut Pixmap, thumb_cache: &ThumbnailCache, text_cache: &mut TextCache, args: &DrawArgs) {
    let s = args.render_scale;
    let settings = &args.dock.config.settings;
    let panel_h = args.content_height;
    let panel_w = args.panel_width;

    let acc = accent(settings);
    let back_hot = matches!(args.wallpaper_hovered, Some(WallpaperHit::Back));
    let back_color = if back_hot { acc.0 } else { 255 };
    if back_hot {
        fill_rrect(pixmap, 0.0, 0.0, WALLPAPER_BACK_ZONE_W * s, panel_h * s, 0.0, (acc.0, acc.1, acc.2, 60));
    }
    let mut chevron = Paint::default();
    chevron.set_color_rgba8(back_color, if back_hot { acc.1 } else { 255 }, if back_hot { acc.2 } else { 255 }, 220);
    chevron.anti_alias = true;
    let ccx = WALLPAPER_BACK_ZONE_W * 0.5 * s;
    let ccy = panel_h * 0.5 * s;
    let arm = 5.0 * s;
    let mut pb = tiny_skia::PathBuilder::new();
    pb.move_to(ccx + arm * 0.4, ccy - arm);
    pb.line_to(ccx - arm * 0.6, ccy);
    pb.line_to(ccx + arm * 0.4, ccy + arm);
    if let Some(chevron_path) = pb.finish() {
        let stroke = tiny_skia::Stroke { width: 1.8 * s, line_cap: tiny_skia::LineCap::Round, line_join: tiny_skia::LineJoin::Round, ..Default::default() };
        pixmap.stroke_path(&chevron_path, &chevron, &stroke, Transform::identity(), None);
    }

    if args.wallpapers.is_empty() {
        let ty = (panel_h * 0.5 - 6.0) * s;
        let tx = (WALLPAPER_BACK_ZONE_W + 6.0) * s;
        let hint = format!("Add images to {}", crate::wallpaper::suggested_dir());
        draw_text(pixmap, text_cache, &hint, tx, ty, 8.5 * s, &text_dim_hex(settings), 500);
        return;
    }

    let (tw_logical, th_logical) = crate::menu::wallpaper_thumb_size(panel_w, panel_h, false);
    let tw = tw_logical * s;
    let th = th_logical * s;
    let viewport_x0 = WALLPAPER_BACK_ZONE_W * s;
    let ty = WALLPAPER_PADDING * s;
    let tw_i = tw.round().max(1.0) as u32;
    let th_i = th.round().max(1.0) as u32;

    for (i, entry) in args.wallpapers.iter().enumerate() {
        let local_x = i as f32 * (tw_logical + WALLPAPER_GAP);
        let tx = viewport_x0 + (local_x - args.wallpaper_scroll_x) * s;
        if tx + tw < viewport_x0 || tx > panel_w * s {
            continue;
        }
        let hot = matches!(args.wallpaper_hovered, Some(WallpaperHit::Thumbnail(idx)) if idx == i);

        if let Some(thumb) = thumb_cache.peek(&entry.path, tw_i, th_i) {
            pixmap.draw_pixmap(0, 0, thumb.as_ref().as_ref(), &tiny_skia::PixmapPaint::default(), Transform::from_translate(tx, ty), None);

            let path = rounded_rect_path(tx, ty, tw, th, 11.0 * s);
            let mut ring = Paint::default();
            ring.set_color_rgba8(if hot { acc.0 } else { 255 }, if hot { acc.1 } else { 255 }, if hot { acc.2 } else { 255 }, if hot { 255 } else { 35 });
            ring.anti_alias = true;
            let stroke = tiny_skia::Stroke { width: if hot { 2.0 * s } else { 1.0 * s }, ..Default::default() };
            pixmap.stroke_path(&path, &ring, &stroke, Transform::identity(), None);
        } else {
            fill_rrect(pixmap, tx, ty, tw, th, 5.0 * s, (255, 255, 255, 18));
        }
    }
}

// ----- back zone -----
pub(crate) fn draw_wallpaper_filmstrip_vertical(pixmap: &mut Pixmap, thumb_cache: &ThumbnailCache, text_cache: &mut TextCache, args: &DrawArgs) {
    let s = args.render_scale;
    let settings = &args.dock.config.settings;
    let panel_h = args.content_height;
    let panel_w = args.panel_width;

    let acc = accent(settings);
    let back_hot = matches!(args.wallpaper_hovered, Some(WallpaperHit::Back));
    let back_color = if back_hot { acc.0 } else { 255 };
    if back_hot {
        fill_rrect(pixmap, 0.0, 0.0, panel_w * s, WALLPAPER_BACK_ZONE_W * s, 0.0, (acc.0, acc.1, acc.2, 60));
    }
    let mut chevron = Paint::default();
    chevron.set_color_rgba8(back_color, if back_hot { acc.1 } else { 255 }, if back_hot { acc.2 } else { 255 }, 220);
    chevron.anti_alias = true;
    let ccx = panel_w * 0.5 * s;
    let ccy = WALLPAPER_BACK_ZONE_W * 0.5 * s;
    let arm = 5.0 * s;
    let mut pb = tiny_skia::PathBuilder::new();
    pb.move_to(ccx - arm, ccy + arm * 0.4);
    pb.line_to(ccx, ccy - arm * 0.6);
    pb.line_to(ccx + arm, ccy + arm * 0.4);
    if let Some(chevron_path) = pb.finish() {
        let stroke = tiny_skia::Stroke { width: 1.8 * s, line_cap: tiny_skia::LineCap::Round, line_join: tiny_skia::LineJoin::Round, ..Default::default() };
        pixmap.stroke_path(&chevron_path, &chevron, &stroke, Transform::identity(), None);
    }

    if args.wallpapers.is_empty() {
        let ty = (WALLPAPER_BACK_ZONE_W + 14.0) * s;
        let tx = WALLPAPER_PADDING * s;
        let hint = format!("Add images to {}", crate::wallpaper::suggested_dir());
        draw_text(pixmap, text_cache, &hint, tx, ty, 8.0 * s, &text_dim_hex(settings), 500);
        return;
    }

    let (tw_logical, th_logical) = crate::menu::wallpaper_thumb_size(panel_w, panel_h, true);
    let tw = tw_logical * s;
    let th = th_logical * s;
    let viewport_y0 = WALLPAPER_BACK_ZONE_W * s;
    let tx = WALLPAPER_PADDING * s;
    let tw_i = tw.round().max(1.0) as u32;
    let th_i = th.round().max(1.0) as u32;

    for (i, entry) in args.wallpapers.iter().enumerate() {
        let local_y = i as f32 * (th_logical + WALLPAPER_GAP);
        let ty = viewport_y0 + (local_y - args.wallpaper_scroll_x) * s;
        if ty + th < viewport_y0 || ty > panel_h * s {
            continue;
        }
        let hot = matches!(args.wallpaper_hovered, Some(WallpaperHit::Thumbnail(idx)) if idx == i);

        if let Some(thumb) = thumb_cache.peek(&entry.path, tw_i, th_i) {
            pixmap.draw_pixmap(0, 0, thumb.as_ref().as_ref(), &tiny_skia::PixmapPaint::default(), Transform::from_translate(tx, ty), None);

            let path = rounded_rect_path(tx, ty, tw, th, 11.0 * s);
            let mut ring = Paint::default();
            ring.set_color_rgba8(if hot { acc.0 } else { 255 }, if hot { acc.1 } else { 255 }, if hot { acc.2 } else { 255 }, if hot { 255 } else { 35 });
            ring.anti_alias = true;
            let stroke = tiny_skia::Stroke { width: if hot { 2.0 * s } else { 1.0 * s }, ..Default::default() };
            pixmap.stroke_path(&path, &ring, &stroke, Transform::identity(), None);
        } else {
            fill_rrect(pixmap, tx, ty, tw, th, 5.0 * s, (255, 255, 255, 18));
        }
    }
}
