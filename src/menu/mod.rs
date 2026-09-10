use crate::config::{DockEdge, DockSettings, WidgetKind, WidgetPlacement, WidgetSlot};

mod settings;
pub use settings::*;
mod controls;
pub use controls::*;
mod app_search;
pub use app_search::*;
mod dock_menu;
pub use dock_menu::*;
mod edge_picker;
pub use edge_picker::*;
mod palette_controls;
pub use palette_controls::*;
mod widget_chips;
pub use widget_chips::*;
mod theme_picker;
pub use theme_picker::*;
mod scheme_picker;
pub use scheme_picker::*;
mod font_dropdown;
pub use font_dropdown::*;
mod wallpaper_picker;
pub use wallpaper_picker::*;
mod align_picker;
pub use align_picker::*;


pub const MENU_WIDTH: f32 = 230.0;
pub const MENU_PADDING: f32 = 10.0;
pub const SECTION_LABEL_HEIGHT: f32 = 16.0;
pub const BUTTON_HEIGHT: f32 = 26.0;
pub const SLIDER_ROW_HEIGHT: f32 = 32.0;
pub const TOGGLE_ROW_HEIGHT: f32 = 24.0;
pub const NOTE_ROW_HEIGHT: f32 = 16.0;
pub const ROW_GAP: f32 = 4.0;
pub const STEP_BTN_SIZE: f32 = 16.0;
pub const APP_ENTRY_HEIGHT: f32 = 30.0;
pub const LIST_ROW_GAP: f32 = 2.0;
pub const LIST_MAX_VISIBLE_ROWS: usize = 8;
pub const TAB_SLIDE_DISTANCE: f32 = 28.0;
pub const WIDGETS_TAB_FIXED_H: f32 = 484.0;
pub const SEARCH_BOX_HEIGHT: f32 = 26.0;
pub const HEX_FIELD_H: f32 = 26.0;
pub const TRAY_ITEM_HEIGHT: f32 = 26.0;
pub const TRAY_SEPARATOR_HEIGHT: f32 = 9.0;
pub const CUSTOM_HEX_LABELS: [&str; 5] = ["Accent", "Accent 2", "Panel", "Text", "Text Dim"];

pub const ANIM_STEP_OPEN: f32 = 0.07;
pub const ANIM_STEP_CLOSE: f32 = 0.05;

pub const OSD_MIN_PANEL_H: f32 = 36.0;
pub const OSD_TIMEOUT_MS: u64 = 1400;
// ----- fixed size -----
pub const OSD_NOTIFICATION_BASE_LEN: f32 = 240.0;
pub const OSD_NOTIFICATION_BASE_THICKNESS: f32 = 44.0;
pub const OSD_GROWTH_W: f32 = 40.0;
pub const OSD_GROWTH_H: f32 = 16.0;
pub const NOTIFICATION_MIN_PANEL_H: f32 = 52.0;
pub const NOTIFICATION_TIMEOUT_MS: u64 = 4000;
pub const NOTIFICATION_GROWTH_W: f32 = 60.0;
pub const NOTIFICATION_GROWTH_H: f32 = 24.0;
pub const NOTIFICATION_BODY_MAX_LINES: usize = 8;
pub const APP_SEARCH_WIDTH_GROWTH: f32 = 180.0;
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OsdKind {
    Volume,
    Brightness,
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ButtonKind {
    AddApp,
    QuitDock,
    ChangeIcon,
    RemoveApp,
    Back,
    Suspend,
    Logout,
    Reboot,
    Shutdown,
    CreatePalette,
    SavePalette,
}
impl ButtonKind {
    pub fn label(self) -> &'static str {
        match self {
            ButtonKind::AddApp => "Add App…",
            ButtonKind::QuitDock => "Quit Dock",
            ButtonKind::ChangeIcon => "Change Icon…",
            ButtonKind::RemoveApp => "Remove App from Dock",
            ButtonKind::Back => "‹ Back",
            ButtonKind::Suspend => "Suspend",
            ButtonKind::Logout => "Log Out",
            ButtonKind::Reboot => "Reboot",
            ButtonKind::Shutdown => "Shut Down",
            ButtonKind::CreatePalette => "Create Your Own Palette!",
            ButtonKind::SavePalette => "Save Palette",
        }
    }

    pub fn is_destructive(self) -> bool {
        matches!(self, ButtonKind::QuitDock | ButtonKind::RemoveApp | ButtonKind::Reboot | ButtonKind::Shutdown)
    }
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MenuScreen {
    AddApp,
    IconMenu(usize),
    IconPicker(usize),
    WallpaperPicker,
    PowerMenu,
    TrayMenu,
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MenuCategory {
    Layout,
    Appearance,
    Colors,
    Widgets,
    System,
    // ----- palette button only -----
    CustomPalette,
}
pub const MENU_CATEGORIES: [MenuCategory; 5] = [
    MenuCategory::Layout,
    MenuCategory::Appearance,
    MenuCategory::Colors,
    MenuCategory::Widgets,
    MenuCategory::System,
];
pub fn category_order_index(c: MenuCategory) -> i32 {
    match c {
        MenuCategory::Layout => 0,
        MenuCategory::Appearance => 1,
        MenuCategory::Colors => 2,
        MenuCategory::Widgets => 3,
        MenuCategory::System => 4,
        MenuCategory::CustomPalette => 5,
    }
}
impl MenuCategory {
    pub fn label(self) -> &'static str {
        match self {
            MenuCategory::Layout => "Layout",
            MenuCategory::Appearance => "Appearance",
            MenuCategory::Colors => "Themes",
            MenuCategory::Widgets => "Widgets",
            MenuCategory::System => "System",
            MenuCategory::CustomPalette => "Themes",
        }
    }
}
#[derive(Clone, Copy)]
pub enum ControlKind {
    Section(&'static str),
    Note(&'static str),
    IconHeader(usize),
    Slider(SettingId),
    Toggle(SettingId),
    Button(ButtonKind),
    AppEntry(usize),
    SearchBox,
    IconChoice(usize),
    EdgePicker,
    WidgetChips,
    ThemePicker,
    SchemeDropdown,
    DockFontDropdown,
    SystemFontDropdown,
    HexField(usize),
    NameField,
    PaletteModePicker,
    PanelBlendPicker,
    TrayItem(usize),
    TraySeparator,
    AlignPicker,
}
#[derive(Clone, Copy)]
pub struct Control {
    pub kind: ControlKind,
    pub y: f32,
    pub height: f32,
}
pub fn slider_geometry(control_y: f32) -> (f32, f32, f32, f32, f32) {
    let track_row_y = control_y + 14.0;
    let minus_x = MENU_PADDING;
    let plus_x = MENU_WIDTH - MENU_PADDING - STEP_BTN_SIZE;
    let track_x0 = minus_x + STEP_BTN_SIZE + 6.0;
    let track_x1 = plus_x - 6.0;
    (minus_x, track_x0, track_x1, plus_x, track_row_y)
}
pub fn parse_hex(s: &str) -> Option<(u8, u8, u8)> {
    let s = s.strip_prefix('#').unwrap_or(s);
    if s.len() == 3 {
        let mut chars = s.chars();
        let r = chars.next()?;
        let g = chars.next()?;
        let b = chars.next()?;
        let r = u8::from_str_radix(&r.to_string().repeat(2), 16).ok()?;
        let g = u8::from_str_radix(&g.to_string().repeat(2), 16).ok()?;
        let b = u8::from_str_radix(&b.to_string().repeat(2), 16).ok()?;
        return Some((r, g, b));
    }
    if s.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&s[0..2], 16).ok()?;
    let g = u8::from_str_radix(&s[2..4], 16).ok()?;
    let b = u8::from_str_radix(&s[4..6], 16).ok()?;
    Some((r, g, b))
}
pub fn slider_value_from_x(id: SettingId, x: f32, control_y: f32) -> f32 {
    let (_, track_x0, track_x1, _, _) = slider_geometry(control_y);
    let (min, max, _) = id.range();
    let t = ((x - track_x0) / (track_x1 - track_x0).max(1.0)).clamp(0.0, 1.0);
    min + t * (max - min)
}
#[derive(Clone, Copy, Debug)]
pub enum HitTarget {
    Toggle(SettingId),
    SliderTrack(SettingId),
    SliderMinus(SettingId),
    SliderPlus(SettingId),
    Button(ButtonKind),
    AppEntry(usize),
    SearchBox,
    IconChoice(usize),
    Tab(MenuCategory),
    Edge(DockEdge),
    WidgetChip(WidgetKind),
    ThemeOption(Option<usize>),
    SchemeOption(usize),
    SchemeDropdownToggle,
    DockFontDropdownToggle,
    SystemFontDropdownToggle,
    FontOption(usize),
    HexFieldFocus(usize),
    NameFieldFocus,
    PaletteMode(bool),
    PanelBlend(u8),
    DeleteCustomPalette(usize),
    TrayItem(usize),
    Align(crate::config::DockAlign),
}
pub const DOCK_MENU_LEFT_COL_W: f32 = 120.0;
pub const DOCK_MENU_DIVIDER_W: f32 = 1.0;
pub const DOCK_MENU_TAB_H: f32 = 32.0;
pub const DOCK_MENU_RIGHT_COL_W: f32 = 400.0;
pub fn dock_menu_tab_hit_test(x: f32, y: f32) -> Option<HitTarget> {
    if x >= DOCK_MENU_LEFT_COL_W {
        return None;
    }
    let index = ((y - MENU_PADDING) / DOCK_MENU_TAB_H) as usize;
    MENU_CATEGORIES.get(index).map(|&c| HitTarget::Tab(c))
}
pub fn hit_test(controls: &[Control], settings: &DockSettings, panel_width: f32, x: f32, y: f32) -> Option<HitTarget> {
    for c in controls {
        if y < c.y || y > c.y + c.height {
            continue;
        }
        return match c.kind {
            ControlKind::Toggle(id) => Some(HitTarget::Toggle(id)),
            ControlKind::Slider(id) => {
                let (minus_x, _, _, plus_x, _) = slider_geometry(c.y);
                if x >= minus_x && x <= minus_x + STEP_BTN_SIZE {
                    Some(HitTarget::SliderMinus(id))
                } else if x >= plus_x && x <= plus_x + STEP_BTN_SIZE {
                    Some(HitTarget::SliderPlus(id))
                } else {
                    Some(HitTarget::SliderTrack(id))
                }
            }
            ControlKind::Button(b) => Some(HitTarget::Button(b)),
            ControlKind::AppEntry(i) => Some(HitTarget::AppEntry(i)),
            ControlKind::IconChoice(i) => Some(HitTarget::IconChoice(i)),
            ControlKind::SearchBox => Some(HitTarget::SearchBox),
            ControlKind::EdgePicker => edge_picker_hit_test(c.y, panel_width, x, y).map(HitTarget::Edge),
            ControlKind::WidgetChips => widget_chip_hit_test(settings, c.y, panel_width, x, y).map(HitTarget::WidgetChip),
            ControlKind::ThemePicker => theme_picker_hit_test(settings, c.y, panel_width, x, y).map(|hit| match hit {
                ThemeGridHit::Select(choice) => HitTarget::ThemeOption(choice),
                ThemeGridHit::Delete(i) => HitTarget::DeleteCustomPalette(i),
            }),
            ControlKind::SchemeDropdown => Some(HitTarget::SchemeDropdownToggle),
            ControlKind::DockFontDropdown => Some(HitTarget::DockFontDropdownToggle),
            ControlKind::SystemFontDropdown => Some(HitTarget::SystemFontDropdownToggle),
            ControlKind::HexField(i) => Some(HitTarget::HexFieldFocus(i)),
            ControlKind::NameField => Some(HitTarget::NameFieldFocus),
            ControlKind::PaletteModePicker => palette_mode_hit_test(c.y, panel_width, x, y).map(HitTarget::PaletteMode),
            ControlKind::PanelBlendPicker => panel_blend_hit_test(c.y, panel_width, x, y).map(HitTarget::PanelBlend),
            ControlKind::TrayItem(i) => Some(HitTarget::TrayItem(i)),
            ControlKind::AlignPicker => align_hit_test(c.y, panel_width, x, y).map(HitTarget::Align),
            ControlKind::Section(_) | ControlKind::Note(_) | ControlKind::IconHeader(_) | ControlKind::TraySeparator => None,
        };
    }
    None
}
pub fn stepper_speed(hold_frames: u32) -> f32 {
    match hold_frames {
        0..=14 => 1.0,
        15..=44 => 3.0,
        45..=89 => 8.0,
        _ => 18.0,
    }
}
