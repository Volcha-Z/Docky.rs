use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinnedApp {
    pub name: String,
    pub icon: String,
    pub exec: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DockEdge {
    Top,
    Bottom,
    Left,
    Right,
}

impl Default for DockEdge {
    fn default() -> Self {
        DockEdge::Bottom
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DockAlign {
    Left,
    Middle,
    Right,
}

impl Default for DockAlign {
    fn default() -> Self {
        DockAlign::Middle
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WidgetSlot {
    Left,
    Middle,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WidgetKind {
    Clock,
    Battery,
    Media,
    PowerMenu,
    Bluetooth,
    Tray,
    Workspaces,
    Cpu,
    Ram,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WidgetPlacement {
    pub kind: WidgetKind,
    pub slot: WidgetSlot,
}

fn default_widgets() -> Vec<WidgetPlacement> {
    vec![
        WidgetPlacement { kind: WidgetKind::Media, slot: WidgetSlot::Left },
        WidgetPlacement { kind: WidgetKind::Clock, slot: WidgetSlot::Middle },
        WidgetPlacement { kind: WidgetKind::Tray, slot: WidgetSlot::Right },
        WidgetPlacement { kind: WidgetKind::Battery, slot: WidgetSlot::Right },
    ]
}

fn default_matugen_scheme() -> String {
    "scheme-neutral".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomPalette {
    pub name: String,
    pub accent: (u8, u8, u8),
    pub accent2: (u8, u8, u8),
    pub panel: (u8, u8, u8),
    pub text: (u8, u8, u8),
    pub text_dim: (u8, u8, u8),
    pub is_light: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DockSettings {
    pub icon_size: f32,
    pub dock_scale: f32,
    pub widget_scale: f32,
    pub media_smooth_scroll: bool,
    pub media_width_scale: f32,
    pub icon_gap: f32,
    pub magnify_scale: f32,
    pub magnify_radius: f32,
    pub width_padding: f32,
    pub corner_radius: f32,
    pub border_width: f32,
    pub pos_y: i32,
    pub dock_align: DockAlign,
    pub transparency: f32,
    pub blur_enabled: bool,
    pub blur_passes: i32,
    pub blur_size: i32,
    pub blur_vibrancy: f32,
    pub blur_brightness: f32,
    pub blur_contrast: f32,
    pub blur_xray: bool,
    pub accent_r: u8,
    pub accent_g: u8,
    pub accent_b: u8,
    pub accent2_r: u8,
    pub accent2_g: u8,
    pub accent2_b: u8,
    pub on_accent_r: u8,
    pub on_accent_g: u8,
    pub on_accent_b: u8,
    pub panel_r: u8,
    pub panel_g: u8,
    pub panel_b: u8,
    pub text_r: u8,
    pub text_g: u8,
    pub text_b: u8,
    pub text_dim_r: u8,
    pub text_dim_g: u8,
    pub text_dim_b: u8,
    pub accent_from_wallpaper: bool,
    pub last_wallpaper: String,
    #[serde(default)]
    pub custom_theme: bool,
    #[serde(default = "default_matugen_scheme")]
    pub matugen_scheme: String,
    #[serde(default)]
    pub dock_font: String,
    #[serde(default)]
    pub system_font: String,
    pub dock_edge: DockEdge,
    #[serde(default = "default_widgets")]
    pub widgets: Vec<WidgetPlacement>,
    #[serde(default)]
    pub custom_palettes: Vec<CustomPalette>,
}

impl Default for DockSettings {
    fn default() -> Self {
        Self {
            icon_size: 35.688725,
            dock_scale: 0.49597847,
            widget_scale: 1.0,
            media_smooth_scroll: false,
            media_width_scale: 1.2026273,
            icon_gap: 2.858622,
            magnify_scale: 1.35,
            magnify_radius: 80.0,
            width_padding: 67.51996,
            corner_radius: 8.40339,
            border_width: 0.0,
            pos_y: 7,
            dock_align: DockAlign::Middle,
            transparency: 1.0,
            blur_enabled: false,
            blur_passes: 3,
            blur_size: 6,
            blur_vibrancy: 0.2,
            blur_brightness: 0.85,
            blur_contrast: 1.0,
            blur_xray: false,
            accent_r: 203,
            accent_g: 166,
            accent_b: 247,
            accent2_r: 137,
            accent2_g: 180,
            accent2_b: 250,
            on_accent_r: 20,
            on_accent_g: 20,
            on_accent_b: 24,
            panel_r: 37,
            panel_g: 33,
            panel_b: 43,
            text_r: 235,
            text_g: 235,
            text_b: 240,
            text_dim_r: 150,
            text_dim_g: 150,
            text_dim_b: 158,
            accent_from_wallpaper: false,
            custom_theme: false,
            last_wallpaper: String::new(),
            matugen_scheme: default_matugen_scheme(),
            dock_font: "Adwaita Sans".to_string(),
            system_font: String::new(),
            dock_edge: DockEdge::Top,
            widgets: default_widgets(),
            custom_palettes: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub apps: Vec<PinnedApp>,
    pub settings: DockSettings,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            apps: default_apps(),
            settings: DockSettings::default(),
        }
    }
}

fn default_apps() -> Vec<PinnedApp> {
    Vec::new()
}

fn config_path() -> PathBuf {
    let mut dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    dir.push("dockyrs");
    dir.push("config.json");
    dir
}

impl Config {
    pub fn load() -> Self {
        let path = config_path();
        match std::fs::read_to_string(&path) {
            Ok(raw) => serde_json::from_str(&raw).unwrap_or_else(|err| {
                log::warn!("failed to parse config at {path:?}, using defaults: {err}");
                Config::default()
            }),
            Err(_) => Config::default(),
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let raw = serde_json::to_string_pretty(self)?;
        std::fs::write(path, raw)?;
        Ok(())
    }
}
