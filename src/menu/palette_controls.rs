use super::*;

pub const PALETTE_MODE_PICKER_H: f32 = 30.0;
pub const PALETTE_MODE_LABELS: [&str; 2] = ["Dark", "White"];

pub fn palette_mode_segment_rect(index: usize, control_y: f32, panel_width: f32) -> (f32, f32, f32) {
    let seg_w = (panel_width - MENU_PADDING * 2.0) / PALETTE_MODE_LABELS.len() as f32;
    (MENU_PADDING + seg_w * index as f32, seg_w, control_y)
}

// ----- light flag -----
pub fn palette_mode_hit_test(control_y: f32, panel_width: f32, x: f32, y: f32) -> Option<bool> {
    if y < control_y || y > control_y + PALETTE_MODE_PICKER_H {
        return None;
    }
    for i in 0..PALETTE_MODE_LABELS.len() {
        let (seg_x, seg_w, _) = palette_mode_segment_rect(i, control_y, panel_width);
        if x >= seg_x && x <= seg_x + seg_w {
            return Some(i == 1);
        }
    }
    None
}

pub const PANEL_BLEND_PICKER_H: f32 = 30.0;
// ----- index paired -----
pub const PANEL_BLEND_LABELS: [&str; 6] = ["0-100", "20-80", "40-60", "60-40", "80-20", "90-10"];
pub const PANEL_BLEND_ACCENT_PCT: [u8; 6] = [100, 80, 60, 40, 20, 10];

pub fn panel_blend_segment_rect(index: usize, control_y: f32, panel_width: f32) -> (f32, f32, f32) {
    let seg_w = (panel_width - MENU_PADDING * 2.0) / PANEL_BLEND_LABELS.len() as f32;
    (MENU_PADDING + seg_w * index as f32, seg_w, control_y)
}

// ----- accent pct -----
pub fn panel_blend_hit_test(control_y: f32, panel_width: f32, x: f32, y: f32) -> Option<u8> {
    if y < control_y || y > control_y + PANEL_BLEND_PICKER_H {
        return None;
    }
    for i in 0..PANEL_BLEND_LABELS.len() {
        let (seg_x, seg_w, _) = panel_blend_segment_rect(i, control_y, panel_width);
        if x >= seg_x && x <= seg_x + seg_w {
            return Some(PANEL_BLEND_ACCENT_PCT[i]);
        }
    }
    None
}
