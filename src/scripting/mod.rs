#[cfg(not(target_arch = "wasm32"))]
mod bindings;
#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(target_arch = "wasm32")]
mod wasm;

#[cfg(not(target_arch = "wasm32"))]
pub use native::{LuaScriptManager, StaticViewport, ViewportProvider};
#[cfg(target_arch = "wasm32")]
pub use wasm::{LuaScriptManager, StaticViewport, ViewportProvider};
