use slint::LogicalPosition;
use slint::platform::{PointerEventButton, WindowAdapter as _, WindowEvent};
use smithay_client_toolkit::seat::pointer::{PointerEvent, PointerEventKind, PointerHandler};
use wayland_client::protocol::wl_pointer::WlPointer;
use wayland_client::{Connection, QueueHandle};

use super::AppState;

fn button_from_code(code: u32) -> Option<PointerEventButton> {
    // Evdev button codes, see linux/input-event-codes.h.
    match code {
        0x110 => Some(PointerEventButton::Left),
        0x111 => Some(PointerEventButton::Right),
        0x112 => Some(PointerEventButton::Middle),
        0x113 => Some(PointerEventButton::Back),
        0x114 => Some(PointerEventButton::Forward),
        _ => None,
    }
}

impl PointerHandler for AppState {
    fn pointer_frame(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlPointer,
        events: &[PointerEvent],
    ) {
        let Some(wa) = self.window_adapter.upgrade() else {
            return;
        };

        for event in events {
            if event.surface != *wa.layer_wl_surface() {
                continue;
            }
            let position = LogicalPosition::new(event.position.0 as f32, event.position.1 as f32);
            let window_event = match &event.kind {
                PointerEventKind::Enter { .. } => Some(WindowEvent::PointerMoved { position }),
                PointerEventKind::Leave { .. } => Some(WindowEvent::PointerExited),
                PointerEventKind::Motion { .. } => Some(WindowEvent::PointerMoved { position }),
                PointerEventKind::Press { button, .. } => button_from_code(*button)
                    .map(|button| WindowEvent::PointerPressed { position, button }),
                PointerEventKind::Release { button, .. } => button_from_code(*button)
                    .map(|button| WindowEvent::PointerReleased { position, button }),
                PointerEventKind::Axis {
                    horizontal,
                    vertical,
                    ..
                } => Some(WindowEvent::PointerScrolled {
                    position,
                    delta_x: horizontal.absolute as f32,
                    delta_y: vertical.absolute as f32,
                }),
            };
            if let Some(window_event) = window_event {
                wa.window().dispatch_event(window_event);
            }
        }
    }
}
