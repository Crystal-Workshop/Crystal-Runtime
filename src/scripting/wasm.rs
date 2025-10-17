use std::sync::Arc;

use anyhow::{Context, Result};
use log::warn;

use crate::archive::{ArchiveFileEntry, CGameArchive};
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

/// Placeholder script manager for the Web build.
///
/// Browser WebAssembly environments do not currently ship with an embeddable
/// Lua runtime, so scripts are skipped while still reporting the available
/// entries to the host.
pub struct LuaScriptManager {
    archive: Arc<CGameArchive>,
    _data_model: DataModel,
    _input_state: Arc<InputState>,
    _viewport: Arc<dyn ViewportProvider + Send + Sync>,
    launched: usize,
}

impl LuaScriptManager {
    pub fn new(
        archive: Arc<CGameArchive>,
        data_model: DataModel,
        input_state: Arc<InputState>,
        viewport: Arc<dyn ViewportProvider + Send + Sync>,
    ) -> Self {
        Self {
            archive,
            _data_model: data_model,
            _input_state: input_state,
            _viewport: viewport,
            launched: 0,
        }
    }

    pub fn start(&mut self) -> Result<usize> {
        let entries: Vec<ArchiveFileEntry> = self
            .archive
            .files()
            .iter()
            .filter(|entry| entry.name.starts_with("scripts/"))
            .cloned()
            .collect();

        let skipped = entries.len();
        if skipped == 0 {
            self.launched = 0;
            return Ok(0);
        }

        warn!(
            "Lua scripting is not available in the WebAssembly build; skipping {} script(s)",
            skipped
        );
        for entry in entries {
            let _ = self
                .archive
                .extract_entry(&entry)
                .with_context(|| format!("failed to extract {}", entry.name))?;
        }
        self.launched = 0;
        Ok(skipped)
    }

    pub fn wait(&mut self) -> Result<()> {
        Ok(())
    }

    pub fn stop(&mut self) -> Result<()> {
        Ok(())
    }
}
