use std::cell::Cell;
use std::sync::Arc;

use i_slint_renderer_skia::SkiaRenderer;
use slint::platform::Renderer;
use slint::{PhysicalSize, PlatformError, Window};
use smithay_client_toolkit::compositor::FrameCallbackData;
use smithay_client_toolkit::shell::WaylandSurface;
use wayland_client::{Connection, QueueHandle};

use crate::surface::Surface;
use crate::wayland::AppState;

use super::{RenderBackend, RenderOutcome, WaylandHandle};

/// Skia renderer backend.
pub(crate) struct SkiaRenderBackend {
    renderer: SkiaRenderer,
    surface: Surface,
    qh: QueueHandle<AppState>,
    size: Cell<PhysicalSize>,
}

impl SkiaRenderBackend {
    /// Initialize the Skia backend with a given renderer, and binds it to the surface.
    pub(super) fn with_renderer(
        renderer: SkiaRenderer,
        surface: Surface,
        qh: QueueHandle<AppState>,
        connection: &Connection,
        size: PhysicalSize,
    ) -> Result<Self, PlatformError> {
        let handle = Arc::new(WaylandHandle::new(connection, &surface)?);
        renderer.set_window_handle(handle.clone(), handle.clone(), size, None, true)?;
        Ok(Self {
            renderer,
            surface,
            qh,
            size: Cell::new(size),
        })
    }
}

impl RenderBackend for SkiaRenderBackend {
    fn as_core_renderer(&self) -> &dyn Renderer {
        &self.renderer
    }

    fn render_and_present(&self, _: &Window) -> Result<RenderOutcome, PlatformError> {
        let size = self.size.get();
        if size.width == 0 || size.height == 0 {
            return Ok(RenderOutcome::Skipped);
        }
        let wl_surface = self.surface.wl_surface();
        wl_surface.frame(&self.qh, FrameCallbackData(wl_surface.clone()));
        let _ = self.renderer.render()?;
        Ok(RenderOutcome::Presented)
    }

    /// Just update the size tracker. SkiaRenderer owns the surface it renders to,
    /// and will have resized it already when directly called by Slint.
    fn resize(&self, size: PhysicalSize) -> Result<(), PlatformError> {
        self.size.set(size);
        Ok(())
    }
}
