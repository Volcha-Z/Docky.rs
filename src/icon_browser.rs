use std::collections::HashSet;
use std::path::{Path, PathBuf};

fn theme_dirs(theme: &str) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(data_home) = dirs::data_dir() {
        dirs.push(data_home.join("icons").join(theme));
    }
    if let Some(home) = dirs::home_dir() {
        dirs.push(home.join(".icons").join(theme));
    }
    dirs.push(PathBuf::from("/usr/share/icons").join(theme));
    dirs.push(PathBuf::from("/usr/local/share/icons").join(theme));
    dirs
}

fn walk(dir: &Path, out: &mut Vec<String>, seen: &mut HashSet<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, out, seen);
            continue;
        }
        let is_icon = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("svg") || e.eq_ignore_ascii_case("png"))
            .unwrap_or(false);
        if !is_icon {
            continue;
        }
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            if seen.insert(stem.to_string()) {
                out.push(stem.to_string());
            }
        }
    }
}

fn collect(theme: &str, out: &mut Vec<String>, seen: &mut HashSet<String>) {
    for dir in theme_dirs(theme) {
        walk(&dir, out, seen);
    }
}

pub fn list_icon_names(theme: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut seen = HashSet::new();
    collect(theme, &mut names, &mut seen);
    if names.is_empty() {
        collect("hicolor", &mut names, &mut seen);
    }
    if names.is_empty() {
        collect("Adwaita", &mut names, &mut seen);
    }
    names.sort();
    names
}
