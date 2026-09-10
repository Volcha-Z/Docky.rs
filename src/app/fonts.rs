fn read_ini_value(path: &std::path::Path, section: &str, key: &str) -> Option<String> {
    let existing = std::fs::read_to_string(path).ok()?;
    let section_header = format!("[{section}]");
    let key_prefix = format!("{key}=");
    let mut in_section = false;
    for line in existing.lines() {
        let trimmed = line.trim();
        if trimmed == section_header {
            in_section = true;
            continue;
        }
        if trimmed.starts_with('[') {
            in_section = false;
            continue;
        }
        if in_section && trimmed.starts_with(&key_prefix) {
            return Some(trimmed[key_prefix.len()..].to_string());
        }
    }
    None
}

fn merge_ini_key(path: &std::path::Path, section: &str, key: &str, value: &str) {
    let existing = std::fs::read_to_string(path).unwrap_or_default();
    let section_header = format!("[{section}]");
    let key_prefix = format!("{key}=");
    let full_line = format!("{key}={value}");

    let mut lines: Vec<String> = Vec::new();
    let mut in_section = false;
    let mut wrote_key = false;
    let mut saw_section = false;
    for line in existing.lines() {
        let trimmed = line.trim();
        if trimmed == section_header {
            saw_section = true;
            in_section = true;
            lines.push(line.to_string());
            continue;
        }
        if trimmed.starts_with('[') {
            if in_section && !wrote_key {
                lines.push(full_line.clone());
                wrote_key = true;
            }
            in_section = false;
            lines.push(line.to_string());
            continue;
        }
        if in_section && trimmed.starts_with(&key_prefix) {
            lines.push(full_line.clone());
            wrote_key = true;
            continue;
        }
        lines.push(line.to_string());
    }
    if !saw_section {
        lines.push(section_header);
        lines.push(full_line);
    } else if !wrote_key {
        lines.push(full_line);
    }
    let _ = std::fs::write(path, lines.join("\n") + "\n");
}

// ----- gtk ini -----
pub(super) fn apply_system_gtk_font(family: &str) {
    for gtk_dir in ["gtk-3.0", "gtk-4.0"] {
        let Some(mut path) = dirs::config_dir() else { continue };
        path.push(gtk_dir);
        if std::fs::create_dir_all(&path).is_err() {
            continue;
        }
        path.push("settings.ini");
        merge_ini_key(&path, "Settings", "gtk-font-name", &format!("{family} 10"));
    }
}

// ----- qt kdeglobals -----
pub(super) fn apply_system_qt_font(family: &str) {
    let Some(mut path) = dirs::config_dir() else { return };
    path.push("kdeglobals");
    let tail = read_ini_value(&path, "General", "font")
        .and_then(|v| v.split_once(',').map(|(_, rest)| rest.to_string()))
        .unwrap_or_else(|| "10,-1,0,50,0,0,0,0,0".to_string());
    merge_ini_key(&path, "General", "font", &format!("{family},{tail}"));
}

fn merge_flat_config_key(path: &std::path::Path, key: &str, value: &str) {
    let existing = std::fs::read_to_string(path).unwrap_or_default();
    let full_line = format!("{key} {value}");
    let mut lines: Vec<String> = Vec::new();
    let mut wrote = false;
    for line in existing.lines() {
        let trimmed = line.trim_start();
        if !trimmed.starts_with('#') && trimmed.split_whitespace().next() == Some(key) {
            lines.push(full_line.clone());
            wrote = true;
        } else {
            lines.push(line.to_string());
        }
    }
    if !wrote {
        lines.push(full_line);
    }
    let _ = std::fs::write(path, lines.join("\n") + "\n");
}

pub(super) fn apply_kitty_font(family: &str) {
    let Some(mut path) = dirs::config_dir() else { return };
    path.push("kitty");
    path.push("kitty.conf");
    if !path.exists() {
        return;
    }
    merge_flat_config_key(&path, "font_family", family);

    let Ok(entries) = std::fs::read_dir("/tmp") else { return };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if !name.starts_with("kitty-dockyrs.sock-") {
            continue;
        }
        let target = format!("unix:{}", entry.path().display());
        let _ = std::process::Command::new("kitty").args(["@", "--to", &target, "load-config"]).spawn();
    }
}
