use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use mlua::{Lua, VmState};

use crate::archive::{ArchiveFileEntry, CGameArchive};
use crate::data_model::DataModel;
use crate::input::InputState;

use super::bindings::{register_globals, ScriptContext};
use super::common::ViewportProvider;

/// Executes Luau scripts inside the browser when targeting Emscripten.
pub struct LuaScriptManager {
    archive: Arc<CGameArchive>,
    data_model: DataModel,
    input_state: Arc<InputState>,
    viewport: Arc<dyn ViewportProvider + Send + Sync>,
    running: Arc<AtomicBool>,
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
            data_model,
            input_state,
            viewport,
            running: Arc::new(AtomicBool::new(false)),
            launched: 0,
        }
    }

    pub fn start(&mut self) -> Result<usize> {
        self.stop()?;
        let entries: Vec<ArchiveFileEntry> = self
            .archive
            .files()
            .iter()
            .filter(|entry| entry.name.starts_with("scripts/"))
            .cloned()
            .collect();

        if entries.is_empty() {
            self.launched = 0;
            return Ok(0);
        }

        self.running.store(true, Ordering::Release);
        let mut launched = 0usize;
        for entry in entries {
            if !self.running.load(Ordering::Acquire) {
                break;
            }
            run_script(
                Arc::clone(&self.archive),
                self.data_model.clone(),
                Arc::clone(&self.input_state),
                Arc::clone(&self.viewport),
                Arc::clone(&self.running),
                entry,
            )?;
            launched += 1;
        }

        self.running.store(false, Ordering::Release);
        self.launched = launched;
        Ok(self.launched)
    }

    pub fn wait(&mut self) -> Result<()> {
        // Scripts are executed synchronously on the main thread, so there is
        // nothing to wait for once `start` returns.
        Ok(())
    }

    pub fn stop(&mut self) -> Result<()> {
        self.running.store(false, Ordering::Release);
        Ok(())
    }
}

fn run_script(
    archive: Arc<CGameArchive>,
    data_model: DataModel,
    input_state: Arc<InputState>,
    viewport: Arc<dyn ViewportProvider + Send + Sync>,
    running: Arc<AtomicBool>,
    entry: ArchiveFileEntry,
) -> Result<()> {
    let lua = Lua::new();
    let hook_running = Arc::clone(&running);
    lua.set_interrupt(move |_| {
        if !hook_running.load(Ordering::Acquire) {
            Err(mlua::Error::RuntimeError("script stopped by host".into()))
        } else {
            Ok(VmState::Continue)
        }
    });

    let context = ScriptContext::new(data_model, input_state, viewport, running);
    register_globals(&lua, &context)?;

    let source = archive
        .extract_entry(&entry)
        .with_context(|| format!("failed to extract {}", entry.name))?;
    let script =
        String::from_utf8(source).map_err(|err| anyhow!("{} is not UTF-8: {err}", entry.name))?;
    lua.load(&script)
        .set_name(&entry.name)
        .exec()
        .map_err(anyhow::Error::from)
        .context("Lua runtime error")
}
