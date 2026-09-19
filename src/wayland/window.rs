use smithay_client_toolkit::shell::xdg::window::{Window, WindowConfigure, WindowHandler};
use wayland_client::{Connection, QueueHandle};

use super::AppState;

impl WindowHandler for AppState {
    fn request_close(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &Window) {
        self.exit = true;
    }

    fn configure(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &Window,
        configure: WindowConfigure,
        _: u32,
    ) {
        let new_size = (
            configure.new_size.0.map_or(0, |w| w.get()),
            configure.new_size.1.map_or(0, |h| h.get()),
        );
        if new_size.0 > 0 {
            self.width = new_size.0;
        }
        if new_size.1 > 0 {
            self.height = new_size.1;
        }
        if let Some(wa) = self.window_adapter() {
            wa.configure(new_size);
        }
    }
}
