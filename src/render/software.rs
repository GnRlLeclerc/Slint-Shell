use std::cell::{Cell, RefCell};

use slint::platform::Renderer;
use slint::platform::software_renderer::{
    PremultipliedRgbaColor, RepaintBufferType, SoftwareRenderer, TargetPixel,
};
use slint::{PhysicalSize, PlatformError, Window};
use smithay_client_toolkit::compositor::FrameCallbackData;
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shm::Shm;
use smithay_client_toolkit::shm::slot::{Slot, SlotPool};
use wayland_client::QueueHandle;
use wayland_client::protocol::wl_shm;

use crate::surface::Surface;
use crate::wayland::AppState;

use super::{RenderBackend, RenderOutcome};

/// A 32bit ARGB pixel matching `wl_shm::Format::Argb8888`'s in-memory byte layout
/// (little-endian: B, G, R, A), premultiplied.
#[repr(transparent)]
#[derive(Copy, Clone, Default, bytemuck::Pod, bytemuck::Zeroable)]
struct Argb8888Pixel(u32);

impl TargetPixel for Argb8888Pixel {
    fn blend(&mut self, color: PremultipliedRgbaColor) {
        let [b, g, r, a] = self.0.to_le_bytes();
        let ia = (u8::MAX - color.alpha) as u16;
        let blend = |bg: u8, fg: u8| (bg as u16 * ia / 255) as u8 + fg;
        self.0 = u32::from_le_bytes([
            blend(b, color.blue),
            blend(g, color.green),
            blend(r, color.red),
            (a as u16 + color.alpha as u16 - (a as u16 * color.alpha as u16) / 255) as u8,
        ]);
    }

    fn from_rgb(red: u8, green: u8, blue: u8) -> Self {
        Self(u32::from_le_bytes([blue, green, red, 0xff]))
    }

    fn background() -> Self {
        // Transparent background
        Self(0)
    }
}

/// Slint software renderer backend.
/// Full rerender for each frame.
pub(crate) struct SoftwareRenderBackend {
    renderer: SoftwareRenderer,
    surface: Surface,
    qh: QueueHandle<AppState>,
    pool: RefCell<SlotPool>,
    slots: RefCell<[Slot; 2]>,
    next_slot: Cell<usize>,
    size: Cell<PhysicalSize>,
}

impl SoftwareRenderBackend {
    pub(crate) fn new(
        surface: Surface,
        qh: QueueHandle<AppState>,
        shm: &Shm,
        size: PhysicalSize,
    ) -> Result<Self, PlatformError> {
        let stride = buffer_len(size);
        let mut pool = SlotPool::new(stride.max(1) * 2, shm)
            .map_err(|e| PlatformError::Other(format!("failed to create wl_shm pool: {e}")))?;
        let slots = [
            pool.new_slot(stride.max(1))
                .map_err(|e| PlatformError::Other(format!("failed to allocate shm slot: {e}")))?,
            pool.new_slot(stride.max(1))
                .map_err(|e| PlatformError::Other(format!("failed to allocate shm slot: {e}")))?,
        ];
        Ok(Self {
            renderer: SoftwareRenderer::new_with_repaint_buffer_type(RepaintBufferType::NewBuffer),
            surface,
            qh,
            pool: RefCell::new(pool),
            slots: RefCell::new(slots),
            next_slot: Cell::new(0),
            size: Cell::new(size),
        })
    }
}

fn buffer_len(size: PhysicalSize) -> usize {
    size.width as usize * size.height as usize * 4
}

impl RenderBackend for SoftwareRenderBackend {
    fn as_core_renderer(&self) -> &dyn Renderer {
        &self.renderer
    }

    fn render_and_present(&self, _: &Window) -> Result<RenderOutcome, PlatformError> {
        let size = self.size.get();
        if size.width == 0 || size.height == 0 {
            return Ok(RenderOutcome::Skipped);
        }
        let stride = size.width as i32 * 4;
        let index = self.next_slot.get();

        let mut pool = self.pool.borrow_mut();
        let slots = self.slots.borrow();
        let Some(canvas) = slots[index].canvas(&mut pool) else {
            // The compositor hasn't released this buffer yet; try again once it does.
            return Ok(RenderOutcome::Skipped);
        };

        let pixels: &mut [Argb8888Pixel] = bytemuck::cast_slice_mut(canvas);
        let region = self.renderer.render(pixels, size.width as usize);

        let buffer = pool
            .create_buffer_in(
                &slots[index],
                size.width as i32,
                size.height as i32,
                stride,
                wl_shm::Format::Argb8888,
            )
            .map_err(|e| PlatformError::Other(format!("failed to create wl_shm buffer: {e}")))?;
        drop(slots);
        drop(pool);
        self.next_slot.set(1 - index);

        let surface = self.surface.wl_surface();
        for (pos, size) in region.iter() {
            surface.damage_buffer(pos.x, pos.y, size.width as i32, size.height as i32);
        }
        surface.frame(&self.qh, FrameCallbackData(surface.clone()));
        buffer
            .attach_to(surface)
            .map_err(|e| PlatformError::Other(format!("failed to attach wl_shm buffer: {e}")))?;
        self.surface.commit();
        Ok(RenderOutcome::Presented)
    }

    fn resize(&self, size: PhysicalSize) -> Result<(), PlatformError> {
        if size == self.size.get() {
            return Ok(());
        }
        let stride = buffer_len(size).max(1);
        let mut pool = self.pool.borrow_mut();
        pool.resize(stride * 2)
            .map_err(|e| PlatformError::Other(format!("failed to resize wl_shm pool: {e}")))?;
        let new_slots = [
            pool.new_slot(stride)
                .map_err(|e| PlatformError::Other(format!("failed to allocate shm slot: {e}")))?,
            pool.new_slot(stride)
                .map_err(|e| PlatformError::Other(format!("failed to allocate shm slot: {e}")))?,
        ];
        drop(pool);
        *self.slots.borrow_mut() = new_slots;
        self.next_slot.set(0);
        self.size.set(size);
        Ok(())
    }
}
