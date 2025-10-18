use glam::{Mat4, Vec3};
use winit::event::MouseButton as WinitMouseButton;
use winit::keyboard::{KeyCode as WinitKeyCode, PhysicalKey};

use crate::{
    data_model::DataModel,
    input::{KeyCode, MouseButton, NamedKey},
    render::{CameraParams, LightParams},
    scene::SceneObject,
};

pub fn camera_from_objects(objects: &[SceneObject], aspect: f32) -> CameraParams {
    let default_position = Vec3::new(0.0, 2.0, 6.0);
    let default_target = Vec3::ZERO;
    let (position, rotation, fov) = objects
        .iter()
        .find(|o| o.object_type == "camera")
        .map(|camera| (camera.position, camera.rotation, camera.fov))
        .unwrap_or((default_position, Vec3::ZERO, 60.0));

    let rotation_matrix = Mat4::from_rotation_z(rotation.z.to_radians())
        * Mat4::from_rotation_y(rotation.y.to_radians())
        * Mat4::from_rotation_x(rotation.x.to_radians());
    let forward = (rotation_matrix * Vec3::new(0.0, 0.0, -1.0).extend(0.0)).truncate();
    let up = (rotation_matrix * Vec3::Y.extend(0.0)).truncate();
    let target = if forward.length_squared() > f32::EPSILON {
        position + forward.normalize()
    } else {
        default_target
    };
    let view = Mat4::look_at_rh(position, target, up);
    let projection = Mat4::perspective_rh_gl(fov.to_radians(), aspect.max(0.01), 0.1, 100.0);
    CameraParams {
        view_proj: projection * view,
        position,
    }
}

pub fn light_from_objects(objects: &[SceneObject]) -> LightParams {
    objects
        .iter()
        .find(|o| o.object_type == "light")
        .map(|light| LightParams {
            position: light.position,
            color: light.color,
            intensity: light.intensity.max(0.1),
        })
        .unwrap_or(LightParams {
            position: Vec3::new(3.0, 5.0, -3.0),
            color: Vec3::splat(1.0),
            intensity: 1.0,
        })
}

pub fn print_final_state(model: &DataModel) {
    println!("Final object states:");
    for object in model.all_objects() {
        println!(
            " - {} pos=({:.2}, {:.2}, {:.2}) color=({:.2}, {:.2}, {:.2})",
            object.name,
            object.position.x,
            object.position.y,
            object.position.z,
            object.color.x,
            object.color.y,
            object.color.z
        );
    }
}

pub fn map_keycode(key: &PhysicalKey) -> Option<KeyCode> {
    let PhysicalKey::Code(code) = key else {
        return None;
    };

    use WinitKeyCode as Key;
    Some(match code {
        Key::Space => KeyCode::Named(NamedKey::Space),
        Key::Enter => KeyCode::Named(NamedKey::Enter),
        Key::Tab => KeyCode::Named(NamedKey::Tab),
        Key::ArrowLeft => KeyCode::Named(NamedKey::Left),
        Key::ArrowRight => KeyCode::Named(NamedKey::Right),
        Key::ArrowUp => KeyCode::Named(NamedKey::Up),
        Key::ArrowDown => KeyCode::Named(NamedKey::Down),
        Key::Escape => KeyCode::Named(NamedKey::Escape),
        Key::Backspace => KeyCode::Named(NamedKey::Backspace),
        Key::Home => KeyCode::Named(NamedKey::Home),
        Key::End => KeyCode::Named(NamedKey::End),
        Key::PageUp => KeyCode::Named(NamedKey::PageUp),
        Key::PageDown => KeyCode::Named(NamedKey::PageDown),
        Key::ShiftLeft => KeyCode::Named(NamedKey::LeftShift),
        Key::ShiftRight => KeyCode::Named(NamedKey::RightShift),
        Key::ControlLeft => KeyCode::Named(NamedKey::LeftCtrl),
        Key::ControlRight => KeyCode::Named(NamedKey::RightCtrl),
        Key::AltLeft => KeyCode::Named(NamedKey::LeftAlt),
        Key::AltRight => KeyCode::Named(NamedKey::RightAlt),
        Key::Digit0 => KeyCode::Digit(0),
        Key::Digit1 => KeyCode::Digit(1),
        Key::Digit2 => KeyCode::Digit(2),
        Key::Digit3 => KeyCode::Digit(3),
        Key::Digit4 => KeyCode::Digit(4),
        Key::Digit5 => KeyCode::Digit(5),
        Key::Digit6 => KeyCode::Digit(6),
        Key::Digit7 => KeyCode::Digit(7),
        Key::Digit8 => KeyCode::Digit(8),
        Key::Digit9 => KeyCode::Digit(9),
        Key::KeyA => KeyCode::Character('A'),
        Key::KeyB => KeyCode::Character('B'),
        Key::KeyC => KeyCode::Character('C'),
        Key::KeyD => KeyCode::Character('D'),
        Key::KeyE => KeyCode::Character('E'),
        Key::KeyF => KeyCode::Character('F'),
        Key::KeyG => KeyCode::Character('G'),
        Key::KeyH => KeyCode::Character('H'),
        Key::KeyI => KeyCode::Character('I'),
        Key::KeyJ => KeyCode::Character('J'),
        Key::KeyK => KeyCode::Character('K'),
        Key::KeyL => KeyCode::Character('L'),
        Key::KeyM => KeyCode::Character('M'),
        Key::KeyN => KeyCode::Character('N'),
        Key::KeyO => KeyCode::Character('O'),
        Key::KeyP => KeyCode::Character('P'),
        Key::KeyQ => KeyCode::Character('Q'),
        Key::KeyR => KeyCode::Character('R'),
        Key::KeyS => KeyCode::Character('S'),
        Key::KeyT => KeyCode::Character('T'),
        Key::KeyU => KeyCode::Character('U'),
        Key::KeyV => KeyCode::Character('V'),
        Key::KeyW => KeyCode::Character('W'),
        Key::KeyX => KeyCode::Character('X'),
        Key::KeyY => KeyCode::Character('Y'),
        Key::KeyZ => KeyCode::Character('Z'),
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

pub fn map_mouse_button(button: WinitMouseButton) -> MouseButton {
    let index = match button {
        WinitMouseButton::Left => 0,
        WinitMouseButton::Right => 1,
        WinitMouseButton::Middle => 2,
        WinitMouseButton::Back => 3,
        WinitMouseButton::Forward => 4,
        WinitMouseButton::Other(value) => value,
    } as u8;
    MouseButton::new(index)
}
