use std::rc::{Rc, Weak};

use calloop::LoopHandle;
use smithay_client_toolkit::output::OutputState;
use smithay_client_toolkit::registry::RegistryState;
use smithay_client_toolkit::seat::SeatState;
use smithay_client_toolkit::shm::Shm;
use wayland_client::protocol::{wl_keyboard::WlKeyboard, wl_pointer::WlPointer};

use crate::window_adapter::LayerWindowAdapter;

/// Shared app state
pub(crate) struct AppState {
    pub registry: RegistryState,
    pub seats: SeatState,
    pub outputs: OutputState,
    pub shm: Shm,
    pub window_adapter: Weak<LayerWindowAdapter>,

    /// Configured logical size
    pub width: u32,
    pub height: u32,
    pub exit: bool,

    pub keyboard: Option<WlKeyboard>,
    /// Exclusively used to send key repeat events
    pub loop_handle: LoopHandle<'static, AppState>,
    pub pointer: Option<WlPointer>,
}

impl AppState {
    pub(super) fn window_adapter(&self) -> Option<Rc<LayerWindowAdapter>> {
        self.window_adapter.upgrade()
    }
}
