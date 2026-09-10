use super::*;

impl App {
    pub(crate) fn show_notification(&mut self, title: String, body: String, qh: &QueueHandle<Self>) {
        if self.wallpaper_mode.is_some() || self.dock_menu_mode.is_some() || self.app_search_mode.is_some() || self.menu.is_some() || self.osd_mode.is_some() || self.clipboard_mode.is_some() {
            return;
        }
        let length = menu::OSD_NOTIFICATION_BASE_LEN + menu::NOTIFICATION_GROWTH_W;
        let mut body_lines = 1usize;
        let (panel_w, panel_h) = if self.dock.is_vertical() {
            let cross = menu::OSD_NOTIFICATION_BASE_THICKNESS.max(menu::NOTIFICATION_MIN_PANEL_H) + menu::NOTIFICATION_GROWTH_H;
            (cross, length)
        } else {
            // ----- grow for wrapped body -----
            let pad = menu::MENU_PADDING;
            let text_x = pad + 9.0 * 2.0 + 14.0;
            let max_w = (length - text_x - pad).max(1.0);
            body_lines = menu_render::wrap_to_width(&body, 8.5, max_w, menu::NOTIFICATION_BODY_MAX_LINES).len().max(1);
            let block_h = 9.5 * 1.4 + 3.0 + body_lines as f32 * 8.5 * 1.4;
            let cross = (block_h + pad * 2.0).max(menu::NOTIFICATION_MIN_PANEL_H + menu::NOTIFICATION_GROWTH_H);
            (length, cross)
        };
        let timeout = (menu::NOTIFICATION_TIMEOUT_MS + body_lines.saturating_sub(1) as u64 * 900).min(15000);
        let needs_resize = match self.notification_mode.as_ref() {
            None => true,
            Some(m) => m.panel_w != panel_w || m.panel_h != panel_h,
        };
        if self.notification_mode.is_none() {
            self.notification_mode = Some(NotificationMode {
                title,
                body,
                anim: 0.0,
                target_anim: 1.0,
                closing: false,
                panel_w,
                panel_h,
            });
        } else if let Some(m) = self.notification_mode.as_mut() {
            m.title = title;
            m.body = body;
            m.closing = false;
            m.target_anim = 1.0;
            m.panel_w = panel_w;
            m.panel_h = panel_h;
        }
        if needs_resize {
            let s = &self.dock.config.settings;
            let (anchor, margin) = edge_anchor_margin(s.dock_edge, s.dock_align, s.pos_y, 0);
            self.layer.set_anchor(anchor);
            self.layer.set_margin(margin.0, margin.1, margin.2, margin.3);
            self.layer.set_size(panel_w as u32, panel_h as u32);
        }
        self.layer.set_layer(Layer::Overlay);
        let _ = self.notification_reset_tx.send(timeout);
        self.request_redraw(qh);
    }

    pub(crate) fn close_notification_mode(&mut self, qh: &QueueHandle<Self>) {
        if let Some(m) = self.notification_mode.as_mut() {
            m.closing = true;
            m.target_anim = 0.0;
        }
        self.request_redraw(qh);
    }

    pub(super) fn draw_notification_mode(&mut self, qh: &QueueHandle<Self>) {
        let scale = self.output_scale.max(1) as f32;
        let Some(m) = self.notification_mode.as_mut() else {
            return;
        };
        let linear = m.anim.clamp(0.0, 1.0);
        let eased = (0.5 - 0.5 * (std::f32::consts::PI * linear).cos()).max(if m.closing { 0.0 } else { 0.04 });

        let width = (m.panel_w * scale).round() as i32;
        let height = (m.panel_h * scale).round() as i32;
        if width <= 0 || height <= 0 {
            return;
        }

        let args = menu_render::NotificationArgs {
            title: &m.title,
            body: &m.body,
            panel_w: m.panel_w,
            panel_h: m.panel_h,
            dock: &self.dock,
            render_scale: scale,
        };
        let mut pixmap = tiny_skia::Pixmap::new(width as u32, height as u32).unwrap();
        if !m.closing && eased >= 0.999 {
            menu_render::draw_notification(&mut pixmap, &mut self.text_cache, &args);
        } else {
            let mut content = tiny_skia::Pixmap::new(width as u32, height as u32).unwrap();
            menu_render::draw_notification(&mut content, &mut self.text_cache, &args);
            let paint = tiny_skia::PixmapPaint {
                opacity: eased,
                ..Default::default()
            };
            if m.closing {
                let (full_w, full_h) = (width as f32, height as f32);
                let (rw, rh) = (full_w * linear, full_h * linear);
                let rect = tiny_skia::Rect::from_xywh((full_w - rw) / 2.0, (full_h - rh) / 2.0, rw.max(0.0), rh.max(0.0));
                if let Some(rect) = rect {
                    let mut mask = tiny_skia::Mask::new(width as u32, height as u32).unwrap();
                    let path = tiny_skia::PathBuilder::from_rect(rect);
                    mask.fill_path(&path, tiny_skia::FillRule::Winding, true, tiny_skia::Transform::identity());
                    pixmap.draw_pixmap(0, 0, content.as_ref(), &paint, tiny_skia::Transform::identity(), Some(&mask));
                }
            } else {
                pixmap.draw_pixmap(0, 0, content.as_ref(), &paint, tiny_skia::Transform::identity(), None);
            }
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

    pub(super) fn tick_notification_frame(&mut self, qh: &QueueHandle<Self>) {
        let Some(m) = self.notification_mode.as_mut() else {
            return;
        };
        let animating = if m.anim < m.target_anim {
            m.anim = (m.anim + menu::ANIM_STEP_OPEN).min(m.target_anim);
            true
        } else if m.anim > m.target_anim {
            m.anim = (m.anim - menu::ANIM_STEP_CLOSE).max(m.target_anim);
            true
        } else {
            false
        };
        let closing = m.closing;
        let anim = m.anim;

        if closing && anim <= 0.0 {
            self.notification_mode = None;
            self.layer.set_layer(Layer::Top);
            let (w, h) = self.dock.base_size();
            self.layer.set_size(w, h);
            self.draw(qh);
            trim_heap();
            return;
        }
        if !animating {
            return;
        }
        self.draw_notification_mode(qh);
    }
}
