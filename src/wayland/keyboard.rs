use slint::SharedString;
use slint::platform::{Key, WindowAdapter as _, WindowEvent};
use smithay_client_toolkit::seat::keyboard::{
    KeyEvent, KeyboardHandler, Keysym, Modifiers, RawModifiers,
};
use wayland_client::protocol::wl_keyboard::WlKeyboard;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::{Connection, QueueHandle};
use xkeysym::key;

use super::AppState;

/// Dispatch a key event to the slint window.
fn dispatch_key_event(
    app_state: &mut AppState,
    event: KeyEvent,
    to_window_event: impl FnOnce(SharedString) -> WindowEvent,
) {
    let Some(text) = key_text(event.keysym, event.utf8.as_deref()) else {
        return;
    };
    if let Some(wa) = app_state.window_adapter.upgrade() {
        wa.window().dispatch_event(to_window_event(text));
    }
}

/// Dispatch a key repeat event to the slint window.
/// Used 2 times:
/// - compositor-triggered in `KeyboardHandler::repeat_key`
/// - client-triggered (by registering this callback when a keyboard is detected)
pub(crate) fn dispatch_repeat(app_state: &mut AppState, _: &WlKeyboard, event: KeyEvent) {
    dispatch_key_event(app_state, event, |text| WindowEvent::KeyPressRepeated {
        text,
    });
}

/// Maps a received keysym to a slint [`Key`] or a string
fn key_text(keysym: Keysym, utf8: Option<&str>) -> Option<SharedString> {
    let named: Option<Key> = match keysym.raw() {
        key::BackSpace => Some(Key::Backspace),
        key::Tab => Some(Key::Tab),
        key::Return | key::KP_Enter => Some(Key::Return),
        key::Escape => Some(Key::Escape),
        key::Delete | key::KP_Delete => Some(Key::Delete),
        key::Shift_L => Some(Key::Shift),
        key::Shift_R => Some(Key::ShiftR),
        key::Control_L => Some(Key::Control),
        key::Control_R => Some(Key::ControlR),
        key::Alt_L => Some(Key::Alt),
        key::Mode_switch | key::ISO_Level3_Shift => Some(Key::AltGr),
        key::Caps_Lock => Some(Key::CapsLock),
        key::Meta_L | key::Super_L => Some(Key::Meta),
        key::Meta_R | key::Super_R => Some(Key::MetaR),
        key::space | key::KP_Space => Some(Key::Space),
        key::Up | key::KP_Up => Some(Key::UpArrow),
        key::Down | key::KP_Down => Some(Key::DownArrow),
        key::Left | key::KP_Left => Some(Key::LeftArrow),
        key::Right | key::KP_Right => Some(Key::RightArrow),
        key::Insert | key::KP_Insert => Some(Key::Insert),
        key::Home | key::KP_Home => Some(Key::Home),
        key::End | key::KP_End => Some(Key::End),
        key::Page_Up | key::KP_Page_Up => Some(Key::PageUp),
        key::Page_Down | key::KP_Page_Down => Some(Key::PageDown),
        key::Scroll_Lock => Some(Key::ScrollLock),
        key::Pause => Some(Key::Pause),
        key::Sys_Req => Some(Key::SysReq),
        key::Menu => Some(Key::Menu),
        key::F1 => Some(Key::F1),
        key::F2 => Some(Key::F2),
        key::F3 => Some(Key::F3),
        key::F4 => Some(Key::F4),
        key::F5 => Some(Key::F5),
        key::F6 => Some(Key::F6),
        key::F7 => Some(Key::F7),
        key::F8 => Some(Key::F8),
        key::F9 => Some(Key::F9),
        key::F10 => Some(Key::F10),
        key::F11 => Some(Key::F11),
        key::F12 => Some(Key::F12),
        key::F13 => Some(Key::F13),
        key::F14 => Some(Key::F14),
        key::F15 => Some(Key::F15),
        key::F16 => Some(Key::F16),
        key::F17 => Some(Key::F17),
        key::F18 => Some(Key::F18),
        key::F19 => Some(Key::F19),
        key::F20 => Some(Key::F20),
        key::F21 => Some(Key::F21),
        key::F22 => Some(Key::F22),
        key::F23 => Some(Key::F23),
        key::F24 => Some(Key::F24),
        _ => None,
    };
    if let Some(named) = named {
        return Some(named.into());
    }
    utf8.filter(|s| !s.is_empty()).map(SharedString::from)
}

impl KeyboardHandler for AppState {
    fn enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlKeyboard,
        _: &WlSurface,
        _: u32,
        _: &[u32],
        _: &[Keysym],
    ) {
    }

    fn leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlKeyboard,
        _: &WlSurface,
        _: u32,
    ) {
    }

    fn press_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlKeyboard,
        _: u32,
        event: KeyEvent,
    ) {
        dispatch_key_event(self, event, |text| WindowEvent::KeyPressed { text });
    }

    fn repeat_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        keyboard: &WlKeyboard,
        _: u32,
        event: KeyEvent,
    ) {
        // Compositor-triggered key repeat
        dispatch_repeat(self, keyboard, event);
    }

    fn release_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlKeyboard,
        _: u32,
        event: KeyEvent,
    ) {
        dispatch_key_event(self, event, |text| WindowEvent::KeyReleased { text });
    }

    fn update_modifiers(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlKeyboard,
        _: u32,
        _: Modifiers,
        _: RawModifiers,
        _: u32,
    ) {
        // Already tracked by sctk and slint
    }
}
