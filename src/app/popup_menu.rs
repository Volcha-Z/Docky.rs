use super::*;

impl App {
    pub(super) fn open_menu(&mut self, screen: menu::MenuScreen, qh: &QueueHandle<Self>) {
        self.menu = None;
        self.held_key = None;

        let app_entries = if screen == menu::MenuScreen::AddApp {
            desktop::list_all_desktop_entries()
        } else {
            Vec::new()
        };
        let (controls, content_height) = menu::build_controls(screen, app_entries.len(), 0);

        let surface = self.compositor.create_surface(qh);
        let layer = self.layer_shell.create_layer_surface(qh, surface, Layer::Overlay, Some("dockyrs-menu"), None);
        let s = &self.dock.config.settings;
        let (anchor, margin) = edge_anchor_margin(s.dock_edge, s.dock_align, s.pos_y, self.dock.thickness() as i32 + MENU_GAP);
        layer.set_anchor(anchor);
        layer.set_margin(margin.0, margin.1, margin.2, margin.3);
        layer.set_size(menu::MENU_WIDTH as u32, content_height as u32);
        layer.set_keyboard_interactivity(KeyboardInteractivity::OnDemand);
        layer.set_exclusive_zone(-1);
        layer.commit();

        let pool_size = (menu::MENU_WIDTH as usize * content_height as usize * 16).max(65536);
        let pool = SlotPool::new(pool_size, &self.shm).expect("failed to create menu shm pool");

        self.menu = Some(MenuState {
            layer,
            pool,
            screen,
            controls,
            content_height,
            filtered_app_entries: app_entries.clone(),
            app_entries,
            all_icon_names: Vec::new(),
            filtered_icon_names: Vec::new(),
            search_query: String::new(),
            anim: 0.0,
            target_anim: 1.0,
            closing: false,
            awaiting_frame: false,
            hovered: None,
            dragging_slider: None,
            held_stepper: None,
            list_scroll: 0,
            list_selected: 0,
        });
    }
    pub(super) fn close_menu(&mut self, qh: &QueueHandle<Self>) {
        if let Some(menu) = self.menu.as_mut() {
            menu.closing = true;
            menu.target_anim = 0.0;
            menu.dragging_slider = None;
            menu.held_stepper = None;
        }
        self.request_menu_redraw(qh);
    }

    pub(super) fn switch_menu_screen(&mut self, screen: menu::MenuScreen, qh: &QueueHandle<Self>) {
        let app_entries = if screen == menu::MenuScreen::AddApp {
            desktop::list_all_desktop_entries()
        } else {
            Vec::new()
        };
        let (controls, content_height) = menu::build_controls(screen, app_entries.len(), 0);
        let s = &self.dock.config.settings;
        let (anchor, margin) = edge_anchor_margin(s.dock_edge, s.dock_align, s.pos_y, self.dock.thickness() as i32 + MENU_GAP);
        if let Some(menu) = self.menu.as_mut() {
            menu.screen = screen;
            menu.controls = controls;
            menu.content_height = content_height;
            menu.filtered_app_entries = app_entries.clone();
            menu.app_entries = app_entries;
            menu.search_query.clear();
            menu.hovered = None;
            menu.list_scroll = 0;
            menu.list_selected = 0;
            menu.layer.set_anchor(anchor);
            menu.layer.set_margin(margin.0, margin.1, margin.2, margin.3);
            menu.layer.set_size(menu::MENU_WIDTH as u32, content_height as u32);
        }
        self.request_menu_redraw(qh);
    }
    pub(super) fn draw_menu(&mut self, qh: &QueueHandle<Self>) {
        let scale = self.output_scale.max(1) as f32;
        let transparency = self.dock.config.settings.transparency;
        let Some(menu) = self.menu.as_mut() else {
            return;
        };
        let linear = menu.anim.clamp(0.0, 1.0);
        let eased = 0.5 - 0.5 * (std::f32::consts::PI * linear).cos();

        let width = (menu::MENU_WIDTH * scale).round() as i32;
        let full_h = (menu.content_height * scale).round().max(1.0) as u32;

        let args = menu_render::DrawArgs {
            screen: menu.screen,
            controls: &menu.controls,
            content_height: menu.content_height,
            panel_width: menu::MENU_WIDTH,
            dock: &self.dock,
            app_entries: &menu.filtered_app_entries,
            icon_choices: &menu.filtered_icon_names,
            search_query: &menu.search_query,
            wallpapers: &[],
            wallpaper_hovered: None,
            wallpaper_scroll_x: 0.0,
            hovered: menu.hovered,
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

        if !menu.closing && linear >= 0.999 {
            if width <= 0 || full_h == 0 {
                return;
            }
            let mut pixmap = tiny_skia::Pixmap::new(width as u32, full_h).unwrap();
            menu_render::draw_content(&mut pixmap, &mut self.icon_cache, &mut self.text_cache, &mut self.thumbnail_cache, &args);
            menu.layer.set_size(menu::MENU_WIDTH as u32, menu.content_height.round() as u32);

            let stride = width * 4;
            let (buffer, canvas) = menu
                .pool
                .create_buffer(width, full_h as i32, stride, wl_shm::Format::Argb8888)
                .expect("failed to create menu shm buffer");
            bgra_from_rgba(pixmap.data(), canvas);

            let surface = menu.layer.wl_surface();
            surface.set_buffer_scale(self.output_scale.max(1));
            buffer.attach_to(surface).expect("failed to attach menu buffer");
            surface.damage_buffer(0, 0, width, full_h as i32);
            surface.frame(qh, surface.clone());
            menu.awaiting_frame = true;
            surface.commit();
            return;
        }

        let mut full_pixmap = tiny_skia::Pixmap::new(width as u32, full_h).unwrap();
        menu_render::draw_content(&mut full_pixmap, &mut self.icon_cache, &mut self.text_cache, &mut self.thumbnail_cache, &args);

        let (height, pixmap) = if menu.closing {
            let height = full_h as i32;
            if width <= 0 || height <= 0 {
                return;
            }
            let mut pixmap = tiny_skia::Pixmap::new(width as u32, height as u32).unwrap();
            let revealed_px = (height as f32 * linear).round().clamp(0.0, height as f32);
            if revealed_px > 0.5 {
                if let Some(rect) = tiny_skia::Rect::from_xywh(0.0, height as f32 - revealed_px, width as f32, revealed_px) {
                    let mut mask = tiny_skia::Mask::new(width as u32, height as u32).unwrap();
                    let path = tiny_skia::PathBuilder::from_rect(rect);
                    mask.fill_path(&path, tiny_skia::FillRule::Winding, true, tiny_skia::Transform::identity());
                    let paint = tiny_skia::PixmapPaint {
                        opacity: anim_opacity(transparency, eased),
                        ..Default::default()
                    };
                    pixmap.draw_pixmap(0, 0, full_pixmap.as_ref(), &paint, tiny_skia::Transform::identity(), Some(&mask));
                }
            }
            (height, pixmap)
        } else {
            let revealed_h = (menu.content_height * linear).max(1.0);
            let height = (revealed_h * scale).round() as i32;
            if width <= 0 || height <= 0 {
                return;
            }
            menu.layer.set_size(menu::MENU_WIDTH as u32, revealed_h.round() as u32);

            let mut pixmap = tiny_skia::Pixmap::new(width as u32, height as u32).unwrap();
            let paint = tiny_skia::PixmapPaint {
                opacity: anim_opacity(transparency, eased),
                ..Default::default()
            };
            let y_shift = height as f32 - full_h as f32;
            pixmap.draw_pixmap(0, 0, full_pixmap.as_ref(), &paint, tiny_skia::Transform::from_translate(0.0, y_shift), None);
            (height, pixmap)
        };

        let stride = width * 4;
        let (buffer, canvas) = menu
            .pool
            .create_buffer(width, height, stride, wl_shm::Format::Argb8888)
            .expect("failed to create menu shm buffer");
        bgra_from_rgba(pixmap.data(), canvas);

        let surface = menu.layer.wl_surface();
        surface.set_buffer_scale(self.output_scale.max(1));
        buffer.attach_to(surface).expect("failed to attach menu buffer");
        surface.damage_buffer(0, 0, width, height);
        surface.frame(qh, surface.clone());
        menu.awaiting_frame = true;
        surface.commit();
    }
    pub(super) fn request_menu_redraw(&mut self, qh: &QueueHandle<Self>) {
        let awaiting = self.menu.as_ref().map(|m| m.awaiting_frame).unwrap_or(true);
        if !awaiting && self.menu.is_some() {
            self.draw_menu(qh);
        }
    }

    pub(super) fn tick_menu_frame(&mut self, qh: &QueueHandle<Self>) {
        let Some(menu) = self.menu.as_mut() else {
            return;
        };
        menu.awaiting_frame = false;
        let animating = if menu.anim < menu.target_anim {
            menu.anim = (menu.anim + menu::ANIM_STEP_OPEN).min(menu.target_anim);
            true
        } else if menu.anim > menu.target_anim {
            menu.anim = (menu.anim - menu::ANIM_STEP_CLOSE).max(menu.target_anim);
            true
        } else {
            false
        };
        let closing = menu.closing;
        let anim = menu.anim;
        let has_stepper = menu.held_stepper.is_some();

        if closing && anim <= 0.0 {
            self.menu = None;
            trim_heap();
            return;
        }

        let key_repeating = self.held_key.is_some();
        if let Some((keysym, steps)) = self.poll_held_key() {
            for _ in 0..steps {
                self.handle_search_key(KeyEvent { time: 0, raw_code: 0, keysym, utf8: None }, qh);
            }
        }

        if !animating && !key_repeating {
            return;
        }

        if has_stepper {
            self.tick_held_stepper(qh);
            return;
        }

        self.draw_menu(qh);
    }

    pub(super) fn tick_held_stepper(&mut self, qh: &QueueHandle<Self>) {
        let Some((id, dir, frames)) = self.menu.as_ref().and_then(|m| m.held_stepper) else {
            return;
        };
        let (_, _, step) = id.range();
        let speed = menu::stepper_speed(frames);
        let cur = id.get(&self.dock.config.settings);
        id.set(&mut self.dock.config.settings, cur + dir * step * speed * 0.05);
        self.on_setting_changed(id, qh, false);
        if let Some(menu) = self.menu.as_mut() {
            menu.held_stepper = Some((id, dir, frames + 1));
        }
        self.request_menu_redraw(qh);
    }

    pub(super) fn start_stepper(&mut self, id: menu::SettingId, dir: f32, qh: &QueueHandle<Self>) {
        let (_, _, step) = id.range();
        let cur = id.get(&self.dock.config.settings);
        id.set(&mut self.dock.config.settings, cur + dir * step);
        self.on_setting_changed(id, qh, true);
        if let Some(menu) = self.menu.as_mut() {
            menu.held_stepper = Some((id, dir, 0));
        }
        self.request_menu_redraw(qh);
    }

    pub(super) fn on_setting_changed(&mut self, id: menu::SettingId, qh: &QueueHandle<Self>, force_external: bool) {
        if id.affects_layout() {
            self.relayout_dock(qh);
        } else {
            self.request_redraw(qh);
        }

        let now = std::time::Instant::now();
        let should_send = force_external
            || self
                .last_hyprctl_send
                .map(|t| now.duration_since(t).as_millis() >= 80)
                .unwrap_or(true);
        if !should_send {
            return;
        }
        self.last_hyprctl_send = Some(now);

        if let Some(field) = id.lua_blur_field() {
            let value = id.hyprctl_value(&self.dock.config.settings);
            let expr: String = ["hl.config({decoration={blur={", field, "=", &value, "}}})"].concat();
            let _ = std::process::Command::new("hyprctl").args(["eval", &expr]).spawn();
        }
        if let Err(err) = self.dock.config.save() {
            log::warn!("failed to save dock config: {err}");
        }
    }
}
