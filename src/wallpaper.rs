use std::path::PathBuf;

pub struct WallpaperEntry {
    pub path: PathBuf,
    pub name: String,
}

fn candidate_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(home) = dirs::home_dir() {
        dirs.push(home.join("Pictures").join("Wallpapers"));
        dirs.push(home.join(".config/hypr/wallpapers"));
        dirs.push(home.join("wallpapers"));
        dirs.push(home.join("Pictures"));
    }
    dirs
}

pub fn scan_wallpapers() -> Vec<WallpaperEntry> {
    for dir in candidate_dirs() {
        let Ok(read_dir) = std::fs::read_dir(&dir) else {
            continue;
        };
        let mut found = Vec::new();
        for entry in read_dir.flatten() {
            let path = entry.path();
            let is_image = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| matches!(e.to_lowercase().as_str(), "png" | "jpg" | "jpeg" | "webp"))
                .unwrap_or(false);
            if is_image {
                let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("wallpaper").to_string();
                found.push(WallpaperEntry { path, name });
            }
        }
        if !found.is_empty() {
            found.sort_by(|a, b| a.name.cmp(&b.name));
            return found;
        }
    }
    Vec::new()
}

pub fn suggested_dir() -> String {
    candidate_dirs()
        .into_iter()
        .next()
        .map(|p| p.to_string_lossy().replacen(&dirs::home_dir().map(|h| h.to_string_lossy().to_string()).unwrap_or_default(), "~", 1))
        .unwrap_or_else(|| "~/Pictures/Wallpapers".to_string())
}

pub fn apply_wallpaper(path: PathBuf) {
    std::thread::spawn(move || {
        let running = std::process::Command::new("awww")
            .arg("query")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if !running {
            let _ = std::process::Command::new("awww-daemon").spawn();
            std::thread::sleep(std::time::Duration::from_millis(700));
        }
        let _ = std::process::Command::new("awww")
            .args(["img", &path.to_string_lossy(), "--transition-type", "grow", "--transition-duration", "1.0"])
            .status();
    });
}

pub fn current_wallpaper_path() -> Option<PathBuf> {
    let output = std::process::Command::new("awww").arg("query").output().ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    let line = text.lines().next()?;
    let path_str = line.split("image: ").nth(1)?.trim();
    if path_str.is_empty() {
        None
    } else {
        Some(PathBuf::from(path_str))
    }
}

// ----- matugen roles -----
pub struct ColorScheme {
    pub primary: (u8, u8, u8),
    pub secondary: (u8, u8, u8),
    pub on_primary: (u8, u8, u8),
    pub surface: (u8, u8, u8),
    pub on_surface: (u8, u8, u8),
    pub outline: (u8, u8, u8),
}

pub fn extract_color_scheme(path: &std::path::Path, scheme: &str) -> Option<ColorScheme> {
    let output = std::process::Command::new("matugen")
        .args(["--type", scheme, "image", &path.to_string_lossy(), "--source-color-index", "0", "--json", "hex", "--mode", "dark"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
    let role = |name: &str| -> Option<(u8, u8, u8)> { hex_to_rgb(json["colors"][name]["dark"]["color"].as_str()?) };
    Some(ColorScheme {
        primary: role("primary")?,
        secondary: role("secondary")?,
        on_primary: role("on_primary")?,
        surface: role("surface")?,
        on_surface: role("on_surface")?,
        outline: role("outline")?,
    })
}

fn hex_to_rgb(hex: &str) -> Option<(u8, u8, u8)> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some((r, g, b))
}
