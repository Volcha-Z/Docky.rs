use super::*;

impl App {
    pub(super) fn handle_dock_pointer_event(&mut self, event: &PointerEvent, qh: &QueueHandle<Self>) {
        if self.notification_mode.is_some() {
            if let PointerEventKind::Press { .. } = event.kind {
                self.close_notification_mode(qh);
            }
            return;
        }
        if self.osd_mode.is_some() {
            if let PointerEventKind::Press { .. } = event.kind {
                self.close_osd_mode(qh);
            }
            return;
        }
        if self.app_search_mode.is_some() {
            self.handle_app_search_pointer_event(event, qh);
            return;
        }
        if self.dock_menu_mode.is_some() {
            self.handle_dock_menu_pointer_event(event, qh);
            return;
        }
        if self.wallpaper_mode.is_some() {
            self.handle_wallpaper_pointer_event(event, qh);
            return;
        }
        if self.clipboard_mode.is_some() {
            self.handle_clipboard_pointer_event(event, qh);
            return;
        }
        match event.kind {
            PointerEventKind::Enter { .. } | PointerEventKind::Motion { .. } => {
                let (x, y) = event.position;
                self.dock.set_pointer(Some((x, y)));

                if self.pointer_down {
                    if self.dock.dragging_index.is_some() {
                        self.dock.drag_to(x as f32, y as f32);
                    } else if let (Some(press_idx), Some((px, py))) = (self.press_icon_index, self.press_pos) {
                        let dist = ((x - px).powi(2) + (y - py).powi(2)).sqrt() as f32;
                        if dist > dock::DRAG_THRESHOLD {
                            self.dock.start_drag(press_idx);
                            self.dock.drag_to(x as f32, y as f32);
                        }
                    }
                }
                self.request_redraw(qh);
            }
            PointerEventKind::Leave { .. } => {
                self.dock.set_pointer(None);
                self.request_redraw(qh);
            }
            PointerEventKind::Press { button, .. } => {
                if self.menu.is_some() {
                    self.close_menu(qh);
                    return;
                }
                if self.popup_mode.as_ref().map(|p| !(p.closing && p.anim <= 0.0)).unwrap_or(false) {
                    self.close_popup_mode(qh);
                    return;
                }
                let (x, y) = event.position;
                if button == BTN_LEFT {
                    self.pointer_down = true;
                    self.press_pos = Some((x, y));
                    self.press_icon_index = self.dock.icon_at(x, y);

                    if self.press_icon_index.is_none() {
                        // ----- hit test -----
                        let tray_count = self.tray.lock().unwrap().len();
                        if let Some(kind) = render::widget_hit_test(&self.dock, &self.widgets, tray_count, x, y) {
                            match kind {
                                crate::config::WidgetKind::Media => {
                                    if render::media_toggle_hit(&self.dock, &self.widgets, tray_count, x, y) {
                                        widgets::media_toggle();
                                    }
                                    return;
                                }
                                crate::config::WidgetKind::PowerMenu => {
                                    self.open_power_menu(qh);
                                    return;
                                }
                                crate::config::WidgetKind::Bluetooth => {
                                    if let Some(bt) = &self.widgets.bluetooth {
                                        widgets::bluetooth_toggle(bt.powered);
                                    }
                                    return;
                                }
                                crate::config::WidgetKind::Tray => {
                                    if let Some(idx) = render::tray_icon_hit(&self.dock, &self.widgets, tray_count, x, y) {
                                        let icons = self.tray.lock().unwrap();
                                        if let Some(item) = icons.get(idx) {
                                            crate::tray::activate(item.service.clone(), item.path.clone());
                                        }
                                    }
                                    return;
                                }
                                crate::config::WidgetKind::Workspaces => {
                                    if let Some(id) = render::workspace_dot_hit(&self.dock, &self.widgets, tray_count, x, y) {
                                        widgets::workspace_switch(id);
                                    }
                                    return;
                                }
                                _ => {}
                            }
                        }
                        let now = std::time::Instant::now();
                        let is_double = self
                            .last_empty_click
                            .map(|(t, px, py)| now.duration_since(t).as_millis() < 400 && (x - px).abs() < 12.0 && (y - py).abs() < 12.0)
                            .unwrap_or(false);
                        if is_double {
                            self.last_empty_click = None;
                            self.open_wallpaper_picker(qh);
                        } else {
                            self.last_empty_click = Some((now, x, y));
                        }
                    }
                } else if button == BTN_RIGHT {
                    match self.dock.icon_at(x, y) {
                        Some(idx) => self.open_menu(menu::MenuScreen::IconMenu(idx), qh),
                        None => {
                            let tray_count = self.tray.lock().unwrap().len();
                            let tray_idx = matches!(render::widget_hit_test(&self.dock, &self.widgets, tray_count, x, y), Some(crate::config::WidgetKind::Tray))
                                .then(|| render::tray_icon_hit(&self.dock, &self.widgets, tray_count, x, y))
                                .flatten();
                            match tray_idx {
                                Some(idx) => self.open_tray_menu(idx, qh),
                                None => self.open_dock_menu(qh),
                            }
                        }
                    }
                }
            }
            PointerEventKind::Release { button, .. } if button == BTN_LEFT => {
                self.pointer_down = false;
                if self.dock.dragging_index.is_some() {
                    self.dock.end_drag();
                    if let Err(err) = self.dock.config.save() {
                        log::warn!("failed to save dock config: {err}");
                    }
                } else if let Some(idx) = self.press_icon_index {
                    launch_app(&self.dock.icons[idx].app.exec);
                }
                self.press_pos = None;
                self.press_icon_index = None;
                self.request_redraw(qh);
            }
            _ => {}
        }
    }
}
