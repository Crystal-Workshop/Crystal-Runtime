use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use anyhow::{Context, Result};
use mlua::{Lua, VmState};

use crate::archive::{ArchiveFileEntry, CGameArchive};
use crate::data_model::DataModel;
use crate::input::InputState;

use super::bindings::{register_globals, ScriptContext};
use super::common::ViewportProvider;

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
        let mut executed = 0usize;

        for entry in entries {
            if !self.running.load(Ordering::Acquire) {
                break;
            }

            let lua = Lua::new();
            let interrupt_flag = Arc::clone(&self.running);
            lua.set_interrupt(move |_| {
                if !interrupt_flag.load(Ordering::Acquire) {
                    Err(mlua::Error::RuntimeError("script stopped by host".into()))
                } else {
                    Ok(VmState::Continue)
                }
            });

            let context = ScriptContext::new(
                self.data_model.clone(),
                Arc::clone(&self.input_state),
                Arc::clone(&self.viewport),
                Arc::clone(&self.running),
            );
            register_globals(&lua, &context)?;

            let source = self
                .archive
                .extract_entry(&entry)
                .with_context(|| format!("failed to extract {}", entry.name))?;
            let script = String::from_utf8(source)
                .with_context(|| format!("{} is not UTF-8", entry.name))?;

            lua.load(&script)
                .set_name(&entry.name)
                .exec()
                .with_context(|| format!("Lua runtime error in {}", entry.name))?;

            executed += 1;
        }

        self.launched = executed;
        self.running.store(false, Ordering::Release);
        Ok(self.launched)
    }

    pub fn wait(&mut self) -> Result<()> {
        Ok(())
    }

    pub fn stop(&mut self) -> Result<()> {
        self.running.store(false, Ordering::Release);
        Ok(())
    }
}
