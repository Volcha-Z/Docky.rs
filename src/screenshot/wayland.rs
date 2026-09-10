use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols_wlr::screencopy::v1::client::{
    zwlr_screencopy_frame_v1::{self, ZwlrScreencopyFrameV1},
    zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1,
};

use crate::app::App;

wayland_client::delegate_noop!(App: ZwlrScreencopyManagerV1);

impl Dispatch<ZwlrScreencopyFrameV1, ()> for App {
    fn event(state: &mut Self, _: &ZwlrScreencopyFrameV1, event: zwlr_screencopy_frame_v1::Event, _: &(), _: &Connection, qh: &QueueHandle<Self>) {
        match event {
            zwlr_screencopy_frame_v1::Event::Buffer { format, width, height, stride } => {
                if let Some(screenshot) = &mut state.screenshot {
                    screenshot.set_spec(format, width, height, stride);
                }
            }
            zwlr_screencopy_frame_v1::Event::BufferDone => state.prepare_screenshot(),
            zwlr_screencopy_frame_v1::Event::Flags { flags } => {
                if let Some(screenshot) = &mut state.screenshot {
                    screenshot.set_flags(flags);
                }
            }
            zwlr_screencopy_frame_v1::Event::Ready { .. } => state.finish_screenshot(qh),
            zwlr_screencopy_frame_v1::Event::Failed => {
                if let Some(screenshot) = &mut state.screenshot {
                    screenshot.fail();
                }
            }
            _ => {}
        }
    }
}
