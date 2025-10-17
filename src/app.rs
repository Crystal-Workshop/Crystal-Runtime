use glam::{Mat4, Vec3};
use winit::event::{MouseButton as WinitMouseButton, VirtualKeyCode};

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

pub fn map_keycode(code: VirtualKeyCode) -> Option<KeyCode> {
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

pub fn map_mouse_button(button: WinitMouseButton) -> MouseButton {
    let index = match button {
        WinitMouseButton::Left => 0,
        WinitMouseButton::Right => 1,
        WinitMouseButton::Middle => 2,
        WinitMouseButton::Other(value) => value,
    } as u8;
    MouseButton::new(index)
}
