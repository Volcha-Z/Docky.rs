use super::*;
use crate::config::DockAlign;

pub const ALIGN_PICKER_H: f32 = 30.0;
pub const ALIGN_LABELS: [&str; 3] = ["Left", "Middle", "Right"];
pub const ALIGN_VALUES: [DockAlign; 3] = [DockAlign::Left, DockAlign::Middle, DockAlign::Right];

pub fn align_segment_rect(index: usize, control_y: f32, panel_width: f32) -> (f32, f32, f32) {
    let seg_w = (panel_width - MENU_PADDING * 2.0) / ALIGN_LABELS.len() as f32;
    (MENU_PADDING + seg_w * index as f32, seg_w, control_y)
}

pub fn align_hit_test(control_y: f32, panel_width: f32, x: f32, y: f32) -> Option<DockAlign> {
    if y < control_y || y > control_y + ALIGN_PICKER_H {
        return None;
    }
    for i in 0..ALIGN_LABELS.len() {
        let (seg_x, seg_w, _) = align_segment_rect(i, control_y, panel_width);
        if x >= seg_x && x <= seg_x + seg_w {
            return Some(ALIGN_VALUES[i]);
        }
    }
    None
}
