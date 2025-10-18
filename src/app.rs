use parking_lot::RwLock;

use glam::{Mat4, Vec3};

use crate::data_model::DataModel;
use crate::render::{CameraParams, LightParams};
use crate::scene::SceneObject;
use crate::scripting::ViewportProvider;

#[derive(Debug)]
pub struct WindowViewport {
    size: RwLock<(u32, u32)>,
}

impl WindowViewport {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            size: RwLock::new((width, height)),
        }
    }

    pub fn update(&self, width: u32, height: u32) {
        *self.size.write() = (width.max(1), height.max(1));
    }
}

impl ViewportProvider for WindowViewport {
    fn viewport_size(&self) -> (u32, u32) {
        *self.size.read()
    }
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
