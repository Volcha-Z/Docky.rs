use super::*;

impl App {
    pub(crate) fn toggle_wallpaper_picker(&mut self, qh: &QueueHandle<Self>) {
        if self.wallpaper_mode.is_some() {
            self.close_wallpaper_mode(qh);
        } else {
            self.open_wallpaper_picker(qh);
        }
    }

    pub(super) fn open_wallpaper_picker(&mut self, qh: &QueueHandle<Self>) {
        self.dock_menu_mode = None;
        self.app_search_mode = None;
        self.clipboard_mode = None;
        self.osd_mode = None;
        self.notification_mode = None;
        self.popup_mode = None;
        self.held_key = None;
        self.layer.set_layer(Layer::Top);
        self.menu = None;
        let wallpapers = wallpaper::scan_wallpapers();
        let is_vertical = self.dock.is_vertical();
        let dock_along = (if is_vertical { self.dock.base_size().1 } else { self.dock.base_size().0 }) as f32;
        let along = dock_along.max(WALLPAPER_PANEL_MIN_W);
        let cross = WALLPAPER_PANEL_H;
        let (panel_w, panel_h) = if is_vertical { (cross, along) } else { (along, cross) };
        self.layer.set_keyboard_interactivity(KeyboardInteractivity::Exclusive);
        let s = &self.dock.config.settings;
        let (anchor, margin) = edge_anchor_margin(s.dock_edge, s.dock_align, s.pos_y, 0);
        self.layer.set_anchor(anchor);
        self.layer.set_margin(margin.0, margin.1, margin.2, margin.3);
        self.layer.set_size(panel_w as u32, panel_h as u32);

        let (tw_logical, th_logical) = menu::wallpaper_thumb_size(panel_w, panel_h, is_vertical);
        let scale = self.output_scale.max(1) as f32;
        let thumb_w = (tw_logical * scale).round().max(1.0) as u32;
        let thumb_h = (th_logical * scale).round().max(1.0) as u32;
        let thumb_radius = 11.0 * scale;

        let (thumb_request_tx, request_rx) = std::sync::mpsc::channel::<(std::path::PathBuf, u32, u32)>();
        let (result_tx, thumb_result_rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            while let Ok((path, w, h)) = request_rx.recv() {
                let pixmap = crate::thumbnail_cache::load(&path, w, h, thumb_radius);
                if result_tx.send((path, w, h, pixmap)).is_err() {
                    return;
                }
            }
        });

        self.wallpaper_mode = Some(WallpaperMode {
            wallpapers,
            hovered: None,
            scroll_x: 0.0,
            scroll_target: 0.0,
            anim: 0.0,
            target_anim: 1.0,
            closing: false,
            panel_w,
            panel_h,
            is_vertical,
            thumb_w,
            thumb_h,
            thumb_requested: std::collections::HashSet::new(),
            thumb_request_tx,
            thumb_result_rx,
        });
        self.request_visible_thumbnails();
        self.request_redraw(qh);
    }

    pub(super) fn request_visible_thumbnails(&mut self) {
        let Some(wp) = self.wallpaper_mode.as_mut() else {
            return;
        };
        let (tw_logical, th_logical) = menu::wallpaper_thumb_size(wp.panel_w, wp.panel_h, wp.is_vertical);
        let along_size = if wp.is_vertical { th_logical } else { tw_logical };
        let viewport_along = if wp.is_vertical { wp.panel_h } else { wp.panel_w };
        let margin = along_size * 2.0;
        let visible_0 = wp.scroll_x - margin;
        let visible_1 = wp.scroll_x + viewport_along + margin;
        for (idx, entry) in wp.wallpapers.iter().enumerate() {
            let local = idx as f32 * (along_size + menu::WALLPAPER_GAP);
            if local + along_size < visible_0 || local > visible_1 {
                continue;
            }
            if wp.thumb_requested.contains(&entry.path) {
                continue;
            }
            wp.thumb_requested.insert(entry.path.clone());
            let _ = wp.thumb_request_tx.send((entry.path.clone(), wp.thumb_w, wp.thumb_h));
        }
    }

    pub(super) fn choose_wallpaper(&mut self, index: usize, qh: &QueueHandle<Self>) {
        let path = self.wallpaper_mode.as_ref().and_then(|w| w.wallpapers.get(index)).map(|e| e.path.clone());
        if let Some(path) = &path {
            wallpaper::apply_wallpaper(path.clone());
            self.dock.config.settings.last_wallpaper = path.to_string_lossy().to_string();
            if self.dock.config.settings.accent_from_wallpaper {
                self.sync_accent_from_last_wallpaper();
                let dir = repo_dir();
                let _ = std::process::Command::new(dir.join("sync-dolphin-theme.sh")).spawn();
                let _ = std::process::Command::new(dir.join("sync-kitty-theme.sh")).spawn();
                let _ = std::process::Command::new(dir.join("sync-p10k-theme.sh")).spawn();
            } else {
                let _ = self.dock.config.save();
            }
        }
        if let Some(wp) = self.wallpaper_mode.as_mut() {
            wp.hovered = Some(menu::WallpaperHit::Thumbnail(index));
        }
        self.request_redraw(qh);
    }

    pub(super) fn sync_accent_from_last_wallpaper(&mut self) {
        let mut path = self.dock.config.settings.last_wallpaper.clone();
        if path.is_empty() {
            path = wallpaper::current_wallpaper_path().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
            self.dock.config.settings.last_wallpaper = path.clone();
        }
        if path.is_empty() {
            return;
        }
        let matugen_scheme = self.dock.config.settings.matugen_scheme.clone();
        if let Some(scheme) = wallpaper::extract_color_scheme(std::path::Path::new(&path), &matugen_scheme) {
            let s = &mut self.dock.config.settings;
            (s.accent_r, s.accent_g, s.accent_b) = scheme.primary;
            (s.accent2_r, s.accent2_g, s.accent2_b) = scheme.secondary;
            (s.on_accent_r, s.on_accent_g, s.on_accent_b) = scheme.on_primary;
            (s.panel_r, s.panel_g, s.panel_b) = scheme.surface;
            (s.text_r, s.text_g, s.text_b) = scheme.on_surface;
            (s.text_dim_r, s.text_dim_g, s.text_dim_b) = scheme.outline;
        }
        let _ = self.dock.config.save();
    }

    pub(super) fn close_wallpaper_mode(&mut self, qh: &QueueHandle<Self>) {
        if let Some(wp) = self.wallpaper_mode.as_mut() {
            wp.closing = true;
            wp.target_anim = 0.0;
        }
        self.request_redraw(qh);
    }

    pub(super) fn scroll_wallpaper_into_view(&mut self, index: usize, qh: &QueueHandle<Self>) {
        let Some(wp) = self.wallpaper_mode.as_mut() else {
            return;
        };
        let (tw, th) = menu::wallpaper_thumb_size(wp.panel_w, wp.panel_h, wp.is_vertical);
        let along = if wp.is_vertical { th } else { tw };
        let cx = index as f32 * (along + menu::WALLPAPER_GAP);
        let viewport_along = ((if wp.is_vertical { wp.panel_h } else { wp.panel_w }) - menu::WALLPAPER_BACK_ZONE_W).max(1.0);
        let mut target = wp.scroll_target;
        if cx < target {
            target = cx;
        } else if cx + along > target + viewport_along {
            target = cx + along - viewport_along;
        }
        let max_scroll = menu::wallpaper_max_scroll(wp.wallpapers.len(), wp.panel_w, wp.panel_h, wp.is_vertical);
        wp.scroll_target = target.clamp(0.0, max_scroll);
        self.request_redraw(qh);
    }
    pub(super) fn handle_wallpaper_key(&mut self, keysym: Keysym, qh: &QueueHandle<Self>) {
        let Some(wp) = self.wallpaper_mode.as_ref() else {
            return;
        };
        if keysym == Keysym::Escape {
            self.close_wallpaper_mode(qh);
            return;
        }
        if wp.wallpapers.is_empty() {
            return;
        }
        let current = match wp.hovered {
            Some(menu::WallpaperHit::Thumbnail(i)) => i,
            _ => 0,
        };
        match keysym {
            Keysym::Left | Keysym::Up => {
                let next = current.saturating_sub(1);
                if let Some(wp) = self.wallpaper_mode.as_mut() {
                    wp.hovered = Some(menu::WallpaperHit::Thumbnail(next));
                }
                self.scroll_wallpaper_into_view(next, qh);
            }
            Keysym::Right | Keysym::Down => {
                let next = (current + 1).min(wp.wallpapers.len() - 1);
                if let Some(wp) = self.wallpaper_mode.as_mut() {
                    wp.hovered = Some(menu::WallpaperHit::Thumbnail(next));
                }
                self.scroll_wallpaper_into_view(next, qh);
            }
            Keysym::Return => {
                self.choose_wallpaper(current, qh);
            }
            _ => {}
        }
    }
    pub(super) fn draw_wallpaper_mode(&mut self, qh: &QueueHandle<Self>) {
        let scale = self.output_scale.max(1) as f32;
        if let Some(wp) = self.wallpaper_mode.as_ref() {
            while let Ok((path, w, h, pixmap)) = wp.thumb_result_rx.try_recv() {
                self.thumbnail_cache.insert(&path, w, h, pixmap);
            }
        }
        self.request_visible_thumbnails();
        let transparency = self.dock.config.settings.transparency;
        let Some(wp) = self.wallpaper_mode.as_mut() else {
            return;
        };
        let linear = wp.anim.clamp(0.0, 1.0);
        let eased = 0.5 - 0.5 * (std::f32::consts::PI * linear).cos();

        let width = (wp.panel_w * scale).round() as i32;
        let height = (wp.panel_h * scale).round() as i32;
        if width <= 0 || height <= 0 {
            return;
        }

        let args = menu_render::DrawArgs {
            screen: menu::MenuScreen::WallpaperPicker,
            controls: &[],
            content_height: wp.panel_h,
            panel_width: wp.panel_w,
            dock: &self.dock,
            app_entries: &[],
            icon_choices: &[],
            search_query: "",
            wallpapers: &wp.wallpapers,
            wallpaper_hovered: wp.hovered,
            wallpaper_scroll_x: wp.scroll_x,
            hovered: None,
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
            menu_render::draw_content(&mut pixmap, &mut self.icon_cache, &mut self.text_cache, &self.thumbnail_cache, &args);
        } else {
            let mut content = tiny_skia::Pixmap::new(width as u32, height as u32).unwrap();
            menu_render::draw_content(&mut content, &mut self.icon_cache, &mut self.text_cache, &self.thumbnail_cache, &args);
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
    pub(super) fn tick_wallpaper_frame(&mut self, qh: &QueueHandle<Self>) {
        let Some(wp) = self.wallpaper_mode.as_mut() else {
            return;
        };
        let fade_animating = if wp.anim < wp.target_anim {
            wp.anim = (wp.anim + menu::ANIM_STEP_OPEN).min(wp.target_anim);
            true
        } else if wp.anim > wp.target_anim {
            wp.anim = (wp.anim - menu::ANIM_STEP_CLOSE).max(wp.target_anim);
            true
        } else {
            false
        };
        let scroll_delta = wp.scroll_target - wp.scroll_x;
        let scroll_animating = if scroll_delta.abs() > 0.5 {
            wp.scroll_x += scroll_delta * 0.28;
            true
        } else {
            wp.scroll_x = wp.scroll_target;
            false
        };
        let closing = wp.closing;
        let anim = wp.anim;

        if closing && anim <= 0.0 {
            self.wallpaper_mode = None;
            self.layer.set_keyboard_interactivity(KeyboardInteractivity::None);
            let (w, h) = self.dock.base_size();
            self.layer.set_size(w, h);
            self.draw(qh);
            self.thumbnail_cache.clear();
            trim_heap();
            return;
        }

        let key_repeating = self.held_key.is_some();
        if let Some((keysym, steps)) = self.poll_held_key() {
            for _ in 0..steps {
                self.handle_wallpaper_key(keysym, qh);
            }
        }

        if !fade_animating && !scroll_animating && !key_repeating {
            return;
        }
        self.draw_wallpaper_mode(qh);
    }
    pub(super) fn handle_wallpaper_pointer_event(&mut self, event: &PointerEvent, qh: &QueueHandle<Self>) {
        match event.kind {
            PointerEventKind::Enter { .. } | PointerEventKind::Motion { .. } => {
                let (x, y) = event.position;
                if let Some(wp) = self.wallpaper_mode.as_mut() {
                    wp.hovered = menu::wallpaper_hit_test(wp.wallpapers.len(), wp.panel_w, wp.panel_h, wp.is_vertical, wp.scroll_x, x as f32, y as f32);
                }
                self.request_redraw(qh);
            }
            PointerEventKind::Leave { .. } => {
                if let Some(wp) = self.wallpaper_mode.as_mut() {
                    wp.hovered = None;
                }
                self.request_redraw(qh);
            }
            PointerEventKind::Press { button, .. } if button == BTN_LEFT => {
                let (x, y) = event.position;
                let hit = self
                    .wallpaper_mode
                    .as_ref()
                    .and_then(|wp| menu::wallpaper_hit_test(wp.wallpapers.len(), wp.panel_w, wp.panel_h, wp.is_vertical, wp.scroll_x, x as f32, y as f32));
                match hit {
                    Some(menu::WallpaperHit::Back) => self.close_wallpaper_mode(qh),
                    Some(menu::WallpaperHit::Thumbnail(i)) => self.choose_wallpaper(i, qh),
                    None => {}
                }
            }
            PointerEventKind::Axis { horizontal, vertical, .. } => {
                let delta = if horizontal.absolute != 0.0 { horizontal.absolute } else { vertical.absolute };
                if let Some(wp) = self.wallpaper_mode.as_mut() {
                    let max_scroll = menu::wallpaper_max_scroll(wp.wallpapers.len(), wp.panel_w, wp.panel_h, wp.is_vertical);
                    wp.scroll_target = (wp.scroll_target + delta as f32).clamp(0.0, max_scroll);
                    wp.scroll_x = wp.scroll_target;
                }
                self.request_redraw(qh);
            }
            _ => {}
        }
    }
}
