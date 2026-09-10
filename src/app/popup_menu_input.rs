use super::*;

impl App {
    pub(super) fn handle_menu_click(&mut self, hit: Option<menu::HitTarget>, x: f32, qh: &QueueHandle<Self>) {
        use menu::HitTarget;
        match hit {
            Some(HitTarget::Toggle(id)) => {
                id.toggle(&mut self.dock.config.settings);
                self.on_setting_changed(id, qh, true);
                self.request_menu_redraw(qh);
            }
            Some(HitTarget::SliderTrack(id)) => {
                let cy = self.menu.as_ref().and_then(|m| {
                    m.controls.iter().find_map(|c| match c.kind {
                        menu::ControlKind::Slider(sid) if sid == id => Some(c.y),
                        _ => None,
                    })
                });
                if let Some(cy) = cy {
                    let v = menu::slider_value_from_x(id, x, cy);
                    id.set(&mut self.dock.config.settings, v);
                    self.on_setting_changed(id, qh, true);
                }
                if let Some(menu) = self.menu.as_mut() {
                    menu.dragging_slider = Some(id);
                }
                self.request_menu_redraw(qh);
            }
            Some(HitTarget::SliderMinus(id)) => self.start_stepper(id, -1.0, qh),
            Some(HitTarget::SliderPlus(id)) => self.start_stepper(id, 1.0, qh),
            Some(HitTarget::Button(kind)) => self.handle_menu_button(kind, qh),
            Some(HitTarget::AppEntry(index)) => self.add_app_from_entry(index, qh),
            Some(HitTarget::IconChoice(index)) => self.choose_icon(index, qh),
            Some(HitTarget::TrayItem(_))
            | Some(HitTarget::SearchBox)
            | Some(HitTarget::Tab(_))
            | Some(HitTarget::Edge(_))
            | Some(HitTarget::WidgetChip(_))
            | Some(HitTarget::ThemeOption(_))
            | Some(HitTarget::SchemeOption(_))
            | Some(HitTarget::SchemeDropdownToggle)
            | Some(HitTarget::DockFontDropdownToggle)
            | Some(HitTarget::SystemFontDropdownToggle)
            | Some(HitTarget::FontOption(_))
            | Some(HitTarget::HexFieldFocus(_))
            | Some(HitTarget::NameFieldFocus)
            | Some(HitTarget::PaletteMode(_))
            | Some(HitTarget::PanelBlend(_))
            | Some(HitTarget::DeleteCustomPalette(_))
            | Some(HitTarget::Align(_)) => {}
            None => {}
        }
    }

    pub(super) fn handle_menu_button(&mut self, kind: menu::ButtonKind, qh: &QueueHandle<Self>) {
        match kind {
            menu::ButtonKind::AddApp => self.switch_menu_screen(menu::MenuScreen::AddApp, qh),
            menu::ButtonKind::Back => {
                if let Some(menu::MenuScreen::IconPicker(idx)) = self.menu.as_ref().map(|m| m.screen) {
                    self.switch_menu_screen(menu::MenuScreen::IconMenu(idx), qh);
                } else {
                    self.close_menu(qh);
                }
            }
            menu::ButtonKind::QuitDock => self.exit = true,
            menu::ButtonKind::ChangeIcon => {
                if let Some(menu::MenuScreen::IconMenu(idx)) = self.menu.as_ref().map(|m| m.screen) {
                    self.open_icon_picker(idx, qh);
                }
            }
            menu::ButtonKind::RemoveApp => {
                if let Some(menu::MenuScreen::IconMenu(idx)) = self.menu.as_ref().map(|m| m.screen) {
                    self.dock.remove_icon(idx);
                    if let Err(err) = self.dock.config.save() {
                        log::warn!("failed to save dock config: {err}");
                    }
                    let (w, h) = self.dock.base_size();
                    self.layer.set_size(w, h);
                    self.request_redraw(qh);
                }
                self.close_menu(qh);
            }
            menu::ButtonKind::Suspend => {
                crate::power::suspend();
                self.close_menu(qh);
            }
            menu::ButtonKind::Logout => {
                crate::power::logout();
                self.close_menu(qh);
            }
            menu::ButtonKind::Reboot => {
                crate::power::reboot();
                self.close_menu(qh);
            }
            menu::ButtonKind::Shutdown => {
                crate::power::poweroff();
                self.close_menu(qh);
            }
            menu::ButtonKind::CreatePalette | menu::ButtonKind::SavePalette => {}
        }
    }

    pub(super) fn add_app_from_entry(&mut self, index: usize, qh: &QueueHandle<Self>) {
        let entry = self.menu.as_ref().and_then(|m| m.filtered_app_entries.get(index)).map(|e| PinnedApp {
            name: e.name.clone(),
            icon: e.icon.clone(),
            exec: e.exec.clone(),
        });
        if let Some(app) = entry {
            self.dock.add_app(app);
            if let Err(err) = self.dock.config.save() {
                log::warn!("failed to save dock config: {err}");
            }
            let (w, h) = self.dock.base_size();
            self.layer.set_size(w, h);
            self.request_redraw(qh);
        }
        self.switch_menu_screen(menu::MenuScreen::AddApp, qh);
    }

    pub(super) fn open_icon_picker(&mut self, target: usize, qh: &QueueHandle<Self>) {
        let all_icon_names = icon_browser::list_icon_names(crate::ICON_THEME);
        let screen = menu::MenuScreen::IconPicker(target);
        let filtered_len = all_icon_names.len().min(200);
        let (controls, content_height) = menu::build_controls(screen, filtered_len, 0);
        if let Some(menu) = self.menu.as_mut() {
            menu.screen = screen;
            menu.search_query.clear();
            menu.filtered_icon_names = all_icon_names.iter().take(200).cloned().collect();
            menu.all_icon_names = all_icon_names;
            menu.controls = controls;
            menu.content_height = content_height;
            menu.hovered = None;
            menu.list_scroll = 0;
            menu.list_selected = 0;
            menu.layer.set_size(menu::MENU_WIDTH as u32, content_height as u32);
        }
        self.request_menu_redraw(qh);
    }

    pub(super) fn choose_icon(&mut self, filtered_index: usize, qh: &QueueHandle<Self>) {
        let chosen = self.menu.as_ref().and_then(|m| m.filtered_icon_names.get(filtered_index)).cloned();
        let target = self.menu.as_ref().and_then(|m| match m.screen {
            menu::MenuScreen::IconPicker(idx) => Some(idx),
            _ => None,
        });
        if let (Some(name), Some(idx)) = (chosen, target) {
            self.dock.set_icon(idx, name);
            if let Err(err) = self.dock.config.save() {
                log::warn!("failed to save dock config: {err}");
            }
            self.request_redraw(qh);
            self.switch_menu_screen(menu::MenuScreen::IconMenu(idx), qh);
        }
    }

    pub(super) fn refresh_add_app_search(&mut self, qh: &QueueHandle<Self>) {
        let Some(menu) = self.menu.as_mut() else {
            return;
        };
        let menu::MenuScreen::AddApp = menu.screen else {
            return;
        };
        let query = menu.search_query.to_lowercase();
        menu.filtered_app_entries = menu
            .app_entries
            .iter()
            .filter(|e| query.is_empty() || e.name.to_lowercase().contains(&query))
            .cloned()
            .collect();
        let (controls, content_height) = menu::build_controls(menu.screen, menu.filtered_app_entries.len(), 0);
        menu.controls = controls;
        menu.content_height = content_height;
        menu.list_scroll = 0;
        menu.list_selected = 0;
        menu.layer.set_size(menu::MENU_WIDTH as u32, content_height as u32);
        self.request_menu_redraw(qh);
    }

    pub(super) fn refresh_icon_search(&mut self, qh: &QueueHandle<Self>) {
        let Some(menu) = self.menu.as_mut() else {
            return;
        };
        let menu::MenuScreen::IconPicker(_) = menu.screen else {
            return;
        };
        let query = menu.search_query.to_lowercase();
        menu.filtered_icon_names = menu
            .all_icon_names
            .iter()
            .filter(|name| query.is_empty() || name.to_lowercase().contains(&query))
            .take(200)
            .cloned()
            .collect();
        let (controls, content_height) = menu::build_controls(menu.screen, menu.filtered_icon_names.len(), 0);
        menu.controls = controls;
        menu.content_height = content_height;
        menu.list_scroll = 0;
        menu.list_selected = 0;
        menu.layer.set_size(menu::MENU_WIDTH as u32, content_height as u32);
        self.request_menu_redraw(qh);
    }

    pub(super) fn list_len_for_screen(&self, screen: menu::MenuScreen) -> Option<usize> {
        match screen {
            menu::MenuScreen::AddApp => self.menu.as_ref().map(|m| m.filtered_app_entries.len()),
            menu::MenuScreen::IconPicker(_) => self.menu.as_ref().map(|m| m.filtered_icon_names.len()),
            _ => None,
        }
    }

    pub(super) fn rebuild_menu_list_controls(&mut self, qh: &QueueHandle<Self>) {
        let Some(menu) = self.menu.as_ref() else {
            return;
        };
        let screen = menu.screen;
        let Some(count) = self.list_len_for_screen(screen) else {
            return;
        };
        let scroll = self.menu.as_ref().map(|m| m.list_scroll).unwrap_or(0);
        let (controls, content_height) = menu::build_controls(screen, count, scroll);
        if let Some(menu) = self.menu.as_mut() {
            menu.controls = controls;
            menu.content_height = content_height;
            menu.layer.set_size(menu::MENU_WIDTH as u32, content_height as u32);
        }
        self.request_menu_redraw(qh);
    }

    pub(super) fn move_menu_list_selection(&mut self, dir: isize, qh: &QueueHandle<Self>) {
        let Some(screen) = self.menu.as_ref().map(|m| m.screen) else {
            return;
        };
        let Some(count) = self.list_len_for_screen(screen) else {
            return;
        };
        if count == 0 {
            return;
        }
        let Some(menu) = self.menu.as_mut() else {
            return;
        };
        let cur = menu.list_selected as isize;
        let next = (cur + dir).clamp(0, count as isize - 1) as usize;
        menu.list_selected = next;
        if next < menu.list_scroll {
            menu.list_scroll = next;
        } else if next >= menu.list_scroll + menu::LIST_MAX_VISIBLE_ROWS {
            menu.list_scroll = next + 1 - menu::LIST_MAX_VISIBLE_ROWS;
        }
        menu.hovered = match screen {
            menu::MenuScreen::AddApp => Some(menu::HitTarget::AppEntry(next)),
            menu::MenuScreen::IconPicker(_) => Some(menu::HitTarget::IconChoice(next)),
            _ => None,
        };
        self.rebuild_menu_list_controls(qh);
    }

    pub(super) fn handle_search_key(&mut self, event: KeyEvent, qh: &QueueHandle<Self>) {
        let screen = self.menu.as_ref().map(|m| m.screen);
        if let Some(screen) = screen {
            if self.list_len_for_screen(screen).is_some() {
                match event.keysym {
                    Keysym::Down => {
                        self.move_menu_list_selection(1, qh);
                        return;
                    }
                    Keysym::Up => {
                        self.move_menu_list_selection(-1, qh);
                        return;
                    }
                    Keysym::Return => {
                        let selected = self.menu.as_ref().map(|m| m.list_selected);
                        if let Some(selected) = selected {
                            match screen {
                                menu::MenuScreen::AddApp => self.add_app_from_entry(selected, qh),
                                menu::MenuScreen::IconPicker(_) => self.choose_icon(selected, qh),
                                _ => {}
                            }
                        }
                        return;
                    }
                    _ => {}
                }
            }
        }

        let screen = self.menu.as_ref().map(|m| m.screen);
        let has_search_box = matches!(screen, Some(menu::MenuScreen::IconPicker(_)) | Some(menu::MenuScreen::AddApp));
        if !has_search_box {
            return;
        }
        if event.keysym == Keysym::Escape {
            self.close_menu(qh);
            return;
        }
        if let Some(menu) = self.menu.as_mut() {
            if event.keysym == Keysym::BackSpace {
                menu.search_query.pop();
            } else if let Some(text) = &event.utf8 {
                for ch in text.chars() {
                    if !ch.is_control() {
                        menu.search_query.push(ch);
                    }
                }
            }
        }
        match screen {
            Some(menu::MenuScreen::AddApp) => self.refresh_add_app_search(qh),
            Some(menu::MenuScreen::IconPicker(_)) => self.refresh_icon_search(qh),
            _ => {}
        }
    }
    pub(super) fn handle_menu_pointer_event(&mut self, event: &PointerEvent, qh: &QueueHandle<Self>) {
        match event.kind {
            PointerEventKind::Enter { .. } | PointerEventKind::Motion { .. } => {
                let (x, y) = event.position;
                let dragging = self.menu.as_ref().and_then(|m| m.dragging_slider);
                if let Some(id) = dragging {
                    let cy = self.menu.as_ref().and_then(|m| {
                        m.controls.iter().find_map(|c| match c.kind {
                            menu::ControlKind::Slider(sid) if sid == id => Some(c.y),
                            _ => None,
                        })
                    });
                    if let Some(cy) = cy {
                        let v = menu::slider_value_from_x(id, x as f32, cy);
                        id.set(&mut self.dock.config.settings, v);
                        self.on_setting_changed(id, qh, false);
                    }
                } else if let Some(menu) = self.menu.as_mut() {
                    menu.hovered = menu::hit_test(&menu.controls, &self.dock.config.settings, menu::MENU_WIDTH, x as f32, y as f32);
                    match menu.hovered {
                        Some(menu::HitTarget::AppEntry(i)) | Some(menu::HitTarget::IconChoice(i)) => menu.list_selected = i,
                        _ => {}
                    }
                }
                self.request_menu_redraw(qh);
            }
            PointerEventKind::Leave { .. } => {
                if let Some(menu) = self.menu.as_mut() {
                    menu.hovered = None;
                }
                self.request_menu_redraw(qh);
            }
            PointerEventKind::Press { button, .. } if button == BTN_LEFT => {
                let (x, y) = event.position;
                let hit = self
                    .menu
                    .as_ref()
                    .and_then(|m| menu::hit_test(&m.controls, &self.dock.config.settings, menu::MENU_WIDTH, x as f32, y as f32));
                self.handle_menu_click(hit, x as f32, qh);
            }
            PointerEventKind::Release { button, .. } if button == BTN_LEFT => {
                let active = self.menu.as_ref().and_then(|m| m.dragging_slider.or(m.held_stepper.map(|(id, _, _)| id)));
                if let Some(menu) = self.menu.as_mut() {
                    menu.dragging_slider = None;
                    menu.held_stepper = None;
                }
                if let Some(id) = active {
                    self.on_setting_changed(id, qh, true);
                }
            }
            PointerEventKind::Axis { horizontal, vertical, .. } => {
                let delta = if vertical.absolute != 0.0 { vertical.absolute } else { horizontal.absolute };
                let screen = self.menu.as_ref().map(|m| m.screen);
                let Some(screen) = screen else { return };
                let Some(count) = self.list_len_for_screen(screen) else { return };
                let (_, visible) = menu::list_window(count, 0);
                let max_scroll = count.saturating_sub(visible);
                let step = if delta > 0.0 { 1isize } else { -1isize };
                if let Some(menu) = self.menu.as_mut() {
                    let cur = menu.list_scroll as isize;
                    menu.list_scroll = (cur + step).clamp(0, max_scroll as isize) as usize;
                }
                self.rebuild_menu_list_controls(qh);
            }
            _ => {}
        }
    }
}
