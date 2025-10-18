use std::sync::Arc;

use anyhow::Result;
use log::warn;

use crate::archive::CGameArchive;
use crate::data_model::DataModel;
use crate::input::InputState;

/// Provides viewport dimensions for Lua scripts.
pub trait ViewportProvider: Send + Sync {
    fn viewport_size(&self) -> (u32, u32);
}

/// Simple viewport that always reports the same resolution.
#[derive(Debug, Clone, Copy)]
pub struct StaticViewport {
    pub width: u32,
    pub height: u32,
}

impl StaticViewport {
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

impl ViewportProvider for StaticViewport {
    fn viewport_size(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

/// Stubbed Lua script manager used in WebAssembly builds.
pub struct LuaScriptManager;

impl LuaScriptManager {
    pub fn new(
        _archive: Arc<CGameArchive>,
        _data_model: DataModel,
        _input_state: Arc<InputState>,
        _viewport: Arc<dyn ViewportProvider + Send + Sync>,
    ) -> Self {
        warn!("Lua scripting is not available in WebAssembly builds");
        Self
    }

    pub fn start(&mut self) -> Result<usize> {
        warn!("Ignoring request to start Lua scripts on WebAssembly");
        Ok(0)
    }

    pub fn wait(&mut self) -> Result<()> {
        Ok(())
    }

    pub fn update(&mut self) -> Result<()> {
        Ok(())
    }

    pub fn stop(&mut self) -> Result<()> {
        Ok(())
    }
}
