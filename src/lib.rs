#[cfg(not(any(
    feature = "renderer-software",
    feature = "renderer-skia",
    feature = "renderer-femtovg"
)))]
compile_error!(
    "slint-shell needs at least one renderer: enable renderer-software, renderer-skia \
     (-wgpu) or renderer-femtovg (-wgpu)"
);

mod platform;
mod renderers;
mod surface;
mod wayland;
mod window_adapter;

pub use renderers::{Graphics, RENDERER_ENV_VAR, RendererKind};
pub use smithay_client_toolkit::shell::wlr_layer::{Anchor, KeyboardInteractivity, Layer};
pub use smithay_client_toolkit::shell::xdg::window::WindowDecorations;

use crate::platform::WaylandPlatform;

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
    pub renderer: RendererKind,
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
            renderer: RendererKind::default(),
        }
    }
}

/// Regular xdg window configuration options
pub struct WindowOptions {
    pub title: &'static str,
    pub app_id: &'static str,
    /// Desired size in logical pixels.
    /// None defaults to compositor preferrence.
    pub size: (Option<u32>, Option<u32>),
    pub decorations: WindowDecorations,
    pub renderer: RendererKind,
}

impl Default for WindowOptions {
    fn default() -> Self {
        Self {
            title: "slint-shell window",
            app_id: "slint-shell",
            size: (Some(800), Some(600)),
            decorations: WindowDecorations::ServerDefault,
            renderer: RendererKind::default(),
        }
    }
}

/// Wayland backend options (layer or window)
pub enum Options {
    Layer(LayerShellOptions),
    Window(WindowOptions),
}

impl Options {
    pub(crate) fn renderer(&self) -> RendererKind {
        match self {
            Options::Layer(options) => options.renderer,
            Options::Window(options) => options.renderer,
        }
    }
}

/// Set the slint platform to the Wayland backend with the given options.
pub fn init(options: Options) -> Result<(), slint::PlatformError> {
    let backend = WaylandPlatform::new(options)?;
    slint::platform::set_platform(Box::new(backend))
        .map_err(|e| slint::PlatformError::Other(e.to_string()))
}
