use super::*;

pub fn build_category_controls(category: MenuCategory, settings: &DockSettings) -> Vec<Control> {
    let mut controls = Vec::new();
    let mut y = MENU_PADDING;
    match category {
        MenuCategory::Layout => {
            controls.push(Control { kind: ControlKind::EdgePicker, y, height: EDGE_PICKER_H });
            y += EDGE_PICKER_H + ROW_GAP;
            controls.push(Control { kind: ControlKind::AlignPicker, y, height: ALIGN_PICKER_H });
            y += ALIGN_PICKER_H + ROW_GAP;
            for id in LAYOUT_SETTINGS {
                push_setting_row(&mut controls, &mut y, id);
            }
        }
        MenuCategory::Appearance => {
            for id in APPEARANCE_SETTINGS {
                push_setting_row(&mut controls, &mut y, id);
            }
            y += ROW_GAP * 2.0;
            controls.push(Control { kind: ControlKind::Section("Dock Font"), y, height: SECTION_LABEL_HEIGHT });
            y += SECTION_LABEL_HEIGHT;
            controls.push(Control { kind: ControlKind::DockFontDropdown, y, height: FONT_DROPDOWN_H });
            y += FONT_DROPDOWN_H + ROW_GAP * 2.0;

            controls.push(Control { kind: ControlKind::Section("System Font"), y, height: SECTION_LABEL_HEIGHT });
            y += SECTION_LABEL_HEIGHT;
            controls.push(Control { kind: ControlKind::SystemFontDropdown, y, height: FONT_DROPDOWN_H });
        }
        MenuCategory::Colors => {
            controls.push(Control { kind: ControlKind::Section("Matugen Style"), y, height: SECTION_LABEL_HEIGHT });
            y += SECTION_LABEL_HEIGHT;
            controls.push(Control { kind: ControlKind::SchemeDropdown, y, height: SCHEME_DROPDOWN_H });
            y += SCHEME_DROPDOWN_H + ROW_GAP * 2.0;

            controls.push(Control { kind: ControlKind::Section("Color Theme"), y, height: SECTION_LABEL_HEIGHT });
            y += SECTION_LABEL_HEIGHT;
            let theme_h = theme_picker_layout(settings, DOCK_MENU_RIGHT_COL_W, y).total_h - y;
            controls.push(Control { kind: ControlKind::ThemePicker, y, height: theme_h });
            y += theme_h + ROW_GAP * 2.0;

            controls.push(Control { kind: ControlKind::Button(ButtonKind::CreatePalette), y, height: BUTTON_HEIGHT });
        }
        MenuCategory::CustomPalette => {
            controls.push(Control { kind: ControlKind::Button(ButtonKind::Back), y, height: BUTTON_HEIGHT });
            y += BUTTON_HEIGHT + ROW_GAP * 2.0;

            controls.push(Control { kind: ControlKind::Section("Create Your Own Palette"), y, height: SECTION_LABEL_HEIGHT });
            y += SECTION_LABEL_HEIGHT;

            controls.push(Control { kind: ControlKind::NameField, y, height: HEX_FIELD_H });
            y += HEX_FIELD_H + ROW_GAP * 2.0;

            controls.push(Control { kind: ControlKind::Section("Select Palette Mode"), y, height: SECTION_LABEL_HEIGHT });
            y += SECTION_LABEL_HEIGHT;
            controls.push(Control { kind: ControlKind::PaletteModePicker, y, height: PALETTE_MODE_PICKER_H });
            y += PALETTE_MODE_PICKER_H + ROW_GAP * 2.0;

            for i in 0..CUSTOM_HEX_LABELS.len() {
                controls.push(Control { kind: ControlKind::HexField(i), y, height: HEX_FIELD_H });
                y += HEX_FIELD_H + ROW_GAP;
                if i == 1 {
                    y += ROW_GAP;
                    controls.push(Control { kind: ControlKind::Section("Panel Blend (base-accent)"), y, height: SECTION_LABEL_HEIGHT });
                    y += SECTION_LABEL_HEIGHT;
                    controls.push(Control { kind: ControlKind::PanelBlendPicker, y, height: PANEL_BLEND_PICKER_H });
                    y += PANEL_BLEND_PICKER_H + ROW_GAP * 2.0;
                }
            }
            y += ROW_GAP;

            controls.push(Control { kind: ControlKind::Section("if you're unsure of what colors you put visit"), y, height: SECTION_LABEL_HEIGHT });
            y += SECTION_LABEL_HEIGHT;
            controls.push(Control { kind: ControlKind::Section("https://coolors.co/ :D"), y, height: SECTION_LABEL_HEIGHT });
            y += SECTION_LABEL_HEIGHT + ROW_GAP * 2.0;

            controls.push(Control { kind: ControlKind::Button(ButtonKind::SavePalette), y, height: BUTTON_HEIGHT });
        }
        MenuCategory::Widgets => {
            push_setting_row(&mut controls, &mut y, SettingId::WidgetScale);
            y += ROW_GAP * 2.0;
            let layout = widget_chip_layout(settings, DOCK_MENU_RIGHT_COL_W, y);
            controls.push(Control { kind: ControlKind::WidgetChips, y, height: layout.total_h - y });
            y = layout.total_h + ROW_GAP * 2.0;
            push_setting_row(&mut controls, &mut y, SettingId::MediaSmoothScroll);
            push_note_row(&mut controls, &mut y, "turning this off might help if ur seeking 0 cpu usage><");
            push_setting_row(&mut controls, &mut y, SettingId::MediaWidthScale);
            push_note_row(&mut controls, &mut y, "you might wanna be careful wtih this ... ");
        }
        MenuCategory::System => {
            controls.push(Control { kind: ControlKind::Button(ButtonKind::AddApp), y, height: BUTTON_HEIGHT });
            y += BUTTON_HEIGHT + ROW_GAP;
            controls.push(Control { kind: ControlKind::Button(ButtonKind::QuitDock), y, height: BUTTON_HEIGHT });
        }
    }
    controls
}

pub fn dock_menu_content_height(category: MenuCategory, settings: &DockSettings) -> f32 {
    if category == MenuCategory::Widgets {
        return WIDGETS_TAB_FIXED_H;
    }
    let controls = build_category_controls(category, settings);
    let natural = controls.last().map(|c| c.y + c.height).unwrap_or(0.0);
    // ----- dropdown clipping -----
    let dropdown_bottom = controls
        .iter()
        .filter_map(|ctl| {
            let overlay_h = match ctl.kind {
                ControlKind::SchemeDropdown => scheme_picker_layout(DOCK_MENU_RIGHT_COL_W, 0.0).total_h,
                ControlKind::DockFontDropdown | ControlKind::SystemFontDropdown => FONT_LIST_MAX_ROWS as f32 * FONT_ROW_H,
                _ => return None,
            };
            Some(ctl.y + ctl.height + 4.0 + overlay_h)
        })
        .fold(0.0_f32, f32::max);
    let sidebar_min = MENU_PADDING * 2.0 + MENU_CATEGORIES.len() as f32 * DOCK_MENU_TAB_H;
    natural.max(dropdown_bottom).max(sidebar_min) + MENU_PADDING
}
