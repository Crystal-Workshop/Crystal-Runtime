mod bindings;
mod common;
#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(target_arch = "wasm32")]
mod wasm;

pub use common::{StaticViewport, ViewportProvider};
#[cfg(not(target_arch = "wasm32"))]
pub use native::LuaScriptManager;
#[cfg(target_arch = "wasm32")]
pub use wasm::LuaScriptManager;
