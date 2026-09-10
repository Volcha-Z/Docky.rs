use crate::config::PinnedApp;
use crate::desktop::{self, DesktopEntry};
use crate::dock;
use crate::dock::Dock;
use crate::icon_cache::IconCache;
use crate::menu;
use crate::menu_render;
use crate::render;
use crate::text::TextCache;
use crate::icon_browser;
use crate::thumbnail_cache::ThumbnailCache;
use crate::wallpaper::{self, WallpaperEntry};
use crate::widgets;
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_keyboard, delegate_layer, delegate_output, delegate_registry, delegate_seat,
    delegate_shm,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        keyboard::{KeyEvent, KeyboardHandler, Keysym, Modifiers},
        pointer::{PointerEvent, PointerEventKind, PointerHandler},
        Capability, SeatHandler, SeatState,
    },
    shell::{
        wlr_layer::{Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface, LayerSurfaceConfigure},
        WaylandSurface,
    },
    shm::{slot::SlotPool, Shm, ShmHandler},
};
use wayland_client::{
    backend::ObjectId,
    protocol::{wl_callback, wl_keyboard, wl_output, wl_pointer, wl_seat, wl_shm, wl_surface},
    Connection, Dispatch, QueueHandle,
};
use wayland_protocols_wlr::data_control::v1::client::{
    zwlr_data_control_device_v1::ZwlrDataControlDeviceV1, zwlr_data_control_manager_v1::ZwlrDataControlManagerV1,
    zwlr_data_control_offer_v1::ZwlrDataControlOfferV1, zwlr_data_control_source_v1::ZwlrDataControlSourceV1,
};

mod draw;
mod pointer;
mod dock_menu;
mod dock_menu_input;
mod dock_popup;
mod wallpaper_picker;
mod app_search;
mod osd;
mod notification;
mod popup_menu;
mod popup_menu_input;
mod clipboard_ui;
mod screenshot;
mod fonts;
use fonts::{apply_kitty_font, apply_system_gtk_font, apply_system_qt_font};
mod handlers;

const BTN_LEFT: u32 = 0x110;
const BTN_RIGHT: u32 = 0x111;
const MENU_GAP: i32 = 0;
const WALLPAPER_PANEL_H: f32 = 170.0;
const WALLPAPER_PANEL_MIN_W: f32 = 640.0;
const EMPTY_CUSTOM_HEX: [String; 5] = [String::new(), String::new(), String::new(), String::new(), String::new()];

pub(crate) fn single_edge_anchor_margin(edge: crate::config::DockEdge, pos_y: i32) -> (Anchor, (i32, i32, i32, i32)) {
    use crate::config::DockEdge;
    match edge {
        DockEdge::Bottom => (Anchor::BOTTOM, (0, 0, pos_y, 0)),
        DockEdge::Top => (Anchor::TOP, (pos_y, 0, 0, 0)),
        DockEdge::Left => (Anchor::LEFT, (0, 0, 0, pos_y)),
        DockEdge::Right => (Anchor::RIGHT, (0, pos_y, 0, 0)),
    }
}

pub(crate) fn edge_anchor_margin(edge: crate::config::DockEdge, align: crate::config::DockAlign, pos_y: i32, extra: i32) -> (Anchor, (i32, i32, i32, i32)) {
    use crate::config::{DockAlign, DockEdge};
    match edge {
        DockEdge::Bottom => {
            let a = match align {
                DockAlign::Middle => Anchor::BOTTOM,
                DockAlign::Left => Anchor::BOTTOM | Anchor::LEFT,
                DockAlign::Right => Anchor::BOTTOM | Anchor::RIGHT,
            };
            (a, (0, 0, pos_y + extra, 0))
        }
        DockEdge::Top => {
            let a = match align {
                DockAlign::Middle => Anchor::TOP,
                DockAlign::Left => Anchor::TOP | Anchor::LEFT,
                DockAlign::Right => Anchor::TOP | Anchor::RIGHT,
            };
            (a, (pos_y + extra, 0, 0, 0))
        }
        DockEdge::Left => {
            let a = match align {
                DockAlign::Middle => Anchor::LEFT,
                DockAlign::Left => Anchor::LEFT | Anchor::TOP,
                DockAlign::Right => Anchor::LEFT | Anchor::BOTTOM,
            };
            (a, (0, 0, 0, pos_y + extra))
        }
        DockEdge::Right => {
            let a = match align {
                DockAlign::Middle => Anchor::RIGHT,
                DockAlign::Left => Anchor::RIGHT | Anchor::TOP,
                DockAlign::Right => Anchor::RIGHT | Anchor::BOTTOM,
            };
            (a, (0, pos_y + extra, 0, 0))
        }
    }
}

pub(crate) struct MenuState {
    layer: LayerSurface,
    pool: SlotPool,
    screen: menu::MenuScreen,
    controls: Vec<menu::Control>,
    content_height: f32,
    app_entries: Vec<DesktopEntry>,
    filtered_app_entries: Vec<DesktopEntry>,
    all_icon_names: Vec<String>,
    filtered_icon_names: Vec<String>,
    search_query: String,
    anim: f32,
    target_anim: f32,
    closing: bool,
    awaiting_frame: bool,
    hovered: Option<menu::HitTarget>,
    dragging_slider: Option<menu::SettingId>,
    held_stepper: Option<(menu::SettingId, f32, u32)>,
    list_scroll: usize,
    list_selected: usize,
}

pub(crate) struct DockPopupMode {
    layer: LayerSurface,
    pool: SlotPool,
    awaiting_frame: bool,
    screen: menu::MenuScreen,
    controls: Vec<menu::Control>,
    content_height: f32,
    hovered: Option<menu::HitTarget>,
    anim: f32,
    target_anim: f32,
    closing: bool,
    tray_items: Vec<crate::tray::TrayMenuItem>,
    tray_service: String,
    tray_menu_path: String,
    tray_stack: Vec<Vec<crate::tray::TrayMenuItem>>,
    center: Option<(f32, f32)>,
    surface_w: f32,
    surface_h: f32,
    box_x: f32,
    box_y: f32,
}

pub(crate) struct DockMenuMode {
    category: menu::MenuCategory,
    controls: Vec<menu::Control>,
    hovered: Option<menu::HitTarget>,
    dragging_slider: Option<menu::SettingId>,
    held_stepper: Option<(menu::SettingId, f32, u32)>,
    dragging_widget: Option<(crate::config::WidgetKind, f32, f32)>,
    anim: f32,
    target_anim: f32,
    closing: bool,
    panel_w: f32,
    panel_h: f32,
    slide_anim: f32,
    slide_dir: f32,
    open_dropdown: menu::OpenDropdown,
    dropdown_anim: f32,
    dropdown_scroll: usize,
    font_query: String,
    dropdown_selected: usize,
    pub(crate) custom_hex: [String; 5],
    custom_light: bool,
    custom_focus: Option<usize>,
    pub(crate) custom_name: String,
    custom_name_focused: bool,
    custom_panel_blend: Option<u8>,
}

pub(crate) struct WallpaperMode {
    wallpapers: Vec<WallpaperEntry>,
    hovered: Option<menu::WallpaperHit>,
    scroll_x: f32,
    scroll_target: f32,
    anim: f32,
    target_anim: f32,
    closing: bool,
    panel_w: f32,
    panel_h: f32,
    is_vertical: bool,
    thumb_w: u32,
    thumb_h: u32,
    thumb_requested: std::collections::HashSet<std::path::PathBuf>,
    thumb_request_tx: std::sync::mpsc::Sender<(std::path::PathBuf, u32, u32)>,
    thumb_result_rx: std::sync::mpsc::Receiver<(std::path::PathBuf, u32, u32, Option<tiny_skia::Pixmap>)>,
}

pub(crate) struct AppSearchMode {
    query: String,
    all_entries: Vec<DesktopEntry>,
    filtered: Vec<DesktopEntry>,
    controls: Vec<menu::Control>,
    selected: usize,
    hovered: Option<menu::HitTarget>,
    scroll_x: f32,
    scroll_target: f32,
    highlight_x: f32,
    highlight_target: f32,
    content_anim: f32,
    anim: f32,
    target_anim: f32,
    closing: bool,
    panel_w: f32,
    panel_h: f32,
    is_vertical: bool,
}

pub(crate) struct OsdMode {
    kind: menu::OsdKind,
    level: u8,
    muted: bool,
    anim: f32,
    target_anim: f32,
    closing: bool,
    panel_w: f32,
    panel_h: f32,
}

pub(crate) struct NotificationMode {
    title: String,
    body: String,
    anim: f32,
    target_anim: f32,
    closing: bool,
    panel_w: f32,
    panel_h: f32,
}

pub struct App {
    pub registry_state: RegistryState,
    pub output_state: OutputState,
    pub seat_state: SeatState,
    pub shm: Shm,
    pub pool: SlotPool,
    pub compositor: CompositorState,
    pub layer_shell: LayerShell,
    pub layer: LayerSurface,
    pub reserve_layer: LayerSurface,
    pub pointer: Option<wl_pointer::WlPointer>,
    pub keyboard: Option<wl_keyboard::WlKeyboard>,
    pub dock: Dock,
    pub icon_cache: IconCache,
    pub text_cache: TextCache,
    pub frame_pixmap: Option<tiny_skia::Pixmap>,
    pub thumbnail_cache: ThumbnailCache,
    pub available_fonts: std::rc::Rc<Vec<String>>,
    pub output_scale: i32,
    pub awaiting_frame: bool,
    pub exit: bool,
    pub first_configure: bool,
    pub pointer_down: bool,
    pub press_pos: Option<(f64, f64)>,
    pub press_icon_index: Option<usize>,
    pub menu: Option<MenuState>,
    pub popup_mode: Option<DockPopupMode>,
    pub wallpaper_mode: Option<WallpaperMode>,
    pub dock_menu_mode: Option<DockMenuMode>,
    pub app_search_mode: Option<AppSearchMode>,
    pub osd_mode: Option<OsdMode>,
    pub osd_reset_tx: std::sync::mpsc::Sender<()>,
    pub notification_mode: Option<NotificationMode>,
    pub notification_reset_tx: std::sync::mpsc::Sender<u64>,
    pub widgets: crate::widgets::WidgetSnapshot,
    pub tray: crate::tray::TrayState,
    pub last_tray_count: usize,
    pub last_hyprctl_send: Option<std::time::Instant>,
    pub last_empty_click: Option<(std::time::Instant, f64, f64)>,
    pub marquee: render::MarqueeState,
    pub marquee_tick_tx: std::sync::mpsc::Sender<u64>,
    pub marquee_rate: u64,
    pub modifiers: Modifiers,
    pub held_key: Option<(Keysym, u32, f32)>,
    pub seat: Option<wl_seat::WlSeat>,
    pub conn: Connection,
    pub qh: QueueHandle<App>,
    pub screenshot: Option<crate::screenshot::ScreenshotState>,
    pub clipboard_manager: Option<ZwlrDataControlManagerV1>,
    pub clipboard_device: Option<ZwlrDataControlDeviceV1>,
    pub clipboard_offer: Option<ZwlrDataControlOfferV1>,
    pub clipboard_offers: std::collections::HashMap<ObjectId, Vec<String>>,
    pub clipboard_source: Option<ZwlrDataControlSourceV1>,
    pub clipboard_copy_bytes: std::sync::Arc<Vec<u8>>,
    pub clipboard_history: crate::clipboard::ClipboardHistory,
    pub clipboard_ready_at: std::time::Instant,
    pub clipboard_mode: Option<ClipboardMode>,
    pub clip_tx: std::sync::mpsc::Sender<(String, Vec<u8>)>,
    pub paste_tx: std::sync::mpsc::Sender<(crate::clipboard::PasteTarget, String)>,
}

pub(crate) struct ClipboardMode {
    query: String,
    filtered: Vec<usize>,
    selected: usize,
    scroll_y: f32,
    scroll_target: f32,
    hovered: Option<usize>,
    anim: f32,
    target_anim: f32,
    closing: bool,
    panel_w: f32,
    panel_h: f32,
    previews: std::collections::HashMap<usize, crate::clipboard::ScaledPreview>,
}



fn rgb_to_hex(r: u8, g: u8, b: u8) -> String {
    format!("{r:02x}{g:02x}{b:02x}")
}

// ----- skip blur -----
pub(super) fn anim_opacity(transparency: f32, eased: f32) -> f32 {
    if transparency >= 0.999 {
        1.0
    } else {
        eased
    }
}

fn bgra_from_rgba(src: &[u8], dst: &mut [u8]) {
    for (s, d) in src.chunks_exact(4).zip(dst.chunks_exact_mut(4)) {
        d[0] = s[2];
        d[1] = s[1];
        d[2] = s[0];
        d[3] = s[3];
    }
}

fn launch_app(exec: &str) {
    let exec = exec.to_string();
    let _ = std::process::Command::new("sh").arg("-c").arg(format!("{} &", exec)).spawn();
}

fn repo_dir() -> std::path::PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent()?.parent()?.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| std::path::PathBuf::from("."))
}

pub(crate) fn trim_heap() {
    unsafe { libc::malloc_trim(0) };
}
