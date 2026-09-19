mod compositor;
mod keyboard;
mod layer_shell;
mod output;
mod pointer;
mod registry;
mod seat;
mod shm;
mod state;
mod window;

pub(crate) use state::AppState;

smithay_client_toolkit::delegate_dispatch2!(AppState);
smithay_client_toolkit::delegate_registry!(AppState);
