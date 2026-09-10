use crate::config::{Config, DockEdge, PinnedApp};

const EASE_FACTOR: f32 = 0.35;
const SETTLE_EPSILON: f32 = 0.001;
pub const DRAG_THRESHOLD: f32 = 6.0;
const V_EDGE_PADDING: f32 = 12.0;
// ----- static base -----
const REFERENCE_ICON_SIZE: f32 = 44.0;
const REFERENCE_GAP: f32 = 8.0;
const MAX_DOCK_WIDTH: f32 = 2400.0;
// ----- nonzero size -----
const WIDGET_BAR_MIN: f32 = 60.0;

pub struct DockIcon {
    pub app: PinnedApp,
    pub scale: f32,
    pub target_scale: f32,
    pub x: f32,
    pub target_x: f32,
}

pub struct Dock {
    pub icons: Vec<DockIcon>,
    pub config: Config,
    pub pointer_pos: Option<(f64, f64)>,
    pub dragging_index: Option<usize>,
    // ----- caller syncs -----
    pub widget_bar_content_len: f32,
}

impl Dock {
    pub fn new(config: Config) -> Self {
        let icons: Vec<DockIcon> = config
            .apps
            .iter()
            .cloned()
            .map(|app| DockIcon {
                app,
                scale: 1.0,
                target_scale: 1.0,
                x: 0.0,
                target_x: 0.0,
            })
            .collect();

        let mut dock = Self {
            icons,
            config,
            pointer_pos: None,
            dragging_index: None,
            widget_bar_content_len: 0.0,
        };
        dock.update_layout_targets();
        for icon in &mut dock.icons {
            icon.x = icon.target_x;
        }
        dock
    }

    fn effective_icon_size(&self) -> f32 {
        let s = &self.config.settings;
        s.icon_size * s.dock_scale
    }

    fn effective_gap(&self) -> f32 {
        let s = &self.config.settings;
        s.icon_gap * s.dock_scale
    }

    pub fn is_vertical(&self) -> bool {
        matches!(self.config.settings.dock_edge, DockEdge::Left | DockEdge::Right)
    }

    pub fn cross_len(&self) -> f32 {
        let s = &self.config.settings;
        let icon_size = REFERENCE_ICON_SIZE * s.dock_scale;
        icon_size * s.magnify_scale + V_EDGE_PADDING * s.dock_scale * 2.0
    }

    pub fn base_size(&self) -> (u32, u32) {
        let s = &self.config.settings;
        let cross_len = self.cross_len();
        if self.icons.is_empty() {
            let bar_len = (self.widget_bar_content_len + s.width_padding * s.widget_scale * 2.0).max(WIDGET_BAR_MIN).min(MAX_DOCK_WIDTH);
            return if self.is_vertical() {
                (cross_len.ceil() as u32, bar_len.ceil() as u32)
            } else {
                (bar_len.ceil() as u32, cross_len.ceil() as u32)
            };
        }
        let icon_size = REFERENCE_ICON_SIZE * s.dock_scale;
        let n = self.icons.len() as f32;
        let spacing = REFERENCE_GAP * s.dock_scale;
        let main_len = (n * icon_size + (n - 1.0).max(0.0) * spacing + s.width_padding * s.dock_scale * 2.0).min(MAX_DOCK_WIDTH);
        if self.is_vertical() {
            (cross_len.ceil() as u32, main_len.ceil() as u32)
        } else {
            (main_len.ceil() as u32, cross_len.ceil() as u32)
        }
    }

    pub fn thickness(&self) -> u32 {
        let (w, h) = self.base_size();
        if self.is_vertical() { w } else { h }
    }

    fn main_axis_len(&self) -> f32 {
        let (w, h) = self.base_size();
        if self.is_vertical() { h as f32 } else { w as f32 }
    }

    fn rest_centers(&self) -> impl Iterator<Item = f32> + use<> {
        let icon_size = self.effective_icon_size();
        let spacing = self.effective_gap();
        let total = self.main_axis_len();
        let count = self.icons.len();
        let content = count as f32 * icon_size + (count as f32 - 1.0).max(0.0) * spacing;
        let start = (total - content) / 2.0;
        (0..count).map(move |i| start + i as f32 * (icon_size + spacing) + icon_size / 2.0)
    }

    pub fn relayout(&mut self) {
        self.update_layout_targets();
        for icon in &mut self.icons {
            if self.dragging_index.is_none() {
                icon.x = icon.target_x;
            }
        }
    }

    pub fn add_app(&mut self, app: PinnedApp) {
        let x = self.base_size().0 as f32;
        self.icons.push(DockIcon {
            app,
            scale: 1.0,
            target_scale: 1.0,
            x,
            target_x: x,
        });
        self.relayout();
        self.config.apps = self.icons.iter().map(|i| i.app.clone()).collect();
    }

    pub fn remove_icon(&mut self, index: usize) {
        if index < self.icons.len() {
            self.icons.remove(index);
            self.relayout();
            self.config.apps = self.icons.iter().map(|i| i.app.clone()).collect();
        }
    }

    pub fn set_icon(&mut self, index: usize, icon_name: String) {
        if let Some(icon) = self.icons.get_mut(index) {
            icon.app.icon = icon_name;
            self.config.apps = self.icons.iter().map(|i| i.app.clone()).collect();
        }
    }

    fn update_layout_targets(&mut self) {
        let centers = self.rest_centers();
        let dragging = self.dragging_index;
        for (i, (icon, cx)) in self.icons.iter_mut().zip(centers).enumerate() {
            if Some(i) == dragging {
                continue;
            }
            icon.target_x = cx;
        }
    }

    pub fn update_magnification(&mut self) {
        let s = &self.config.settings;
        let (base_scale, radius, max_scale) = (1.0_f32, s.magnify_radius, s.magnify_scale);
        let centers = self.rest_centers();
        let vertical = self.is_vertical();
        match self.pointer_pos {
            Some((px, py)) => {
                let main = if vertical { py as f32 } else { px as f32 };
                for (icon, cx) in self.icons.iter_mut().zip(centers) {
                    let dist = (main - cx).abs();
                    let t = (dist / radius).min(1.0);
                    let falloff = 0.5 * (1.0 + (std::f32::consts::PI * t).cos());
                    icon.target_scale = base_scale + (max_scale - base_scale) * falloff;
                }
            }
            None => {
                for icon in &mut self.icons {
                    icon.target_scale = base_scale;
                }
            }
        }
    }

    pub fn set_pointer(&mut self, pos: Option<(f64, f64)>) {
        self.pointer_pos = pos;
        self.update_magnification();
    }

    pub fn start_drag(&mut self, index: usize) {
        self.dragging_index = Some(index);
    }

    pub fn drag_to(&mut self, cursor_x: f32, cursor_y: f32) {
        let Some(index) = self.dragging_index else {
            return;
        };
        let cursor_main = if self.is_vertical() { cursor_y } else { cursor_x };
        self.icons[index].x = cursor_main;
        self.icons[index].target_x = cursor_main;

        let centers = self.rest_centers();
        let mut nearest = 0usize;
        let mut best = f32::MAX;
        for (i, c) in centers.enumerate() {
            let d = (cursor_main - c).abs();
            if d < best {
                best = d;
                nearest = i;
            }
        }
        if nearest != index {
            let icon = self.icons.remove(index);
            self.icons.insert(nearest, icon);
            self.dragging_index = Some(nearest);
            self.update_layout_targets();
        }
    }

    pub fn end_drag(&mut self) {
        self.dragging_index = None;
        self.update_layout_targets();
        self.config.apps = self.icons.iter().map(|i| i.app.clone()).collect();
    }

    pub fn step_animation(&mut self) -> bool {
        let mut animating = false;
        for icon in &mut self.icons {
            let dscale = icon.target_scale - icon.scale;
            if dscale.abs() > SETTLE_EPSILON {
                icon.scale += dscale * EASE_FACTOR;
                animating = true;
            } else {
                icon.scale = icon.target_scale;
            }

            let dx = icon.target_x - icon.x;
            if dx.abs() > SETTLE_EPSILON {
                icon.x += dx * EASE_FACTOR;
                animating = true;
            } else {
                icon.x = icon.target_x;
            }
        }
        animating
    }

    pub fn elevate_dir(&self) -> (f32, f32) {
        match self.config.settings.dock_edge {
            DockEdge::Bottom => (0.0, -1.0),
            DockEdge::Top => (0.0, 1.0),
            DockEdge::Left => (1.0, 0.0),
            DockEdge::Right => (-1.0, 0.0),
        }
    }

    pub fn layout(&self) -> Vec<(f32, f32, f32)> {
        let s = &self.config.settings;
        let icon_size = self.effective_icon_size();
        let cross_total = self.thickness() as f32;
        let edge_padding = V_EDGE_PADDING * s.dock_scale;
        let baseline = match s.dock_edge {
            DockEdge::Bottom | DockEdge::Right => cross_total - edge_padding,
            DockEdge::Top | DockEdge::Left => edge_padding,
        };
        let grows_positive = matches!(s.dock_edge, DockEdge::Top | DockEdge::Left);
        let vertical = self.is_vertical();

        self.icons
            .iter()
            .map(|icon| {
                let w = icon_size * icon.scale;
                let cross = if grows_positive { baseline + w / 2.0 } else { baseline - w / 2.0 };
                if vertical {
                    (cross, icon.x, w)
                } else {
                    (icon.x, cross, w)
                }
            })
            .collect()
    }

    pub fn icon_at(&self, x: f64, y: f64) -> Option<usize> {
        for (i, (cx, cy, w)) in self.layout().iter().enumerate() {
            let half = w / 2.0;
            if (x as f32) >= cx - half
                && (x as f32) <= cx + half
                && (y as f32) >= cy - half
                && (y as f32) <= cy + half
            {
                return Some(i);
            }
        }
        None
    }
}
