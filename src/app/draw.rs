use super::*;

impl App {
    pub(super) fn draw(&mut self, qh: &QueueHandle<Self>) {
        self.draw_ex(qh, false, false);
    }

    pub(super) fn draw_ex(&mut self, qh: &QueueHandle<Self>, advance_marquee: bool, advance_ws: bool) {
        if self.notification_mode.is_some() {
            self.draw_notification_mode(qh);
        } else if self.osd_mode.is_some() {
            self.draw_osd_mode(qh);
        } else if self.app_search_mode.is_some() {
            self.draw_app_search_mode(qh);
        } else if self.dock_menu_mode.is_some() {
            self.draw_dock_menu_mode(qh);
        } else if self.wallpaper_mode.is_some() {
            self.draw_wallpaper_mode(qh);
        } else if self.clipboard_mode.is_some() {
            self.draw_clipboard_mode(qh);
        } else {
            self.draw_icons(qh, advance_marquee, advance_ws);
        }
    }

    pub(crate) fn tick_marquee(&mut self, qh: &QueueHandle<Self>) {
        self.draw_ex(qh, true, false);
    }

    pub(crate) fn tick_workspace(&mut self, qh: &QueueHandle<Self>) {
        self.draw_ex(qh, false, true);
    }

    // ----- idle throttle -----
    pub(super) fn set_marquee_rate(&mut self, rate: u64) {
        if rate != self.marquee_rate {
            self.marquee_rate = rate;
            let _ = self.marquee_tick_tx.send(rate);
        }
    }

    pub(super) fn draw_icons(&mut self, qh: &QueueHandle<Self>, advance_marquee: bool, advance_ws: bool) {
        let scale = self.output_scale.max(1) as f32;
        let (base_w, base_h) = self.dock.base_size();
        let width = (base_w as f32 * scale).round() as i32;
        let height = (base_h as f32 * scale).round() as i32;
        if width <= 0 || height <= 0 {
            return;
        }
        let stride = width * 4;

        let (buffer, canvas) = self
            .pool
            .create_buffer(width, height, stride, wl_shm::Format::Argb8888)
            .expect("failed to create shm buffer");

        // ----- reused buffer -----
        let pixmap = match &mut self.frame_pixmap {
            Some(p) if p.width() == width as u32 && p.height() == height as u32 => p,
            slot => {
                *slot = Some(tiny_skia::Pixmap::new(width as u32, height as u32).unwrap());
                slot.as_mut().unwrap()
            }
        };
        let tray_icons = self.tray.lock().unwrap();
        let needs_marquee = render::draw(
            pixmap,
            &self.dock,
            &mut self.icon_cache,
            &mut self.text_cache,
            &self.widgets,
            &tray_icons,
            &mut self.marquee,
            advance_marquee,
            advance_ws,
            scale,
        );
        drop(tray_icons);
        bgra_from_rgba(pixmap.data(), canvas);
        let rate: u64 = if needs_marquee { render::MARQUEE_TICK_MS } else { 0 };
        self.set_marquee_rate(rate);

        let surface = self.layer.wl_surface();
        surface.set_buffer_scale(self.output_scale.max(1));
        buffer.attach_to(surface).expect("failed to attach buffer");
        surface.damage_buffer(0, 0, width, height);
        surface.frame(qh, surface.clone());
        self.awaiting_frame = true;
        surface.commit();
    }

    pub(crate) fn request_redraw(&mut self, qh: &QueueHandle<Self>) {
        if !self.awaiting_frame {
            self.draw(qh);
        }
    }

    // ----- layer borrowed -----
    fn layer_is_borrowed(&self) -> bool {
        self.dock_menu_mode.is_some()
            || self.osd_mode.is_some()
            || self.wallpaper_mode.is_some()
            || self.notification_mode.is_some()
            || self.app_search_mode.is_some()
            || self.clipboard_mode.is_some()
    }

    pub(super) fn relayout_dock(&mut self, qh: &QueueHandle<Self>) {
        self.sync_widget_bar_len();
        self.dock.relayout();
        if !self.layer_is_borrowed() {
            let (w, h) = self.dock.base_size();
            self.layer.set_size(w, h);
            let s = &self.dock.config.settings;
            let (anchor, margin) = edge_anchor_margin(s.dock_edge, s.dock_align, s.pos_y, 0);
            self.layer.set_anchor(anchor);
            self.layer.set_margin(margin.0, margin.1, margin.2, margin.3);
            self.layer.set_exclusive_zone(-1);
            let (reserve_anchor, reserve_margin) = single_edge_anchor_margin(s.dock_edge, s.pos_y);
            self.reserve_layer.set_anchor(reserve_anchor);
            self.reserve_layer.set_margin(reserve_margin.0, reserve_margin.1, reserve_margin.2, reserve_margin.3);
            self.reserve_layer.set_exclusive_zone(self.dock.thickness() as i32 + s.pos_y);
            self.reserve_layer.commit();
        }
        self.request_redraw(qh);
    }
    fn widget_placed(&self, kind: crate::config::WidgetKind) -> bool {
        self.dock.config.settings.widgets.iter().any(|w| w.kind == kind)
    }

    pub(crate) fn refresh_clock(&mut self, qh: &QueueHandle<Self>) {
        if !self.dock.icons.is_empty() || !self.widget_placed(crate::config::WidgetKind::Clock) {
            return;
        }
        if !self.widgets.refresh_clock() {
            return;
        }
        self.sync_widget_bar_len();
        self.relayout_dock(qh);
    }

    pub(crate) fn refresh_media(&mut self, qh: &QueueHandle<Self>) {
        if !self.dock.icons.is_empty() || !self.widget_placed(crate::config::WidgetKind::Media) {
            return;
        }
        self.widgets.refresh_media();
        // ----- fixed text budget -----
        self.request_redraw(qh);
    }

    pub(crate) fn refresh_battery(&mut self, qh: &QueueHandle<Self>) {
        if !self.dock.icons.is_empty() || !self.widget_placed(crate::config::WidgetKind::Battery) {
            return;
        }
        if !self.widgets.refresh_battery() {
            return;
        }
        self.sync_widget_bar_len();
        self.relayout_dock(qh);
    }

    pub(crate) fn refresh_bluetooth(&mut self, qh: &QueueHandle<Self>) {
        if !self.dock.icons.is_empty() || !self.widget_placed(crate::config::WidgetKind::Bluetooth) {
            return;
        }
        self.widgets.refresh_bluetooth();
        self.sync_widget_bar_len();
        self.relayout_dock(qh);
    }

    pub(crate) fn refresh_workspaces(&mut self, qh: &QueueHandle<Self>) {
        if self.dock.icons.is_empty() {
            self.widgets.refresh_workspaces();
            self.sync_widget_bar_len();
            self.relayout_dock(qh);
        }
    }

    pub(crate) fn refresh_cpu_ram(&mut self, qh: &QueueHandle<Self>) {
        let active = self
            .dock
            .config
            .settings
            .widgets
            .iter()
            .any(|w| matches!(w.kind, crate::config::WidgetKind::Cpu | crate::config::WidgetKind::Ram));
        if !active {
            return;
        }
        self.widgets.refresh_cpu_ram();
        self.request_redraw(qh);
    }

    pub(crate) fn sync_tray_layout(&mut self, qh: &QueueHandle<Self>) {
        let count = self.tray.lock().unwrap().len();
        if count != self.last_tray_count {
            self.last_tray_count = count;
            self.sync_widget_bar_len();
            self.relayout_dock(qh);
        }
    }

    pub(crate) fn sync_widget_bar_len(&mut self) {
        let tray_count = self.tray.lock().unwrap().len();
        let is_vertical = self.dock.is_vertical();
        let cross_len = self.dock.cross_len();
        let widget_scale = self.dock.config.settings.widget_scale;
        self.dock.widget_bar_content_len =
            render::widget_bar_natural_len(&self.dock.config.settings, &self.widgets, tray_count, is_vertical, cross_len, widget_scale);
    }
}
