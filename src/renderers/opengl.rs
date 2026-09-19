#[cfg(feature = "renderer-femtovg")]
use std::error::Error;
#[cfg(feature = "renderer-femtovg")]
use std::ffi::{CStr, c_void};
#[cfg(feature = "renderer-femtovg")]
use std::num::NonZeroU32;
#[cfg(feature = "renderer-femtovg")]
use std::rc::Rc;

#[cfg(feature = "renderer-femtovg")]
use glutin::config::{ConfigTemplateBuilder, GlConfig};
#[cfg(feature = "renderer-femtovg")]
use glutin::context::{
    ContextApi, ContextAttributesBuilder, NotCurrentGlContext, PossiblyCurrentContext,
    PossiblyCurrentGlContext,
};
#[cfg(feature = "renderer-femtovg")]
use glutin::display::{Display, DisplayApiPreference, GetGlDisplay, GlDisplay};
#[cfg(feature = "renderer-femtovg")]
use glutin::surface::{
    GlSurface, Surface as GlSurfaceHandle, SurfaceAttributesBuilder, WindowSurface,
};
#[cfg(feature = "renderer-femtovg")]
use i_slint_renderer_femtovg::opengl::OpenGLInterface;
#[cfg(feature = "renderer-femtovg")]
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use slint::{PhysicalSize, PlatformError};
use wayland_client::{Connection, QueueHandle};

use crate::surface::Surface;
use crate::wayland::AppState;

#[cfg(feature = "renderer-femtovg")]
use super::WaylandHandle;

/// Skia backend with OpenGL
#[cfg(feature = "renderer-skia")]
pub(super) fn skia(
    surface: Surface,
    qh: QueueHandle<AppState>,
    connection: &Connection,
    size: PhysicalSize,
) -> Result<super::skia::SkiaRenderBackend, PlatformError> {
    use crate::renderers::skia::SkiaRenderBackend;
    use i_slint_renderer_skia::{SkiaRenderer, SkiaSharedContext};

    let renderer = SkiaRenderer::default_opengl(&SkiaSharedContext::default());
    SkiaRenderBackend::with_renderer(renderer, surface, qh, connection, size)
}

/// FemtoVG with OpenGL, using glutin
#[cfg(feature = "renderer-femtovg")]
pub(super) fn femtovg(
    surface: Surface,
    qh: QueueHandle<AppState>,
    connection: &Connection,
    size: PhysicalSize,
) -> Result<
    super::femtovg::FemtoVGRenderBackend<i_slint_renderer_femtovg::opengl::OpenGLBackend>,
    PlatformError,
> {
    use i_slint_renderer_femtovg::{
        FemtoVGOpenGLRenderer, FemtoVGOpenGLRendererExt, FemtoVGRendererExt,
    };

    use super::femtovg::FemtoVGRenderBackend;

    let handle = WaylandHandle::new(connection, &surface)?;
    let context = GlContext::new(&handle, size)?;
    let renderer = FemtoVGOpenGLRenderer::new_suspended();
    renderer.set_opengl_context(SharedGlContext(Rc::new(context)))?;
    Ok(FemtoVGRenderBackend::with_renderer(
        renderer, surface, qh, size,
    ))
}

/// EGL context for femtovg.
#[cfg(feature = "renderer-femtovg")]
struct GlContext {
    context: PossiblyCurrentContext,
    surface: GlSurfaceHandle<WindowSurface>,
}

#[cfg(feature = "renderer-femtovg")]
impl GlContext {
    fn new(handle: &WaylandHandle, size: PhysicalSize) -> Result<Self, PlatformError> {
        use super::{err, non_zero};

        let raw_display = handle
            .display_handle()
            .map_err(err("no wayland display handle"))?
            .as_raw();
        let raw_window = handle
            .window_handle()
            .map_err(err("no wayland window handle"))?
            .as_raw();

        // SAFETY: the handles come from a live wl_display/wl_surface pair, see `WaylandHandle`.
        let display = unsafe { Display::new(raw_display, DisplayApiPreference::Egl) }
            .map_err(err("failed to create the EGL display"))?;

        let template = ConfigTemplateBuilder::new().with_alpha_size(8).build();
        // SAFETY: the template carries no window handle, so there is nothing to outlive.
        let config = unsafe { display.find_configs(template) }
            .map_err(err("failed to enumerate EGL configs"))?
            .max_by_key(|config| config.alpha_size())
            .ok_or_else(|| PlatformError::Other("no EGL config with an alpha channel".into()))?;

        let attributes = ContextAttributesBuilder::new()
            .with_context_api(ContextApi::Gles(None))
            .build(Some(raw_window));
        // SAFETY: `raw_window` refers to the wl_surface this renderer is bound to.
        let context = unsafe { display.create_context(&config, &attributes) }
            .map_err(err("failed to create the GLES context"))?;

        let (width, height) = non_zero(size);
        let surface_attributes =
            SurfaceAttributesBuilder::<WindowSurface>::new().build(raw_window, width, height);
        // SAFETY: same wl_surface, and glutin builds the wl_egl_window from it.
        let surface = unsafe { display.create_window_surface(&config, &surface_attributes) }
            .map_err(err("failed to create the EGL window surface"))?;

        let context = context
            .make_current(&surface)
            .map_err(err("failed to make the GLES context current"))?;
        Ok(Self { context, surface })
    }
}

/// Newtype to allow OpenGLInterface trait implementation.
#[cfg(feature = "renderer-femtovg")]
struct SharedGlContext(Rc<GlContext>);

// SAFETY: every method forwards to glutin.
#[cfg(feature = "renderer-femtovg")]
unsafe impl OpenGLInterface for SharedGlContext {
    fn ensure_current(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        if !self.0.context.is_current() {
            self.0.context.make_current(&self.0.surface)?;
        }
        Ok(())
    }

    fn swap_buffers(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.0.surface.swap_buffers(&self.0.context)?;
        Ok(())
    }

    fn resize(
        &self,
        width: NonZeroU32,
        height: NonZeroU32,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.0.surface.resize(&self.0.context, width, height);
        Ok(())
    }

    fn get_proc_address(&self, name: &CStr) -> *const c_void {
        self.0.context.display().get_proc_address(name)
    }
}
