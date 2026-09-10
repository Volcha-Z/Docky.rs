mod encode;
mod region;
mod storage;
mod wayland;

use smithay_client_toolkit::{
    compositor::{CompositorState, Region},
    output::OutputState,
    shell::{
        wlr_layer::{Anchor, KeyboardInteractivity, Layer, LayerShell},
        WaylandSurface,
    },
    shm::{
        slot::{Buffer, SlotPool},
        Shm,
    },
};
use wayland_client::{
    globals::{BindError, GlobalList},
    protocol::{wl_output, wl_shm},
    QueueHandle, WEnum,
};
use wayland_protocols_wlr::screencopy::v1::client::{
    zwlr_screencopy_frame_v1::{Flags, ZwlrScreencopyFrameV1},
    zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1,
};

use crate::app::App;
pub use region::{RegionSelector, Selection};

#[derive(Clone, Copy)]
pub enum CaptureTarget {
    Full,
    Region(Selection),
}

#[derive(Clone, Copy)]
pub struct FrameSpec {
    pub format: wl_shm::Format,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
}

pub struct CaptureImage {
    pub png: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub thumbnail: Vec<u8>,
}

struct Capture {
    frame: ZwlrScreencopyFrameV1,
    spec: Option<FrameSpec>,
    pool: Option<SlotPool>,
    buffer: Option<Buffer>,
    inverted: bool,
}

pub struct ScreenshotState {
    manager: ZwlrScreencopyManagerV1,
    output: wl_output::WlOutput,
    active: Option<Capture>,
    pub selector: RegionSelector,
}

impl ScreenshotState {
    pub fn new(globals: &GlobalList, qh: &QueueHandle<App>, compositor: &CompositorState, layer_shell: &LayerShell, output_state: &OutputState) -> Option<Self> {
        let manager: ZwlrScreencopyManagerV1 = bind_manager(globals, qh).ok()?;
        let output = output_state.outputs().next()?;
        let surface = compositor.create_surface(qh);
        let layer = layer_shell.create_layer_surface(qh, surface, Layer::Overlay, Some("dockyrs-region"), Some(&output));
        layer.set_anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT);
        layer.set_size(0, 0);
        layer.set_exclusive_zone(-1);
        layer.set_keyboard_interactivity(KeyboardInteractivity::None);
        if let Ok(region) = Region::new(compositor) {
            layer.wl_surface().set_input_region(Some(region.wl_region()));
        }
        Some(Self {
            manager,
            output,
            active: None,
            selector: RegionSelector::new(layer),
        })
    }

    pub fn start(&mut self, target: CaptureTarget, qh: &QueueHandle<App>) {
        if self.active.is_some() {
            return;
        }
        let frame = match target {
            CaptureTarget::Full => self.manager.capture_output(0, &self.output, qh, ()),
            CaptureTarget::Region(selection) => {
                self.manager.capture_output_region(0, &self.output, selection.x, selection.y, selection.width, selection.height, qh, ())
            }
        };
        self.active = Some(Capture {
            frame,
            spec: None,
            pool: None,
            buffer: None,
            inverted: false,
        });
    }

    pub fn set_spec(&mut self, format: WEnum<wl_shm::Format>, width: u32, height: u32, stride: u32) {
        let WEnum::Value(format @ (wl_shm::Format::Argb8888 | wl_shm::Format::Xrgb8888)) = format else {
            return;
        };
        if let Some(capture) = &mut self.active {
            capture.spec = Some(FrameSpec {
                format,
                width,
                height,
                stride,
            });
        }
    }

    pub fn prepare(&mut self, shm: &Shm) {
        let Some(capture) = &mut self.active else {
            return;
        };
        let Some(spec) = capture.spec else {
            capture.frame.destroy();
            self.active = None;
            return;
        };
        let Some(length) = spec.stride.checked_mul(spec.height).map(|value| value as usize) else {
            return;
        };
        let Ok(mut pool) = SlotPool::new(length, shm) else {
            return;
        };
        let Ok((buffer, _)) = pool.create_buffer(spec.width as i32, spec.height as i32, spec.stride as i32, spec.format) else {
            return;
        };
        capture.frame.copy(buffer.wl_buffer());
        capture.pool = Some(pool);
        capture.buffer = Some(buffer);
    }

    pub fn set_flags(&mut self, flags: WEnum<Flags>) {
        if let Some(capture) = &mut self.active {
            capture.inverted = matches!(flags, WEnum::Value(value) if value.contains(Flags::YInvert));
        }
    }

    pub fn finish(&mut self) -> Option<CaptureImage> {
        let mut capture = self.active.take()?;
        capture.frame.destroy();
        let spec = capture.spec?;
        let buffer = capture.buffer.take()?;
        let canvas = buffer.canvas(capture.pool.as_mut()?)?;
        encode::encode(canvas, spec, capture.inverted)
    }

    pub fn fail(&mut self) {
        if let Some(capture) = self.active.take() {
            capture.frame.destroy();
        }
    }

    pub fn active(&self) -> bool {
        self.active.is_some()
    }

    pub fn is_selector_surface(&self, surface: &wayland_client::protocol::wl_surface::WlSurface) -> bool {
        self.selector.layer.wl_surface() == surface
    }

    pub fn is_selector_layer(&self, layer: &smithay_client_toolkit::shell::wlr_layer::LayerSurface) -> bool {
        &self.selector.layer == layer
    }
}

pub fn save(image: &CaptureImage) {
    let _ = storage::save(&image.png);
}

fn bind_manager(globals: &GlobalList, qh: &QueueHandle<App>) -> Result<ZwlrScreencopyManagerV1, BindError> {
    globals.bind(qh, 1..=3, ())
}
