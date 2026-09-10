use super::*;

impl App {
    pub(super) fn open_power_menu(&mut self, qh: &QueueHandle<Self>) {
        let (controls, content_height) = menu::build_controls(menu::MenuScreen::PowerMenu, 0, 0);
        let tray_count = self.tray.lock().unwrap().len();
        let center = render::widget_center(&self.dock, &self.widgets, tray_count, crate::config::WidgetKind::PowerMenu);
        self.create_popup_surface(menu::MenuScreen::PowerMenu, controls, content_height, Vec::new(), String::new(), String::new(), center, qh);
    }

    pub(super) fn open_tray_menu(&mut self, idx: usize, qh: &QueueHandle<Self>) {
        let item = self.tray.lock().unwrap().get(idx).cloned();
        let Some(item) = item else { return };
        let Some(menu_path) = item.menu_path.clone() else { return };
        let items = crate::tray::fetch_menu(&item.service, &menu_path, 0);
        if items.is_empty() {
            return;
        }
        let (controls, content_height) = menu::build_tray_menu_controls(&items, false);
        let tray_count = self.tray.lock().unwrap().len();
        let center = render::tray_icon_center(&self.dock, &self.widgets, tray_count, idx);
        self.create_popup_surface(menu::MenuScreen::TrayMenu, controls, content_height, items, item.service, menu_path, center, qh);
    }

    fn popup_geometry(&self, content_height: f32, center: Option<(f32, f32)>) -> (f32, f32, f32, f32) {
        let is_vertical = self.dock.is_vertical();
        let (base_w, base_h) = self.dock.base_size();
        if is_vertical {
            let cy = center.map(|(_, y)| y).unwrap_or(base_h as f32 / 2.0);
            let box_y = (cy - content_height / 2.0).clamp(0.0, (base_h as f32 - content_height).max(0.0));
            (menu::MENU_WIDTH, base_h as f32, 0.0, box_y)
        } else {
            let cx = center.map(|(x, _)| x).unwrap_or(base_w as f32 / 2.0);
            let box_x = (cx - menu::MENU_WIDTH / 2.0).clamp(0.0, (base_w as f32 - menu::MENU_WIDTH).max(0.0));
            (base_w as f32, content_height, box_x, 0.0)
        }
    }

    // ----- full redeclare -----
    fn popup_anchor_margin(&self) -> (Anchor, (i32, i32, i32, i32)) {
        let s = &self.dock.config.settings;
        edge_anchor_margin(s.dock_edge, s.dock_align, s.pos_y, self.dock.thickness() as i32 + MENU_GAP)
    }

    // ----- submenu navigation -----
    fn set_tray_items(&mut self, items: Vec<crate::tray::TrayMenuItem>, has_back: bool, qh: &QueueHandle<Self>) {
        let (controls, content_height) = menu::build_tray_menu_controls(&items, has_back);
        let center = self.popup_mode.as_ref().and_then(|p| p.center);
        let (surface_w, surface_h, box_x, box_y) = self.popup_geometry(content_height, center);
        let (anchor, margin) = self.popup_anchor_margin();
        let Some(p) = self.popup_mode.as_mut() else { return };
        p.layer.set_anchor(anchor);
        p.layer.set_margin(margin.0, margin.1, margin.2, margin.3);
        p.layer.set_size(surface_w as u32, surface_h as u32);
        p.tray_items = items;
        p.controls = controls;
        p.content_height = content_height;
        p.surface_w = surface_w;
        p.surface_h = surface_h;
        p.box_x = box_x;
        p.box_y = box_y;
        p.hovered = None;
        p.layer.wl_surface().commit();
        self.request_popup_redraw(qh);
    }

    #[allow(clippy::too_many_arguments)]
    fn create_popup_surface(
        &mut self,
        screen: menu::MenuScreen,
        controls: Vec<menu::Control>,
        content_height: f32,
        tray_items: Vec<crate::tray::TrayMenuItem>,
        tray_service: String,
        tray_menu_path: String,
        center: Option<(f32, f32)>,
        qh: &QueueHandle<Self>,
    ) {
        self.menu = None;
        self.popup_mode = None;

        let (surface_w, surface_h, box_x, box_y) = self.popup_geometry(content_height, center);

        let surface = self.compositor.create_surface(qh);
        let layer = self.layer_shell.create_layer_surface(qh, surface, Layer::Overlay, Some("dockyrs-menu"), None);
        let (anchor, margin) = self.popup_anchor_margin();
        layer.set_anchor(anchor);
        layer.set_margin(margin.0, margin.1, margin.2, margin.3);
        layer.set_size(surface_w as u32, surface_h as u32);
        layer.set_keyboard_interactivity(KeyboardInteractivity::OnDemand);
        layer.set_exclusive_zone(-1);
        layer.commit();

        let pool_size = (surface_w as usize * surface_h as usize * 16).max(65536);
        let pool = SlotPool::new(pool_size, &self.shm).expect("failed to create popup shm pool");

        self.popup_mode = Some(DockPopupMode {
            layer,
            pool,
            awaiting_frame: false,
            screen,
            controls,
            content_height,
            hovered: None,
            anim: 0.0,
            target_anim: 1.0,
            closing: false,
            tray_items,
            tray_service,
            tray_menu_path,
            tray_stack: Vec::new(),
            center,
            surface_w,
            surface_h,
            box_x,
            box_y,
        });
    }

    pub(super) fn close_popup_mode(&mut self, qh: &QueueHandle<Self>) {
        if let Some(p) = self.popup_mode.as_mut() {
            p.closing = true;
            p.target_anim = 0.0;
        }
        self.request_popup_redraw(qh);
    }

    pub(super) fn request_popup_redraw(&mut self, qh: &QueueHandle<Self>) {
        if self.popup_mode.as_ref().is_some_and(|p| !p.awaiting_frame) {
            self.draw_popup_mode(qh);
        }
    }

    pub(super) fn draw_popup_mode(&mut self, qh: &QueueHandle<Self>) {
        use crate::config::DockEdge;
        let scale = self.output_scale.max(1) as f32;
        let transparency = self.dock.config.settings.transparency;
        let dock_edge = self.dock.config.settings.dock_edge;
        let Some(p) = self.popup_mode.as_ref() else {
            return;
        };
        let linear = p.anim.clamp(0.0, 1.0);
        let eased = 0.5 - 0.5 * (std::f32::consts::PI * linear).cos();

        let sw = (p.surface_w * scale).round().max(1.0) as i32;
        let sh = (p.surface_h * scale).round().max(1.0) as i32;
        let box_w = menu::MENU_WIDTH * scale;
        let box_h = p.content_height * scale;

        let args = menu_render::DrawArgs {
            screen: p.screen,
            controls: &p.controls,
            content_height: p.content_height,
            panel_width: menu::MENU_WIDTH,
            dock: &self.dock,
            app_entries: &[],
            icon_choices: &[],
            search_query: "",
            wallpapers: &[],
            wallpaper_hovered: None,
            wallpaper_scroll_x: 0.0,
            hovered: p.hovered,
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
            tray_items: &p.tray_items,
        };
        let mut box_pixmap = tiny_skia::Pixmap::new(box_w.round().max(1.0) as u32, box_h.round().max(1.0) as u32).unwrap();
        menu_render::draw_content(&mut box_pixmap, &mut self.icon_cache, &mut self.text_cache, &self.thumbnail_cache, &args);

        let Some(p) = self.popup_mode.as_ref() else {
            return;
        };
        let bx = p.box_x * scale;
        let by = p.box_y * scale;

        let mut pixmap = tiny_skia::Pixmap::new(sw as u32, sh as u32).unwrap();
        let (mask_x, mask_y, mask_w, mask_h) = match dock_edge {
            DockEdge::Left => (bx, by, box_w * linear, box_h),
            DockEdge::Right => (bx + box_w - box_w * linear, by, box_w * linear, box_h),
            DockEdge::Top => (bx, by, box_w, box_h * linear),
            DockEdge::Bottom => (bx, by + box_h - box_h * linear, box_w, box_h * linear),
        };
        if mask_w > 0.5 && mask_h > 0.5 {
            if let Some(rect) = tiny_skia::Rect::from_xywh(mask_x, mask_y, mask_w, mask_h) {
                let mut mask = tiny_skia::Mask::new(sw as u32, sh as u32).unwrap();
                let path = tiny_skia::PathBuilder::from_rect(rect);
                mask.fill_path(&path, tiny_skia::FillRule::Winding, true, tiny_skia::Transform::identity());
                let paint = tiny_skia::PixmapPaint {
                    opacity: anim_opacity(transparency, eased),
                    ..Default::default()
                };
                pixmap.draw_pixmap(0, 0, box_pixmap.as_ref(), &paint, tiny_skia::Transform::from_translate(bx, by), Some(&mask));
            }
        }

        let Some(p) = self.popup_mode.as_mut() else {
            return;
        };
        let stride = sw * 4;
        let (buffer, canvas) = p.pool.create_buffer(sw, sh, stride, wl_shm::Format::Argb8888).expect("failed to create popup shm buffer");
        bgra_from_rgba(pixmap.data(), canvas);

        let surface = p.layer.wl_surface();
        surface.set_buffer_scale(self.output_scale.max(1));
        buffer.attach_to(surface).expect("failed to attach popup buffer");
        surface.damage_buffer(0, 0, sw, sh);
        surface.frame(qh, surface.clone());
        p.awaiting_frame = true;
        surface.commit();
    }

    pub(super) fn tick_popup_frame(&mut self, qh: &QueueHandle<Self>) {
        let Some(p) = self.popup_mode.as_mut() else {
            return;
        };
        p.awaiting_frame = false;
        let animating = if p.anim < p.target_anim {
            p.anim = (p.anim + menu::ANIM_STEP_OPEN).min(p.target_anim);
            true
        } else if p.anim > p.target_anim {
            p.anim = (p.anim - menu::ANIM_STEP_CLOSE).max(p.target_anim);
            true
        } else {
            false
        };
        let closing = p.closing;
        let anim = p.anim;

        if closing && anim <= 0.0 {
            p.layer.wl_surface().attach(None, 0, 0);
            p.layer.wl_surface().commit();
            self.popup_mode = None;
            trim_heap();
            return;
        }
        if !animating {
            return;
        }
        self.draw_popup_mode(qh);
    }

    pub(super) fn handle_popup_pointer_event(&mut self, event: &PointerEvent, qh: &QueueHandle<Self>) {
        let (box_x, box_y) = self.popup_mode.as_ref().map(|p| (p.box_x, p.box_y)).unwrap_or((0.0, 0.0));
        match event.kind {
            PointerEventKind::Enter { .. } | PointerEventKind::Motion { .. } => {
                let (x, y) = event.position;
                if let Some(p) = self.popup_mode.as_mut() {
                    p.hovered = menu::hit_test(&p.controls, &self.dock.config.settings, menu::MENU_WIDTH, x as f32 - box_x, y as f32 - box_y);
                }
                self.request_popup_redraw(qh);
            }
            PointerEventKind::Leave { .. } => {
                if let Some(p) = self.popup_mode.as_mut() {
                    p.hovered = None;
                }
                self.request_popup_redraw(qh);
            }
            PointerEventKind::Press { button, .. } if button == BTN_LEFT => {
                let (x, y) = event.position;
                let hit = self
                    .popup_mode
                    .as_ref()
                    .and_then(|p| menu::hit_test(&p.controls, &self.dock.config.settings, menu::MENU_WIDTH, x as f32 - box_x, y as f32 - box_y));
                self.handle_popup_click(hit, qh);
            }
            _ => {}
        }
    }

    pub(super) fn handle_popup_click(&mut self, hit: Option<menu::HitTarget>, qh: &QueueHandle<Self>) {
        match hit {
            Some(menu::HitTarget::Button(kind)) => match kind {
                menu::ButtonKind::Suspend => {
                    crate::power::suspend();
                    self.close_popup_mode(qh);
                }
                menu::ButtonKind::Logout => {
                    crate::power::logout();
                    self.close_popup_mode(qh);
                }
                menu::ButtonKind::Reboot => {
                    crate::power::reboot();
                    self.close_popup_mode(qh);
                }
                menu::ButtonKind::Shutdown => {
                    crate::power::poweroff();
                    self.close_popup_mode(qh);
                }
                menu::ButtonKind::Back => {
                    let Some(p) = self.popup_mode.as_mut() else { return };
                    let Some(items) = p.tray_stack.pop() else { return };
                    let has_back = !p.tray_stack.is_empty();
                    self.set_tray_items(items, has_back, qh);
                }
                _ => {}
            },
            Some(menu::HitTarget::TrayItem(index)) => {
                let Some(p) = self.popup_mode.as_ref() else { return };
                let Some(item) = p.tray_items.get(index).cloned() else { return };
                if !item.enabled {
                    return;
                }
                if item.has_submenu {
                    let (service, menu_path) = (p.tray_service.clone(), p.tray_menu_path.clone());
                    let submenu = crate::tray::fetch_menu(&service, &menu_path, item.id);
                    if submenu.is_empty() {
                        return;
                    }
                    let Some(p) = self.popup_mode.as_mut() else { return };
                    p.tray_stack.push(p.tray_items.clone());
                    self.set_tray_items(submenu, true, qh);
                } else {
                    crate::tray::send_menu_event(p.tray_service.clone(), p.tray_menu_path.clone(), item.id);
                    self.close_popup_mode(qh);
                }
            }
            _ => {}
        }
    }
}
