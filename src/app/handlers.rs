use super::*;

impl CompositorHandler for App {
    fn scale_factor_changed(&mut self, _conn: &Connection, qh: &QueueHandle<Self>, _surface: &wl_surface::WlSurface, new_factor: i32) {
        self.output_scale = new_factor;
        self.request_redraw(qh);
        self.request_menu_redraw(qh);
    }

    fn transform_changed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: wl_output::Transform) {}

    fn frame(&mut self, _conn: &Connection, qh: &QueueHandle<Self>, surface: &wl_surface::WlSurface, _time: u32) {
        if self.screenshot.as_ref().is_some_and(|s| s.is_selector_surface(surface)) {
            self.draw_region_frame();
            return;
        }
        if surface == self.layer.wl_surface() {
            self.awaiting_frame = false;
            if self.notification_mode.is_some() {
                self.tick_notification_frame(qh);
            } else if self.osd_mode.is_some() {
                self.tick_osd_frame(qh);
            } else if self.app_search_mode.is_some() {
                self.tick_app_search_frame(qh);
            } else if self.dock_menu_mode.is_some() {
                self.tick_dock_menu_frame(qh);
            } else if self.wallpaper_mode.is_some() {
                self.tick_wallpaper_frame(qh);
            } else if self.clipboard_mode.is_some() {
                self.tick_clipboard_frame(qh);
            } else {
                let icons_animating = self.dock.step_animation();
                if self.marquee.workspace_animating() {
                    self.tick_workspace(qh);
                } else if icons_animating {
                    self.draw(qh);
                }
            }
            return;
        }
        let is_menu = self.menu.as_ref().map(|m| surface == m.layer.wl_surface()).unwrap_or(false);
        if is_menu {
            self.tick_menu_frame(qh);
            return;
        }
        let is_popup = self.popup_mode.as_ref().map(|p| surface == p.layer.wl_surface()).unwrap_or(false);
        if is_popup {
            self.tick_popup_frame(qh);
        }
    }

    fn surface_enter(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: &wl_output::WlOutput) {}
    fn surface_leave(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: &wl_output::WlOutput) {}
}

impl OutputHandler for App {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }
    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
}

impl LayerShellHandler for App {
    fn closed(&mut self, _: &Connection, _: &QueueHandle<Self>, layer: &LayerSurface) {
        if layer.wl_surface() == self.layer.wl_surface() {
            self.exit = true;
        } else if self.menu.as_ref().map(|m| m.layer.wl_surface() == layer.wl_surface()).unwrap_or(false) {
            self.menu = None;
        } else if self.popup_mode.as_ref().map(|p| p.layer.wl_surface() == layer.wl_surface()).unwrap_or(false) {
            self.popup_mode = None;
        }
    }

    fn configure(&mut self, _conn: &Connection, qh: &QueueHandle<Self>, layer: &LayerSurface, configure: LayerSurfaceConfigure, _serial: u32) {
        if self.screenshot.as_ref().is_some_and(|s| s.is_selector_layer(layer)) {
            let (w, h) = configure.new_size;
            self.configure_region(w, h);
            return;
        }
        if layer.wl_surface() == self.layer.wl_surface() {
            if self.first_configure {
                self.first_configure = false;
            }
            self.draw(qh);
        } else if layer.wl_surface() == self.reserve_layer.wl_surface() {
            let (buffer, canvas) = self.pool.create_buffer(1, 1, 4, wl_shm::Format::Argb8888).expect("failed to create reserve buffer");
            canvas.fill(0);
            let surface = self.reserve_layer.wl_surface();
            buffer.attach_to(surface).expect("failed to attach reserve buffer");
            surface.commit();
        } else if self.menu.as_ref().is_some_and(|m| m.layer.wl_surface() == layer.wl_surface() && !m.closing) {
            self.draw_menu(qh);
        } else if self.popup_mode.as_ref().is_some_and(|p| p.layer.wl_surface() == layer.wl_surface() && !p.closing) {
            self.draw_popup_mode(qh);
        }
    }
}

impl SeatHandler for App {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}

    fn new_capability(&mut self, _conn: &Connection, qh: &QueueHandle<Self>, seat: wl_seat::WlSeat, capability: Capability) {
        if capability == Capability::Pointer && self.pointer.is_none() {
            self.pointer = Some(self.seat_state.get_pointer(qh, &seat).expect("get pointer"));
        }
        if capability == Capability::Keyboard && self.keyboard.is_none() {
            self.keyboard = Some(self.seat_state.get_keyboard(qh, &seat, None).expect("get keyboard"));
        }
    }

    fn remove_capability(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat, capability: Capability) {
        if capability == Capability::Pointer {
            if let Some(pointer) = self.pointer.take() {
                pointer.release();
            }
        }
        if capability == Capability::Keyboard {
            if let Some(keyboard) = self.keyboard.take() {
                keyboard.release();
            }
        }
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}

impl PointerHandler for App {
    fn pointer_frame(&mut self, _conn: &Connection, qh: &QueueHandle<Self>, _pointer: &wl_pointer::WlPointer, events: &[PointerEvent]) {
        for event in events {
            if self.screenshot.as_ref().is_some_and(|s| s.is_selector_surface(&event.surface)) {
                self.handle_region_pointer_event(event, qh);
                continue;
            }
            let is_menu = self.menu.as_ref().map(|m| event.surface == *m.layer.wl_surface()).unwrap_or(false);
            let is_popup = self.popup_mode.as_ref().map(|p| event.surface == *p.layer.wl_surface()).unwrap_or(false);
            if is_menu {
                self.handle_menu_pointer_event(event, qh);
            } else if is_popup {
                self.handle_popup_pointer_event(event, qh);
            } else if event.surface == *self.layer.wl_surface() {
                self.handle_dock_pointer_event(event, qh);
            }
        }
    }
}

impl App {
    // ----- stepper acceleration -----
    pub(super) fn poll_held_key(&mut self) -> Option<(Keysym, u32)> {
        let (keysym, frames, mut accum) = self.held_key?;
        accum += crate::menu::stepper_speed(frames) / 60.0;
        let steps = accum.floor();
        accum -= steps;
        self.held_key = Some((keysym, frames + 1, accum));
        (steps > 0.0).then_some((keysym, steps as u32))
    }
}

impl KeyboardHandler for App {
    fn enter(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _: &wl_surface::WlSurface, _: u32, _: &[u32], _: &[Keysym]) {}
    fn leave(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _: &wl_surface::WlSurface, _: u32) {}

    fn press_key(&mut self, _: &Connection, qh: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _: u32, event: KeyEvent) {
        if self.screenshot.as_ref().is_some_and(|s| s.selector.mapped) {
            if event.keysym == Keysym::Escape {
                self.close_region(None, qh);
            }
            return;
        }
        self.held_key = matches!(event.keysym, Keysym::Left | Keysym::Right | Keysym::Up | Keysym::Down).then_some((event.keysym, 0, 0.0));
        if self.clipboard_mode.is_some() {
            self.handle_clipboard_key(event, qh);
        } else if self.app_search_mode.is_some() {
            self.handle_app_search_key(event, qh);
        } else if self.wallpaper_mode.is_some() {
            self.handle_wallpaper_key(event.keysym, qh);
        } else if self.dock_menu_mode.is_some() {
            self.handle_dock_menu_key(event, qh);
        } else {
            self.handle_search_key(event, qh);
        }
    }

    fn release_key(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _: u32, event: KeyEvent) {
        if self.held_key.is_some_and(|(k, ..)| k == event.keysym) {
            self.held_key = None;
        }
    }
    fn update_modifiers(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _: u32, modifiers: Modifiers, _: u32) {
        self.modifiers = modifiers;
    }
}

impl ShmHandler for App {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

impl ProvidesRegistryState for App {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState, SeatState];
}

impl Dispatch<wl_callback::WlCallback, ()> for App {
    fn event(_: &mut Self, _: &wl_callback::WlCallback, _: wl_callback::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

delegate_compositor!(App);
delegate_output!(App);
delegate_shm!(App);
delegate_seat!(App);
smithay_client_toolkit::delegate_pointer!(App);
delegate_keyboard!(App);
delegate_layer!(App);
delegate_registry!(App);
