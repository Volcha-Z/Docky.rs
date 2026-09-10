use super::*;

pub struct ThemePreset {
    pub name: &'static str,
    pub swatches: [(u8, u8, u8); 5],
    pub is_light: bool,
}

pub const THEME_PRESETS: &[ThemePreset] = &[
    // ----- accent swatch -----
    ThemePreset { name: "Catppuccin Mocha", swatches: [(203, 166, 247), (137, 180, 250), (250, 179, 135), (166, 227, 161), (245, 194, 231)], is_light: false },
    ThemePreset { name: "Dracula", swatches: [(189, 147, 249), (255, 121, 198), (139, 233, 253), (255, 184, 108), (80, 250, 123)], is_light: false },
    ThemePreset { name: "Nord", swatches: [(94, 129, 172), (129, 161, 193), (208, 135, 112), (163, 190, 140), (191, 97, 106)], is_light: false },
    ThemePreset { name: "Tokyo Night", swatches: [(122, 162, 247), (187, 154, 247), (255, 158, 100), (158, 206, 106), (247, 118, 142)], is_light: false },
    ThemePreset { name: "Gruvbox Dark", swatches: [(131, 165, 152), (254, 128, 25), (250, 189, 47), (251, 73, 52), (211, 134, 155)], is_light: false },
    ThemePreset { name: "Monokai", swatches: [(249, 38, 114), (166, 226, 46), (102, 217, 239), (174, 129, 255), (253, 151, 31)], is_light: false },
    ThemePreset { name: "One Dark", swatches: [(97, 175, 239), (224, 108, 117), (198, 120, 221), (209, 154, 102), (152, 195, 121)], is_light: false },
    ThemePreset { name: "Rose Pine", swatches: [(235, 188, 186), (196, 167, 231), (246, 193, 119), (49, 116, 143), (235, 111, 146)], is_light: false },
    ThemePreset { name: "Solarized Dark", swatches: [(42, 161, 152), (38, 139, 210), (108, 113, 196), (203, 75, 22), (133, 153, 0)], is_light: false },
    ThemePreset { name: "Kanagawa", swatches: [(149, 127, 184), (126, 156, 216), (255, 160, 102), (152, 187, 108), (195, 64, 67)], is_light: false },
    ThemePreset { name: "Gruvbox Light", swatches: [(7, 102, 120), (143, 63, 113), (175, 58, 3), (121, 116, 14), (157, 0, 6)], is_light: true },
    ThemePreset { name: "Catppuccin Latte", swatches: [(136, 57, 239), (30, 102, 245), (254, 100, 11), (64, 160, 43), (234, 118, 203)], is_light: true },
    ThemePreset { name: "Solarized Light", swatches: [(42, 161, 152), (38, 139, 210), (108, 113, 196), (203, 75, 22), (133, 153, 0)], is_light: true },
    ThemePreset { name: "One Light", swatches: [(64, 120, 242), (80, 161, 79), (166, 38, 164), (152, 104, 1), (228, 86, 73)], is_light: true },
    ThemePreset { name: "Rose Pine Dawn", swatches: [(180, 99, 122), (144, 122, 169), (234, 157, 52), (40, 105, 131), (86, 148, 159)], is_light: true },
    ThemePreset { name: "Everforest Light", swatches: [(141, 161, 1), (53, 167, 124), (223, 160, 0), (245, 125, 38), (248, 85, 82)], is_light: true },
    ThemePreset { name: "Ayu Light", swatches: [(250, 141, 62), (57, 158, 230), (76, 191, 153), (134, 179, 0), (245, 24, 24)], is_light: true },
    ThemePreset { name: "GitHub Light", swatches: [(9, 105, 218), (130, 80, 223), (188, 76, 0), (26, 127, 55), (207, 34, 38)], is_light: true },
    ThemePreset { name: "Tokyo Night Day", swatches: [(46, 125, 233), (152, 84, 241), (177, 92, 0), (88, 117, 57), (245, 42, 101)], is_light: true },
    ThemePreset { name: "PaperColor Light", swatches: [(0, 135, 175), (135, 0, 175), (215, 95, 0), (95, 135, 0), (215, 0, 0)], is_light: true },
];

pub const THEME_MATUGEN_H: f32 = 40.0;
pub const THEME_ROW_H: f32 = 40.0;
pub const THEME_ROW_GAP: f32 = 8.0;
pub const THEME_COL_GAP: f32 = 8.0;
pub const THEME_COLS: usize = 3;
pub const THEME_DOT_D: f32 = 7.0;

pub struct ThemeRect {
    pub choice: Option<usize>,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

pub struct ThemePickerLayout {
    pub rects: Vec<ThemeRect>,
    pub total_h: f32,
}

// ----- builtins first -----
pub fn total_palette_count(settings: &DockSettings) -> usize {
    THEME_PRESETS.len() + settings.custom_palettes.len()
}

pub fn palette_name<'a>(settings: &'a DockSettings, index: usize) -> Option<&'a str> {
    if index < THEME_PRESETS.len() {
        THEME_PRESETS.get(index).map(|p| p.name)
    } else {
        settings.custom_palettes.get(index - THEME_PRESETS.len()).map(|c| c.name.as_str())
    }
}

pub fn palette_swatches(settings: &DockSettings, index: usize) -> Option<[(u8, u8, u8); 5]> {
    if index < THEME_PRESETS.len() {
        THEME_PRESETS.get(index).map(|p| p.swatches)
    } else {
        settings.custom_palettes.get(index - THEME_PRESETS.len()).map(|c| [c.accent, c.accent2, c.panel, c.text, c.text_dim])
    }
}

pub fn theme_picker_layout(settings: &DockSettings, panel_width: f32, start_y: f32) -> ThemePickerLayout {
    let content_w = panel_width - MENU_PADDING * 2.0;
    let mut y = start_y;
    let mut rects = vec![ThemeRect { choice: None, x: MENU_PADDING, y, w: content_w, h: THEME_MATUGEN_H }];
    y += THEME_MATUGEN_H + THEME_ROW_GAP;

    let count = total_palette_count(settings);
    let col_w = (content_w - THEME_COL_GAP * (THEME_COLS as f32 - 1.0)) / THEME_COLS as f32;
    for i in 0..count {
        let col = i % THEME_COLS;
        let row = i / THEME_COLS;
        let x = MENU_PADDING + col as f32 * (col_w + THEME_COL_GAP);
        let ry = y + row as f32 * (THEME_ROW_H + THEME_ROW_GAP);
        rects.push(ThemeRect { choice: Some(i), x, y: ry, w: col_w, h: THEME_ROW_H });
    }
    let rows = count.div_ceil(THEME_COLS);
    y += rows as f32 * THEME_ROW_H + rows.saturating_sub(1) as f32 * THEME_ROW_GAP;

    ThemePickerLayout { rects, total_h: y }
}

pub enum ThemeGridHit {
    Select(Option<usize>),
    // ----- builtin offset -----
    Delete(usize),
}

pub const THEME_DELETE_W: f32 = 40.0;
pub const THEME_DELETE_H: f32 = 14.0;

pub fn theme_delete_button_rect(r: &ThemeRect) -> (f32, f32, f32, f32) {
    (r.x + r.w - THEME_DELETE_W - 3.0, r.y + 3.0, THEME_DELETE_W, THEME_DELETE_H)
}

pub fn theme_picker_hit_test(settings: &DockSettings, control_y: f32, panel_width: f32, x: f32, y: f32) -> Option<ThemeGridHit> {
    let layout = theme_picker_layout(settings, panel_width, control_y);
    let r = layout.rects.into_iter().find(|r| x >= r.x && x <= r.x + r.w && y >= r.y && y <= r.y + r.h)?;
    if let Some(i) = r.choice {
        if i >= THEME_PRESETS.len() {
            let (dx, dy, dw, dh) = theme_delete_button_rect(&r);
            if x >= dx && x <= dx + dw && y >= dy && y <= dy + dh {
                return Some(ThemeGridHit::Delete(i - THEME_PRESETS.len()));
            }
        }
    }
    Some(ThemeGridHit::Select(r.choice))
}
