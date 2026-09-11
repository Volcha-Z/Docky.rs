mod clipboard;
mod config;
mod desktop;
mod dock;
mod icon_browser;
mod icon_cache;
mod ipc;
mod menu;
mod menu_render;
mod power;
mod render;
mod screenshot;
mod text;
mod tray;
mod thumbnail_cache;
mod wallpaper;
mod app;
mod widgets;

use config::Config;
use dock::Dock;
use icon_cache::IconCache;
use thumbnail_cache::ThumbnailCache;
use smithay_client_toolkit::{
    compositor::CompositorState,
    output::OutputState,
    registry::RegistryState,
    seat::SeatState,
    shell::{
        wlr_layer::{KeyboardInteractivity, Layer, LayerShell},
        WaylandSurface,
    },
    shm::{slot::SlotPool, Shm},
};
use text::TextCache;
use app::App;
use wayland_client::{globals::registry_queue_init, Connection};

pub const ICON_THEME: &str = "WhiteSur";

fn main() -> anyhow::Result<()> {
    unsafe { libc::mallopt(libc::M_ARENA_MAX, 1) };
    env_logger::init();

    match std::env::args().nth(1).as_deref() {
        Some("--toggle-search") => {
            ipc::send_message("toggle-search");
            return Ok(());
        }
        Some("--osd-volume") => {
            ipc::send_message("osd-volume");
            return Ok(());
        }
        Some("--osd-brightness") => {
            ipc::send_message("osd-brightness");
            return Ok(());
        }
        Some("--toggle-wallpaper") => {
            ipc::send_message("toggle-wallpaper");
            return Ok(());
        }
        Some("--toggle-clipboard") => {
            ipc::send_message("toggle-clipboard");
            return Ok(());
        }
        Some("--screenshot-full") => {
            ipc::send_message("screenshot-full");
            return Ok(());
        }
        Some("--screenshot-region") => {
            ipc::send_message("screenshot-region");
            return Ok(());
        }
        Some("--toggle-dock-menu") => {
            ipc::send_message("toggle-dock-menu");
            return Ok(());
        }
        Some("--test-notification") => {
            ipc::send_message("test-notification");
            return Ok(());
        }
        Some("--notify") => {
            let title = std::env::args().nth(2).unwrap_or_default();
            let body = std::env::args().nth(3).unwrap_or_default();
            ipc::send_message(&format!("notify\u{1f}{title}\u{1f}{body}"));
            return Ok(());
        }
        _ => {}
    }

    let config = Config::load();
    let mut dock = Dock::new(config);
    let initial_widgets = widgets::WidgetSnapshot::refresh();
    if dock.icons.is_empty() {
        let is_vertical = dock.is_vertical();
        let cross_len = dock.cross_len();
        // ----- resized later -----
        let widget_scale = dock.config.settings.widget_scale;
        dock.widget_bar_content_len = render::widget_bar_natural_len(&dock.config.settings, &initial_widgets, 0, is_vertical, cross_len, widget_scale);
    }
    let (base_w, base_h) = dock.base_size();

    let conn = Connection::connect_to_env()?;
    let (globals, mut event_queue) = registry_queue_init::<App>(&conn)?;
    let qh = event_queue.handle();

    let compositor = CompositorState::bind(&globals, &qh)?;
    let layer_shell = LayerShell::bind(&globals, &qh)?;
    let shm = Shm::bind(&globals, &qh)?;
    let clipboard_manager = globals
        .bind::<wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_manager_v1::ZwlrDataControlManagerV1, _, _>(&qh, 1..=2, ())
        .ok();

    let seat_state = SeatState::new(&globals, &qh);
    let seat = seat_state.seats().next();
    // ----- eager avoids race -----
    let clipboard_device = clipboard_manager.as_ref().zip(seat.as_ref()).map(|(m, s)| m.get_data_device(s, &qh, ()));

    let surface = compositor.create_surface(&qh);
    let layer =
        layer_shell.create_layer_surface(&qh, surface, Layer::Top, Some("dockyrs"), None);

    let s = &dock.config.settings;
    let (anchor, margin) = app::edge_anchor_margin(s.dock_edge, s.dock_align, s.pos_y, 0);
    layer.set_anchor(anchor);
    layer.set_size(base_w, base_h);
    layer.set_margin(margin.0, margin.1, margin.2, margin.3);
    layer.set_exclusive_zone(-1);
    layer.set_keyboard_interactivity(KeyboardInteractivity::None);
    layer.commit();

    let reserve_surface = compositor.create_surface(&qh);
    let reserve_layer = layer_shell.create_layer_surface(&qh, reserve_surface, Layer::Top, Some("dockyrs-reserve"), None);
    let (reserve_anchor, reserve_margin) = app::single_edge_anchor_margin(s.dock_edge, s.pos_y);
    reserve_layer.set_anchor(reserve_anchor);
    reserve_layer.set_size(1, 1);
    reserve_layer.set_margin(reserve_margin.0, reserve_margin.1, reserve_margin.2, reserve_margin.3);
    reserve_layer.set_exclusive_zone(dock.thickness() as i32 + s.pos_y);
    reserve_layer.set_keyboard_interactivity(KeyboardInteractivity::None);
    if let Ok(empty_region) = smithay_client_toolkit::compositor::Region::new(&compositor) {
        reserve_layer.wl_surface().set_input_region(Some(empty_region.wl_region()));
    }
    reserve_layer.commit();

    let pool_size = (base_w as usize * 3) * (base_h as usize * 3) * 4;
    let pool = SlotPool::new(pool_size.max(4096), &shm)?;

    let (osd_reset_tx, osd_reset_rx) = std::sync::mpsc::channel::<()>();
    let (notification_reset_tx, notification_reset_rx) = std::sync::mpsc::channel::<u64>();
    let (marquee_tick_tx, marquee_tick_rx) = std::sync::mpsc::channel::<u64>();

    let tray_state: tray::TrayState = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let tray_tick_pending = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    tray::spawn(tray_state.clone(), tray_tick_pending.clone(), conn.clone(), qh.clone());

    let available_fonts = std::rc::Rc::new(text::list_font_families());
    let mut text_cache = TextCache::new();
    text_cache.set_default_family(&dock.config.settings.dock_font);

    let (paste_tx, paste_rx) = std::sync::mpsc::channel::<(clipboard::PasteTarget, String)>();
    let (clip_tx, clip_rx) = std::sync::mpsc::channel::<(String, Vec<u8>)>();

    let output_state = OutputState::new(&globals, &qh);
    let screenshot = screenshot::ScreenshotState::new(&globals, &qh, &compositor, &layer_shell, &output_state);

    let mut app = App {
        registry_state: RegistryState::new(&globals),
        output_state,
        seat_state,
        shm,
        pool,
        compositor,
        layer_shell,
        layer,
        reserve_layer,
        pointer: None,
        keyboard: None,
        dock,
        icon_cache: IconCache::new(ICON_THEME),
        text_cache,
        frame_pixmap: None,
        available_fonts: available_fonts.clone(),
        thumbnail_cache: ThumbnailCache::new(),
        output_scale: 1,
        awaiting_frame: false,
        exit: false,
        first_configure: true,
        pointer_down: false,
        press_pos: None,
        press_icon_index: None,
        menu: None,
        popup_mode: None,
        wallpaper_mode: None,
        dock_menu_mode: None,
        app_search_mode: None,
        osd_mode: None,
        osd_reset_tx,
        notification_mode: None,
        notification_reset_tx,
        widgets: initial_widgets,
        tray: tray_state,
        last_tray_count: 0,
        last_hyprctl_send: None,
        last_empty_click: None,
        marquee: render::MarqueeState::default(),
        marquee_tick_tx,
        marquee_rate: 0,
        modifiers: Default::default(),
        held_key: None,
        seat,
        conn: conn.clone(),
        qh: qh.clone(),
        screenshot,
        clipboard_manager,
        clipboard_device,
        clipboard_offer: None,
        clipboard_offers: std::collections::HashMap::new(),
        clipboard_source: None,
        clipboard_copy_bytes: std::sync::Arc::new(Vec::new()),
        clipboard_history: clipboard::ClipboardHistory::new(),
        clipboard_ready_at: std::time::Instant::now() + std::time::Duration::from_millis(500),
        clipboard_mode: None,
        clip_tx,
        paste_tx,
    };

    let (ipc_tx, ipc_rx) = std::sync::mpsc::channel::<ipc::IpcMessage>();
    ipc::spawn_listener(ipc_tx.clone(), conn.clone(), qh.clone());
    ipc::spawn_brightness_watcher(ipc_tx.clone(), conn.clone(), qh.clone());
    ipc::spawn_battery_watcher(ipc_tx.clone(), conn.clone(), qh.clone());
    ipc::spawn_bluetooth_watcher(ipc_tx.clone(), conn.clone(), qh.clone());
    ipc::spawn_media_watcher(ipc_tx.clone(), conn.clone(), qh.clone());
    ipc::spawn_workspace_watcher(ipc_tx, conn.clone(), qh.clone());

    let clock_tick_pending = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    spawn_clock_ticker(clock_tick_pending.clone(), conn.clone(), qh.clone());

    let cpu_ram_tick_pending = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    spawn_cpu_ram_ticker(cpu_ram_tick_pending.clone(), conn.clone(), qh.clone());

    let osd_timeout_pending = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    spawn_osd_timer(osd_reset_rx, osd_timeout_pending.clone(), conn.clone(), qh.clone());

    let notification_timeout_pending = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    spawn_notification_timer(notification_reset_rx, notification_timeout_pending.clone(), conn.clone(), qh.clone());

    let marquee_tick_pending = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    spawn_marquee_ticker(marquee_tick_rx, marquee_tick_pending.clone(), conn.clone(), qh.clone());

    loop {
        event_queue.blocking_dispatch(&mut app)?;
        while let Ok(msg) = ipc_rx.try_recv() {
            match msg {
                ipc::IpcMessage::ToggleSearch => app.toggle_app_search(&qh),
                ipc::IpcMessage::OsdVolume => app.show_osd(menu::OsdKind::Volume, &qh),
                ipc::IpcMessage::OsdBrightness => app.show_osd(menu::OsdKind::Brightness, &qh),
                ipc::IpcMessage::MediaChanged => app.refresh_media(&qh),
                ipc::IpcMessage::BatteryChanged => app.refresh_battery(&qh),
                ipc::IpcMessage::BluetoothChanged => app.refresh_bluetooth(&qh),
                ipc::IpcMessage::WorkspacesChanged => app.refresh_workspaces(&qh),
                ipc::IpcMessage::ToggleWallpaper => app.toggle_wallpaper_picker(&qh),
                ipc::IpcMessage::ToggleClipboard => app.toggle_clipboard(&qh),
                ipc::IpcMessage::ScreenshotFull => app.start_full_screenshot(&qh),
                ipc::IpcMessage::ScreenshotRegion => app.start_region_screenshot(&qh),
                ipc::IpcMessage::ToggleDockMenu => app.toggle_dock_menu(&qh),
                ipc::IpcMessage::TestNotification => {
                    app.show_notification("Test Notification".to_string(), "This is a test notification from Docky.rs".to_string(), &qh)
                }
                ipc::IpcMessage::Notify(title, body) => app.show_notification(title, body, &qh),
            }
        }
        while let Ok((target, text)) = paste_rx.try_recv() {
            app.apply_pending_paste(target, text, &qh);
        }
        while let Ok((mime, data)) = clip_rx.try_recv() {
            app.ingest_clipboard_capture(mime, data, &qh);
        }
        if clock_tick_pending.swap(false, std::sync::atomic::Ordering::SeqCst) {
            app.refresh_clock(&qh);
        }
        if cpu_ram_tick_pending.swap(false, std::sync::atomic::Ordering::SeqCst) {
            app.refresh_cpu_ram(&qh);
        }
        if tray_tick_pending.swap(false, std::sync::atomic::Ordering::SeqCst) {
            app.sync_tray_layout(&qh);
        }
        if osd_timeout_pending.swap(false, std::sync::atomic::Ordering::SeqCst) {
            app.close_osd_mode(&qh);
        }
        if notification_timeout_pending.swap(false, std::sync::atomic::Ordering::SeqCst) {
            app.close_notification_mode(&qh);
        }
        if marquee_tick_pending.swap(false, std::sync::atomic::Ordering::SeqCst) {
            app.tick_marquee(&qh);
        }
        if app.exit {
            break;
        }
    }

    Ok(())
}

fn spawn_clock_ticker(flag: std::sync::Arc<std::sync::atomic::AtomicBool>, conn: Connection, qh: wayland_client::QueueHandle<App>) {
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_secs(20));
        flag.store(true, std::sync::atomic::Ordering::SeqCst);
        conn.display().sync(&qh, ());
        let _ = conn.flush();
    });
}

fn spawn_cpu_ram_ticker(flag: std::sync::Arc<std::sync::atomic::AtomicBool>, conn: Connection, qh: wayland_client::QueueHandle<App>) {
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_secs(15));
        flag.store(true, std::sync::atomic::Ordering::SeqCst);
        conn.display().sync(&qh, ());
        let _ = conn.flush();
    });
}

fn spawn_osd_timer(
    reset_rx: std::sync::mpsc::Receiver<()>,
    flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
    conn: Connection,
    qh: wayland_client::QueueHandle<App>,
) {
    use std::sync::mpsc::RecvTimeoutError;
    std::thread::spawn(move || loop {
        if reset_rx.recv().is_err() {
            return;
        }
        loop {
            match reset_rx.recv_timeout(std::time::Duration::from_millis(menu::OSD_TIMEOUT_MS)) {
                Ok(()) => continue,
                Err(RecvTimeoutError::Timeout) => break,
                Err(RecvTimeoutError::Disconnected) => return,
            }
        }
        flag.store(true, std::sync::atomic::Ordering::SeqCst);
        conn.display().sync(&qh, ());
        let _ = conn.flush();
    });
}

fn spawn_notification_timer(
    reset_rx: std::sync::mpsc::Receiver<u64>,
    flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
    conn: Connection,
    qh: wayland_client::QueueHandle<App>,
) {
    use std::sync::mpsc::RecvTimeoutError;
    std::thread::spawn(move || {
        let mut dur;
        loop {
            match reset_rx.recv() {
                Ok(ms) => dur = ms,
                Err(_) => return,
            }
            loop {
                match reset_rx.recv_timeout(std::time::Duration::from_millis(dur)) {
                    Ok(ms) => {
                        dur = ms;
                        continue;
                    }
                    Err(RecvTimeoutError::Timeout) => break,
                    Err(RecvTimeoutError::Disconnected) => return,
                }
            }
            flag.store(true, std::sync::atomic::Ordering::SeqCst);
            conn.display().sync(&qh, ());
            let _ = conn.flush();
        }
    });
}

fn spawn_marquee_ticker(
    rx: std::sync::mpsc::Receiver<u64>,
    flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
    conn: Connection,
    qh: wayland_client::QueueHandle<App>,
) {
    use std::sync::mpsc::RecvTimeoutError;
    std::thread::spawn(move || {
        let mut interval_ms: u64 = 0;
        loop {
            if interval_ms == 0 {
                match rx.recv() {
                    Ok(v) => interval_ms = v,
                    Err(_) => return,
                }
                continue;
            }
            match rx.recv_timeout(std::time::Duration::from_millis(interval_ms)) {
                Ok(v) => interval_ms = v,
                Err(RecvTimeoutError::Timeout) => {
                    flag.store(true, std::sync::atomic::Ordering::SeqCst);
                    conn.display().sync(&qh, ());
                    let _ = conn.flush();
                }
                Err(RecvTimeoutError::Disconnected) => return,
            }
        }
    });
}
