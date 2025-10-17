#[cfg(not(target_arch = "wasm32"))]
pub mod native;
#[cfg(target_arch = "wasm32")]
pub mod wasm;

#[cfg(not(target_arch = "wasm32"))]
pub use native::{CameraParams, LightParams, Renderer};
#[cfg(target_arch = "wasm32")]
pub use wasm::{CameraParams, LightParams, Renderer};
