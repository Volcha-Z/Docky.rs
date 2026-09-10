use super::*;

impl App {
    pub(super) fn handle_dock_menu_click(&mut self, hit: Option<menu::HitTarget>, right_x: f32, qh: &QueueHandle<Self>) {
        use menu::HitTarget;
        match hit {
            Some(HitTarget::Tab(category)) => self.switch_dock_menu_category(category, qh),
            Some(HitTarget::Toggle(id)) => {
                id.toggle(&mut self.dock.config.settings);
                self.on_setting_changed(id, qh, true);
                self.request_redraw(qh);
            }
            Some(HitTarget::SliderTrack(id)) => {
                let cy = self.dock_menu_mode.as_ref().and_then(|dm| {
                    dm.controls.iter().find_map(|c| match c.kind {
                        menu::ControlKind::Slider(sid) if sid == id => Some(c.y),
                        _ => None,
                    })
                });
                if let Some(cy) = cy {
                    let v = menu::slider_value_from_x(id, right_x, cy);
                    id.set(&mut self.dock.config.settings, v);
                    self.on_setting_changed(id, qh, true);
                }
                if let Some(dm) = self.dock_menu_mode.as_mut() {
                    dm.dragging_slider = Some(id);
                }
                self.request_redraw(qh);
            }
            Some(HitTarget::SliderMinus(id)) => self.start_dock_menu_stepper(id, -1.0, qh),
            Some(HitTarget::SliderPlus(id)) => self.start_dock_menu_stepper(id, 1.0, qh),
            Some(HitTarget::Button(kind)) => self.handle_dock_menu_button(kind, qh),
            Some(HitTarget::Edge(edge)) => self.choose_dock_edge(edge, qh),
            Some(HitTarget::ThemeOption(choice)) => self.choose_theme(choice, qh),
            Some(HitTarget::SchemeDropdownToggle) => {
                self.toggle_dock_menu_dropdown(menu::OpenDropdown::Scheme, qh);
            }
            Some(HitTarget::DockFontDropdownToggle) => {
                self.toggle_dock_menu_dropdown(menu::OpenDropdown::DockFont, qh);
            }
            Some(HitTarget::SystemFontDropdownToggle) => {
                self.toggle_dock_menu_dropdown(menu::OpenDropdown::SystemFont, qh);
            }
            Some(HitTarget::WidgetChip(_)) => {}
            Some(HitTarget::HexFieldFocus(i)) => {
                if let Some(dm) = self.dock_menu_mode.as_mut() {
                    dm.custom_focus = Some(i);
                    dm.custom_name_focused = false;
                }
                self.request_redraw(qh);
            }
            Some(HitTarget::NameFieldFocus) => {
                if let Some(dm) = self.dock_menu_mode.as_mut() {
                    dm.custom_name_focused = true;
                    dm.custom_focus = None;
                }
                self.request_redraw(qh);
            }
            Some(HitTarget::PaletteMode(is_light)) => {
                if let Some(dm) = self.dock_menu_mode.as_mut() {
                    dm.custom_light = is_light;
                }
                self.reseed_custom_palette_base(is_light);
                self.request_redraw(qh);
            }
            Some(HitTarget::PanelBlend(accent_pct)) => {
                self.apply_panel_blend(accent_pct, qh);
            }
            Some(HitTarget::DeleteCustomPalette(i)) => self.delete_custom_palette(i, qh),
            Some(HitTarget::Align(align)) => self.choose_dock_align(align, qh),
            _ => {}
        }
    }

    pub(super) fn drop_widget_chip(&mut self, kind: crate::config::WidgetKind, x: f32, y: f32, qh: &QueueHandle<Self>) {
        let chips_y = self
            .dock_menu_mode
            .as_ref()
            .and_then(|dm| dm.controls.iter().find_map(|c| matches!(c.kind, menu::ControlKind::WidgetChips).then_some(c.y)))
            .unwrap_or(menu::MENU_PADDING);
        let target = menu::widget_drop_target(&self.dock.config.settings, chips_y, menu::DOCK_MENU_RIGHT_COL_W, x, y);
        menu::place_widget(&mut self.dock.config.settings, kind, target);
        let _ = self.dock.config.save();
        if let Some(dm) = self.dock_menu_mode.as_mut() {
            dm.controls = menu::build_category_controls(dm.category, &self.dock.config.settings);
        }
        // ----- deferred to close -----
        self.sync_widget_bar_len();
        self.request_redraw(qh);
    }

    pub(super) fn choose_dock_edge(&mut self, edge: crate::config::DockEdge, qh: &QueueHandle<Self>) {
        self.dock.config.settings.dock_edge = edge;
        let _ = self.dock.config.save();
        let s = &self.dock.config.settings;
        let (anchor, margin) = edge_anchor_margin(s.dock_edge, s.dock_align, s.pos_y, 0);
        self.layer.set_anchor(anchor);
        self.layer.set_margin(margin.0, margin.1, margin.2, margin.3);
        let (reserve_anchor, reserve_margin) = single_edge_anchor_margin(s.dock_edge, s.pos_y);
        self.reserve_layer.set_anchor(reserve_anchor);
        self.reserve_layer.set_margin(reserve_margin.0, reserve_margin.1, reserve_margin.2, reserve_margin.3);
        self.reserve_layer.set_exclusive_zone(self.dock.thickness() as i32 + s.pos_y);
        self.reserve_layer.commit();
        self.request_redraw(qh);
    }

    pub(super) fn choose_dock_align(&mut self, align: crate::config::DockAlign, qh: &QueueHandle<Self>) {
        self.dock.config.settings.dock_align = align;
        let _ = self.dock.config.save();
        let s = &self.dock.config.settings;
        let (anchor, margin) = edge_anchor_margin(s.dock_edge, s.dock_align, s.pos_y, 0);
        self.layer.set_anchor(anchor);
        self.layer.set_margin(margin.0, margin.1, margin.2, margin.3);
        self.request_redraw(qh);
    }

    pub(super) fn choose_theme(&mut self, choice: Option<usize>, qh: &QueueHandle<Self>) {
        match choice {
            None => {
                self.dock.config.settings.accent_from_wallpaper = true;
                self.dock.config.settings.custom_theme = false;
                self.sync_accent_from_last_wallpaper();
                let dir = repo_dir();
                let _ = std::process::Command::new(dir.join("sync-dolphin-theme.sh")).spawn();
                let _ = std::process::Command::new(dir.join("sync-kitty-theme.sh")).spawn();
                let _ = std::process::Command::new(dir.join("sync-p10k-theme.sh")).spawn();
            }
            Some(index) if index < menu::THEME_PRESETS.len() => {
                let Some(preset) = menu::THEME_PRESETS.get(index) else {
                    return;
                };
                let (ar, ag, ab) = preset.swatches[0];
                let (br, bg, bb) = preset.swatches[1];
                let luma = 0.299 * ar as f32 + 0.587 * ag as f32 + 0.114 * ab as f32;
                let on = if luma > 140.0 { (20, 20, 24) } else { (245, 245, 248) };
                let base = if preset.is_light { (245, 245, 248) } else { (18, 18, 20) };
                let mix = |b: u8, c: u8| (b as f32 * 0.9 + c as f32 * 0.1).round() as u8;
                let panel = (mix(base.0, ar), mix(base.1, ag), mix(base.2, ab));
                let (text, text_dim) = if preset.is_light { ((30, 30, 34), (100, 100, 108)) } else { ((235, 235, 240), (150, 150, 158)) };
                let s = &mut self.dock.config.settings;
                s.accent_from_wallpaper = false;
                s.custom_theme = false;
                (s.accent_r, s.accent_g, s.accent_b) = (ar, ag, ab);
                (s.accent2_r, s.accent2_g, s.accent2_b) = (br, bg, bb);
                (s.on_accent_r, s.on_accent_g, s.on_accent_b) = on;
                (s.panel_r, s.panel_g, s.panel_b) = panel;
                (s.text_r, s.text_g, s.text_b) = text;
                (s.text_dim_r, s.text_dim_g, s.text_dim_b) = text_dim;
                let _ = self.dock.config.save();
                let dir = repo_dir();
                let _ = std::process::Command::new(dir.join("sync-dolphin-theme.sh")).spawn();
                let _ = std::process::Command::new(dir.join("sync-kitty-theme.sh")).spawn();
                let _ = std::process::Command::new(dir.join("sync-p10k-theme.sh")).spawn();
            }
            // ----- exact colors -----
            Some(index) => {
                let Some(custom) = self.dock.config.settings.custom_palettes.get(index - menu::THEME_PRESETS.len()).cloned() else {
                    return;
                };
                let luma = 0.299 * custom.accent.0 as f32 + 0.587 * custom.accent.1 as f32 + 0.114 * custom.accent.2 as f32;
                let on = if luma > 140.0 { (20, 20, 24) } else { (245, 245, 248) };
                let s = &mut self.dock.config.settings;
                s.accent_from_wallpaper = false;
                s.custom_theme = true;
                (s.accent_r, s.accent_g, s.accent_b) = custom.accent;
                (s.accent2_r, s.accent2_g, s.accent2_b) = custom.accent2;
                (s.on_accent_r, s.on_accent_g, s.on_accent_b) = on;
                (s.panel_r, s.panel_g, s.panel_b) = custom.panel;
                (s.text_r, s.text_g, s.text_b) = custom.text;
                (s.text_dim_r, s.text_dim_g, s.text_dim_b) = custom.text_dim;
                let _ = self.dock.config.save();
                let dir = repo_dir();
                let _ = std::process::Command::new(dir.join("sync-dolphin-theme.sh")).spawn();
                let _ = std::process::Command::new(dir.join("sync-kitty-theme.sh")).spawn();
                let _ = std::process::Command::new(dir.join("sync-p10k-theme.sh")).spawn();
            }
        }
        self.request_redraw(qh);
    }

    pub(super) fn choose_matugen_scheme(&mut self, index: usize, qh: &QueueHandle<Self>) {
        let Some(&(id, _)) = menu::MATUGEN_SCHEMES.get(index) else {
            return;
        };
        self.dock.config.settings.matugen_scheme = id.to_string();
        let _ = self.dock.config.save();
        if self.dock.config.settings.accent_from_wallpaper {
            self.sync_accent_from_last_wallpaper();
        }
        self.request_redraw(qh);
    }

    pub(super) fn toggle_dock_menu_dropdown(&mut self, which: menu::OpenDropdown, qh: &QueueHandle<Self>) {
        if let Some(dm) = self.dock_menu_mode.as_mut() {
            dm.open_dropdown = if dm.open_dropdown == which { menu::OpenDropdown::None } else { which };
            dm.dropdown_scroll = 0;
            dm.font_query.clear();
            dm.dropdown_selected = 0;
        }
        self.request_redraw(qh);
    }

    pub(super) fn choose_dock_font(&mut self, index: usize, qh: &QueueHandle<Self>) {
        let Some(family) = self.available_fonts.get(index) else {
            return;
        };
        let family = family.clone();
        self.dock.config.settings.dock_font = family.clone();
        self.text_cache.set_default_family(&family);
        let _ = self.dock.config.save();
        self.relayout_dock(qh);
    }

    pub(super) fn choose_system_font(&mut self, index: usize, qh: &QueueHandle<Self>) {
        let Some(family) = self.available_fonts.get(index) else {
            return;
        };
        let family = family.clone();
        self.dock.config.settings.system_font = family.clone();
        let _ = self.dock.config.save();
        apply_system_gtk_font(&family);
        apply_system_qt_font(&family);
        apply_kitty_font(&family);
        // ----- needs gsettings schema -----
        let _ = std::process::Command::new("gsettings")
            .args(["set", "org.gnome.desktop.interface", "font-name", &format!("{} 10", family)])
            .spawn();
        self.request_redraw(qh);
    }

    pub(super) fn handle_dock_menu_pointer_event(&mut self, event: &PointerEvent, qh: &QueueHandle<Self>) {
        let right_col_x0 = menu::DOCK_MENU_LEFT_COL_W + menu::DOCK_MENU_DIVIDER_W;
        match event.kind {
            PointerEventKind::Enter { .. } | PointerEventKind::Motion { .. } => {
                let (x, y) = event.position;
                let dragging = self.dock_menu_mode.as_ref().and_then(|dm| dm.dragging_slider);
                let dragging_widget = self.dock_menu_mode.as_ref().and_then(|dm| dm.dragging_widget.map(|(k, _, _)| k));
                if let Some(id) = dragging {
                    let cy = self.dock_menu_mode.as_ref().and_then(|dm| {
                        dm.controls.iter().find_map(|c| match c.kind {
                            menu::ControlKind::Slider(sid) if sid == id => Some(c.y),
                            _ => None,
                        })
                    });
                    if let Some(cy) = cy {
                        let v = menu::slider_value_from_x(id, x as f32 - right_col_x0, cy);
                        id.set(&mut self.dock.config.settings, v);
                        self.on_setting_changed(id, qh, false);
                    }
                } else if let Some(kind) = dragging_widget {
                    if let Some(dm) = self.dock_menu_mode.as_mut() {
                        dm.dragging_widget = Some((kind, x as f32 - right_col_x0, y as f32));
                    }
                } else if let Some(dm) = self.dock_menu_mode.as_mut() {
                    let right_x = x as f32 - right_col_x0;
                    let overlay_hovered = match dm.open_dropdown {
                        menu::OpenDropdown::Scheme => dm.controls.iter().find(|c| matches!(c.kind, menu::ControlKind::SchemeDropdown)).and_then(|c| {
                            menu::scheme_picker_hit_test(c.y + c.height + 4.0, menu::DOCK_MENU_RIGHT_COL_W, right_x, y as f32).map(menu::HitTarget::SchemeOption)
                        }),
                        menu::OpenDropdown::DockFont => {
                            let filtered_count = menu::filter_font_indices(&self.available_fonts, &dm.font_query).len();
                            dm.controls.iter().find(|c| matches!(c.kind, menu::ControlKind::DockFontDropdown)).and_then(|c| {
                                menu::font_list_hit_test(filtered_count, menu::DOCK_MENU_RIGHT_COL_W, c.y + c.height + 4.0, dm.dropdown_scroll, right_x, y as f32).map(menu::HitTarget::FontOption)
                            })
                        }
                        menu::OpenDropdown::SystemFont => {
                            let filtered_count = menu::filter_font_indices(&self.available_fonts, &dm.font_query).len();
                            dm.controls.iter().find(|c| matches!(c.kind, menu::ControlKind::SystemFontDropdown)).and_then(|c| {
                                menu::font_list_hit_test(filtered_count, menu::DOCK_MENU_RIGHT_COL_W, c.y + c.height + 4.0, dm.dropdown_scroll, right_x, y as f32).map(menu::HitTarget::FontOption)
                            })
                        }
                        menu::OpenDropdown::None => None,
                    };
                    dm.hovered = overlay_hovered.or_else(|| {
                        if (x as f32) < menu::DOCK_MENU_LEFT_COL_W {
                            menu::dock_menu_tab_hit_test(x as f32, y as f32)
                        } else {
                            menu::hit_test(&dm.controls, &self.dock.config.settings, menu::DOCK_MENU_RIGHT_COL_W, right_x, y as f32)
                        }
                    });
                }
                self.request_redraw(qh);
            }
            PointerEventKind::Leave { .. } => {
                if let Some(dm) = self.dock_menu_mode.as_mut() {
                    dm.hovered = None;
                }
                self.request_redraw(qh);
            }
            PointerEventKind::Press { button, .. } if button == BTN_RIGHT => {
                self.close_dock_menu(qh);
            }
            PointerEventKind::Press { button, .. } if button == BTN_LEFT => {
                let (x, y) = event.position;
                let right_x = x as f32 - right_col_x0;
                let open_dropdown = self.dock_menu_mode.as_ref().map(|dm| dm.open_dropdown).unwrap_or(menu::OpenDropdown::None);
                if open_dropdown != menu::OpenDropdown::None {
                    let font_matches = self.dock_menu_mode.as_ref().map(|dm| menu::filter_font_indices(&self.available_fonts, &dm.font_query)).unwrap_or_default();
                    let overlay_hit = self.dock_menu_mode.as_ref().and_then(|dm| match open_dropdown {
                        menu::OpenDropdown::Scheme => {
                            let c = dm.controls.iter().find(|c| matches!(c.kind, menu::ControlKind::SchemeDropdown))?;
                            menu::scheme_picker_hit_test(c.y + c.height + 4.0, menu::DOCK_MENU_RIGHT_COL_W, right_x, y as f32)
                        }
                        menu::OpenDropdown::DockFont => {
                            let c = dm.controls.iter().find(|c| matches!(c.kind, menu::ControlKind::DockFontDropdown))?;
                            menu::font_list_hit_test(font_matches.len(), menu::DOCK_MENU_RIGHT_COL_W, c.y + c.height + 4.0, dm.dropdown_scroll, right_x, y as f32)
                        }
                        menu::OpenDropdown::SystemFont => {
                            let c = dm.controls.iter().find(|c| matches!(c.kind, menu::ControlKind::SystemFontDropdown))?;
                            menu::font_list_hit_test(font_matches.len(), menu::DOCK_MENU_RIGHT_COL_W, c.y + c.height + 4.0, dm.dropdown_scroll, right_x, y as f32)
                        }
                        menu::OpenDropdown::None => None,
                    });
                    if let Some(local_index) = overlay_hit {
                        match open_dropdown {
                            menu::OpenDropdown::Scheme => self.choose_matugen_scheme(local_index, qh),
                            menu::OpenDropdown::DockFont => {
                                if let Some(&global_index) = font_matches.get(local_index) {
                                    self.choose_dock_font(global_index, qh);
                                }
                            }
                            menu::OpenDropdown::SystemFont => {
                                if let Some(&global_index) = font_matches.get(local_index) {
                                    self.choose_system_font(global_index, qh);
                                }
                            }
                            menu::OpenDropdown::None => {}
                        }
                    }
                    if let Some(dm) = self.dock_menu_mode.as_mut() {
                        dm.open_dropdown = menu::OpenDropdown::None;
                        dm.font_query.clear();
                        dm.dropdown_selected = 0;
                    }
                    self.request_redraw(qh);
                } else {
                    let hit = if (x as f32) < menu::DOCK_MENU_LEFT_COL_W {
                        menu::dock_menu_tab_hit_test(x as f32, y as f32)
                    } else {
                        self.dock_menu_mode.as_ref().and_then(|dm| {
                            menu::hit_test(&dm.controls, &self.dock.config.settings, menu::DOCK_MENU_RIGHT_COL_W, right_x, y as f32)
                        })
                    };
                    if let Some(menu::HitTarget::WidgetChip(kind)) = hit {
                        if let Some(dm) = self.dock_menu_mode.as_mut() {
                            dm.dragging_widget = Some((kind, right_x, y as f32));
                        }
                        self.request_redraw(qh);
                    } else {
                        self.handle_dock_menu_click(hit, right_x, qh);
                    }
                }
            }
            PointerEventKind::Release { button, .. } if button == BTN_LEFT => {
                let active = self.dock_menu_mode.as_ref().and_then(|dm| dm.dragging_slider.or(dm.held_stepper.map(|(id, _, _)| id)));
                let dropped = self.dock_menu_mode.as_ref().and_then(|dm| dm.dragging_widget);
                if let Some(dm) = self.dock_menu_mode.as_mut() {
                    dm.dragging_slider = None;
                    dm.held_stepper = None;
                    dm.dragging_widget = None;
                }
                if let Some(id) = active {
                    self.on_setting_changed(id, qh, true);
                }
                if let Some((kind, x, y)) = dropped {
                    self.drop_widget_chip(kind, x, y, qh);
                }
            }
            PointerEventKind::Axis { horizontal, vertical, .. } => {
                let delta = if vertical.absolute != 0.0 { vertical.absolute } else { horizontal.absolute };
                if let Some(dm) = self.dock_menu_mode.as_mut() {
                    if matches!(dm.open_dropdown, menu::OpenDropdown::DockFont | menu::OpenDropdown::SystemFont) {
                        let item_count = menu::filter_font_indices(&self.available_fonts, &dm.font_query).len();
                        let max_scroll = item_count.saturating_sub(menu::FONT_LIST_MAX_ROWS);
                        let step = if delta > 0.0 { 1isize } else { -1isize };
                        let cur = dm.dropdown_scroll as isize;
                        dm.dropdown_scroll = (cur + step).clamp(0, max_scroll as isize) as usize;
                        self.request_redraw(qh);
                    }
                }
            }
            _ => {}
        }
    }

    pub(super) fn handle_dock_menu_key(&mut self, event: KeyEvent, qh: &QueueHandle<Self>) {
        let Some(dm) = self.dock_menu_mode.as_ref() else {
            return;
        };
        let which = dm.open_dropdown;
        if !matches!(which, menu::OpenDropdown::DockFont | menu::OpenDropdown::SystemFont) {
            if dm.custom_name_focused {
                self.handle_custom_name_key(event, qh);
            } else if dm.custom_focus.is_some() {
                self.handle_custom_hex_key(event, qh);
            }
            return;
        }

        match event.keysym {
            Keysym::Escape => {
                if let Some(dm) = self.dock_menu_mode.as_mut() {
                    dm.open_dropdown = menu::OpenDropdown::None;
                    dm.font_query.clear();
                    dm.dropdown_selected = 0;
                    dm.dropdown_scroll = 0;
                }
                self.request_redraw(qh);
                return;
            }
            Keysym::BackSpace => {
                if let Some(dm) = self.dock_menu_mode.as_mut() {
                    dm.font_query.pop();
                    dm.dropdown_selected = 0;
                    dm.dropdown_scroll = 0;
                }
                self.request_redraw(qh);
                return;
            }
            Keysym::Down => {
                let count = menu::filter_font_indices(&self.available_fonts, &dm.font_query).len();
                if let Some(dm) = self.dock_menu_mode.as_mut() {
                    if count > 0 {
                        dm.dropdown_selected = (dm.dropdown_selected + 1).min(count - 1);
                        if dm.dropdown_selected >= dm.dropdown_scroll + menu::FONT_LIST_MAX_ROWS {
                            let max_scroll = count.saturating_sub(menu::FONT_LIST_MAX_ROWS);
                            dm.dropdown_scroll = (dm.dropdown_selected + 1 - menu::FONT_LIST_MAX_ROWS).min(max_scroll);
                        }
                    }
                }
                self.request_redraw(qh);
                return;
            }
            Keysym::Up => {
                if let Some(dm) = self.dock_menu_mode.as_mut() {
                    dm.dropdown_selected = dm.dropdown_selected.saturating_sub(1);
                    if dm.dropdown_selected < dm.dropdown_scroll {
                        dm.dropdown_scroll = dm.dropdown_selected;
                    }
                }
                self.request_redraw(qh);
                return;
            }
            Keysym::Return => {
                let matches = menu::filter_font_indices(&self.available_fonts, &dm.font_query);
                let selected = dm.dropdown_selected;
                if let Some(&global_index) = matches.get(selected) {
                    match which {
                        menu::OpenDropdown::DockFont => self.choose_dock_font(global_index, qh),
                        menu::OpenDropdown::SystemFont => self.choose_system_font(global_index, qh),
                        _ => {}
                    }
                }
                if let Some(dm) = self.dock_menu_mode.as_mut() {
                    dm.open_dropdown = menu::OpenDropdown::None;
                    dm.font_query.clear();
                    dm.dropdown_selected = 0;
                    dm.dropdown_scroll = 0;
                }
                self.request_redraw(qh);
                return;
            }
            _ => {}
        }

        if let Some(text) = event.utf8.clone() {
            let mut changed = false;
            if let Some(dm) = self.dock_menu_mode.as_mut() {
                for ch in text.chars() {
                    if !ch.is_control() {
                        dm.font_query.push(ch);
                        changed = true;
                    }
                }
                if changed {
                    dm.dropdown_selected = 0;
                    dm.dropdown_scroll = 0;
                }
            }
            if changed {
                self.request_redraw(qh);
            }
        }
    }

    pub(super) fn handle_custom_hex_key(&mut self, event: KeyEvent, qh: &QueueHandle<Self>) {
        let Some(dm) = self.dock_menu_mode.as_ref() else {
            return;
        };
        let Some(focus) = dm.custom_focus else {
            return;
        };
        match event.keysym {
            Keysym::Escape | Keysym::Return | Keysym::Tab => {
                if let Some(dm) = self.dock_menu_mode.as_mut() {
                    dm.custom_focus = None;
                }
                self.request_redraw(qh);
                return;
            }
            Keysym::BackSpace => {
                if let Some(dm) = self.dock_menu_mode.as_mut() {
                    if let Some(field) = dm.custom_hex.get_mut(focus) {
                        field.pop();
                    }
                }
                self.request_redraw(qh);
                return;
            }
            Keysym::v | Keysym::V if self.modifiers.ctrl => {
                self.request_paste(crate::clipboard::PasteTarget::Hex(focus), qh);
                return;
            }
            Keysym::c | Keysym::C if self.modifiers.ctrl => {
                let text = self.dock_menu_mode.as_ref().and_then(|dm| dm.custom_hex.get(focus).cloned()).unwrap_or_default();
                self.copy_to_clipboard(text, qh);
                return;
            }
            _ => {}
        }

        if let Some(text) = event.utf8.clone() {
            let mut changed = false;
            if let Some(dm) = self.dock_menu_mode.as_mut() {
                if let Some(field) = dm.custom_hex.get_mut(focus) {
                    for ch in text.chars() {
                        if field.len() < 6 && ch.is_ascii_hexdigit() {
                            field.push(ch.to_ascii_lowercase());
                            changed = true;
                        }
                    }
                }
            }
            if changed {
                self.request_redraw(qh);
            }
        }
    }

    pub(super) fn handle_custom_name_key(&mut self, event: KeyEvent, qh: &QueueHandle<Self>) {
        match event.keysym {
            Keysym::Escape | Keysym::Return | Keysym::Tab => {
                if let Some(dm) = self.dock_menu_mode.as_mut() {
                    dm.custom_name_focused = false;
                }
                self.request_redraw(qh);
                return;
            }
            Keysym::BackSpace => {
                if let Some(dm) = self.dock_menu_mode.as_mut() {
                    dm.custom_name.pop();
                }
                self.request_redraw(qh);
                return;
            }
            Keysym::v | Keysym::V if self.modifiers.ctrl => {
                self.request_paste(crate::clipboard::PasteTarget::Name, qh);
                return;
            }
            Keysym::c | Keysym::C if self.modifiers.ctrl => {
                let text = self.dock_menu_mode.as_ref().map(|dm| dm.custom_name.clone()).unwrap_or_default();
                self.copy_to_clipboard(text, qh);
                return;
            }
            _ => {}
        }

        if let Some(text) = event.utf8.clone() {
            let mut changed = false;
            if let Some(dm) = self.dock_menu_mode.as_mut() {
                for ch in text.chars() {
                    if dm.custom_name.chars().count() < 24 && !ch.is_control() {
                        dm.custom_name.push(ch);
                        changed = true;
                    }
                }
            }
            if changed {
                self.request_redraw(qh);
            }
        }
    }
}
