use super::*;

pub const MATUGEN_SCHEMES: &[(&str, &str)] = &[
    ("scheme-tonal-spot", "Tonal Spot"),
    ("scheme-vibrant", "Vibrant"),
    ("scheme-expressive", "Expressive"),
    ("scheme-fidelity", "Fidelity"),
    ("scheme-content", "Content"),
    ("scheme-fruit-salad", "Fruit Salad"),
    ("scheme-monochrome", "Monochrome"),
    ("scheme-neutral", "Neutral"),
    ("scheme-rainbow", "Rainbow"),
];

pub const SCHEME_DROPDOWN_H: f32 = 32.0;
pub const SCHEME_ROW_H: f32 = 30.0;
pub const SCHEME_ROW_GAP: f32 = 8.0;
pub const SCHEME_COL_GAP: f32 = 8.0;
pub const SCHEME_COLS: usize = 3;

pub struct SchemeRect {
    pub index: usize,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

pub struct SchemePickerLayout {
    pub rects: Vec<SchemeRect>,
    pub total_h: f32,
}

pub fn scheme_picker_layout(panel_width: f32, start_y: f32) -> SchemePickerLayout {
    let content_w = panel_width - MENU_PADDING * 2.0;
    let col_w = (content_w - SCHEME_COL_GAP * (SCHEME_COLS as f32 - 1.0)) / SCHEME_COLS as f32;
    let mut rects = Vec::new();
    for i in 0..MATUGEN_SCHEMES.len() {
        let col = i % SCHEME_COLS;
        let row = i / SCHEME_COLS;
        let x = MENU_PADDING + col as f32 * (col_w + SCHEME_COL_GAP);
        let ry = start_y + row as f32 * (SCHEME_ROW_H + SCHEME_ROW_GAP);
        rects.push(SchemeRect { index: i, x, y: ry, w: col_w, h: SCHEME_ROW_H });
    }
    let rows = MATUGEN_SCHEMES.len().div_ceil(SCHEME_COLS);
    let total_h = start_y + rows as f32 * SCHEME_ROW_H + rows.saturating_sub(1) as f32 * SCHEME_ROW_GAP;
    SchemePickerLayout { rects, total_h }
}

pub fn scheme_picker_hit_test(control_y: f32, panel_width: f32, x: f32, y: f32) -> Option<usize> {
    let layout = scheme_picker_layout(panel_width, control_y);
    layout.rects.into_iter().find(|r| x >= r.x && x <= r.x + r.w && y >= r.y && y <= r.y + r.h).map(|r| r.index)
}
