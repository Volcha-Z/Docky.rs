use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::process::Stdio;
use std::sync::mpsc::Sender;
use wayland_client::{Connection, QueueHandle};

#[derive(Clone, Debug)]
pub enum IpcMessage {
    ToggleSearch,
    OsdVolume,
    OsdBrightness,
    MediaChanged,
    BatteryChanged,
    BluetoothChanged,
    WorkspacesChanged,
    ToggleWallpaper,
    ToggleClipboard,
    ScreenshotFull,
    ScreenshotRegion,
    ToggleDockMenu,
    TestNotification,
    Notify(String, String),
}

const NOTIFY_SEP: char = '\u{1f}';

fn socket_path() -> std::path::PathBuf {
    let dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());
    std::path::PathBuf::from(dir).join("dockyrs.sock")
}

pub fn send_message(text: &str) {
    if let Ok(mut stream) = UnixStream::connect(socket_path()) {
        let _ = stream.write_all(text.as_bytes());
    }
}

// ----- wakes loop -----
pub fn spawn_listener(tx: Sender<IpcMessage>, conn: Connection, qh: QueueHandle<crate::app::App>) {
    let path = socket_path();
    let _ = std::fs::remove_file(&path);
    let listener = match UnixListener::bind(&path) {
        Ok(l) => l,
        Err(err) => {
            log::warn!("failed to bind dockyrs ipc socket at {path:?}: {err}");
            return;
        }
    };
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            handle_client(stream, &tx, &conn, &qh);
        }
    });
}

fn handle_client(mut stream: UnixStream, tx: &Sender<IpcMessage>, conn: &Connection, qh: &QueueHandle<crate::app::App>) {
    let mut buf = [0u8; 1024];
    let Ok(n) = stream.read(&mut buf) else {
        return;
    };
    let text = String::from_utf8_lossy(&buf[..n]);
    let msg = match text.as_ref() {
        "toggle-search" => Some(IpcMessage::ToggleSearch),
        "osd-volume" => Some(IpcMessage::OsdVolume),
        "osd-brightness" => Some(IpcMessage::OsdBrightness),
        "toggle-wallpaper" => Some(IpcMessage::ToggleWallpaper),
        "toggle-clipboard" => Some(IpcMessage::ToggleClipboard),
        "screenshot-full" => Some(IpcMessage::ScreenshotFull),
        "screenshot-region" => Some(IpcMessage::ScreenshotRegion),
        "toggle-dock-menu" => Some(IpcMessage::ToggleDockMenu),
        "test-notification" => Some(IpcMessage::TestNotification),
        other => other.strip_prefix("notify").and_then(|rest| {
            let mut parts = rest.strip_prefix(NOTIFY_SEP)?.splitn(2, NOTIFY_SEP);
            let title = parts.next()?.to_string();
            let body = parts.next().unwrap_or("").to_string();
            Some(IpcMessage::Notify(title, body))
        }),
    };
    let Some(msg) = msg else {
        return;
    };
    if tx.send(msg).is_ok() {
        conn.display().sync(qh, ());
        let _ = conn.flush();
    }
}

pub fn spawn_brightness_watcher(tx: Sender<IpcMessage>, conn: Connection, qh: QueueHandle<crate::app::App>) {
    let Some(dir) = crate::widgets::backlight_dir() else {
        return;
    };
    let path = dir.join("brightness");
    std::thread::spawn(move || {
        let Ok(mut inotify) = inotify::Inotify::init() else {
            return;
        };
        if inotify.watches().add(&path, inotify::WatchMask::MODIFY).is_err() {
            return;
        }
        let mut buffer = [0u8; 1024];
        loop {
            let Ok(events) = inotify.read_events_blocking(&mut buffer) else {
                return;
            };
            if events.count() == 0 {
                continue;
            }
            if tx.send(IpcMessage::OsdBrightness).is_ok() {
                conn.display().sync(&qh, ());
                let _ = conn.flush();
            }
        }
    });
}

pub fn spawn_battery_watcher(tx: Sender<IpcMessage>, conn: Connection, qh: QueueHandle<crate::app::App>) {
    let Some(dir) = crate::widgets::battery_dir() else {
        return;
    };
    std::thread::spawn(move || {
        let Ok(mut inotify) = inotify::Inotify::init() else {
            return;
        };
        // ----- separate watches -----
        if inotify.watches().add(dir.join("capacity"), inotify::WatchMask::MODIFY).is_err()
            || inotify.watches().add(dir.join("status"), inotify::WatchMask::MODIFY).is_err()
        {
            return;
        }
        let mut buffer = [0u8; 1024];
        loop {
            let Ok(events) = inotify.read_events_blocking(&mut buffer) else {
                return;
            };
            if events.count() == 0 {
                continue;
            }
            if tx.send(IpcMessage::BatteryChanged).is_ok() {
                conn.display().sync(&qh, ());
                let _ = conn.flush();
            }
        }
    });
}

// ----- parent death signal -----
fn die_with_parent(cmd: &mut std::process::Command) -> &mut std::process::Command {
    use std::os::unix::process::CommandExt;
    unsafe {
        cmd.pre_exec(|| {
            libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM);
            Ok(())
        });
    }
    cmd
}

pub fn spawn_media_watcher(tx: Sender<IpcMessage>, conn: Connection, qh: QueueHandle<crate::app::App>) {
    std::thread::spawn(move || loop {
        let mut cmd = std::process::Command::new("playerctl");
        cmd.args(["metadata", "--follow", "--format", "{{status}}"]).stdout(Stdio::piped()).stderr(Stdio::null());
        let child = die_with_parent(&mut cmd).spawn();
        let Ok(mut child) = child else {
            std::thread::sleep(std::time::Duration::from_secs(5));
            continue;
        };
        if let Some(stdout) = child.stdout.take() {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                let _ = line;
                if tx.send(IpcMessage::MediaChanged).is_ok() {
                    conn.display().sync(&qh, ());
                    let _ = conn.flush();
                }
            }
        }
        let _ = child.wait();
        std::thread::sleep(std::time::Duration::from_secs(3));
    });
}

pub fn spawn_bluetooth_watcher(tx: Sender<IpcMessage>, conn: Connection, qh: QueueHandle<crate::app::App>) {
    std::thread::spawn(move || {
        let _ = watch_bluetooth(&tx, &conn, &qh);
    });
}

fn watch_bluetooth(tx: &Sender<IpcMessage>, conn: &Connection, qh: &QueueHandle<crate::app::App>) -> zbus::Result<()> {
    let system = zbus::blocking::Connection::system()?;
    let rule = zbus::MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        .interface("org.freedesktop.DBus.Properties")?
        .member("PropertiesChanged")?
        .path_namespace("/org/bluez")?
        .build();
    let iter = zbus::blocking::MessageIterator::for_match_rule(rule, &system, Some(8))?;
    for msg in iter.flatten() {
        type Changed = (String, std::collections::HashMap<String, zbus::zvariant::OwnedValue>, Vec<String>);
        let Ok((_iface, changed, invalidated)) = msg.body().deserialize::<Changed>() else {
            continue;
        };
        // ----- ignore unrelated churn -----
        let relevant = |k: &str| k == "Powered" || k == "Connected";
        if !changed.keys().any(|k| relevant(k)) && !invalidated.iter().any(|k| relevant(k)) {
            continue;
        }
        if tx.send(IpcMessage::BluetoothChanged).is_ok() {
            conn.display().sync(qh, ());
            let _ = conn.flush();
        }
    }
    Ok(())
}

pub fn spawn_workspace_watcher(tx: Sender<IpcMessage>, conn: Connection, qh: QueueHandle<crate::app::App>) {
    std::thread::spawn(move || loop {
        let Ok(sig) = std::env::var("HYPRLAND_INSTANCE_SIGNATURE") else { return };
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into());
        let path = std::path::PathBuf::from(runtime_dir).join("hypr").join(&sig).join(".socket2.sock");
        let Ok(stream) = UnixStream::connect(&path) else {
            std::thread::sleep(std::time::Duration::from_secs(3));
            continue;
        };
        for line in BufReader::new(stream).lines().map_while(Result::ok) {
            if line.starts_with("workspace") || line.starts_with("focusedmon") {
                if tx.send(IpcMessage::WorkspacesChanged).is_ok() {
                    conn.display().sync(&qh, ());
                    let _ = conn.flush();
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    });
}
