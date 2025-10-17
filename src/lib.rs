//! Core modules for the Crystal runtime, rewritten in Rust.
//!
//! The crate exposes high level building blocks that can be composed to
//! build bespoke runtimes or tooling around the Crystal authoring
//! environment.  Rendering and platform integration are intentionally kept
//! outside of the crate so that the code remains testable and easy to
//! embed in headless tools.

pub mod app;
pub mod archive;
pub mod data_model;
pub mod input;
pub mod obj;
pub mod render;
pub mod scene;
pub mod scripting;
#[cfg(target_arch = "wasm32")]
pub mod web;

pub use archive::{ArchiveFileEntry, CGameArchive};
pub use data_model::DataModel;
pub use input::{InputState, KeyCode, MouseButton, NamedKey};
pub use obj::{load_obj_from_str, ObjMesh};
pub use render::{CameraParams, LightParams, Renderer};
pub use scene::{Light, Scene, SceneObject};
pub use scripting::{LuaScriptManager, StaticViewport, ViewportProvider};
