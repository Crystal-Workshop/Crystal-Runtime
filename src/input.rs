use std::collections::HashSet;

use glam::Vec2;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

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
