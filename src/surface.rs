use slint::PlatformError;
use smithay_client_toolkit::compositor::CompositorState;
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shell::wlr_layer::{LayerShell, LayerSurface};
use smithay_client_toolkit::shell::xdg::XdgShell;
use smithay_client_toolkit::shell::xdg::window::Window;
use wayland_client::QueueHandle;
use wayland_client::globals::GlobalList;
use wayland_client::protocol::wl_surface::WlSurface;

use crate::wayland::AppState;
use crate::{LayerShellOptions, Options, WindowOptions};

/// Wayland surface
#[derive(Clone)]
pub(crate) enum Surface {
    Layer(LayerSurface),
    Window(Window),
}

impl Surface {
    /// Create a new surface based on the options provided. Also returns a fallback size.
    pub(crate) fn new(
        options: &Options,
        globals: &GlobalList,
        qh: &QueueHandle<AppState>,
        compositor: &CompositorState,
    ) -> Result<(Self, (u32, u32)), PlatformError> {
        match options {
            Options::Layer(options) => Self::new_layer(options, globals, qh, compositor),
            Options::Window(options) => Self::new_window(options, globals, qh, compositor),
        }
    }

    fn new_layer(
        options: &LayerShellOptions,
        globals: &GlobalList,
        qh: &QueueHandle<AppState>,
        compositor: &CompositorState,
    ) -> Result<(Self, (u32, u32)), PlatformError> {
        let layer_shell = LayerShell::bind(globals, qh).map_err(|e| {
            PlatformError::Other(format!(
                "wlr-layer-shell is not available on this compositor: {e}"
            ))
        })?;
        let wl_surface = compositor.create_surface(qh);
        let layer = layer_shell.create_layer_surface(
            qh,
            wl_surface,
            options.layer,
            Some(options.namespace),
            None,
        );
        layer.set_anchor(options.anchor);
        layer.set_keyboard_interactivity(options.keyboard_interactivity);
        let (top, right, bottom, left) = options.margin;
        layer.set_margin(top, right, bottom, left);
        if let Some(zone) = options.exclusive_zone {
            layer.set_exclusive_zone(zone);
        }
        let fallback_size = (options.size.0.unwrap_or(256), options.size.1.unwrap_or(32));
        layer.set_size(options.size.0.unwrap_or(0), options.size.1.unwrap_or(0));
        Ok((Self::Layer(layer), fallback_size))
    }

    fn new_window(
        options: &WindowOptions,
        globals: &GlobalList,
        qh: &QueueHandle<AppState>,
        compositor: &CompositorState,
    ) -> Result<(Self, (u32, u32)), PlatformError> {
        let xdg_shell = XdgShell::bind(globals, qh).map_err(|e| {
            PlatformError::Other(format!(
                "xdg_wm_base is not available on this compositor: {e}"
            ))
        })?;
        let wl_surface = compositor.create_surface(qh);
        let window = xdg_shell.create_window(wl_surface, options.decorations, qh);
        window.set_title(options.title);
        window.set_app_id(options.app_id);
        let fallback_size = (options.size.0.unwrap_or(800), options.size.1.unwrap_or(600));
        Ok((Self::Window(window), fallback_size))
    }
}

impl WaylandSurface for Surface {
    fn wl_surface(&self) -> &WlSurface {
        match self {
            Surface::Layer(layer) => layer.wl_surface(),
            Surface::Window(window) => window.wl_surface(),
        }
    }
}
