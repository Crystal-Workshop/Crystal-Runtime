use std::sync::Arc;

use anyhow::{anyhow, Result};
use glam::Vec2;
use gloo_events::EventListener;
use wasm_bindgen::JsCast;
use web_sys::{window, HtmlCanvasElement, KeyboardEvent, MouseEvent};

use super::{InputState, KeyCode, MouseButton, NamedKey};

/// Handles DOM input events and updates the shared [`InputState`].
pub struct WasmInputHandler {
    listeners: Vec<EventListener>,
}

impl WasmInputHandler {
    pub fn attach(canvas: &HtmlCanvasElement, input: Arc<InputState>) -> Result<Self> {
        let window = window().ok_or_else(|| anyhow!("window not available"))?;
        let document = window
            .document()
            .ok_or_else(|| anyhow!("document not available"))?;

        let mut listeners = Vec::new();

        // Keyboard focus on the whole document so keys are captured even when the canvas has focus.
        {
            let input_state = Arc::clone(&input);
            listeners.push(EventListener::new(&document, "keydown", move |event| {
                let event = event.dyn_ref::<KeyboardEvent>().unwrap();
                if let Some(code) = map_key(event) {
                    event.prevent_default();
                    input_state.set_key_down(code);
                }
            }));
        }

        {
            let input_state = Arc::clone(&input);
            listeners.push(EventListener::new(&document, "keyup", move |event| {
                let event = event.dyn_ref::<KeyboardEvent>().unwrap();
                if let Some(code) = map_key(event) {
                    event.prevent_default();
                    input_state.set_key_up(code);
                }
            }));
        }

        {
            let input_state = Arc::clone(&input);
            listeners.push(EventListener::new(canvas, "mousedown", move |event| {
                let event = event.dyn_ref::<MouseEvent>().unwrap();
                let button = MouseButton::new(event.button() as u8);
                input_state.set_mouse_button_down(button);
            }));
        }

        {
            let input_state = Arc::clone(&input);
            listeners.push(EventListener::new(canvas, "mouseup", move |event| {
                let event = event.dyn_ref::<MouseEvent>().unwrap();
                let button = MouseButton::new(event.button() as u8);
                input_state.set_mouse_button_up(button);
            }));
        }

        {
            let input_state = Arc::clone(&input);
            listeners.push(EventListener::new(canvas, "mousemove", move |event| {
                let event = event.dyn_ref::<MouseEvent>().unwrap();
                input_state.set_mouse_position(Vec2::new(
                    event.offset_x() as f32,
                    event.offset_y() as f32,
                ));
            }));
        }

        Ok(Self { listeners })
    }
}

impl Drop for WasmInputHandler {
    fn drop(&mut self) {
        self.listeners.clear();
    }
}

fn map_key(event: &KeyboardEvent) -> Option<KeyCode> {
    let key = event.key();
    match key.as_str() {
        "ArrowLeft" => Some(KeyCode::Named(NamedKey::Left)),
        "ArrowRight" => Some(KeyCode::Named(NamedKey::Right)),
        "ArrowUp" => Some(KeyCode::Named(NamedKey::Up)),
        "ArrowDown" => Some(KeyCode::Named(NamedKey::Down)),
        "Escape" => Some(KeyCode::Named(NamedKey::Escape)),
        " " => Some(KeyCode::Named(NamedKey::Space)),
        other => KeyCode::from_name(other),
    }
}
