mod platform;
mod render;
mod wayland;
mod window_adapter;

pub use smithay_client_toolkit::shell::wlr_layer::{Anchor, KeyboardInteractivity, Layer};

/// Layer shell configuration options
pub struct LayerShellOptions {
    pub namespace: &'static str,
    pub layer: Layer,
    pub anchor: Anchor,
    /// (top, right, bottom, left), in logical pixels.
    pub margin: (i32, i32, i32, i32),
    pub exclusive_zone: Option<i32>,
    pub keyboard_interactivity: KeyboardInteractivity,
    /// Desired size in logical pixels.
    /// None defaults to anchored sides / compositor preferrence.
    pub size: (Option<u32>, Option<u32>),
}

impl Default for LayerShellOptions {
    fn default() -> Self {
        Self {
            namespace: "slint-shell",
            layer: Layer::Top,
            anchor: Anchor::empty(),
            margin: (0, 0, 0, 0),
            exclusive_zone: None,
            keyboard_interactivity: KeyboardInteractivity::None,
            size: (Some(256), Some(256)),
        }
    }
}

/// Set the platform to use for the next slint windows to be created,
/// with the specified options.
pub fn init(options: LayerShellOptions) -> Result<(), slint::PlatformError> {
    let backend = platform::WaylandPlatform::new(options)?;
    slint::platform::set_platform(Box::new(backend))
        .map_err(|e| slint::PlatformError::Other(e.to_string()))
}
