use super::*;

pub const WIDGET_CHIP_W: f32 = 100.0;
pub const WIDGET_CHIP_H: f32 = 24.0;
pub const WIDGET_CHIP_GAP: f32 = 6.0;
pub const WIDGET_COL_GAP: f32 = 10.0;
pub const WIDGET_GROUP_LABEL_H: f32 = 16.0;
pub const WIDGET_SECTION_GAP: f32 = 10.0;

pub const WIDGET_KIND_ORDER: [WidgetKind; 9] = [
    WidgetKind::Clock,
    WidgetKind::Battery,
    WidgetKind::Media,
    WidgetKind::PowerMenu,
    WidgetKind::Bluetooth,
    WidgetKind::Tray,
    WidgetKind::Workspaces,
    WidgetKind::Cpu,
    WidgetKind::Ram,
];

pub fn widget_label(kind: WidgetKind) -> &'static str {
    match kind {
        WidgetKind::Clock => "Clock",
        WidgetKind::Battery => "Battery",
        WidgetKind::Media => "Media Player",
        WidgetKind::PowerMenu => "Power Menu",
        WidgetKind::Bluetooth => "Bluetooth",
        WidgetKind::Tray => "System Tray",
        WidgetKind::Workspaces => "Workspaces",
        WidgetKind::Cpu => "CPU",
        WidgetKind::Ram => "RAM",
    }
}

fn widget_available(settings: &DockSettings) -> Vec<WidgetKind> {
    WIDGET_KIND_ORDER.iter().copied().filter(|k| !settings.widgets.iter().any(|p| p.kind == *k)).collect()
}

fn widget_in_slot(settings: &DockSettings, slot: WidgetSlot) -> Vec<WidgetKind> {
    settings.widgets.iter().filter(|p| p.slot == slot).map(|p| p.kind).collect()
}

// ----- none removes -----
pub fn place_widget(settings: &mut DockSettings, kind: WidgetKind, slot: Option<WidgetSlot>) {
    settings.widgets.retain(|p| p.kind != kind);
    if let Some(slot) = slot {
        settings.widgets.push(WidgetPlacement { kind, slot });
    }
}

pub struct WidgetChipRect {
    pub kind: WidgetKind,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub assigned: bool,
}

pub struct WidgetChipLayout {
    pub rects: Vec<WidgetChipRect>,
    pub available_label_y: f32,
    pub columns_label_y: f32,
    pub col_w: f32,
    pub total_h: f32,
}

pub fn widget_chip_layout(settings: &DockSettings, panel_width: f32, start_y: f32) -> WidgetChipLayout {
    let content_w = panel_width - MENU_PADDING * 2.0;
    let mut y = start_y;
    let mut rects = Vec::new();

    let available_label_y = y;
    y += WIDGET_GROUP_LABEL_H;
    let avail = widget_available(settings);
    let cols = ((content_w + WIDGET_CHIP_GAP) / (WIDGET_CHIP_W + WIDGET_CHIP_GAP)).floor().max(1.0) as usize;
    for (i, kind) in avail.iter().enumerate() {
        let col = i % cols;
        let row = i / cols;
        let x = MENU_PADDING + col as f32 * (WIDGET_CHIP_W + WIDGET_CHIP_GAP);
        let cy = y + row as f32 * (WIDGET_CHIP_H + WIDGET_CHIP_GAP);
        rects.push(WidgetChipRect { kind: *kind, x, y: cy, w: WIDGET_CHIP_W, h: WIDGET_CHIP_H, assigned: false });
    }
    let avail_rows = if avail.is_empty() { 0 } else { avail.len().div_ceil(cols) };
    if avail_rows > 0 {
        y += avail_rows as f32 * (WIDGET_CHIP_H + WIDGET_CHIP_GAP) - WIDGET_CHIP_GAP;
    }
    y += WIDGET_SECTION_GAP;

    let columns_label_y = y;
    y += WIDGET_GROUP_LABEL_H;
    let col_w = (content_w - WIDGET_COL_GAP * 2.0) / 3.0;
    let slots = [WidgetSlot::Left, WidgetSlot::Middle, WidgetSlot::Right];
    let mut max_rows = 0usize;
    for (ci, slot) in slots.iter().enumerate() {
        let members = widget_in_slot(settings, *slot);
        let col_x = MENU_PADDING + ci as f32 * (col_w + WIDGET_COL_GAP);
        for (ri, kind) in members.iter().enumerate() {
            let cy = y + ri as f32 * (WIDGET_CHIP_H + WIDGET_CHIP_GAP);
            rects.push(WidgetChipRect { kind: *kind, x: col_x, y: cy, w: col_w, h: WIDGET_CHIP_H, assigned: true });
        }
        max_rows = max_rows.max(members.len());
    }
    y += max_rows.max(1) as f32 * WIDGET_CHIP_H + max_rows.saturating_sub(1) as f32 * WIDGET_CHIP_GAP;

    WidgetChipLayout { rects, available_label_y, columns_label_y, col_w, total_h: y }
}

pub fn widget_chip_hit_test(settings: &DockSettings, control_y: f32, panel_width: f32, x: f32, y: f32) -> Option<WidgetKind> {
    let layout = widget_chip_layout(settings, panel_width, control_y);
    layout.rects.into_iter().find(|r| x >= r.x && x <= r.x + r.w && y >= r.y && y <= r.y + r.h).map(|r| r.kind)
}

/// none means dropped available
pub fn widget_drop_target(settings: &DockSettings, control_y: f32, panel_width: f32, x: f32, y: f32) -> Option<WidgetSlot> {
    let layout = widget_chip_layout(settings, panel_width, control_y);
    if y < layout.columns_label_y {
        return None;
    }
    let content_w = panel_width - MENU_PADDING * 2.0;
    let col_w = (content_w - WIDGET_COL_GAP * 2.0) / 3.0;
    let rel_x = x - MENU_PADDING;
    if rel_x < col_w + WIDGET_COL_GAP {
        Some(WidgetSlot::Left)
    } else if rel_x < (col_w + WIDGET_COL_GAP) * 2.0 {
        Some(WidgetSlot::Middle)
    } else {
        Some(WidgetSlot::Right)
    }
}
