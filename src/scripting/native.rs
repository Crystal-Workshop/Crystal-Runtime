use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use anyhow::{anyhow, Context, Result};
use mlua::{HookTriggers, Lua};

use crate::archive::{ArchiveFileEntry, CGameArchive};
use crate::data_model::DataModel;
use crate::input::InputState;

use super::bindings::{register_globals, ScriptContext};

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

/// Manages the lifecycle of Lua scripts embedded in a `.cgame` archive.
pub struct LuaScriptManager {
    archive: Arc<CGameArchive>,
    data_model: DataModel,
    input_state: Arc<InputState>,
    viewport: Arc<dyn ViewportProvider + Send + Sync>,
    running: Arc<AtomicBool>,
    threads: Vec<JoinHandle<Result<()>>>,
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
            threads: Vec::new(),
        }
    }

    /// Launches a Lua state for every file stored under the `scripts/` prefix.
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
            return Ok(0);
        }

        self.running.store(true, Ordering::Release);
        for entry in entries {
            let archive = Arc::clone(&self.archive);
            let data_model = self.data_model.clone();
            let input_state = Arc::clone(&self.input_state);
            let viewport = Arc::clone(&self.viewport);
            let running = Arc::clone(&self.running);
            let handle = thread::spawn(move || {
                run_script_thread(archive, data_model, input_state, viewport, running, entry)
            });
            self.threads.push(handle);
        }
        Ok(self.threads.len())
    }

    /// Blocks until every running script finishes.
    pub fn wait(&mut self) -> Result<()> {
        self.join_threads()
    }

    /// Requests that all scripts stop and waits for them to exit.
    pub fn stop(&mut self) -> Result<()> {
        self.running.store(false, Ordering::Release);
        self.join_threads()
    }

    fn join_threads(&mut self) -> Result<()> {
        if self.threads.is_empty() {
            return Ok(());
        }
        let handles = std::mem::take(&mut self.threads);
        let mut errors = Vec::new();
        for handle in handles {
            match handle.join() {
                Ok(Ok(())) => {}
                Ok(Err(err)) => errors.push(err),
                Err(panic) => errors.push(anyhow!("script thread panicked: {:?}", panic)),
            }
        }
        if errors.is_empty() {
            self.running.store(false, Ordering::Release);
            Ok(())
        } else {
            let message = errors
                .into_iter()
                .map(|err| err.to_string())
                .collect::<Vec<_>>()
                .join("; ");
            Err(anyhow!("{message}"))
        }
    }
}

impl Drop for LuaScriptManager {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

fn run_script_thread(
    archive: Arc<CGameArchive>,
    data_model: DataModel,
    input_state: Arc<InputState>,
    viewport: Arc<dyn ViewportProvider + Send + Sync>,
    running: Arc<AtomicBool>,
    entry: ArchiveFileEntry,
) -> Result<()> {
    let lua = Lua::new();
    let hook_running = Arc::clone(&running);
    lua.set_hook(
        HookTriggers {
            every_nth_instruction: Some(1000),
            ..Default::default()
        },
        move |_, _| {
            if !hook_running.load(Ordering::Acquire) {
                Err(mlua::Error::RuntimeError("script stopped by host".into()))
            } else {
                Ok(())
            }
        },
    );

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::archive::CGameArchive;
    use crate::data_model::DataModel;
    use crate::scene::{Scene, SceneObject};
    use glam::Vec3;
    use once_cell::sync::Lazy;
    use tempfile::NamedTempFile;

    use std::io::Write;

    static SCENE_XML: Lazy<String> = Lazy::new(|| {
        "<scene>\n  <object>\n    <name>Cube</name>\n    <type>mesh</type>\n  </object>\n</scene>\n"
            .to_string()
    });

    fn build_archive(script: &str) -> (NamedTempFile, Arc<CGameArchive>) {
        let mut tmp = NamedTempFile::new().unwrap();
        let scene_bytes = SCENE_XML.as_bytes();
        let script_bytes = script.as_bytes();

        let mut buffer = Vec::new();
        buffer.extend_from_slice(b"CGME");
        buffer.extend_from_slice(&1u32.to_le_bytes());
        buffer.extend_from_slice(&0u64.to_le_bytes());

        let header_len = buffer.len() as u64;
        buffer.extend_from_slice(script_bytes);
        let script_offset = header_len;
        let script_size = script_bytes.len() as u64;

        let scene_offset = header_len + script_size;
        buffer.extend_from_slice(scene_bytes);
        let scene_size = scene_bytes.len() as u64;

        let toc_offset = scene_offset + scene_size;
        buffer.extend_from_slice(&1u32.to_le_bytes());
        buffer.extend_from_slice(&("scripts/test.lua".len() as u32).to_le_bytes());
        buffer.extend_from_slice(b"scripts/test.lua");
        buffer.extend_from_slice(&script_offset.to_le_bytes());
        buffer.extend_from_slice(&script_size.to_le_bytes());
        buffer.extend_from_slice(&scene_offset.to_le_bytes());
        buffer.extend_from_slice(&scene_size.to_le_bytes());

        buffer[8..16].copy_from_slice(&toc_offset.to_le_bytes());
        tmp.write_all(&buffer).unwrap();
        let archive = Arc::new(CGameArchive::open(tmp.path()).unwrap());
        (tmp, archive)
    }

    #[test]
    fn script_updates_data_model() {
        let (_tmp, archive) =
            build_archive("local cube = place.get('Cube') cube.color = Color3.new(255,0,0)");
        let scene = Scene {
            objects: vec![SceneObject {
                name: "Cube".into(),
                ..SceneObject::default()
            }],
            lights: vec![],
        };
        let model = DataModel::from_objects(scene.objects.clone());
        let input = Arc::new(InputState::new());
        let viewport: Arc<dyn ViewportProvider + Send + Sync> =
            Arc::new(StaticViewport::new(1280, 720));
        let mut manager = LuaScriptManager::new(archive, model.clone(), input, viewport);
        manager.start().unwrap();
        manager.wait().unwrap();
        let cube = model.get("Cube").unwrap();
        assert_eq!(cube.color, Vec3::new(1.0, 0.0, 0.0));
    }
}
