use super::*;

impl App {
    pub(crate) fn toggle_dock_menu(&mut self, qh: &QueueHandle<Self>) {
        if self.dock_menu_mode.is_some() {
            self.close_dock_menu(qh);
        } else {
            self.open_dock_menu(qh);
        }
    }

    pub(super) fn open_dock_menu(&mut self, qh: &QueueHandle<Self>) {
        self.wallpaper_mode = None;
        self.clipboard_mode = None;
        self.app_search_mode = None;
        self.osd_mode = None;
        self.notification_mode = None;
        self.popup_mode = None;
        self.held_key = None;
        self.layer.set_layer(Layer::Top);
        self.menu = None;
        let category = menu::MenuCategory::Layout;
        let controls = menu::build_category_controls(category, &self.dock.config.settings);
        let panel_w = menu::DOCK_MENU_LEFT_COL_W + menu::DOCK_MENU_DIVIDER_W + menu::DOCK_MENU_RIGHT_COL_W;
        let panel_h = menu::dock_menu_content_height(category, &self.dock.config.settings);
        let s = &self.dock.config.settings;
        let (anchor, margin) = edge_anchor_margin(s.dock_edge, s.dock_align, s.pos_y, 0);
        self.layer.set_anchor(anchor);
        self.layer.set_margin(margin.0, margin.1, margin.2, margin.3);
        self.layer.set_size(panel_w as u32, panel_h as u32);
        self.layer.set_keyboard_interactivity(KeyboardInteractivity::OnDemand);
        self.dock_menu_mode = Some(DockMenuMode {
            category,
            controls,
            hovered: None,
            dragging_slider: None,
            held_stepper: None,
            dragging_widget: None,
            anim: 0.0,
            target_anim: 1.0,
            closing: false,
            panel_w,
            panel_h,
            slide_anim: 1.0,
            slide_dir: 0.0,
            open_dropdown: menu::OpenDropdown::None,
            dropdown_anim: 0.0,
            dropdown_scroll: 0,
            font_query: String::new(),
            dropdown_selected: 0,
            custom_hex: Default::default(),
            custom_light: true,
            custom_focus: None,
            custom_name: String::new(),
            custom_name_focused: false,
            custom_panel_blend: None,
        });
        self.request_redraw(qh);
    }

    pub(super) fn close_dock_menu(&mut self, qh: &QueueHandle<Self>) {
        if let Some(dm) = self.dock_menu_mode.as_mut() {
            dm.closing = true;
            dm.target_anim = 0.0;
            dm.dragging_slider = None;
            dm.held_stepper = None;
        }
        self.request_redraw(qh);
    }

    pub(super) fn switch_dock_menu_category(&mut self, category: menu::MenuCategory, qh: &QueueHandle<Self>) {
        let panel_w = self.dock_menu_mode.as_ref().map(|dm| dm.panel_w);
        if let Some(dm) = self.dock_menu_mode.as_mut() {
            if category != dm.category {
                dm.slide_dir = (menu::category_order_index(category) - menu::category_order_index(dm.category)).signum() as f32;
                dm.slide_anim = 0.0;
                dm.panel_h = menu::dock_menu_content_height(category, &self.dock.config.settings);
            }
            dm.category = category;
            dm.controls = menu::build_category_controls(category, &self.dock.config.settings);
            dm.hovered = None;
            dm.open_dropdown = menu::OpenDropdown::None;
            dm.dropdown_scroll = 0;
            dm.font_query.clear();
            dm.dropdown_selected = 0;
        }
        if let (Some(panel_w), Some(panel_h)) = (panel_w, self.dock_menu_mode.as_ref().map(|dm| dm.panel_h)) {
            let s = &self.dock.config.settings;
            let (anchor, margin) = edge_anchor_margin(s.dock_edge, s.dock_align, s.pos_y, 0);
            self.layer.set_anchor(anchor);
            self.layer.set_margin(margin.0, margin.1, margin.2, margin.3);
            self.layer.set_size(panel_w as u32, panel_h as u32);
        }
        self.request_redraw(qh);
    }

    pub(super) fn draw_dock_menu_mode(&mut self, qh: &QueueHandle<Self>) {
        let scale = self.output_scale.max(1) as f32;
        let transparency = self.dock.config.settings.transparency;
        let Some(dm) = self.dock_menu_mode.as_mut() else {
            return;
        };
        let linear = dm.anim.clamp(0.0, 1.0);
        let eased = 0.5 - 0.5 * (std::f32::consts::PI * linear).cos();

        let width = (dm.panel_w * scale).round() as i32;
        let height = (dm.panel_h * scale).round() as i32;
        if width <= 0 || height <= 0 {
            return;
        }

        let slide_t = dm.slide_anim.clamp(0.0, 1.0);
        let slide_eased = 0.5 - 0.5 * (std::f32::consts::PI * slide_t).cos();
        let slide_offset = dm.slide_dir * (1.0 - slide_eased) * menu::TAB_SLIDE_DISTANCE * scale;

        let args = menu_render::DockMenuArgs {
            category: dm.category,
            controls: &dm.controls,
            panel_w: dm.panel_w,
            panel_h: dm.panel_h,
            slide_offset,
            dock: &self.dock,
            hovered: dm.hovered,
            render_scale: scale,
            dragging_widget: dm.dragging_widget,
            open_dropdown: dm.open_dropdown,
            dropdown_anim: dm.dropdown_anim,
            dropdown_scroll: dm.dropdown_scroll,
            available_fonts: &self.available_fonts,
            font_query: &dm.font_query,
            dropdown_selected: dm.dropdown_selected,
            custom_hex: &dm.custom_hex,
            custom_light: dm.custom_light,
            custom_focus: dm.custom_focus,
            custom_name: &dm.custom_name,
            custom_name_focused: dm.custom_name_focused,
            custom_panel_blend: dm.custom_panel_blend,
        };
        let mut pixmap = tiny_skia::Pixmap::new(width as u32, height as u32).unwrap();
        if eased >= 0.999 {
            menu_render::draw_dock_menu(&mut pixmap, &mut self.icon_cache, &mut self.text_cache, &args);
        } else {
            let mut content = tiny_skia::Pixmap::new(width as u32, height as u32).unwrap();
            menu_render::draw_dock_menu(&mut content, &mut self.icon_cache, &mut self.text_cache, &args);
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

    pub(super) fn tick_dock_menu_frame(&mut self, qh: &QueueHandle<Self>) {
        let Some(dm) = self.dock_menu_mode.as_mut() else {
            return;
        };
        let fade_animating = if dm.anim < dm.target_anim {
            dm.anim = (dm.anim + menu::ANIM_STEP_OPEN).min(dm.target_anim);
            true
        } else if dm.anim > dm.target_anim {
            dm.anim = (dm.anim - menu::ANIM_STEP_CLOSE).max(dm.target_anim);
            true
        } else {
            false
        };
        let dropdown_target = if dm.open_dropdown != menu::OpenDropdown::None { 1.0 } else { 0.0 };
        let dropdown_animating = if dm.dropdown_anim < dropdown_target {
            dm.dropdown_anim = (dm.dropdown_anim + menu::ANIM_STEP_OPEN).min(dropdown_target);
            true
        } else if dm.dropdown_anim > dropdown_target {
            dm.dropdown_anim = (dm.dropdown_anim - menu::ANIM_STEP_CLOSE).max(dropdown_target);
            true
        } else {
            false
        };
        let slide_animating = if dm.slide_anim < 1.0 {
            dm.slide_anim = (dm.slide_anim + menu::ANIM_STEP_OPEN).min(1.0);
            true
        } else {
            false
        };
        let closing = dm.closing;
        let anim = dm.anim;
        let has_stepper = dm.held_stepper.is_some();

        if closing && anim <= 0.0 {
            self.dock_menu_mode = None;
            self.layer.set_keyboard_interactivity(KeyboardInteractivity::None);
            self.relayout_dock(qh);
            trim_heap();
            return;
        }
        if has_stepper {
            self.tick_dock_menu_held_stepper(qh);
            return;
        }

        let key_repeating = self.held_key.is_some();
        if let Some((keysym, steps)) = self.poll_held_key() {
            for _ in 0..steps {
                self.handle_dock_menu_key(KeyEvent { time: 0, raw_code: 0, keysym, utf8: None }, qh);
            }
        }

        if !fade_animating && !dropdown_animating && !slide_animating && !key_repeating {
            return;
        }
        self.draw_dock_menu_mode(qh);
    }

    pub(super) fn tick_dock_menu_held_stepper(&mut self, qh: &QueueHandle<Self>) {
        let Some((id, dir, frames)) = self.dock_menu_mode.as_ref().and_then(|dm| dm.held_stepper) else {
            return;
        };
        let (_, _, step) = id.range();
        let speed = menu::stepper_speed(frames);
        let cur = id.get(&self.dock.config.settings);
        id.set(&mut self.dock.config.settings, cur + dir * step * speed * 0.05);
        self.on_setting_changed(id, qh, false);
        if let Some(dm) = self.dock_menu_mode.as_mut() {
            dm.held_stepper = Some((id, dir, frames + 1));
        }
        self.request_redraw(qh);
    }

    pub(super) fn start_dock_menu_stepper(&mut self, id: menu::SettingId, dir: f32, qh: &QueueHandle<Self>) {
        let (_, _, step) = id.range();
        let cur = id.get(&self.dock.config.settings);
        id.set(&mut self.dock.config.settings, cur + dir * step);
        self.on_setting_changed(id, qh, true);
        if let Some(dm) = self.dock_menu_mode.as_mut() {
            dm.held_stepper = Some((id, dir, 0));
        }
        self.request_redraw(qh);
    }

    pub(super) fn handle_dock_menu_button(&mut self, kind: menu::ButtonKind, qh: &QueueHandle<Self>) {
        match kind {
            menu::ButtonKind::AddApp => {
                self.dock_menu_mode = None;
                self.relayout_dock(qh);
                self.open_menu(menu::MenuScreen::AddApp, qh);
            }
            menu::ButtonKind::QuitDock => self.exit = true,
            menu::ButtonKind::CreatePalette => self.open_custom_palette(qh),
            menu::ButtonKind::SavePalette => self.save_custom_palette(qh),
            menu::ButtonKind::Back => {
                if matches!(self.dock_menu_mode.as_ref().map(|dm| dm.category), Some(menu::MenuCategory::CustomPalette)) {
                    self.switch_dock_menu_category(menu::MenuCategory::Colors, qh);
                }
            }
            _ => {}
        }
    }

    pub(super) fn open_custom_palette(&mut self, qh: &QueueHandle<Self>) {
        let s = &self.dock.config.settings;
        let hex = [
            rgb_to_hex(s.accent_r, s.accent_g, s.accent_b),
            rgb_to_hex(s.accent2_r, s.accent2_g, s.accent2_b),
            rgb_to_hex(s.panel_r, s.panel_g, s.panel_b),
            rgb_to_hex(s.text_r, s.text_g, s.text_b),
            rgb_to_hex(s.text_dim_r, s.text_dim_g, s.text_dim_b),
        ];
        let panel_luma = 0.299 * s.panel_r as f32 + 0.587 * s.panel_g as f32 + 0.114 * s.panel_b as f32;
        let is_light = panel_luma > 140.0;
        self.switch_dock_menu_category(menu::MenuCategory::CustomPalette, qh);
        if let Some(dm) = self.dock_menu_mode.as_mut() {
            dm.custom_hex = hex;
            dm.custom_light = is_light;
            dm.custom_focus = None;
            dm.custom_name.clear();
            dm.custom_name_focused = false;
            dm.custom_panel_blend = None;
        }
        self.request_redraw(qh);
    }

    pub(super) fn reseed_custom_palette_base(&mut self, is_light: bool) {
        let (panel, text, text_dim) = if is_light { ((245, 245, 248), (30, 30, 34), (100, 100, 108)) } else { ((18, 18, 20), (235, 235, 240), (150, 150, 158)) };
        if let Some(dm) = self.dock_menu_mode.as_mut() {
            dm.custom_hex[2] = rgb_to_hex(panel.0, panel.1, panel.2);
            dm.custom_hex[3] = rgb_to_hex(text.0, text.1, text.2);
            dm.custom_hex[4] = rgb_to_hex(text_dim.0, text_dim.1, text_dim.2);
        }
    }

    // ----- preset blend ratio -----
    pub(super) fn apply_panel_blend(&mut self, accent_pct: u8, qh: &QueueHandle<Self>) {
        let Some(dm) = self.dock_menu_mode.as_mut() else {
            return;
        };
        let base = if dm.custom_light { (245u8, 245u8, 248u8) } else { (18u8, 18u8, 20u8) };
        let accent = menu::parse_hex(&dm.custom_hex[0]).unwrap_or((self.dock.config.settings.accent_r, self.dock.config.settings.accent_g, self.dock.config.settings.accent_b));
        let t = accent_pct as f32 / 100.0;
        let mix = |b: u8, a: u8| (b as f32 * (1.0 - t) + a as f32 * t).round() as u8;
        let panel = (mix(base.0, accent.0), mix(base.1, accent.1), mix(base.2, accent.2));
        dm.custom_hex[2] = rgb_to_hex(panel.0, panel.1, panel.2);
        dm.custom_panel_blend = Some(accent_pct);
        self.request_redraw(qh);
    }

    pub(super) fn delete_custom_palette(&mut self, index: usize, qh: &QueueHandle<Self>) {
        if index >= self.dock.config.settings.custom_palettes.len() {
            return;
        }
        self.dock.config.settings.custom_palettes.remove(index);
        let _ = self.dock.config.save();

        let category = self.dock_menu_mode.as_ref().map(|dm| dm.category);
        let Some(category) = category else {
            self.request_redraw(qh);
            return;
        };
        let panel_h = menu::dock_menu_content_height(category, &self.dock.config.settings);
        let panel_w = self.dock_menu_mode.as_ref().map(|dm| dm.panel_w);
        if let Some(dm) = self.dock_menu_mode.as_mut() {
            dm.controls = menu::build_category_controls(category, &self.dock.config.settings);
            dm.panel_h = panel_h;
            dm.hovered = None;
        }
        if let Some(panel_w) = panel_w {
            let s = &self.dock.config.settings;
            let (anchor, margin) = edge_anchor_margin(s.dock_edge, s.dock_align, s.pos_y, 0);
            self.layer.set_anchor(anchor);
            self.layer.set_margin(margin.0, margin.1, margin.2, margin.3);
            self.layer.set_size(panel_w as u32, panel_h as u32);
        }
        self.request_redraw(qh);
    }

    pub(super) fn save_custom_palette(&mut self, qh: &QueueHandle<Self>) {
        let Some(dm) = self.dock_menu_mode.as_ref() else {
            return;
        };
        let hex = dm.custom_hex.clone();
        let is_light = dm.custom_light;
        let name = if dm.custom_name.trim().is_empty() { "My Palette".to_string() } else { dm.custom_name.trim().to_string() };
        let cur = &self.dock.config.settings;
        let fallback = [
            (cur.accent_r, cur.accent_g, cur.accent_b),
            (cur.accent2_r, cur.accent2_g, cur.accent2_b),
            (cur.panel_r, cur.panel_g, cur.panel_b),
            (cur.text_r, cur.text_g, cur.text_b),
            (cur.text_dim_r, cur.text_dim_g, cur.text_dim_b),
        ];
        let color_at = |i: usize| menu::parse_hex(&hex[i]).unwrap_or(fallback[i]);
        let custom = crate::config::CustomPalette {
            name,
            accent: color_at(0),
            accent2: color_at(1),
            panel: color_at(2),
            text: color_at(3),
            text_dim: color_at(4),
            is_light,
        };
        self.dock.config.settings.custom_palettes.push(custom);
        let _ = self.dock.config.save();
        self.switch_dock_menu_category(menu::MenuCategory::Colors, qh);
    }
}
