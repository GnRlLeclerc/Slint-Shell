use smithay_client_toolkit::shell::wlr_layer::{
    LayerShellHandler, LayerSurface, LayerSurfaceConfigure,
};
use wayland_client::{Connection, QueueHandle};

use super::AppState;

impl LayerShellHandler for AppState {
    fn closed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &LayerSurface) {
        self.exit = true;
    }

    fn configure(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _: u32,
    ) {
        if configure.new_size.0 > 0 {
            self.width = configure.new_size.0;
        }
        if configure.new_size.1 > 0 {
            self.height = configure.new_size.1;
        }
        if let Some(wa) = self.window_adapter() {
            wa.configure(configure.new_size);
        }
    }
}
