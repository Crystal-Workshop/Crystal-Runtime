mod common;

#[cfg(not(target_arch = "wasm32"))]
mod native;

#[cfg(target_arch = "wasm32")]
mod wasm;

pub use common::{CameraParams, LightParams};

#[cfg(not(target_arch = "wasm32"))]
pub use native::Renderer;

#[cfg(target_arch = "wasm32")]
pub use wasm::Renderer;
