use smithay_client_toolkit::seat::{Capability, SeatHandler, SeatState};
use wayland_client::protocol::wl_seat::{self, WlSeat};
use wayland_client::{Connection, QueueHandle};

use super::AppState;
use super::keyboard;

impl SeatHandler for AppState {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seats
    }

    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}

    fn new_capability(
        &mut self,
        _: &Connection,
        qh: &QueueHandle<Self>,
        seat: WlSeat,
        capability: Capability,
    ) {
        match capability {
            Capability::Keyboard if self.keyboard.is_none() => {
                if let Ok(keyboard) = self.seats.get_keyboard_with_repeat(
                    qh,
                    &seat,
                    None,
                    self.loop_handle.clone(),
                    Box::new(keyboard::dispatch_repeat),
                ) {
                    self.keyboard = Some(keyboard);
                }
            }
            Capability::Pointer if self.pointer.is_none() => {
                if let Ok(pointer) = self.seats.get_pointer(qh, &seat) {
                    self.pointer = Some(pointer);
                }
            }
            _ => {}
        }
    }

    fn remove_capability(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: WlSeat,
        capability: Capability,
    ) {
        match capability {
            Capability::Keyboard => {
                if let Some(k) = self.keyboard.take() {
                    k.release();
                }
            }
            Capability::Pointer => {
                if let Some(p) = self.pointer.take() {
                    p.release();
                }
            }
            _ => {}
        }
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: WlSeat) {}
}
