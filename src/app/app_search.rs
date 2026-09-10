use super::*;

impl App {
    pub(crate) fn toggle_app_search(&mut self, qh: &QueueHandle<Self>) {
        if self.app_search_mode.is_some() {
            self.close_app_search_mode(qh);
        } else {
            self.open_app_search(qh);
        }
    }

    pub(super) fn open_app_search(&mut self, qh: &QueueHandle<Self>) {
        self.wallpaper_mode = None;
        self.clipboard_mode = None;
        self.dock_menu_mode = None;
        self.osd_mode = None;
        self.notification_mode = None;
        self.popup_mode = None;
        self.held_key = None;
        self.layer.set_layer(Layer::Top);
        self.menu = None;

        let all_entries = desktop::list_all_desktop_entries();
        let is_vertical = self.dock.is_vertical();
        let (controls, cross_fixed) = menu::build_app_search_controls();
        let (panel_w, panel_h) = if is_vertical {
            let dock_thickness = self.dock.thickness() as f32;
            (menu::app_search_vertical_cross(dock_thickness), menu::app_search_vertical_along())
        } else {
            let dock_along = self.dock.base_size().0 as f32;
            (dock_along + menu::APP_SEARCH_WIDTH_GROWTH, cross_fixed)
        };
        self.layer.set_keyboard_interactivity(KeyboardInteractivity::Exclusive);
        let s = &self.dock.config.settings;
        let (anchor, margin) = edge_anchor_margin(s.dock_edge, s.dock_align, s.pos_y, 0);
        self.layer.set_anchor(anchor);
        self.layer.set_margin(margin.0, margin.1, margin.2, margin.3);
        self.layer.set_size(panel_w as u32, panel_h as u32);
        self.app_search_mode = Some(AppSearchMode {
            query: String::new(),
            all_entries,
            filtered: Vec::new(),
            controls,
            selected: 0,
            hovered: None,
            scroll_x: 0.0,
            scroll_target: 0.0,
            highlight_x: 0.0,
            highlight_target: 0.0,
            content_anim: 0.0,
            anim: 0.0,
            target_anim: 1.0,
            closing: false,
            panel_w,
            panel_h,
            is_vertical,
        });
        self.refresh_app_search(qh);
    }

    pub(super) fn close_app_search_mode(&mut self, qh: &QueueHandle<Self>) {
        if let Some(m) = self.app_search_mode.as_mut() {
            m.closing = true;
            m.target_anim = 0.0;
        }
        self.request_redraw(qh);
    }

    pub(super) fn refresh_app_search(&mut self, qh: &QueueHandle<Self>) {
        let Some(m) = self.app_search_mode.as_mut() else {
            return;
        };
        let query = m.query.to_lowercase();
        let mut scored: Vec<(usize, u8)> = m
            .all_entries
            .iter()
            .enumerate()
            .filter_map(|(i, e)| {
                let name = e.name.to_lowercase();
                if query.is_empty() || name.starts_with(&query) {
                    Some((i, 0))
                } else if name.contains(&query) {
                    Some((i, 1))
                } else {
                    None
                }
            })
            .collect();
        scored.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| m.all_entries[a.0].name.to_lowercase().cmp(&m.all_entries[b.0].name.to_lowercase())));
        m.filtered = scored.into_iter().take(menu::SEARCH_MAX_RESULTS).map(|(i, _)| m.all_entries[i].clone()).collect();
        m.selected = 0;
        m.scroll_x = 0.0;
        m.scroll_target = 0.0;
        m.highlight_x = 0.0;
        m.highlight_target = 0.0;
        m.content_anim = 0.0;
        self.request_redraw(qh);
    }

    pub(super) fn scroll_search_into_view(&mut self, index: usize, qh: &QueueHandle<Self>) {
        let Some(m) = self.app_search_mode.as_mut() else {
            return;
        };
        let cx = menu::app_search_card_along(index);
        let viewport_along = menu::app_search_viewport_along(m.panel_w, m.panel_h, m.is_vertical);
        let mut target = m.scroll_target;
        if cx < target {
            target = cx;
        } else if cx + menu::APP_CARD_W > target + viewport_along {
            target = cx + menu::APP_CARD_W - viewport_along;
        }
        let max_scroll = menu::app_search_strip_max_scroll(m.filtered.len(), viewport_along);
        m.scroll_target = target.clamp(0.0, max_scroll);
        m.highlight_target = cx;
        self.request_redraw(qh);
    }

    pub(super) fn launch_search_result(&mut self, index: usize, qh: &QueueHandle<Self>) {
        if let Some(entry) = self.app_search_mode.as_ref().and_then(|m| m.filtered.get(index)) {
            launch_app(&entry.exec);
        }
        self.close_app_search_mode(qh);
    }

    pub(super) fn draw_app_search_mode(&mut self, qh: &QueueHandle<Self>) {
        let scale = self.output_scale.max(1) as f32;
        let transparency = self.dock.config.settings.transparency;
        let Some(m) = self.app_search_mode.as_mut() else {
            return;
        };
        let linear = m.anim.clamp(0.0, 1.0);
        let eased = 0.5 - 0.5 * (std::f32::consts::PI * linear).cos();

        let width = (m.panel_w * scale).round() as i32;
        let height = (m.panel_h * scale).round() as i32;
        if width <= 0 || height <= 0 {
            return;
        }

        let effective_hovered = m.hovered.or(if m.filtered.is_empty() { None } else { Some(menu::HitTarget::AppEntry(m.selected)) });
        let args = menu_render::DrawArgs {
            screen: menu::MenuScreen::AddApp,
            controls: &m.controls,
            content_height: m.panel_h,
            panel_width: m.panel_w,
            dock: &self.dock,
            app_entries: &m.filtered,
            icon_choices: &[],
            search_query: &m.query,
            wallpapers: &[],
            wallpaper_hovered: None,
            wallpaper_scroll_x: m.scroll_x,
            hovered: effective_hovered,
            render_scale: scale,
            available_fonts: &[],
            open_dropdown: menu::OpenDropdown::None,
            font_query: "",
            custom_hex: &EMPTY_CUSTOM_HEX,
            custom_light: true,
            custom_focus: None,
            custom_name: "",
            custom_name_focused: false,
            custom_panel_blend: None,
            tray_items: &[],
        };
        let mut pixmap = tiny_skia::Pixmap::new(width as u32, height as u32).unwrap();
        if eased >= 0.999 {
            menu_render::draw_app_search(&mut pixmap, &mut self.icon_cache, &mut self.text_cache, &args, m.highlight_x, m.content_anim);
        } else {
            let mut content = tiny_skia::Pixmap::new(width as u32, height as u32).unwrap();
            menu_render::draw_app_search(&mut content, &mut self.icon_cache, &mut self.text_cache, &args, m.highlight_x, m.content_anim);
            let paint = tiny_skia::PixmapPaint {
                opacity: anim_opacity(transparency, eased),
                ..Default::default()
            };
            pixmap.draw_pixmap(0, 0, content.as_ref(), &paint, tiny_skia::Transform::identity(), None);
        }

        let stride = width * 4;
        let (buffer, canvas) = self
            .pool
            .create_buffer(width, height, stride, wl_shm::Format::Argb8888)
            .expect("failed to create shm buffer");
        bgra_from_rgba(pixmap.data(), canvas);

        let surface = self.layer.wl_surface();
        surface.set_buffer_scale(self.output_scale.max(1));
        buffer.attach_to(surface).expect("failed to attach buffer");
        surface.damage_buffer(0, 0, width, height);
        surface.frame(qh, surface.clone());
        self.awaiting_frame = true;
        surface.commit();
    }

    pub(super) fn tick_app_search_frame(&mut self, qh: &QueueHandle<Self>) {
        let Some(m) = self.app_search_mode.as_mut() else {
            return;
        };
        let fade_animating = if m.anim < m.target_anim {
            m.anim = (m.anim + menu::ANIM_STEP_OPEN).min(m.target_anim);
            true
        } else if m.anim > m.target_anim {
            m.anim = (m.anim - menu::ANIM_STEP_CLOSE).max(m.target_anim);
            true
        } else {
            false
        };
        let scroll_delta = m.scroll_target - m.scroll_x;
        let scroll_animating = if scroll_delta.abs() > 0.5 {
            m.scroll_x += scroll_delta * 0.28;
            true
        } else {
            m.scroll_x = m.scroll_target;
            false
        };
        let highlight_delta = m.highlight_target - m.highlight_x;
        let highlight_animating = if highlight_delta.abs() > 0.5 {
            m.highlight_x += highlight_delta * 0.35;
            true
        } else {
            m.highlight_x = m.highlight_target;
            false
        };
        let content_animating = if m.content_anim < 1.0 {
            m.content_anim = (m.content_anim + 0.16).min(1.0);
            true
        } else {
            false
        };
        let closing = m.closing;
        let anim = m.anim;

        if closing && anim <= 0.0 {
            self.app_search_mode = None;
            self.layer.set_keyboard_interactivity(KeyboardInteractivity::None);
            let (w, h) = self.dock.base_size();
            self.layer.set_size(w, h);
            self.draw(qh);
            trim_heap();
            return;
        }

        let key_repeating = self.held_key.is_some();
        if let Some((keysym, steps)) = self.poll_held_key() {
            for _ in 0..steps {
                self.handle_app_search_key(KeyEvent { time: 0, raw_code: 0, keysym, utf8: None }, qh);
            }
        }

        if !fade_animating && !scroll_animating && !highlight_animating && !content_animating && !key_repeating {
            return;
        }
        self.draw_app_search_mode(qh);
    }

    pub(super) fn handle_app_search_pointer_event(&mut self, event: &PointerEvent, qh: &QueueHandle<Self>) {
        match event.kind {
            PointerEventKind::Enter { .. } | PointerEventKind::Motion { .. } => {
                let (x, y) = event.position;
                if let Some(m) = self.app_search_mode.as_mut() {
                    let strip_hit = menu::app_search_strip_hit_test(m.filtered.len(), m.panel_w, m.panel_h, m.is_vertical, m.scroll_x, x as f32, y as f32);
                    m.hovered = strip_hit.map(menu::HitTarget::AppEntry);
                    if let Some(i) = strip_hit {
                        m.highlight_target = menu::app_search_card_along(i);
                    }
                }
                self.request_redraw(qh);
            }
            PointerEventKind::Leave { .. } => {
                if let Some(m) = self.app_search_mode.as_mut() {
                    m.hovered = None;
                }
                self.request_redraw(qh);
            }
            PointerEventKind::Press { button, .. } if button == BTN_LEFT => {
                let (x, y) = event.position;
                let hit = self
                    .app_search_mode
                    .as_ref()
                    .and_then(|m| menu::app_search_strip_hit_test(m.filtered.len(), m.panel_w, m.panel_h, m.is_vertical, m.scroll_x, x as f32, y as f32));
                if let Some(i) = hit {
                    self.launch_search_result(i, qh);
                }
            }
            PointerEventKind::Press { button, .. } if button == BTN_RIGHT => {
                self.close_app_search_mode(qh);
            }
            PointerEventKind::Axis { horizontal, vertical, .. } => {
                let delta = if horizontal.absolute != 0.0 { horizontal.absolute } else { vertical.absolute };
                if let Some(m) = self.app_search_mode.as_mut() {
                    let viewport_along = menu::app_search_viewport_along(m.panel_w, m.panel_h, m.is_vertical);
                    let max_scroll = menu::app_search_strip_max_scroll(m.filtered.len(), viewport_along);
                    m.scroll_target = (m.scroll_target + delta as f32).clamp(0.0, max_scroll);
                }
                self.request_redraw(qh);
            }
            _ => {}
        }
    }

    pub(super) fn handle_app_search_key(&mut self, event: KeyEvent, qh: &QueueHandle<Self>) {
        if event.keysym == Keysym::Escape {
            self.close_app_search_mode(qh);
            return;
        }
        match event.keysym {
            Keysym::Return => {
                let selected = self.app_search_mode.as_ref().map(|m| m.selected).unwrap_or(0);
                self.launch_search_result(selected, qh);
                return;
            }
            Keysym::Right | Keysym::Down => {
                let next = self
                    .app_search_mode
                    .as_ref()
                    .filter(|m| !m.filtered.is_empty())
                    .map(|m| (m.selected + 1).min(m.filtered.len() - 1));
                if let Some(next) = next {
                    if let Some(m) = self.app_search_mode.as_mut() {
                        m.selected = next;
                        m.hovered = None;
                    }
                    self.scroll_search_into_view(next, qh);
                }
                return;
            }
            Keysym::Left | Keysym::Up => {
                let next = self.app_search_mode.as_ref().map(|m| m.selected.saturating_sub(1));
                if let Some(next) = next {
                    if let Some(m) = self.app_search_mode.as_mut() {
                        m.selected = next;
                        m.hovered = None;
                    }
                    self.scroll_search_into_view(next, qh);
                }
                return;
            }
            _ => {}
        }
        if let Some(m) = self.app_search_mode.as_mut() {
            if event.keysym == Keysym::BackSpace {
                m.query.pop();
            } else if let Some(text) = &event.utf8 {
                for ch in text.chars() {
                    if !ch.is_control() {
                        m.query.push(ch);
                    }
                }
            }
        }
        self.refresh_app_search(qh);
    }
}
