use std::{env, fs, path::PathBuf};

pub fn save(png: &[u8]) -> Option<PathBuf> {
    let root = env::var_os("HOME").map(PathBuf::from)?;
    let directory = root.join("Pictures").join("Screenshots");
    fs::create_dir_all(&directory).ok()?;
    let path = directory.join(format!("Screenshot_{}.png", timestamp()));
    fs::write(&path, png).ok()?;
    Some(path)
}

fn timestamp() -> String {
    let mut now: libc::time_t = 0;
    let mut local = libc::tm {
        tm_sec: 0,
        tm_min: 0,
        tm_hour: 0,
        tm_mday: 0,
        tm_mon: 0,
        tm_year: 0,
        tm_wday: 0,
        tm_yday: 0,
        tm_isdst: 0,
        tm_gmtoff: 0,
        tm_zone: std::ptr::null(),
    };
    // ----- ffi clock -----
    unsafe {
        libc::time(&mut now);
        libc::localtime_r(&now, &mut local);
    }
    format!(
        "{:04}-{:02}-{:02}_{:02}-{:02}-{:02}",
        local.tm_year + 1900,
        local.tm_mon + 1,
        local.tm_mday,
        local.tm_hour,
        local.tm_min,
        local.tm_sec
    )
}
