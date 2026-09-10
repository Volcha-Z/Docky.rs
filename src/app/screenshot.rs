use super::*;

use crate::screenshot::{CaptureTarget, Selection};

impl App {
    pub(crate) fn start_full_screenshot(&mut self, qh: &QueueHandle<Self>) {
        if let Some(screenshot) = &mut self.screenshot {
            screenshot.start(CaptureTarget::Full, qh);
        }
    }

    pub(crate) fn start_region_screenshot(&mut self, qh: &QueueHandle<Self>) {
        let _ = qh;
        if self.screenshot.as_ref().is_some_and(|s| s.selector.mapped) {
            self.close_region(None, qh);
            return;
        }
        if let Some(screenshot) = &mut self.screenshot {
            if !screenshot.active() {
                screenshot.selector.open(&self.shm);
            }
        }
    }

    pub(crate) fn prepare_screenshot(&mut self) {
        if let Some(screenshot) = &mut self.screenshot {
            screenshot.prepare(&self.shm);
        }
    }

    pub(crate) fn finish_screenshot(&mut self, qh: &QueueHandle<Self>) {
        let Some(image) = self.screenshot.as_mut().and_then(crate::screenshot::ScreenshotState::finish) else {
            return;
        };
        crate::screenshot::save(&image);
        let entry = crate::clipboard::ClipboardEntry::from_screenshot(image.png, image.width, image.height, image.thumbnail);
        self.set_clipboard_entry(&entry, qh);
        self.clipboard_history.add(entry);
        self.clipboard_history.save();
        if self.clipboard_mode.is_some() {
            self.refresh_clipboard_filter(qh);
        }
        trim_heap();
    }

    pub(super) fn configure_region(&mut self, width: u32, height: u32) {
        if let Some(screenshot) = &mut self.screenshot {
            screenshot.selector.configure(width, height, &self.shm);
        }
    }

    pub(super) fn region_press(&mut self, position: (f64, f64)) {
        if let Some(screenshot) = &mut self.screenshot {
            screenshot.selector.press(position);
        }
        self.request_region_frame();
    }

    pub(super) fn region_motion(&mut self, position: (f64, f64)) {
        if let Some(screenshot) = &mut self.screenshot {
            screenshot.selector.motion(position);
        }
        self.request_region_frame();
    }

    pub(super) fn region_release(&mut self, position: (f64, f64), qh: &QueueHandle<Self>) {
        let selection = self.screenshot.as_mut().and_then(|screenshot| screenshot.selector.release(position));
        if let Some(selection) = selection {
            self.close_region(Some(selection), qh);
        }
    }

    pub(super) fn close_region(&mut self, selection: Option<Selection>, qh: &QueueHandle<Self>) {
        let Some(screenshot) = &mut self.screenshot else {
            return;
        };
        screenshot.selector.close(&self.compositor);
        if let Some(selection) = selection {
            screenshot.start(CaptureTarget::Region(selection), qh);
        }
        trim_heap();
    }

    pub(super) fn draw_region_frame(&mut self) {
        let Some(selector) = self.screenshot.as_mut().map(|screenshot| &mut screenshot.selector) else {
            return;
        };
        selector.frame_pending = false;
        if selector.redraw_pending {
            selector.draw();
        }
        if selector.redraw_pending {
            self.request_region_frame();
        }
    }

    pub(super) fn handle_region_pointer_event(&mut self, event: &PointerEvent, qh: &QueueHandle<Self>) {
        let pos = event.position;
        match event.kind {
            PointerEventKind::Press { button, .. } if button == BTN_LEFT => self.region_press(pos),
            PointerEventKind::Motion { .. } => self.region_motion(pos),
            PointerEventKind::Release { button, .. } if button == BTN_LEFT => self.region_release(pos, qh),
            _ => {}
        }
    }

    fn request_region_frame(&mut self) {
        let Some(selector) = self.screenshot.as_mut().map(|screenshot| &mut screenshot.selector) else {
            return;
        };
        if selector.frame_pending || !selector.mapped {
            return;
        }
        let surface = selector.layer.wl_surface().clone();
        surface.frame(&self.qh, surface.clone());
        selector.frame_pending = true;
        selector.layer.commit();
    }
}
