use super::*;

use crate::menu_render::{clip_panel_h, ClipArgs, CLIP_HEADER_H, CLIP_PANEL_W, CLIP_ROW_H, CLIP_VISIBLE_ROWS};

impl App {
    pub(crate) fn toggle_clipboard(&mut self, qh: &QueueHandle<Self>) {
        if self.clipboard_mode.is_some() {
            self.close_clipboard_mode(qh);
        } else {
            self.open_clipboard(qh);
        }
    }

    pub(super) fn open_clipboard(&mut self, qh: &QueueHandle<Self>) {
        self.wallpaper_mode = None;
        self.dock_menu_mode = None;
        self.app_search_mode = None;
        self.osd_mode = None;
        self.notification_mode = None;
        self.popup_mode = None;
        self.menu = None;
        self.held_key = None;
        self.clipboard_history.ensure_loaded();

        self.layer.set_layer(Layer::Top);
        let panel_w = CLIP_PANEL_W;
        let panel_h = clip_panel_h();
        self.layer.set_keyboard_interactivity(KeyboardInteractivity::Exclusive);
        let s = &self.dock.config.settings;
        let (anchor, margin) = edge_anchor_margin(s.dock_edge, s.dock_align, s.pos_y, 0);
        self.layer.set_anchor(anchor);
        self.layer.set_margin(margin.0, margin.1, margin.2, margin.3);
        self.layer.set_size(panel_w as u32, panel_h as u32);

        self.clipboard_mode = Some(ClipboardMode {
            query: String::new(),
            filtered: Vec::new(),
            selected: 0,
            scroll_y: 0.0,
            scroll_target: 0.0,
            hovered: None,
            anim: 0.0,
            target_anim: 1.0,
            closing: false,
            panel_w,
            panel_h,
            previews: std::collections::HashMap::new(),
        });
        self.refresh_clipboard_filter(qh);
    }

    pub(super) fn close_clipboard_mode(&mut self, qh: &QueueHandle<Self>) {
        if let Some(cm) = self.clipboard_mode.as_mut() {
            cm.closing = true;
            cm.target_anim = 0.0;
        }
        self.held_key = None;
        self.request_redraw(qh);
    }

    pub(crate) fn refresh_clipboard_filter(&mut self, qh: &QueueHandle<Self>) {
        let entries = self.clipboard_history.entries();
        let Some(cm) = self.clipboard_mode.as_mut() else {
            return;
        };
        let query = cm.query.to_lowercase();
        cm.filtered = entries
            .iter()
            .enumerate()
            .filter(|(_, e)| query.is_empty() || e.matches(&query))
            .map(|(i, _)| i)
            .collect();
        cm.selected = cm.selected.min(cm.filtered.len().saturating_sub(1));
        cm.previews.retain(|k, _| cm.filtered.contains(k));
        self.scroll_clipboard_into_view(qh);
        self.request_redraw(qh);
    }

    fn scroll_clipboard_into_view(&mut self, qh: &QueueHandle<Self>) {
        let Some(cm) = self.clipboard_mode.as_mut() else {
            return;
        };
        let row_top = cm.selected as f32 * CLIP_ROW_H;
        let viewport = CLIP_ROW_H * CLIP_VISIBLE_ROWS as f32;
        let mut target = cm.scroll_target;
        if row_top < target {
            target = row_top;
        } else if row_top + CLIP_ROW_H > target + viewport {
            target = row_top + CLIP_ROW_H - viewport;
        }
        let max_scroll = (cm.filtered.len() as f32 * CLIP_ROW_H - viewport).max(0.0);
        cm.scroll_target = target.clamp(0.0, max_scroll);
        self.request_redraw(qh);
    }

    pub(super) fn handle_clipboard_key(&mut self, event: KeyEvent, qh: &QueueHandle<Self>) {
        match event.keysym {
            Keysym::Escape => {
                self.close_clipboard_mode(qh);
                return;
            }
            Keysym::Return | Keysym::KP_Enter => {
                self.paste_clipboard_selected(qh);
                return;
            }
            Keysym::Down => {
                if let Some(cm) = self.clipboard_mode.as_mut() {
                    if !cm.filtered.is_empty() {
                        cm.selected = (cm.selected + 1).min(cm.filtered.len() - 1);
                        cm.hovered = None;
                    }
                }
                self.scroll_clipboard_into_view(qh);
                return;
            }
            Keysym::Up => {
                if let Some(cm) = self.clipboard_mode.as_mut() {
                    cm.selected = cm.selected.saturating_sub(1);
                    cm.hovered = None;
                }
                self.scroll_clipboard_into_view(qh);
                return;
            }
            Keysym::Delete => {
                self.delete_clipboard_selected(qh);
                return;
            }
            Keysym::BackSpace => {
                if let Some(cm) = self.clipboard_mode.as_mut() {
                    cm.query.pop();
                }
                self.refresh_clipboard_filter(qh);
                return;
            }
            _ => {}
        }
        if let Some(text) = &event.utf8 {
            let mut changed = false;
            if let Some(cm) = self.clipboard_mode.as_mut() {
                for ch in text.chars() {
                    if !ch.is_control() {
                        cm.query.push(ch);
                        changed = true;
                    }
                }
            }
            if changed {
                self.refresh_clipboard_filter(qh);
            }
        }
    }

    fn paste_clipboard_selected(&mut self, qh: &QueueHandle<Self>) {
        let entry = self
            .clipboard_mode
            .as_ref()
            .and_then(|cm| cm.filtered.get(cm.selected).copied())
            .and_then(|i| self.clipboard_history.entries().get(i).cloned());
        let Some(entry) = entry else {
            return;
        };
        self.set_clipboard_entry(&entry, qh);
        self.close_clipboard_mode(qh);
        crate::clipboard::paste_active();
    }

    fn delete_clipboard_selected(&mut self, qh: &QueueHandle<Self>) {
        let index = self.clipboard_mode.as_ref().and_then(|cm| cm.filtered.get(cm.selected).copied());
        if let Some(index) = index {
            if self.clipboard_history.remove(index) {
                self.clipboard_history.save();
            }
        }
        self.refresh_clipboard_filter(qh);
    }

    pub(super) fn handle_clipboard_pointer_event(&mut self, event: &PointerEvent, qh: &QueueHandle<Self>) {
        let (px, py) = event.position;
        match event.kind {
            PointerEventKind::Enter { .. } | PointerEventKind::Motion { .. } => {
                let hit = self.clipboard_row_at(px as f32, py as f32);
                if let Some(cm) = self.clipboard_mode.as_mut() {
                    if cm.hovered != hit {
                        cm.hovered = hit;
                        self.request_redraw(qh);
                    }
                }
            }
            PointerEventKind::Leave { .. } => {
                if let Some(cm) = self.clipboard_mode.as_mut() {
                    cm.hovered = None;
                }
                self.request_redraw(qh);
            }
            PointerEventKind::Press { button, .. } if button == BTN_LEFT => {
                if let Some(pos) = self.clipboard_row_at(px as f32, py as f32) {
                    if let Some(cm) = self.clipboard_mode.as_mut() {
                        cm.selected = pos;
                    }
                    self.paste_clipboard_selected(qh);
                }
            }
            PointerEventKind::Axis { horizontal, vertical, .. } => {
                let delta = if vertical.absolute != 0.0 { vertical.absolute } else { horizontal.absolute };
                if let Some(cm) = self.clipboard_mode.as_mut() {
                    let viewport = CLIP_ROW_H * CLIP_VISIBLE_ROWS as f32;
                    let max_scroll = (cm.filtered.len() as f32 * CLIP_ROW_H - viewport).max(0.0);
                    cm.scroll_target = (cm.scroll_target + delta as f32 * 0.6).clamp(0.0, max_scroll);
                }
                self.request_redraw(qh);
            }
            _ => {}
        }
    }

    fn clipboard_row_at(&self, _x: f32, y: f32) -> Option<usize> {
        let cm = self.clipboard_mode.as_ref()?;
        if y < CLIP_HEADER_H {
            return None;
        }
        let pos = ((y - CLIP_HEADER_H + cm.scroll_y) / CLIP_ROW_H).floor() as usize;
        (pos < cm.filtered.len()).then_some(pos)
    }

    pub(super) fn tick_clipboard_frame(&mut self, qh: &QueueHandle<Self>) {
        let Some(cm) = self.clipboard_mode.as_mut() else {
            return;
        };
        let fade_animating = if cm.anim < cm.target_anim {
            cm.anim = (cm.anim + menu::ANIM_STEP_OPEN).min(cm.target_anim);
            true
        } else if cm.anim > cm.target_anim {
            cm.anim = (cm.anim - menu::ANIM_STEP_CLOSE).max(cm.target_anim);
            true
        } else {
            false
        };
        let scroll_delta = cm.scroll_target - cm.scroll_y;
        let scroll_animating = if scroll_delta.abs() > 0.5 {
            cm.scroll_y += scroll_delta * 0.3;
            true
        } else {
            cm.scroll_y = cm.scroll_target;
            false
        };
        let closing = cm.closing;
        let anim = cm.anim;

        if closing && anim <= 0.0 {
            self.clipboard_mode = None;
            self.layer.set_keyboard_interactivity(KeyboardInteractivity::None);
            let (w, h) = self.dock.base_size();
            let s = &self.dock.config.settings;
            let (anchor, margin) = edge_anchor_margin(s.dock_edge, s.dock_align, s.pos_y, 0);
            self.layer.set_anchor(anchor);
            self.layer.set_margin(margin.0, margin.1, margin.2, margin.3);
            self.layer.set_size(w, h);
            self.draw(qh);
            trim_heap();
            return;
        }

        let key_repeating = self.held_key.is_some();
        if let Some((keysym, steps)) = self.poll_held_key() {
            for _ in 0..steps {
                self.handle_clipboard_key(KeyEvent { time: 0, raw_code: 0, keysym, utf8: None }, qh);
            }
        }

        if !fade_animating && !scroll_animating && !key_repeating {
            return;
        }
        self.draw_clipboard_mode(qh);
    }

    pub(super) fn draw_clipboard_mode(&mut self, qh: &QueueHandle<Self>) {
        let scale = self.output_scale.max(1) as f32;
        let transparency = self.dock.config.settings.transparency;
        let Some(cm) = self.clipboard_mode.as_mut() else {
            return;
        };
        let linear = cm.anim.clamp(0.0, 1.0);
        let eased = 0.5 - 0.5 * (std::f32::consts::PI * linear).cos();
        let width = (cm.panel_w * scale).round() as i32;
        let height = (cm.panel_h * scale).round() as i32;
        if width <= 0 || height <= 0 {
            return;
        }

        let mut content = tiny_skia::Pixmap::new(width as u32, height as u32).unwrap();
        let args = ClipArgs {
            settings: &self.dock.config.settings,
            entries: self.clipboard_history.entries(),
            filtered: &cm.filtered,
            previews: &mut cm.previews,
            query: &cm.query,
            selected: cm.selected,
            hovered: cm.hovered,
            scroll_y: cm.scroll_y,
            render_scale: scale,
            panel_w: cm.panel_w,
            panel_h: cm.panel_h,
        };
        crate::menu_render::draw_clipboard(&mut content, &mut self.text_cache, args);

        let mut pixmap = tiny_skia::Pixmap::new(width as u32, height as u32).unwrap();
        let paint = tiny_skia::PixmapPaint {
            opacity: anim_opacity(transparency, eased),
            ..Default::default()
        };
        pixmap.draw_pixmap(0, 0, content.as_ref(), &paint, tiny_skia::Transform::identity(), None);

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
}
