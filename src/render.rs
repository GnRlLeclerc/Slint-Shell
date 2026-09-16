mod software;

pub(crate) use software::SoftwareRenderBackend;

use slint::platform::Renderer;
use slint::{PhysicalSize, PlatformError, Window};

pub(crate) enum RenderOutcome {
    /// Rendered and presented: a `wl_surface.frame()` callback was requested
    Presented,
    Skipped,
}

/// Backend renderer abstraction trait
pub(crate) trait RenderBackend {
    fn as_core_renderer(&self) -> &dyn Renderer;

    /// Render the current frame and commit it to the layer surface.
    fn render_and_present(&self, window: &Window) -> Result<RenderOutcome, PlatformError>;

    fn resize(&self, size: PhysicalSize) -> Result<(), PlatformError>;
}
