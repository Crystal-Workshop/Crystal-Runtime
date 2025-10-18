use std::collections::HashSet;

use glam::Vec2;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use winit::event::{MouseButton as WinitMouseButton, VirtualKeyCode};

/// Identifier for a physical keyboard key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KeyCode {
    Named(NamedKey),
    Character(char),
    Digit(u8),
    Function(u8),
}

impl KeyCode {
    pub fn from_name(name: &str) -> Option<Self> {
        if let Some(button) = parse_named_key(name) {
            return Some(button);
        }
        if name.len() == 1 {
            let ch = name.chars().next().unwrap();
            if ch.is_ascii_alphabetic() {
                return Some(Self::Character(ch.to_ascii_uppercase()));
            }
            if ch.is_ascii_digit() {
                return Some(Self::Digit(ch as u8 - b'0'));
            }
        }
        if let Some(function) = name.strip_prefix('F').or_else(|| name.strip_prefix('f')) {
            if let Ok(index) = function.parse::<u8>() {
                if index >= 1 && index <= 25 {
                    return Some(Self::Function(index));
                }
            }
        }
        None
    }
}

fn parse_named_key(name: &str) -> Option<KeyCode> {
    use NamedKey::*;
    let key = match name {
        "Space" => Space,
        "Enter" | "Return" => Enter,
        "Tab" => Tab,
        "Left" => Left,
        "Right" => Right,
        "Up" => Up,
        "Down" => Down,
        "Escape" | "Esc" => Escape,
        "Backspace" => Backspace,
        "Home" => Home,
        "End" => End,
        "PageUp" => PageUp,
        "PageDown" => PageDown,
        "LeftShift" | "LShift" => LeftShift,
        "RightShift" | "RShift" => RightShift,
        "LeftCtrl" | "LControl" => LeftCtrl,
        "RightCtrl" | "RControl" => RightCtrl,
        "LeftAlt" | "LAlt" => LeftAlt,
        "RightAlt" | "RAlt" => RightAlt,
        _ => return None,
    };
    Some(KeyCode::Named(key))
}

/// Friendly names for a subset of keyboard keys used by existing scripts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NamedKey {
    Space,
    Enter,
    Tab,
    Left,
    Right,
    Up,
    Down,
    Escape,
    Backspace,
    Home,
    End,
    PageUp,
    PageDown,
    LeftShift,
    RightShift,
    LeftCtrl,
    RightCtrl,
    LeftAlt,
    RightAlt,
}

/// Identifier for a mouse button (left button is zero).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MouseButton(u8);

impl MouseButton {
    pub const LEFT: Self = Self(0);

    pub fn new(index: u8) -> Self {
        Self(index)
    }

    pub fn index(self) -> u8 {
        self.0
    }
}

/// Maps a winit `VirtualKeyCode` to the internal [`KeyCode`] representation.
pub fn map_virtual_keycode(code: VirtualKeyCode) -> Option<KeyCode> {
    use VirtualKeyCode as Key;
    Some(match code {
        Key::Space => KeyCode::Named(NamedKey::Space),
        Key::Return => KeyCode::Named(NamedKey::Enter),
        Key::Tab => KeyCode::Named(NamedKey::Tab),
        Key::Left => KeyCode::Named(NamedKey::Left),
        Key::Right => KeyCode::Named(NamedKey::Right),
        Key::Up => KeyCode::Named(NamedKey::Up),
        Key::Down => KeyCode::Named(NamedKey::Down),
        Key::Escape => KeyCode::Named(NamedKey::Escape),
        Key::Back => KeyCode::Named(NamedKey::Backspace),
        Key::Home => KeyCode::Named(NamedKey::Home),
        Key::End => KeyCode::Named(NamedKey::End),
        Key::PageUp => KeyCode::Named(NamedKey::PageUp),
        Key::PageDown => KeyCode::Named(NamedKey::PageDown),
        Key::LShift => KeyCode::Named(NamedKey::LeftShift),
        Key::RShift => KeyCode::Named(NamedKey::RightShift),
        Key::LControl => KeyCode::Named(NamedKey::LeftCtrl),
        Key::RControl => KeyCode::Named(NamedKey::RightCtrl),
        Key::LAlt => KeyCode::Named(NamedKey::LeftAlt),
        Key::RAlt => KeyCode::Named(NamedKey::RightAlt),
        Key::Key0 => KeyCode::Digit(0),
        Key::Key1 => KeyCode::Digit(1),
        Key::Key2 => KeyCode::Digit(2),
        Key::Key3 => KeyCode::Digit(3),
        Key::Key4 => KeyCode::Digit(4),
        Key::Key5 => KeyCode::Digit(5),
        Key::Key6 => KeyCode::Digit(6),
        Key::Key7 => KeyCode::Digit(7),
        Key::Key8 => KeyCode::Digit(8),
        Key::Key9 => KeyCode::Digit(9),
        Key::A => KeyCode::Character('A'),
        Key::B => KeyCode::Character('B'),
        Key::C => KeyCode::Character('C'),
        Key::D => KeyCode::Character('D'),
        Key::E => KeyCode::Character('E'),
        Key::F => KeyCode::Character('F'),
        Key::G => KeyCode::Character('G'),
        Key::H => KeyCode::Character('H'),
        Key::I => KeyCode::Character('I'),
        Key::J => KeyCode::Character('J'),
        Key::K => KeyCode::Character('K'),
        Key::L => KeyCode::Character('L'),
        Key::M => KeyCode::Character('M'),
        Key::N => KeyCode::Character('N'),
        Key::O => KeyCode::Character('O'),
        Key::P => KeyCode::Character('P'),
        Key::Q => KeyCode::Character('Q'),
        Key::R => KeyCode::Character('R'),
        Key::S => KeyCode::Character('S'),
        Key::T => KeyCode::Character('T'),
        Key::U => KeyCode::Character('U'),
        Key::V => KeyCode::Character('V'),
        Key::W => KeyCode::Character('W'),
        Key::X => KeyCode::Character('X'),
        Key::Y => KeyCode::Character('Y'),
        Key::Z => KeyCode::Character('Z'),
        Key::F1 => KeyCode::Function(1),
        Key::F2 => KeyCode::Function(2),
        Key::F3 => KeyCode::Function(3),
        Key::F4 => KeyCode::Function(4),
        Key::F5 => KeyCode::Function(5),
        Key::F6 => KeyCode::Function(6),
        Key::F7 => KeyCode::Function(7),
        Key::F8 => KeyCode::Function(8),
        Key::F9 => KeyCode::Function(9),
        Key::F10 => KeyCode::Function(10),
        Key::F11 => KeyCode::Function(11),
        Key::F12 => KeyCode::Function(12),
        _ => return None,
    })
}

/// Maps a winit mouse button index into the internal [`MouseButton`].
pub fn mouse_button_from_winit(button: WinitMouseButton) -> MouseButton {
    let index = match button {
        WinitMouseButton::Left => 0,
        WinitMouseButton::Right => 1,
        WinitMouseButton::Middle => 2,
        WinitMouseButton::Other(value) => value,
    } as u8;
    MouseButton::new(index)
}

/// Thread-safe input snapshot shared with Lua scripts.
#[derive(Debug, Default)]
pub struct InputState {
    keys: RwLock<HashSet<KeyCode>>,
    mouse_buttons: RwLock<HashSet<MouseButton>>,
    mouse_position: RwLock<Vec2>,
}

impl InputState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_key_down(&self, key: KeyCode) {
        self.keys.write().insert(key);
    }

    pub fn set_key_up(&self, key: KeyCode) {
        self.keys.write().remove(&key);
    }

    pub fn set_mouse_button_down(&self, button: MouseButton) {
        self.mouse_buttons.write().insert(button);
    }

    pub fn set_mouse_button_up(&self, button: MouseButton) {
        self.mouse_buttons.write().remove(&button);
    }

    pub fn set_mouse_position(&self, position: Vec2) {
        *self.mouse_position.write() = position;
    }

    pub fn is_key_down(&self, key: KeyCode) -> bool {
        self.keys.read().contains(&key)
    }

    pub fn is_mouse_button_down(&self, button: MouseButton) -> bool {
        self.mouse_buttons.read().contains(&button)
    }

    pub fn is_key_down_by_name(&self, name: &str) -> bool {
        match parse_input_name(name) {
            Some(InputName::Key(key)) => self.is_key_down(key),
            Some(InputName::Mouse(button)) => self.is_mouse_button_down(button),
            None => false,
        }
    }

    pub fn mouse_position(&self) -> Vec2 {
        *self.mouse_position.read()
    }
}

enum InputName {
    Key(KeyCode),
    Mouse(MouseButton),
}

fn parse_input_name(name: &str) -> Option<InputName> {
    if let Some(button) = parse_mouse_button(name) {
        return Some(InputName::Mouse(button));
    }
    KeyCode::from_name(name).map(InputName::Key)
}

fn parse_mouse_button(name: &str) -> Option<MouseButton> {
    if name.len() < 5 {
        return None;
    }
    if !name[..5].eq_ignore_ascii_case("mouse") {
        return None;
    }
    let suffix = &name[5..];
    if suffix.is_empty() {
        return Some(MouseButton::LEFT);
    }
    let index = suffix.parse::<u8>().ok()?;
    let index = index.saturating_sub(1);
    Some(MouseButton::new(index))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_named_and_character_keys() {
        assert_eq!(
            KeyCode::from_name("Space"),
            Some(KeyCode::Named(NamedKey::Space))
        );
        assert_eq!(KeyCode::from_name("a"), Some(KeyCode::Character('A')));
        assert_eq!(KeyCode::from_name("F12"), Some(KeyCode::Function(12)));
    }

    #[test]
    fn mouse_names_are_supported() {
        assert_eq!(mouse_index("Mouse1"), 0);
        assert_eq!(mouse_index("mouse3"), 2);
    }

    #[test]
    fn input_state_tracks_keys() {
        let state = InputState::new();
        state.set_key_down(KeyCode::Named(NamedKey::Space));
        assert!(state.is_key_down_by_name("Space"));
        state.set_key_up(KeyCode::Named(NamedKey::Space));
        assert!(!state.is_key_down_by_name("Space"));
    }

    fn mouse_index(name: &str) -> u8 {
        match parse_input_name(name).unwrap() {
            InputName::Mouse(button) => button.index(),
            InputName::Key(_) => panic!("expected mouse button"),
        }
    }
}
