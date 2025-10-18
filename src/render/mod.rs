#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
pub use native::Renderer;

mod shared;

#[cfg(target_arch = "wasm32")]
mod wasm;
#[cfg(target_arch = "wasm32")]
pub use wasm::Renderer;

use glam::{Mat4, Vec3};

/// Camera parameters consumed by the renderer's uniform buffer.
#[derive(Debug, Clone)]
pub struct CameraParams {
    pub view_proj: Mat4,
    pub position: Vec3,
}

/// Lighting state consumed by the renderer's uniform buffer.
#[derive(Debug, Clone)]
pub struct LightParams {
    pub position: Vec3,
    pub color: Vec3,
    pub intensity: f32,
}
