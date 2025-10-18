#[cfg(not(target_arch = "wasm32"))]
mod bindings;
mod common;
#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(all(target_arch = "wasm32", target_os = "emscripten"))]
mod wasm;
#[cfg(all(target_arch = "wasm32", not(target_os = "emscripten")))]
mod wasm_stub;

pub use common::{StaticViewport, ViewportProvider};
#[cfg(not(target_arch = "wasm32"))]
pub use native::LuaScriptManager;
#[cfg(all(target_arch = "wasm32", target_os = "emscripten"))]
pub use wasm::LuaScriptManager;
#[cfg(all(target_arch = "wasm32", not(target_os = "emscripten")))]
pub use wasm_stub::LuaScriptManager;
