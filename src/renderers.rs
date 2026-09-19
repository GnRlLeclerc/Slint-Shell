#[cfg(feature = "renderer-femtovg")]
mod femtovg;
#[cfg(any(feature = "renderer-skia", feature = "renderer-femtovg"))]
mod opengl;
#[cfg(feature = "renderer-skia")]
mod skia;
#[cfg(feature = "renderer-software")]
mod software;
#[cfg(any(feature = "renderer-skia-wgpu", feature = "renderer-femtovg-wgpu"))]
mod wgpu;

#[cfg(feature = "renderer-femtovg")]
use std::num::NonZeroU32;
#[cfg(any(feature = "renderer-skia", feature = "renderer-femtovg"))]
use std::ptr::NonNull;

#[cfg(any(feature = "renderer-skia", feature = "renderer-femtovg"))]
use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle,
    RawWindowHandle, WaylandDisplayHandle, WaylandWindowHandle, WindowHandle,
};
use slint::platform::Renderer;
use slint::{PhysicalSize, PlatformError, Window};
use smithay_client_toolkit::shm::Shm;
use wayland_client::{Connection, QueueHandle};

#[cfg(feature = "renderer-software")]
use crate::renderers::software::SoftwareRenderBackend;
use crate::surface::Surface;
use crate::wayland::AppState;

pub(crate) enum RenderOutcome {
    /// Rendered and presented: a `wl_surface.frame()` callback was requested
    Presented,
    Skipped,
}

/// Underlying graphics API for not-software renderers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Graphics {
    #[default]
    OpenGL,
    Wgpu,
}

/// Available slint renderers (locked behind features)
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RendererKind {
    #[default]
    Software,
    Skia(Graphics),
    FemtoVG(Graphics),
}

/// Environment variable the examples (and any consumer calling [`RendererKind::from_env`]) read
/// to pick a renderer without a rebuild.
pub const RENDERER_ENV_VAR: &str = "SLINT_SHELL_RENDERER";

impl std::str::FromStr for RendererKind {
    type Err = String;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        match name.to_ascii_lowercase().as_str() {
            "software" | "sw" => Ok(Self::Software),
            "skia" | "skia-opengl" => Ok(Self::Skia(Graphics::OpenGL)),
            "skia-wgpu" => Ok(Self::Skia(Graphics::Wgpu)),
            "femtovg" | "femtovg-opengl" => Ok(Self::FemtoVG(Graphics::OpenGL)),
            "femtovg-wgpu" => Ok(Self::FemtoVG(Graphics::Wgpu)),
            other => Err(format!(
                "unknown renderer {other:?}, expected software, skia, skia-wgpu, femtovg or \
                 femtovg-wgpu"
            )),
        }
    }
}

impl RendererKind {
    /// Load the renderer kind from the environment
    pub fn from_env() -> Result<Self, String> {
        match std::env::var(RENDERER_ENV_VAR) {
            Ok(name) => name.parse(),
            Err(_) => Ok(Self::default()),
        }
    }
}

/// Backend renderer abstraction trait
pub(crate) trait RenderBackend {
    fn as_core_renderer(&self) -> &dyn Renderer;

    /// Render the current frame and commit it to the surface.
    fn render_and_present(&self, window: &Window) -> Result<RenderOutcome, PlatformError>;

    fn resize(&self, size: PhysicalSize) -> Result<(), PlatformError>;
}

#[allow(unused_variables)]
pub(crate) fn new_backend(
    kind: RendererKind,
    surface: Surface,
    qh: QueueHandle<AppState>,
    connection: &Connection,
    shm: &Shm,
    size: PhysicalSize,
) -> Result<Box<dyn RenderBackend>, PlatformError> {
    match kind {
        #[cfg(feature = "renderer-software")]
        RendererKind::Software => Ok(Box::new(SoftwareRenderBackend::new(
            surface, qh, shm, size,
        )?)),
        #[cfg(feature = "renderer-skia")]
        RendererKind::Skia(Graphics::OpenGL) => {
            Ok(Box::new(opengl::skia(surface, qh, connection, size)?))
        }
        #[cfg(feature = "renderer-skia-wgpu")]
        RendererKind::Skia(Graphics::Wgpu) => {
            Ok(Box::new(wgpu::skia(surface, qh, connection, size)?))
        }
        #[cfg(feature = "renderer-femtovg")]
        RendererKind::FemtoVG(Graphics::OpenGL) => {
            Ok(Box::new(opengl::femtovg(surface, qh, connection, size)?))
        }
        #[cfg(feature = "renderer-femtovg-wgpu")]
        RendererKind::FemtoVG(Graphics::Wgpu) => {
            Ok(Box::new(wgpu::femtovg(surface, qh, connection, size)?))
        }
        #[allow(unreachable_patterns)]
        kind => Err(PlatformError::Other(format!(
            "the {kind:?} renderer is not compiled in; rebuild slint-shell with its cargo feature"
        ))),
    }
}

/// Map an error to a PlatformError (required for FemtoVG)
#[cfg(feature = "renderer-femtovg")]
pub(crate) fn err<E: std::fmt::Display>(what: &'static str) -> impl FnOnce(E) -> PlatformError {
    move |e| PlatformError::Other(format!("{what}: {e}"))
}

/// Clamp a size to non-zero values (required by FemtoVG)
#[cfg(feature = "renderer-femtovg")]
pub(crate) fn non_zero(size: PhysicalSize) -> (NonZeroU32, NonZeroU32) {
    (
        NonZeroU32::new(size.width).unwrap_or(NonZeroU32::MIN),
        NonZeroU32::new(size.height).unwrap_or(NonZeroU32::MIN),
    )
}

/// The `wl_display`/`wl_surface` pair, handed to the GPU renderers as raw window handles so they
/// can bind an EGL surface to it. Needs `wayland-client/system` (libwayland) for the pointers.
#[cfg(any(feature = "renderer-skia", feature = "renderer-femtovg"))]
#[derive(Clone, Copy)]
pub(crate) struct WaylandHandle {
    display: NonNull<std::ffi::c_void>,
    surface: NonNull<std::ffi::c_void>,
}

// SAFETY: both are plain libwayland object pointers, which are not tied to any thread. The
// handle borrows them for as long as the connection and the surface it was built from live,
// and both outlive the renderer holding this.
#[cfg(any(feature = "renderer-skia", feature = "renderer-femtovg"))]
unsafe impl Send for WaylandHandle {}
#[cfg(any(feature = "renderer-skia", feature = "renderer-femtovg"))]
unsafe impl Sync for WaylandHandle {}

#[cfg(any(feature = "renderer-skia", feature = "renderer-femtovg"))]
impl WaylandHandle {
    pub(crate) fn new(connection: &Connection, surface: &Surface) -> Result<Self, PlatformError> {
        use smithay_client_toolkit::shell::WaylandSurface;
        use wayland_client::Proxy;

        let display = NonNull::new(connection.backend().display_ptr().cast())
            .ok_or_else(|| PlatformError::Other("the wl_display pointer is null".into()))?;
        let surface = NonNull::new(surface.wl_surface().id().as_ptr().cast())
            .ok_or_else(|| PlatformError::Other("the wl_surface pointer is null".into()))?;
        Ok(Self { display, surface })
    }
}

#[cfg(any(feature = "renderer-skia", feature = "renderer-femtovg"))]
impl HasDisplayHandle for WaylandHandle {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        let handle = WaylandDisplayHandle::new(self.display);
        // SAFETY: the pointer is valid for the lifetime of the borrow, see the Send/Sync note.
        Ok(unsafe { DisplayHandle::borrow_raw(RawDisplayHandle::Wayland(handle)) })
    }
}

#[cfg(any(feature = "renderer-skia", feature = "renderer-femtovg"))]
impl HasWindowHandle for WaylandHandle {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        let handle = WaylandWindowHandle::new(self.surface);
        // SAFETY: the pointer is valid for the lifetime of the borrow, see the Send/Sync note.
        Ok(unsafe { WindowHandle::borrow_raw(RawWindowHandle::Wayland(handle)) })
    }
}
