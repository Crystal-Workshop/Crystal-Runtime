#[cfg(not(target_arch = "wasm32"))]
mod bindings;
#[cfg(not(target_arch = "wasm32"))]
mod native;
mod viewport;
#[cfg(target_arch = "wasm32")]
mod wasm;

pub use viewport::{StaticViewport, ViewportProvider};

#[cfg(not(target_arch = "wasm32"))]
pub use native::LuaScriptManager;
#[cfg(target_arch = "wasm32")]
pub use wasm::LuaScriptManager;
