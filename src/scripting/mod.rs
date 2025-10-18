#[cfg(not(target_arch = "wasm32"))]
mod bindings;

#[cfg(not(target_arch = "wasm32"))]
mod manager;

#[cfg(target_arch = "wasm32")]
mod manager_stub;

#[cfg(not(target_arch = "wasm32"))]
pub use manager::{LuaScriptManager, StaticViewport, ViewportProvider};

#[cfg(target_arch = "wasm32")]
pub use manager_stub::{LuaScriptManager, StaticViewport, ViewportProvider};
