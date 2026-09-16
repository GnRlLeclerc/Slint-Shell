use smithay_client_toolkit::shm::{Shm, ShmHandler};

use super::AppState;

impl ShmHandler for AppState {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}
