mod history;
pub mod paste;
mod storage;

use std::{os::fd::AsFd, time::Instant};

use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols_wlr::data_control::v1::client::{
    zwlr_data_control_device_v1::{self, ZwlrDataControlDeviceV1},
    zwlr_data_control_manager_v1::ZwlrDataControlManagerV1,
    zwlr_data_control_offer_v1::{self, ZwlrDataControlOfferV1},
    zwlr_data_control_source_v1::{self, ZwlrDataControlSourceV1},
};

pub use history::{ClipboardEntry, ScaledPreview};
pub use paste::PasteTarget;

use crate::app::App;

const MAX_ITEMS: usize = 50;
const MAX_BYTES: usize = 10 * 1024 * 1024;
const TEXT_LIMIT: usize = 256 * 1024;
const IMAGE_LIMIT: usize = 6 * 1024 * 1024;

pub struct ClipboardHistory {
    entries: Vec<ClipboardEntry>,
    loaded: bool,
}

impl ClipboardHistory {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            loaded: false,
        }
    }

    pub fn ensure_loaded(&mut self) {
        if !self.loaded {
            self.entries = storage::load();
            self.loaded = true;
        }
    }

    pub fn entries(&self) -> &[ClipboardEntry] {
        &self.entries
    }

    pub fn add(&mut self, entry: ClipboardEntry) {
        self.ensure_loaded();
        if let Some(index) = self.entries.iter().position(|existing| existing.same_content(&entry)) {
            self.entries.remove(index);
        }
        self.entries.insert(0, entry);
        let mut bytes = self.entries.iter().map(ClipboardEntry::size).sum::<usize>();
        while self.entries.len() > MAX_ITEMS || bytes > MAX_BYTES {
            let Some(removed) = self.entries.pop() else {
                break;
            };
            bytes = bytes.saturating_sub(removed.size());
        }
    }

    pub fn remove(&mut self, index: usize) -> bool {
        if index < self.entries.len() {
            self.entries.remove(index);
            true
        } else {
            false
        }
    }

    pub fn save(&self) {
        storage::save(&self.entries);
    }
}

// ----- auto paste -----
pub fn paste_active() {
    let Ok(sig) = std::env::var("HYPRLAND_INSTANCE_SIGNATURE") else {
        return;
    };
    let runtime = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into());
    let path = std::path::PathBuf::from(runtime).join("hypr").join(sig).join(".socket.sock");
    if let Ok(mut stream) = std::os::unix::net::UnixStream::connect(path) {
        use std::io::Write;
        let _ = stream.write_all(b"dispatch sendshortcut CTRL,V,activewindow");
    }
}

fn preferred_mime(mimes: &[String]) -> Option<(String, usize)> {
    if mimes.iter().any(|mime| {
        let mime = mime.to_lowercase();
        mime.contains("password") || mime.contains("secret")
    }) {
        return None;
    }
    for candidate in ["image/png", "image/jpeg", "image/webp"] {
        if mimes.iter().any(|mime| mime == candidate) {
            return Some((candidate.into(), IMAGE_LIMIT));
        }
    }
    for candidate in ["text/plain;charset=utf-8", "text/plain", "UTF8_STRING"] {
        if mimes.iter().any(|mime| mime == candidate) {
            return Some((candidate.into(), TEXT_LIMIT));
        }
    }
    None
}

impl App {
    pub(crate) fn clipboard_offer_mimes(&self, offer: &ZwlrDataControlOfferV1) -> Vec<String> {
        self.clipboard_offers.get(&offer.id()).cloned().unwrap_or_default()
    }

    // ----- capture selection -----
    fn capture_selection(&mut self, offer: &ZwlrDataControlOfferV1, qh: &QueueHandle<Self>) {
        if Instant::now() < self.clipboard_ready_at {
            return;
        }
        let mimes = self.clipboard_offer_mimes(offer);
        let Some((mime, limit)) = preferred_mime(&mimes) else {
            return;
        };
        let Some((read, write)) = paste::make_pipe() else {
            return;
        };
        offer.receive(mime.clone(), write.as_fd());
        let _ = self.conn.flush();
        drop(write);

        let tx = self.clip_tx.clone();
        let conn = self.conn.clone();
        let qh = qh.clone();
        std::thread::spawn(move || {
            use std::io::Read;
            let mut file = std::fs::File::from(read);
            let mut buf = Vec::new();
            let mut chunk = [0_u8; 64 * 1024];
            loop {
                match file.read(&mut chunk) {
                    Ok(0) => break,
                    Ok(n) => {
                        if buf.len() + n > limit {
                            return;
                        }
                        buf.extend_from_slice(&chunk[..n]);
                    }
                    Err(_) => return,
                }
            }
            if !buf.is_empty() {
                let _ = tx.send((mime, buf));
                conn.display().sync(&qh, ());
                let _ = conn.flush();
            }
        });
    }

    // ----- become owner -----
    pub(crate) fn set_clipboard_entry(&mut self, entry: &ClipboardEntry, qh: &QueueHandle<Self>) {
        self.ensure_clipboard_device(qh);
        let (Some(manager), Some(device)) = (self.clipboard_manager.as_ref(), self.clipboard_device.as_ref()) else {
            return;
        };
        self.clipboard_copy_bytes = std::sync::Arc::new(entry.data.to_vec());
        let source = manager.create_data_source(qh, ());
        if entry.is_text() {
            for mime in ["text/plain;charset=utf-8", "text/plain", "UTF8_STRING", "STRING"] {
                source.offer(mime.into());
            }
        } else {
            source.offer(entry.mime.clone());
        }
        device.set_selection(Some(&source));
        self.clipboard_source = Some(source);
        let _ = self.conn.flush();
    }

    pub(crate) fn ingest_clipboard_capture(&mut self, mime: String, data: Vec<u8>, qh: &QueueHandle<Self>) {
        let Some(entry) = ClipboardEntry::from_data(mime, data) else {
            return;
        };
        self.clipboard_history.add(entry);
        self.clipboard_history.save();
        crate::app::trim_heap();
        if self.clipboard_mode.is_some() {
            self.refresh_clipboard_filter(qh);
        }
    }
}

wayland_client::delegate_noop!(App: ZwlrDataControlManagerV1);

impl Dispatch<ZwlrDataControlDeviceV1, ()> for App {
    fn event(state: &mut Self, _: &ZwlrDataControlDeviceV1, event: zwlr_data_control_device_v1::Event, _: &(), _: &Connection, qh: &QueueHandle<Self>) {
        match event {
            zwlr_data_control_device_v1::Event::DataOffer { id } => {
                state.clipboard_offers.insert(id.id(), Vec::new());
            }
            zwlr_data_control_device_v1::Event::Selection { id } => {
                if let Some(old) = state.clipboard_offer.take() {
                    state.clipboard_offers.remove(&old.id());
                    old.destroy();
                }
                if let Some(offer) = id {
                    state.capture_selection(&offer, qh);
                    state.clipboard_offer = Some(offer);
                }
            }
            zwlr_data_control_device_v1::Event::PrimarySelection { id } => {
                if let Some(offer) = id {
                    state.clipboard_offers.remove(&offer.id());
                    offer.destroy();
                }
            }
            _ => {}
        }
    }

    wayland_client::event_created_child!(App, ZwlrDataControlDeviceV1, [
        0 => (ZwlrDataControlOfferV1, ()),
    ]);
}

impl Dispatch<ZwlrDataControlOfferV1, ()> for App {
    fn event(state: &mut Self, offer: &ZwlrDataControlOfferV1, event: zwlr_data_control_offer_v1::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
        if let zwlr_data_control_offer_v1::Event::Offer { mime_type } = event {
            state.clipboard_offers.entry(offer.id()).or_default().push(mime_type);
        }
    }
}

impl Dispatch<ZwlrDataControlSourceV1, ()> for App {
    fn event(state: &mut Self, source: &ZwlrDataControlSourceV1, event: zwlr_data_control_source_v1::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
        match event {
            zwlr_data_control_source_v1::Event::Send { fd, .. } => {
                let bytes = state.clipboard_copy_bytes.clone();
                std::thread::spawn(move || {
                    use std::io::Write;
                    let mut file = std::fs::File::from(fd);
                    let _ = file.write_all(&bytes);
                });
            }
            zwlr_data_control_source_v1::Event::Cancelled => {
                source.destroy();
                if state.clipboard_source.as_ref() == Some(source) {
                    state.clipboard_source = None;
                }
            }
            _ => {}
        }
    }
}
