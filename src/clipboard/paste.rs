use crate::app::App;
use std::os::fd::AsFd;
use wayland_client::QueueHandle;

#[derive(Clone, Copy, Debug)]
pub enum PasteTarget {
    Hex(usize),
    Name,
}

const TEXT_MIME_PREFERENCE: [&str; 4] = ["text/plain;charset=utf-8", "UTF8_STRING", "text/plain", "STRING"];

impl App {
    pub(crate) fn ensure_clipboard_device(&mut self, qh: &QueueHandle<Self>) {
        if self.clipboard_device.is_some() {
            return;
        }
        let (Some(manager), Some(seat)) = (self.clipboard_manager.as_ref(), self.seat.as_ref()) else {
            return;
        };
        self.clipboard_device = Some(manager.get_data_device(seat, qh, ()));
    }

    pub(crate) fn request_paste(&mut self, target: PasteTarget, qh: &QueueHandle<Self>) {
        self.ensure_clipboard_device(qh);
        let Some(offer) = self.clipboard_offer.clone() else {
            return;
        };
        let mimes = self.clipboard_offer_mimes(&offer);
        let Some(mime) = TEXT_MIME_PREFERENCE.iter().find(|m| mimes.iter().any(|o| o == *m)) else {
            return;
        };
        let Some((r, w)) = make_pipe() else {
            return;
        };
        offer.receive(mime.to_string(), w.as_fd());
        let _ = self.conn.flush();
        drop(w);

        let tx = self.paste_tx.clone();
        let conn = self.conn.clone();
        let qh = qh.clone();
        std::thread::spawn(move || {
            use std::io::Read;
            let mut file = std::fs::File::from(r);
            let mut buf = Vec::new();
            if file.read_to_end(&mut buf).is_ok() {
                let text = String::from_utf8_lossy(&buf).to_string();
                let _ = tx.send((target, text));
                conn.display().sync(&qh, ());
                let _ = conn.flush();
            }
        });
    }

    pub(crate) fn apply_pending_paste(&mut self, target: PasteTarget, text: String, qh: &QueueHandle<Self>) {
        let Some(dm) = self.dock_menu_mode.as_mut() else {
            return;
        };
        match target {
            PasteTarget::Hex(i) => {
                let cleaned: String = text.trim().strip_prefix('#').unwrap_or(text.trim()).chars().filter(|c| c.is_ascii_hexdigit()).take(6).collect();
                if let Some(field) = dm.custom_hex.get_mut(i) {
                    *field = cleaned.to_lowercase();
                }
            }
            PasteTarget::Name => {
                let cleaned: String = text.trim().chars().filter(|c| !c.is_control()).take(24).collect();
                dm.custom_name = cleaned;
            }
        }
        self.request_redraw(qh);
    }

    pub(crate) fn copy_to_clipboard(&mut self, text: String, qh: &QueueHandle<Self>) {
        self.ensure_clipboard_device(qh);
        let (Some(manager), Some(device)) = (self.clipboard_manager.as_ref(), self.clipboard_device.as_ref()) else {
            return;
        };
        self.clipboard_copy_bytes = std::sync::Arc::new(text.into_bytes());
        let source = manager.create_data_source(qh, ());
        source.offer("text/plain;charset=utf-8".to_string());
        source.offer("UTF8_STRING".to_string());
        source.offer("text/plain".to_string());
        device.set_selection(Some(&source));
        self.clipboard_source = Some(source);
    }
}

pub(super) fn make_pipe() -> Option<(std::os::fd::OwnedFd, std::os::fd::OwnedFd)> {
    let (r, w) = std::io::pipe().ok()?;
    Some((r.into(), w.into()))
}
