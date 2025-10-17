use glam::{Mat4, Vec3};

/// Camera parameters consumed by the renderer's uniform buffer.
#[derive(Clone, Debug)]
pub struct CameraParams {
    pub view_proj: Mat4,
    pub position: Vec3,
}

/// Lighting state consumed by the renderer's uniform buffer.
#[derive(Clone, Debug)]
pub struct LightParams {
    pub position: Vec3,
    pub color: Vec3,
    pub intensity: f32,
}
