use smithay_client_toolkit::{
    compositor::{CompositorState, Region},
    shell::{
        wlr_layer::{KeyboardInteractivity, LayerSurface},
        WaylandSurface,
    },
    shm::{
        slot::{Buffer, SlotPool},
        Shm,
    },
};
use wayland_client::protocol::wl_shm;

#[derive(Clone, Copy)]
pub struct Selection {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

pub struct RegionSelector {
    pub layer: LayerSurface,
    pub mapped: bool,
    pub configured: bool,
    pub dragging: bool,
    pub frame_pending: bool,
    pub redraw_pending: bool,
    width: u32,
    height: u32,
    scale: u32,
    start: (f64, f64),
    current: (f64, f64),
    drawn: Option<Selection>,
    pool: Option<SlotPool>,
    buffer: Option<Buffer>,
}

impl RegionSelector {
    pub fn new(layer: LayerSurface) -> Self {
        Self {
            layer,
            mapped: false,
            configured: false,
            dragging: false,
            frame_pending: false,
            redraw_pending: false,
            width: 1,
            height: 1,
            scale: 1,
            start: (0.0, 0.0),
            current: (0.0, 0.0),
            drawn: None,
            pool: None,
            buffer: None,
        }
    }

    pub fn open(&mut self, shm: &Shm) {
        if self.mapped {
            return;
        }
        self.mapped = true;
        self.dragging = false;
        self.drawn = None;
        self.layer.set_keyboard_interactivity(KeyboardInteractivity::Exclusive);
        self.layer.wl_surface().set_input_region(None);
        if self.configured {
            self.create_buffer(shm);
            self.draw();
        } else {
            self.layer.commit();
        }
    }

    pub fn configure(&mut self, width: u32, height: u32, shm: &Shm) {
        self.width = width.max(1);
        self.height = height.max(1);
        self.scale = 1;
        self.configured = true;
        if !self.mapped {
            return;
        }
        self.create_buffer(shm);
        self.draw();
    }

    pub fn press(&mut self, position: (f64, f64)) {
        self.start = position;
        self.current = position;
        self.dragging = true;
        self.redraw_pending = true;
    }

    pub fn motion(&mut self, position: (f64, f64)) {
        if !self.dragging {
            return;
        }
        self.current = position;
        self.redraw_pending = true;
    }

    pub fn release(&mut self, position: (f64, f64)) -> Option<Selection> {
        if !self.dragging {
            return None;
        }
        self.current = position;
        let selection = self.selection()?;
        self.dragging = false;
        (selection.width >= 3 && selection.height >= 3).then_some(selection)
    }

    pub fn draw(&mut self) {
        let selection = self.selection();
        let Some(pool) = &mut self.pool else {
            return;
        };
        let Some(buffer) = &self.buffer else {
            return;
        };
        let Some(canvas) = buffer.canvas(pool) else {
            self.redraw_pending = true;
            return;
        };
        if let Some(previous) = self.drawn {
            draw_border(canvas, previous, self.width, self.height, self.scale, [0, 0, 0, 64]);
        }
        if let Some(current) = selection {
            draw_border(canvas, current, self.width, self.height, self.scale, [235, 235, 235, 255]);
        }
        self.drawn = selection;
        self.redraw_pending = false;
        self.layer.wl_surface().damage_buffer(0, 0, (self.width * self.scale) as i32, (self.height * self.scale) as i32);
        if buffer.attach_to(self.layer.wl_surface()).is_ok() {
            self.layer.commit();
        }
    }

    pub fn close(&mut self, compositor: &CompositorState) {
        if !self.mapped {
            return;
        }
        self.mapped = false;
        self.configured = false;
        self.dragging = false;
        self.frame_pending = false;
        self.redraw_pending = false;
        self.layer.set_keyboard_interactivity(KeyboardInteractivity::None);
        if let Ok(region) = Region::new(compositor) {
            self.layer.wl_surface().set_input_region(Some(region.wl_region()));
        }
        self.layer.wl_surface().attach(None, 0, 0);
        self.layer.commit();
        self.buffer = None;
        self.pool = None;
    }

    fn create_buffer(&mut self, shm: &Shm) {
        let buffer_width = self.width * self.scale;
        let buffer_height = self.height * self.scale;
        let length = buffer_width as usize * buffer_height as usize * 4;
        let Ok(mut pool) = SlotPool::new(length, shm) else {
            return;
        };
        let Ok((buffer, canvas)) = pool.create_buffer(buffer_width as i32, buffer_height as i32, buffer_width as i32 * 4, wl_shm::Format::Argb8888) else {
            return;
        };
        for pixel in canvas.chunks_exact_mut(4) {
            pixel.copy_from_slice(&[0, 0, 0, 64]);
        }
        self.layer.wl_surface().set_buffer_scale(self.scale as i32);
        self.pool = Some(pool);
        self.buffer = Some(buffer);
    }

    fn selection(&self) -> Option<Selection> {
        self.dragging.then(|| {
            let x = self.start.0.min(self.current.0).floor().max(0.0) as i32;
            let y = self.start.1.min(self.current.1).floor().max(0.0) as i32;
            let right = self.start.0.max(self.current.0).ceil().min(self.width as f64) as i32;
            let bottom = self.start.1.max(self.current.1).ceil().min(self.height as f64) as i32;
            Selection {
                x,
                y,
                width: right - x,
                height: bottom - y,
            }
        })
    }
}

fn draw_border(canvas: &mut [u8], rect: Selection, width: u32, height: u32, scale: u32, color: [u8; 4]) {
    let width = width * scale;
    let height = height * scale;
    let left = rect.x.max(0) as u32 * scale;
    let top = rect.y.max(0) as u32 * scale;
    let right = ((rect.x + rect.width).max(0) as u32 * scale).min(width);
    let bottom = ((rect.y + rect.height).max(0) as u32 * scale).min(height);
    let thickness = 2 * scale;
    for y in top..bottom {
        for x in left..right {
            if x < left + thickness || x + thickness >= right || y < top + thickness || y + thickness >= bottom {
                let offset = ((y * width + x) * 4) as usize;
                canvas[offset..offset + 4].copy_from_slice(&color);
            }
        }
    }
}
