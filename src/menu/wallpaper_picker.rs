pub const WALLPAPER_BACK_ZONE_W: f32 = 30.0;
pub const WALLPAPER_PADDING: f32 = 6.0;
pub const WALLPAPER_GAP: f32 = 6.0;
pub const WALLPAPER_ASPECT: f32 = 1.6;

// ----- always landscape -----
pub fn wallpaper_thumb_size(panel_w: f32, panel_h: f32, is_vertical: bool) -> (f32, f32) {
    if is_vertical {
        let w = (panel_w - WALLPAPER_PADDING * 2.0).max(1.0);
        (w, w / WALLPAPER_ASPECT)
    } else {
        let h = (panel_h - WALLPAPER_PADDING * 2.0).max(1.0);
        (h * WALLPAPER_ASPECT, h)
    }
}

pub fn wallpaper_content_len(count: usize, panel_w: f32, panel_h: f32, is_vertical: bool) -> f32 {
    let (tw, th) = wallpaper_thumb_size(panel_w, panel_h, is_vertical);
    let along = if is_vertical { th } else { tw };
    (count as f32 * (along + WALLPAPER_GAP) - WALLPAPER_GAP).max(0.0)
}

pub fn wallpaper_max_scroll(count: usize, panel_w: f32, panel_h: f32, is_vertical: bool) -> f32 {
    let viewport_along = ((if is_vertical { panel_h } else { panel_w }) - WALLPAPER_BACK_ZONE_W).max(1.0);
    (wallpaper_content_len(count, panel_w, panel_h, is_vertical) - viewport_along).max(0.0)
}

#[derive(Clone, Copy, Debug)]
pub enum WallpaperHit {
    Back,
    Thumbnail(usize),
}

pub fn wallpaper_hit_test(count: usize, panel_w: f32, panel_h: f32, is_vertical: bool, scroll: f32, x: f32, y: f32) -> Option<WallpaperHit> {
    let (along_pos, cross_pos) = if is_vertical { (y, x) } else { (x, y) };
    if along_pos < WALLPAPER_BACK_ZONE_W {
        return Some(WallpaperHit::Back);
    }
    let (tw, th) = wallpaper_thumb_size(panel_w, panel_h, is_vertical);
    let (along_size, cross_size) = if is_vertical { (th, tw) } else { (tw, th) };
    if cross_pos < WALLPAPER_PADDING || cross_pos > WALLPAPER_PADDING + cross_size {
        return None;
    }
    let local_along = along_pos - WALLPAPER_BACK_ZONE_W + scroll;
    for i in 0..count {
        let c = i as f32 * (along_size + WALLPAPER_GAP);
        if local_along >= c && local_along <= c + along_size {
            return Some(WallpaperHit::Thumbnail(i));
        }
    }
    None
}
