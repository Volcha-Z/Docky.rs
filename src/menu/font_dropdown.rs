use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum OpenDropdown {
    None,
    Scheme,
    DockFont,
    SystemFont,
}

pub const FONT_DROPDOWN_H: f32 = 32.0;
pub const FONT_ROW_H: f32 = 22.0;
pub const FONT_LIST_MAX_ROWS: usize = 10;

// ----- visible window -----
pub fn font_list_window(item_count: usize, scroll_rows: usize) -> (usize, usize) {
    let visible = FONT_LIST_MAX_ROWS.min(item_count);
    let max_scroll = item_count.saturating_sub(visible);
    let first = scroll_rows.min(max_scroll);
    (first, visible)
}

pub fn font_list_hit_test(item_count: usize, panel_width: f32, start_y: f32, scroll_rows: usize, x: f32, y: f32) -> Option<usize> {
    let (first, visible) = font_list_window(item_count, scroll_rows);
    if x < MENU_PADDING || x > panel_width - MENU_PADDING {
        return None;
    }
    for row in 0..visible {
        let ry = start_y + row as f32 * FONT_ROW_H;
        if y >= ry && y <= ry + FONT_ROW_H {
            return Some(first + row);
        }
    }
    None
}

pub fn filter_font_indices(fonts: &[String], query: &str) -> Vec<usize> {
    if query.is_empty() {
        return (0..fonts.len()).collect();
    }
    let q = query.to_lowercase();
    fonts.iter().enumerate().filter(|(_, f)| f.to_lowercase().contains(&q)).map(|(i, _)| i).collect()
}
