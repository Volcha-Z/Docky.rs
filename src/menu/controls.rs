use super::*;

pub(super) const LAYOUT_SETTINGS: [SettingId; 6] = [
    SettingId::DockScale,
    SettingId::IconSize,
    SettingId::IconGap,
    SettingId::WidthPadding,
    SettingId::PosY,
    SettingId::CornerRadius,
];

pub(super) const APPEARANCE_SETTINGS: [SettingId; 9] = [
    SettingId::BorderWidth,
    SettingId::Transparency,
    SettingId::BlurEnabled,
    SettingId::BlurPasses,
    SettingId::BlurSize,
    SettingId::BlurVibrancy,
    SettingId::BlurBrightness,
    SettingId::BlurContrast,
    SettingId::BlurXray,
];

fn row_height(id: SettingId) -> f32 {
    if id.is_toggle() { TOGGLE_ROW_HEIGHT } else { SLIDER_ROW_HEIGHT }
}

pub(super) fn push_setting_row(controls: &mut Vec<Control>, y: &mut f32, id: SettingId) {
    let kind = if id.is_toggle() { ControlKind::Toggle(id) } else { ControlKind::Slider(id) };
    let h = row_height(id);
    controls.push(Control { kind, y: *y, height: h });
    *y += h + ROW_GAP;
}

pub(super) fn push_note_row(controls: &mut Vec<Control>, y: &mut f32, text: &'static str) {
    controls.push(Control { kind: ControlKind::Note(text), y: *y, height: NOTE_ROW_HEIGHT });
    *y += NOTE_ROW_HEIGHT + ROW_GAP;
}

pub fn list_window(item_count: usize, scroll: usize) -> (usize, usize) {
    let visible = LIST_MAX_VISIBLE_ROWS.min(item_count);
    let max_scroll = item_count.saturating_sub(visible);
    let first = scroll.min(max_scroll);
    (first, visible)
}

pub fn build_controls(screen: MenuScreen, list_count: usize, scroll: usize) -> (Vec<Control>, f32) {
    let mut controls = Vec::new();
    let mut y = MENU_PADDING;

    match screen {
        MenuScreen::AddApp => {
            controls.push(Control { kind: ControlKind::Button(ButtonKind::Back), y, height: BUTTON_HEIGHT });
            y += BUTTON_HEIGHT + ROW_GAP;
            controls.push(Control { kind: ControlKind::SearchBox, y, height: SEARCH_BOX_HEIGHT });
            y += SEARCH_BOX_HEIGHT + ROW_GAP;
            let (first, visible) = list_window(list_count, scroll);
            for row in 0..visible {
                controls.push(Control { kind: ControlKind::AppEntry(first + row), y, height: APP_ENTRY_HEIGHT });
                y += APP_ENTRY_HEIGHT + LIST_ROW_GAP;
            }
        }
        MenuScreen::IconMenu(index) => {
            controls.push(Control { kind: ControlKind::IconHeader(index), y, height: SECTION_LABEL_HEIGHT });
            y += SECTION_LABEL_HEIGHT + 4.0;
            controls.push(Control { kind: ControlKind::Button(ButtonKind::ChangeIcon), y, height: BUTTON_HEIGHT });
            y += BUTTON_HEIGHT + ROW_GAP;
            controls.push(Control { kind: ControlKind::Button(ButtonKind::RemoveApp), y, height: BUTTON_HEIGHT });
            y += BUTTON_HEIGHT;
        }
        MenuScreen::IconPicker(_) => {
            controls.push(Control { kind: ControlKind::Button(ButtonKind::Back), y, height: BUTTON_HEIGHT });
            y += BUTTON_HEIGHT + ROW_GAP;
            controls.push(Control { kind: ControlKind::SearchBox, y, height: SEARCH_BOX_HEIGHT });
            y += SEARCH_BOX_HEIGHT + ROW_GAP;
            let (first, visible) = list_window(list_count, scroll);
            for row in 0..visible {
                controls.push(Control { kind: ControlKind::IconChoice(first + row), y, height: APP_ENTRY_HEIGHT });
                y += APP_ENTRY_HEIGHT + LIST_ROW_GAP;
            }
        }
        MenuScreen::WallpaperPicker => {}
        MenuScreen::PowerMenu => {
            for kind in [ButtonKind::Suspend, ButtonKind::Logout, ButtonKind::Reboot, ButtonKind::Shutdown] {
                controls.push(Control { kind: ControlKind::Button(kind), y, height: BUTTON_HEIGHT });
                y += BUTTON_HEIGHT + ROW_GAP;
            }
        }
        // ----- see tray menu builder -----
        MenuScreen::TrayMenu => {}
    }

    y += MENU_PADDING;
    (controls, y)
}

pub fn build_tray_menu_controls(items: &[crate::tray::TrayMenuItem], has_back: bool) -> (Vec<Control>, f32) {
    let mut controls = Vec::new();
    let mut y = MENU_PADDING * 0.6;
    if has_back {
        controls.push(Control { kind: ControlKind::Button(ButtonKind::Back), y, height: BUTTON_HEIGHT });
        y += BUTTON_HEIGHT + ROW_GAP;
    }
    for (i, item) in items.iter().enumerate() {
        if item.is_separator {
            controls.push(Control { kind: ControlKind::TraySeparator, y, height: TRAY_SEPARATOR_HEIGHT });
            y += TRAY_SEPARATOR_HEIGHT;
        } else {
            controls.push(Control { kind: ControlKind::TrayItem(i), y, height: TRAY_ITEM_HEIGHT });
            y += TRAY_ITEM_HEIGHT;
        }
    }
    y += MENU_PADDING * 0.6;
    (controls, y)
}
