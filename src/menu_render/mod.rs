use crate::desktop::DesktopEntry;
use crate::dock::Dock;
use crate::icon_cache::IconCache;
use crate::menu::{
    ButtonKind, Control, ControlKind, HitTarget, MenuScreen, OsdKind, WallpaperHit, MENU_PADDING, STEP_BTN_SIZE, WALLPAPER_BACK_ZONE_W,
    WALLPAPER_GAP, WALLPAPER_PADDING,
};
use crate::text::TextCache;
use crate::thumbnail_cache::ThumbnailCache;
use crate::wallpaper::WallpaperEntry;
use tiny_skia::{Paint, Pixmap, Rect, Transform};
mod app_search;
mod controls;
mod dropdowns;
mod theme_picker;
mod dock_menu;
mod wallpaper;
mod osd;
mod notification;
mod clipboard;

pub use app_search::*;
use controls::*;
use dropdowns::*;
use theme_picker::*;
pub use dock_menu::*;
use wallpaper::*;
pub use osd::*;
pub use notification::*;
pub use clipboard::*;
fn rounded_rect_path(x: f32, y: f32, w: f32, h: f32, r: f32) -> tiny_skia::Path {
    let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
    let mut pb = tiny_skia::PathBuilder::new();
    pb.move_to(x + r, y);
    pb.line_to(x + w - r, y);
    pb.quad_to(x + w, y, x + w, y + r);
    pb.line_to(x + w, y + h - r);
    pb.quad_to(x + w, y + h, x + w - r, y + h);
    pb.line_to(x + r, y + h);
    pb.quad_to(x, y + h, x, y + h - r);
    pb.line_to(x, y + r);
    pb.quad_to(x, y, x + r, y);
    pb.close();
    pb.finish().unwrap()
}

// ----- sharp edge -----
fn flat_side_rect_path(x: f32, y: f32, w: f32, h: f32, r: f32, flat: crate::config::DockEdge) -> tiny_skia::Path {
    use crate::config::DockEdge;
    let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
    let mut pb = tiny_skia::PathBuilder::new();
    match flat {
        DockEdge::Bottom => {
            pb.move_to(x, y + h);
            pb.line_to(x, y + r);
            pb.quad_to(x, y, x + r, y);
            pb.line_to(x + w - r, y);
            pb.quad_to(x + w, y, x + w, y + r);
            pb.line_to(x + w, y + h);
        }
        DockEdge::Top => {
            pb.move_to(x, y);
            pb.line_to(x, y + h - r);
            pb.quad_to(x, y + h, x + r, y + h);
            pb.line_to(x + w - r, y + h);
            pb.quad_to(x + w, y + h, x + w, y + h - r);
            pb.line_to(x + w, y);
        }
        DockEdge::Right => {
            pb.move_to(x + w, y);
            pb.line_to(x + r, y);
            pb.quad_to(x, y, x, y + r);
            pb.line_to(x, y + h - r);
            pb.quad_to(x, y + h, x + r, y + h);
            pb.line_to(x + w, y + h);
        }
        DockEdge::Left => {
            pb.move_to(x, y);
            pb.line_to(x + w - r, y);
            pb.quad_to(x + w, y, x + w, y + r);
            pb.line_to(x + w, y + h - r);
            pb.quad_to(x + w, y + h, x + w - r, y + h);
            pb.line_to(x, y + h);
        }
    }
    pb.close();
    pb.finish().unwrap()
}

fn fill_rrect(pixmap: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, r: f32, color: (u8, u8, u8, u8)) {
    if w <= 0.0 || h <= 0.0 {
        return;
    }
    let mut paint = Paint::default();
    paint.set_color_rgba8(color.0, color.1, color.2, color.3);
    paint.anti_alias = true;
    let path = rounded_rect_path(x, y, w, h, r);
    pixmap.fill_path(&path, &paint, tiny_skia::FillRule::Winding, Transform::identity(), None);
}

fn fill_circle(pixmap: &mut Pixmap, cx: f32, cy: f32, r: f32, color: (u8, u8, u8, u8)) {
    fill_rrect(pixmap, cx - r, cy - r, r * 2.0, r * 2.0, r, color);
}

fn centered_text_y(row_height: f32, size: f32) -> f32 {
    (row_height * 0.5 - size * 0.75).max(0.0)
}

fn draw_text(pixmap: &mut Pixmap, text_cache: &mut TextCache, text: &str, x: f32, y: f32, size: f32, color: &str, weight: u16) {
    if let Some(glyphs) = text_cache.get(text, size, color, weight) {
        let paint = tiny_skia::PixmapPaint::default();
        pixmap.draw_pixmap(0, 0, glyphs.as_ref().as_ref(), &paint, Transform::from_translate(x, y), None);
    }
}

fn draw_value_rtl(pixmap: &mut Pixmap, text_cache: &mut TextCache, text: &str, right_x: f32, y: f32, size: f32, color: &str, weight: u16) {
    let mut x = right_x;
    let mut buf = [0u8; 4];
    for ch in text.chars().rev() {
        let s = ch.encode_utf8(&mut buf);
        if let Some(glyph) = text_cache.get(s, size, color, weight) {
            x -= glyph.width() as f32;
            let paint = tiny_skia::PixmapPaint::default();
            pixmap.draw_pixmap(0, 0, glyph.as_ref().as_ref(), &paint, Transform::from_translate(x, y), None);
        }
    }
}

const HAIRLINE: (u8, u8, u8, u8) = (255, 255, 255, 28);
const KNOB_BORDER: (u8, u8, u8, u8) = (0, 0, 0, 30);
const HOVER_SOFT: (u8, u8, u8, u8) = (255, 255, 255, 22);

fn accent(settings: &crate::config::DockSettings) -> (u8, u8, u8, u8) {
    (settings.accent_r, settings.accent_g, settings.accent_b, 255)
}

fn accent_hex(settings: &crate::config::DockSettings) -> String {
    format!("#{:02x}{:02x}{:02x}", settings.accent_r, settings.accent_g, settings.accent_b)
}

// ----- matugen secondary -----
fn track_bg(settings: &crate::config::DockSettings) -> (u8, u8, u8, u8) {
    (settings.accent2_r, settings.accent2_g, settings.accent2_b, 35)
}

fn accent2_soft(settings: &crate::config::DockSettings) -> (u8, u8, u8, u8) {
    (settings.accent2_r, settings.accent2_g, settings.accent2_b, 40)
}

fn blend_u8(base: u8, tint: u8, amount: f32) -> u8 {
    (base as f32 * (1.0 - amount) + tint as f32 * amount) as u8
}

// ----- matugen surface -----
fn text_hex(settings: &crate::config::DockSettings) -> String {
    let r = blend_u8(settings.text_r, settings.accent_r, 0.25);
    let g = blend_u8(settings.text_g, settings.accent_g, 0.25);
    let b = blend_u8(settings.text_b, settings.accent_b, 0.25);
    format!("#{r:02x}{g:02x}{b:02x}")
}

fn text_dim_hex(settings: &crate::config::DockSettings) -> String {
    let r = blend_u8(settings.text_dim_r, settings.accent2_r, 0.25);
    let g = blend_u8(settings.text_dim_g, settings.accent2_g, 0.25);
    let b = blend_u8(settings.text_dim_b, settings.accent2_b, 0.25);
    format!("#{r:02x}{g:02x}{b:02x}")
}

fn on_accent_hex(settings: &crate::config::DockSettings) -> String {
    format!("#{:02x}{:02x}{:02x}", settings.on_accent_r, settings.on_accent_g, settings.on_accent_b)
}

fn panel_bg(settings: &crate::config::DockSettings) -> (u8, u8, u8, u8) {
    (settings.panel_r, settings.panel_g, settings.panel_b, (255.0 * settings.transparency) as u8)
}
pub struct DrawArgs<'a> {
    pub screen: MenuScreen,
    pub controls: &'a [Control],
    pub content_height: f32,
    pub panel_width: f32,
    pub dock: &'a Dock,
    pub app_entries: &'a [DesktopEntry],
    pub icon_choices: &'a [String],
    pub search_query: &'a str,
    pub wallpapers: &'a [WallpaperEntry],
    pub wallpaper_hovered: Option<WallpaperHit>,
    pub wallpaper_scroll_x: f32,
    pub hovered: Option<HitTarget>,
    pub render_scale: f32,
    pub available_fonts: &'a [String],
    pub open_dropdown: crate::menu::OpenDropdown,
    pub font_query: &'a str,
    pub custom_hex: &'a [String; 5],
    pub custom_light: bool,
    pub custom_focus: Option<usize>,
    pub custom_name: &'a str,
    pub custom_name_focused: bool,
    pub custom_panel_blend: Option<u8>,
    pub tray_items: &'a [crate::tray::TrayMenuItem],
}
pub fn draw_content(pixmap: &mut Pixmap, icon_cache: &mut IconCache, text_cache: &mut TextCache, thumb_cache: &ThumbnailCache, args: &DrawArgs) {
    let s = args.render_scale;
    let w = args.panel_width * s;
    let h = args.content_height * s;
    let settings = &args.dock.config.settings;
    let is_wallpaper_picker = matches!(args.screen, MenuScreen::WallpaperPicker);

    let bg = panel_bg(settings);
    let path = if is_wallpaper_picker {
        rounded_rect_path(0.0, 0.0, w, h, settings.corner_radius * s)
    } else {
        flat_side_rect_path(0.0, 0.0, w, h, settings.corner_radius * s, settings.dock_edge)
    };
    let mut paint = Paint::default();
    paint.set_color_rgba8(bg.0, bg.1, bg.2, bg.3);
    paint.anti_alias = true;
    pixmap.fill_path(&path, &paint, tiny_skia::FillRule::Winding, Transform::identity(), None);

    if is_wallpaper_picker {
        draw_wallpaper_filmstrip(pixmap, thumb_cache, text_cache, args);
        return;
    }

    draw_control_rows(pixmap, icon_cache, text_cache, args.controls, args);
}
fn truncate_label(name: &str, max_chars: usize) -> String {
    if name.chars().count() <= max_chars {
        name.to_string()
    } else {
        let truncated: String = name.chars().take(max_chars.saturating_sub(1)).collect();
        format!("{truncated}\u{2026}")
    }
}
fn draw_control_rows(pixmap: &mut Pixmap, icon_cache: &mut IconCache, text_cache: &mut TextCache, controls: &[Control], args: &DrawArgs) {
    let s = args.render_scale;
    let settings = &args.dock.config.settings;
    for control in controls {
        let y = control.y * s;
        match control.kind {
            ControlKind::Section(label) => {
                let ty = y + centered_text_y(control.height, 9.5) * s;
                draw_text(pixmap, text_cache, label, MENU_PADDING * s, ty, 9.5 * s, &text_dim_hex(settings), 600);
            }
            ControlKind::Note(text) => {
                let ty = y + centered_text_y(control.height, 7.5) * s;
                draw_text(pixmap, text_cache, text, MENU_PADDING * s, ty, 7.5 * s, &text_dim_hex(settings), 400);
            }
            ControlKind::IconHeader(index) => {
                let name = args
                    .dock
                    .icons
                    .get(index)
                    .map(|i| i.app.name.as_str())
                    .unwrap_or("App");
                let ty = y + centered_text_y(control.height, 10.5) * s;
                draw_text(pixmap, text_cache, name, MENU_PADDING * s, ty, 10.5 * s, &text_hex(settings), 700);
            }
            ControlKind::Slider(id) => draw_slider(pixmap, text_cache, control, id, args, y),
            ControlKind::Toggle(id) => draw_toggle(pixmap, text_cache, control, id, args, y),
            ControlKind::Button(kind) => draw_button(pixmap, text_cache, control, kind, args, y),
            ControlKind::AppEntry(index) => draw_app_entry(pixmap, icon_cache, text_cache, control, index, args, y),
            ControlKind::SearchBox => draw_search_box(pixmap, text_cache, control, args, y),
            ControlKind::IconChoice(index) => draw_icon_choice(pixmap, icon_cache, text_cache, control, index, args, y),
            ControlKind::EdgePicker => draw_edge_picker(pixmap, text_cache, control, args, y),
            ControlKind::WidgetChips => draw_widget_chips(pixmap, text_cache, control, args, y),
            ControlKind::ThemePicker => draw_theme_picker(pixmap, text_cache, control, args, y),
            ControlKind::SchemeDropdown => draw_scheme_dropdown(pixmap, text_cache, control, args, y),
            ControlKind::DockFontDropdown => {
                let is_open = matches!(args.open_dropdown, crate::menu::OpenDropdown::DockFont);
                let hot = matches!(args.hovered, Some(HitTarget::DockFontDropdownToggle)) || is_open;
                let display = if is_open && !args.font_query.is_empty() { args.font_query.to_string() } else { args.dock.config.settings.dock_font.clone() };
                draw_font_dropdown(pixmap, text_cache, control, args, y, hot, &display);
            }
            ControlKind::SystemFontDropdown => {
                let is_open = matches!(args.open_dropdown, crate::menu::OpenDropdown::SystemFont);
                let hot = matches!(args.hovered, Some(HitTarget::SystemFontDropdownToggle)) || is_open;
                let display = if is_open && !args.font_query.is_empty() { args.font_query.to_string() } else { args.dock.config.settings.system_font.clone() };
                draw_font_dropdown(pixmap, text_cache, control, args, y, hot, &display);
            }
            ControlKind::HexField(i) => draw_hex_field(pixmap, text_cache, control, args, y, i),
            ControlKind::NameField => draw_name_field(pixmap, text_cache, control, args, y),
            ControlKind::PaletteModePicker => draw_palette_mode_picker(pixmap, text_cache, control, args, y),
            ControlKind::PanelBlendPicker => draw_panel_blend_picker(pixmap, text_cache, control, args, y),
            ControlKind::TrayItem(i) => draw_tray_item(pixmap, text_cache, control, i, args, y),
            ControlKind::TraySeparator => draw_tray_separator(pixmap, control, args, y),
            ControlKind::AlignPicker => draw_align_picker(pixmap, text_cache, control, args, y),
        }
    }
}
fn text_width_estimate(text: &str, size: f32) -> f32 {
    text.chars().count() as f32 * size * 0.56
}
