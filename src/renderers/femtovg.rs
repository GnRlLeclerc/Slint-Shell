use std::cell::Cell;

use i_slint_renderer_femtovg::{FemtoVGRenderer, GraphicsBackend};
use slint::platform::Renderer;
use slint::{PhysicalSize, PlatformError, Window};
use smithay_client_toolkit::compositor::FrameCallbackData;
use smithay_client_toolkit::shell::WaylandSurface;
use wayland_client::QueueHandle;

use crate::surface::Surface;
use crate::wayland::AppState;

use super::{RenderBackend, RenderOutcome};

/// FemtoVG renderer backend.
pub(crate) struct FemtoVGRenderBackend<B: GraphicsBackend> {
    renderer: FemtoVGRenderer<B>,
    surface: Surface,
    qh: QueueHandle<AppState>,
    size: Cell<PhysicalSize>,
}

impl<B: GraphicsBackend> FemtoVGRenderBackend<B> {
    /// Initialize the FemtoVG backend with a renderer that is already bound to the surface.
    pub(super) fn with_renderer(
        renderer: FemtoVGRenderer<B>,
        surface: Surface,
        qh: QueueHandle<AppState>,
        size: PhysicalSize,
    ) -> Self {
        Self {
            renderer,
            surface,
            qh,
            size: Cell::new(size),
        }
    }
}

impl<B: GraphicsBackend> RenderBackend for FemtoVGRenderBackend<B> {
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
        self.renderer.render()?;
        Ok(RenderOutcome::Presented)
    }

    /// Just update the size tracker. FemtoVGRenderer owns the surface it renders to,
    /// and will have resized it already when directly called by Slint.
    fn resize(&self, size: PhysicalSize) -> Result<(), PlatformError> {
        self.size.set(size);
        Ok(())
    }
}
