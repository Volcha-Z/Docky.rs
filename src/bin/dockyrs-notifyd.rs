//! forwards notifications to dockyrs ipc

use std::collections::HashMap;
use std::io::Write;
use std::os::unix::net::UnixStream;
use std::sync::atomic::{AtomicU32, Ordering};

use zbus::fdo::{RequestNameFlags, RequestNameReply};
use zbus::zvariant::Value;

const NOTIFY_SEP: char = '\u{1f}';

fn socket_path() -> std::path::PathBuf {
    let dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into());
    std::path::PathBuf::from(dir).join("dockyrs.sock")
}

fn forward(summary: &str, body: &str) {
    if let Ok(mut stream) = UnixStream::connect(socket_path()) {
        let msg = format!("notify{NOTIFY_SEP}{summary}{NOTIFY_SEP}{body}");
        let _ = stream.write_all(msg.as_bytes());
    }
}

struct Notifications;

#[zbus::interface(name = "org.freedesktop.Notifications")]
impl Notifications {
    #[allow(clippy::too_many_arguments)]
    fn notify(
        &self,
        _app_name: &str,
        replaces_id: u32,
        _app_icon: &str,
        summary: &str,
        body: &str,
        _actions: Vec<&str>,
        _hints: HashMap<&str, Value<'_>>,
        _expire_timeout: i32,
    ) -> u32 {
        forward(summary, body);
        if replaces_id != 0 {
            replaces_id
        } else {
            static NEXT_ID: AtomicU32 = AtomicU32::new(1);
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        }
    }

    fn close_notification(&self, _id: u32) {}

    fn get_capabilities(&self) -> Vec<&str> {
        vec!["body"]
    }

    fn get_server_information(&self) -> (&str, &str, &str, &str) {
        ("dockyrs-notifyd", "dockyrs", env!("CARGO_PKG_VERSION"), "1.2")
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    unsafe { libc::mallopt(libc::M_ARENA_MAX, 1) };
    let conn = zbus::blocking::connection::Builder::session()?
        .serve_at("/org/freedesktop/Notifications", Notifications)?
        .build()?;

    // ----- claim the name every launch -----
    let flags = RequestNameFlags::ReplaceExisting | RequestNameFlags::AllowReplacement;
    if conn.request_name_with_flags("org.freedesktop.Notifications", flags)? != RequestNameReply::PrimaryOwner {
        let hush = || std::process::Stdio::null();
        for daemon in ["dunst", "mako", "swaync", "fnott", "wired"] {
            let _ = std::process::Command::new("systemctl")
                .args(["--user", "stop", &format!("{daemon}.service")])
                .stdout(hush()).stderr(hush()).status();
            let _ = std::process::Command::new("pkill").args(["-x", daemon]).stderr(hush()).status();
        }
        conn.request_name_with_flags("org.freedesktop.Notifications", flags)?;
    }

    loop {
        std::thread::park();
    }
}
