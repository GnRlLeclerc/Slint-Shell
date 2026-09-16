use smithay_client_toolkit::output::{OutputHandler, OutputState};
use wayland_client::protocol::wl_output::WlOutput;
use wayland_client::{Connection, QueueHandle};

use super::AppState;

impl OutputHandler for AppState {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.outputs
    }
    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: WlOutput) {}
    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: WlOutput) {}
    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: WlOutput) {}
}
