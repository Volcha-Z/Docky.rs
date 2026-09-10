use super::*;

pub const EDGE_PICKER_H: f32 = 30.0;
pub const EDGE_OPTIONS: [DockEdge; 4] = [DockEdge::Top, DockEdge::Left, DockEdge::Bottom, DockEdge::Right];

pub fn edge_label(edge: DockEdge) -> &'static str {
    match edge {
        DockEdge::Top => "Top",
        DockEdge::Left => "Left",
        DockEdge::Bottom => "Bottom",
        DockEdge::Right => "Right",
    }
}

pub fn edge_segment_rect(index: usize, control_y: f32, panel_width: f32) -> (f32, f32, f32) {
    let seg_w = (panel_width - MENU_PADDING * 2.0) / EDGE_OPTIONS.len() as f32;
    (MENU_PADDING + seg_w * index as f32, seg_w, control_y)
}

pub fn edge_picker_hit_test(control_y: f32, panel_width: f32, x: f32, y: f32) -> Option<DockEdge> {
    if y < control_y || y > control_y + EDGE_PICKER_H {
        return None;
    }
    for (i, &edge) in EDGE_OPTIONS.iter().enumerate() {
        let (seg_x, seg_w, _) = edge_segment_rect(i, control_y, panel_width);
        if x >= seg_x && x <= seg_x + seg_w {
            return Some(edge);
        }
    }
    None
}
