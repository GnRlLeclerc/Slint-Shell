use smithay_client_toolkit::output::OutputState;
use smithay_client_toolkit::registry::{ProvidesRegistryState, RegistryState};
use smithay_client_toolkit::registry_handlers;
use smithay_client_toolkit::seat::SeatState;

use super::AppState;

impl ProvidesRegistryState for AppState {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry
    }
    registry_handlers![OutputState, SeatState];
}
