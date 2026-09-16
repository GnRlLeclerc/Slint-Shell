use std::cell::Cell;
use std::rc::{Rc, Weak};

use slint::platform::{Renderer, WindowAdapter, WindowEvent};
use slint::{LogicalSize, PhysicalSize, PlatformError, Window, WindowSize};
use smithay_client_toolkit::compositor::FrameCallbackData;
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shell::wlr_layer::LayerSurface;
use wayland_client::QueueHandle;
use wayland_client::protocol::wl_surface::WlSurface;

use crate::render::{RenderBackend, RenderOutcome};
use crate::wayland::AppState;

/// Window adapter for a single `wlr-layer-shell` surface.
pub(crate) struct LayerWindowAdapter {
    window: Window,
    layer: LayerSurface,
    qh: QueueHandle<AppState>,
    render: Box<dyn RenderBackend>,
    size: Cell<PhysicalSize>,
    fallback_size: (u32, u32),
    needs_redraw: Cell<bool>,
    /// Track whether a compositor frame callback is still pending
    frame_pending: Cell<bool>,
}

impl LayerWindowAdapter {
    pub(crate) fn new(
        layer: LayerSurface,
        qh: QueueHandle<AppState>,
        render: Box<dyn RenderBackend>,
        initial_size: PhysicalSize,
        fallback_size: (u32, u32),
    ) -> Rc<Self> {
        Rc::new_cyclic(|weak: &Weak<Self>| Self {
            window: Window::new(weak.clone()),
            layer,
            qh,
            render,
            size: Cell::new(initial_size),
            fallback_size,
            needs_redraw: Cell::new(false),
            frame_pending: Cell::new(false),
        })
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
        self.layer.wl_surface().set_buffer_scale(new_factor);
        self.window.dispatch_event(WindowEvent::ScaleFactorChanged {
            scale_factor: new_factor as f32,
        });
        let physical = logical.to_physical(new_factor as f32);
        if physical != self.size.get() {
            self.size.set(physical);
            let _ = self.render.resize(physical);
        }
    }

    pub(crate) fn layer_wl_surface(&self) -> &WlSurface {
        self.layer.wl_surface()
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
        let surface = self.layer.wl_surface();
        surface.frame(&self.qh, FrameCallbackData(surface.clone()));
        self.layer.commit();
        self.frame_pending.set(true);
    }
}

impl WindowAdapter for LayerWindowAdapter {
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
            self.layer.wl_surface().attach(None, 0, 0);
            self.layer.commit();
        }
        Ok(())
    }

    fn set_size(&self, size: WindowSize) {
        let scale = self.window.scale_factor();
        let logical = size.to_logical(scale);
        self.layer
            .set_size(logical.width as u32, logical.height as u32);
        self.layer.commit();
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
