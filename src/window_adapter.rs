use std::cell::Cell;
use std::rc::{Rc, Weak};

use slint::platform::{Renderer, WindowAdapter, WindowEvent};
use slint::{LogicalSize, PhysicalSize, PlatformError, Window, WindowSize};
use smithay_client_toolkit::compositor::FrameCallbackData;
use smithay_client_toolkit::shell::WaylandSurface;
use wayland_client::QueueHandle;
use wayland_client::protocol::wl_surface::WlSurface;

use crate::renderers::{RenderBackend, RenderOutcome};
use crate::surface::Surface;
use crate::wayland::AppState;

/// Window adapter for a single Wayland surface.
pub(crate) struct ShellWindowAdapter {
    window: Window,
    surface: Surface,
    qh: QueueHandle<AppState>,
    render: Box<dyn RenderBackend>,
    size: Cell<PhysicalSize>,
    fallback_size: (u32, u32),
    needs_redraw: Cell<bool>,
    /// Track whether a compositor frame callback is still pending
    frame_pending: Cell<bool>,
}

impl ShellWindowAdapter {
    pub(crate) fn new(
        surface: Surface,
        qh: QueueHandle<AppState>,
        render: Box<dyn RenderBackend>,
        initial_size: PhysicalSize,
        fallback_size: (u32, u32),
        scale: i32,
    ) -> Rc<Self> {
        let adapter = Rc::new_cyclic(|weak: &Weak<Self>| Self {
            window: Window::new(weak.clone()),
            surface,
            qh,
            render,
            size: Cell::new(initial_size),
            fallback_size,
            needs_redraw: Cell::new(false),
            frame_pending: Cell::new(false),
        });
        // Apply initial scale factor
        adapter.apply_scale_factor(scale);
        adapter
    }

    /// Notify the surface & slint about a scale factor change
    fn apply_scale_factor(&self, scale: i32) {
        self.surface.wl_surface().set_buffer_scale(scale);
        self.window.dispatch_event(WindowEvent::ScaleFactorChanged {
            scale_factor: scale as f32,
        });
    }

    /// Configure a new logical size for the surface
    pub(crate) fn configure(&self, new_size: (u32, u32)) {
        let scale = self.window.scale_factor();
        let logical_w = match new_size.0 > 0 {
            true => new_size.0,
            false => self.fallback_size.0,
        };
        let logical_h = match new_size.1 > 0 {
            true => new_size.1,
            false => self.fallback_size.1,
        };
        let logical = LogicalSize::new(logical_w as f32, logical_h as f32);
        let physical = logical.to_physical(scale);

        if physical != self.size.get() {
            self.size.set(physical);
            let _ = self.render.resize(physical);
            self.window
                .dispatch_event(WindowEvent::Resized { size: logical });
        }
    }

    /// Handle scale factor change and physical resizing.
    pub(crate) fn change_scale_factor(&self, new_factor: i32) {
        let logical = self.size.get().to_logical(self.window.scale_factor());
        self.apply_scale_factor(new_factor);
        let physical = logical.to_physical(new_factor as f32);
        if physical != self.size.get() {
            self.size.set(physical);
            let _ = self.render.resize(physical);
            self.window
                .dispatch_event(WindowEvent::Resized { size: logical });
        }
    }

    pub(crate) fn wl_surface(&self) -> &WlSurface {
        self.surface.wl_surface()
    }

    /// Handle rendering after a callback has completed
    /// (pending committed frame has been processed by the compositor)
    pub(crate) fn handle_frame_callback(&self) {
        self.frame_pending.set(false);
        self.render_if_needed();
    }

    fn render_if_needed(&self) {
        if !self.needs_redraw.replace(false) {
            return;
        }
        match self.render.render_and_present(&self.window) {
            Ok(RenderOutcome::Presented) => {
                // A frame was drawn and committed
                self.frame_pending.set(true);
            }
            Ok(RenderOutcome::Skipped) | Err(_) => {
                // Nothing was presented, manually defer redraw to keep the loop going
                self.needs_redraw.set(true);
                self.defer_redraw();
            }
        }
    }

    /// Recommit the presented buffer to handle redraw requests synchronously
    /// when rendering is not possible (e.g. during component initialization).
    fn defer_redraw(&self) {
        let surface = self.surface.wl_surface();
        surface.frame(&self.qh, FrameCallbackData(surface.clone()));
        self.surface.commit();
        self.frame_pending.set(true);
    }
}

impl WindowAdapter for ShellWindowAdapter {
    fn window(&self) -> &Window {
        &self.window
    }

    fn size(&self) -> PhysicalSize {
        self.size.get()
    }

    fn set_visible(&self, visible: bool) -> Result<(), PlatformError> {
        if visible {
            // Direct rendering call here.
            // (`set_visible` is called by `Window::show()`, all components already initialized)
            self.needs_redraw.set(true);
            self.render_if_needed();
        } else {
            self.surface.attach(None, 0, 0);
            self.surface.commit();
        }
        Ok(())
    }

    fn set_size(&self, size: WindowSize) {
        let scale = self.window.scale_factor();
        let logical = size.to_logical(scale);
        match &self.surface {
            Surface::Layer(layer) => {
                layer.set_size(logical.width as u32, logical.height as u32);
                layer.commit();
            }
            // Resize directly (wayland windows cannot request a size)
            Surface::Window(_) => {
                let physical = logical.to_physical(scale);
                if physical != self.size.get() {
                    self.size.set(physical);
                    let _ = self.render.resize(physical);
                    self.needs_redraw.set(true);
                    self.render_if_needed();
                }
            }
        }
    }

    /// Safe synchronous redraw (callable during component initialization).
    /// Either draw a pending frame or presents the current buffer again.
    fn request_redraw(&self) {
        self.needs_redraw.set(true);
        if !self.frame_pending.get() {
            self.defer_redraw();
        }
    }

    fn renderer(&self) -> &dyn Renderer {
        self.render.as_core_renderer()
    }
}
