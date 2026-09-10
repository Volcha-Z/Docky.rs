use super::*;

pub const SEARCH_MAX_RESULTS: usize = 60;

pub const APP_CARD_W: f32 = 92.0;
pub const APP_CARD_GAP: f32 = 10.0;
pub const APP_CARD_ICON: f32 = 46.0;
pub const APP_CARD_ROW_H: f32 = 78.0;
pub const APP_SEARCH_VERTICAL_VISIBLE: f32 = 5.0;

pub fn build_app_search_controls() -> (Vec<Control>, f32) {
    let controls = vec![Control { kind: ControlKind::SearchBox, y: MENU_PADDING, height: SEARCH_BOX_HEIGHT }];
    let fixed_h = MENU_PADDING + SEARCH_BOX_HEIGHT + ROW_GAP + APP_CARD_ROW_H + MENU_PADDING;
    (controls, fixed_h)
}

pub fn app_search_vertical_cross(dock_thickness: f32) -> f32 {
    dock_thickness.max(APP_CARD_ICON + MENU_PADDING * 2.0)
}

// ----- fixed viewport -----
pub fn app_search_vertical_along() -> f32 {
    let content = APP_SEARCH_VERTICAL_VISIBLE * (APP_CARD_W + APP_CARD_GAP) - APP_CARD_GAP;
    MENU_PADDING + SEARCH_BOX_HEIGHT + ROW_GAP + content + MENU_PADDING
}

pub fn app_search_strip_y() -> f32 {
    MENU_PADDING + SEARCH_BOX_HEIGHT + ROW_GAP
}

pub fn app_search_strip_content_len(count: usize) -> f32 {
    (count as f32 * (APP_CARD_W + APP_CARD_GAP) - APP_CARD_GAP).max(0.0)
}

pub fn app_search_strip_max_scroll(count: usize, viewport_along: f32) -> f32 {
    (app_search_strip_content_len(count) - viewport_along).max(0.0)
}

pub fn app_search_card_along(index: usize) -> f32 {
    index as f32 * (APP_CARD_W + APP_CARD_GAP)
}

pub fn app_search_viewport_along(panel_w: f32, panel_h: f32, is_vertical: bool) -> f32 {
    if is_vertical {
        panel_h - app_search_strip_y()
    } else {
        panel_w
    }
}

pub fn app_search_strip_hit_test(count: usize, panel_w: f32, panel_h: f32, is_vertical: bool, scroll: f32, x: f32, y: f32) -> Option<usize> {
    let row_y = app_search_strip_y();
    let local_along = if is_vertical {
        if x < 0.0 || x > panel_w || y < row_y || y > panel_h {
            return None;
        }
        y - row_y + scroll
    } else {
        if x < 0.0 || x > panel_w || y < row_y || y > row_y + APP_CARD_ROW_H {
            return None;
        }
        x + scroll
    };
    for i in 0..count {
        let c = app_search_card_along(i);
        if local_along >= c && local_along <= c + APP_CARD_W {
            return Some(i);
        }
    }
    None
}
