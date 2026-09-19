use slint::{PhysicalSize, PlatformError};
use wayland_client::{Connection, QueueHandle};

use crate::surface::Surface;
use crate::wayland::AppState;

/// Skia backend with WGPU
#[cfg(feature = "renderer-skia-wgpu")]
pub(super) fn skia(
    surface: Surface,
    qh: QueueHandle<AppState>,
    connection: &Connection,
    size: PhysicalSize,
) -> Result<super::skia::SkiaRenderBackend, PlatformError> {
    use crate::renderers::skia::SkiaRenderBackend;
    use i_slint_renderer_skia::{SkiaRenderer, SkiaSharedContext};

    let renderer = SkiaRenderer::default_wgpu_30(&SkiaSharedContext::default());
    SkiaRenderBackend::with_renderer(renderer, surface, qh, connection, size)
}

/// FemtoVG with WGPU.
#[cfg(feature = "renderer-femtovg-wgpu")]
pub(super) fn femtovg(
    surface: Surface,
    qh: QueueHandle<AppState>,
    connection: &Connection,
    size: PhysicalSize,
) -> Result<
    super::femtovg::FemtoVGRenderBackend<i_slint_renderer_femtovg::wgpu::WGPUBackend>,
    PlatformError,
> {
    use i_slint_renderer_femtovg::wgpu::WGPUBackend;
    use i_slint_renderer_femtovg::{FemtoVGRenderer, FemtoVGRendererExt};
    use wgpu::DisplayAndWindowHandle;

    use super::femtovg::FemtoVGRenderBackend;
    use super::{WaylandHandle, err};

    let handle = WaylandHandle::new(connection, &surface)?;
    let target = Box::new(handle) as Box<dyn DisplayAndWindowHandle>;

    let renderer = FemtoVGRenderer::<WGPUBackend>::new_suspended();
    renderer
        .set_surface(target, size, None, true)
        .map_err(err("failed to create the WGPU surface"))?;

    Ok(FemtoVGRenderBackend::with_renderer(
        renderer, surface, qh, size,
    ))
}
