use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use glam::{Vec2, Vec3};
use js_sys::{Array, Object, Reflect};
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use super::common::ViewportProvider;
use crate::archive::{ArchiveFileEntry, CGameArchive};
use crate::data_model::DataModel;
use crate::input::InputState;

static HOST_CONTEXT: Lazy<RwLock<Option<HostContext>>> = Lazy::new(|| RwLock::new(None));

struct HostContext {
    data_model: DataModel,
    input_state: Arc<InputState>,
    viewport: Arc<dyn ViewportProvider + Send + Sync>,
    running: Arc<AtomicBool>,
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = crystalLuau, js_name = initialize, catch)]
    fn js_initialize() -> Result<(), JsValue>;

    #[wasm_bindgen(js_namespace = crystalLuau, js_name = runScript, catch)]
    fn js_run_script(name: &str, source: &str) -> Result<(), JsValue>;

    #[wasm_bindgen(js_namespace = crystalLuau, js_name = shutdown, catch)]
    fn js_shutdown() -> Result<(), JsValue>;
}

/// Executes Luau scripts in the browser using an externally provided runtime.
pub struct LuaScriptManager {
    archive: Arc<CGameArchive>,
    data_model: DataModel,
    input_state: Arc<InputState>,
    viewport: Arc<dyn ViewportProvider + Send + Sync>,
    running: Arc<AtomicBool>,
    launched: usize,
    runtime_active: bool,
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
            runtime_active: false,
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
        {
            let mut guard = HOST_CONTEXT.write();
            *guard = Some(HostContext {
                data_model: self.data_model.clone(),
                input_state: Arc::clone(&self.input_state),
                viewport: Arc::clone(&self.viewport),
                running: Arc::clone(&self.running),
            });
        }

        js_initialize().map_err(js_to_anyhow)?;
        self.runtime_active = true;

        let mut launched = 0usize;
        let mut run_error: Option<anyhow::Error> = None;
        for entry in entries {
            if !self.running.load(Ordering::Acquire) {
                break;
            }
            match self.execute_entry(entry) {
                Ok(()) => {
                    launched += 1;
                }
                Err(err) => {
                    run_error = Some(err);
                    break;
                }
            }
        }

        let shutdown_result = if self.runtime_active {
            let result = js_shutdown().map_err(js_to_anyhow);
            self.runtime_active = false;
            result
        } else {
            Ok(())
        };

        self.running.store(false, Ordering::Release);
        self.launched = launched;
        HOST_CONTEXT.write().take();

        if let Err(err) = shutdown_result {
            if let Some(run_err) = run_error {
                return Err(run_err.context(err.to_string()));
            }
            return Err(err);
        }

        if let Some(err) = run_error {
            return Err(err);
        }

        Ok(self.launched)
    }

    pub fn wait(&mut self) -> Result<()> {
        Ok(())
    }

    pub fn stop(&mut self) -> Result<()> {
        self.running.store(false, Ordering::Release);
        if self.runtime_active {
            js_shutdown().map_err(js_to_anyhow)?;
            self.runtime_active = false;
        }
        HOST_CONTEXT.write().take();
        Ok(())
    }

    fn execute_entry(&self, entry: ArchiveFileEntry) -> Result<()> {
        let source = self
            .archive
            .extract_entry(&entry)
            .with_context(|| format!("failed to extract {}", entry.name))?;
        let script = String::from_utf8(source)
            .map_err(|err| anyhow!("{} is not UTF-8: {err}", entry.name))?;
        js_run_script(&entry.name, &script).map_err(js_to_anyhow)?;
        Ok(())
    }
}

impl Drop for LuaScriptManager {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

#[wasm_bindgen]
pub fn crystal_luau_host(request: JsValue) -> Result<JsValue, JsValue> {
    let request_obj: Object = request
        .dyn_into()
        .map_err(|_| js_error("host request must be an object"))?;
    let op = get_string(&request_obj, "op")?;

    let (data_model, input_state, viewport, running) = {
        let guard = HOST_CONTEXT.read();
        let Some(context) = guard.as_ref() else {
            return Err(js_error("host context is not initialised"));
        };
        (
            context.data_model.clone(),
            Arc::clone(&context.input_state),
            Arc::clone(&context.viewport),
            Arc::clone(&context.running),
        )
    };

    match op.as_str() {
        "print" => {
            let args_value = get_property(&request_obj, "args")?;
            let args = Array::from(&args_value);
            let mut parts = Vec::new();
            for value in args.iter() {
                parts.push(value.as_string().unwrap_or_else(|| format!("{:?}", value)));
            }
            web_sys::console::log_1(&JsValue::from_str(&format!("[Lua] {}", parts.join("\t"))));
            Ok(JsValue::UNDEFINED)
        }
        "scene_names" => {
            let array = Array::new();
            for object in data_model.all_objects() {
                array.push(&JsValue::from(object.name));
            }
            Ok(array.into())
        }
        "scene_get" => {
            let name = get_string(&request_obj, "name")?;
            let value = data_model
                .get(&name)
                .map(|object| serde_wasm_bindgen::to_value(&object))
                .transpose()
                .map_err(|err| js_error(&format!("failed to serialise object: {err}")))?;
            Ok(value.unwrap_or(JsValue::NULL))
        }
        "scene_set" => {
            let name = get_string(&request_obj, "name")?;
            let field = get_string(&request_obj, "field")?;
            let value = get_property(&request_obj, "value")?;
            let changed = match field.as_str() {
                "position" => data_model.set_position(&name, parse_vec3(&value)?),
                "rotation" => data_model.set_rotation(&name, parse_vec3(&value)?),
                "scale" => data_model.set_scale(&name, parse_vec3(&value)?),
                "color" => data_model.set_color(&name, parse_color(&value)?),
                "fov" => data_model.set_fov(&name, get_number(&value, None)?),
                "intensity" => data_model.set_intensity(&name, get_number(&value, None)?),
                other => return Err(js_error(&format!("unsupported field '{other}'"))),
            };
            Ok(JsValue::from_bool(changed))
        }
        "input_get_key_down" => {
            let name = get_string(&request_obj, "name")?;
            Ok(JsValue::from_bool(input_state.is_key_down_by_name(&name)))
        }
        "input_get_mouse_position" => {
            let pos = input_state.mouse_position();
            Ok(vec2_to_js(pos))
        }
        "screen_get_viewport_size" => {
            let (width, height) = viewport.viewport_size();
            let object = Object::new();
            Reflect::set(
                &object,
                &JsValue::from_str("width"),
                &JsValue::from_f64(width as f64),
            )?;
            Reflect::set(
                &object,
                &JsValue::from_str("height"),
                &JsValue::from_f64(height as f64),
            )?;
            Ok(object.into())
        }
        "should_continue" => Ok(JsValue::from_bool(running.load(Ordering::Acquire))),
        "request_stop" => {
            running.store(false, Ordering::Release);
            Ok(JsValue::UNDEFINED)
        }
        other => Err(js_error(&format!("unknown host operation '{other}'"))),
    }
}

fn js_to_anyhow(value: JsValue) -> anyhow::Error {
    if let Some(error) = value.dyn_ref::<js_sys::Error>() {
        let message = error
            .message()
            .as_string()
            .unwrap_or_else(|| "JavaScript error".into());
        anyhow!(message)
    } else if let Some(message) = value.as_string() {
        anyhow!(message)
    } else {
        anyhow!("JavaScript error: {:?}", value)
    }
}

fn js_error(message: impl AsRef<str>) -> JsValue {
    JsValue::from_str(message.as_ref())
}

fn get_property(object: &Object, name: &str) -> Result<JsValue, JsValue> {
    Reflect::get(object, &JsValue::from_str(name))
}

fn get_string(object: &Object, name: &str) -> Result<String, JsValue> {
    let value = get_property(object, name)?;
    value
        .as_string()
        .ok_or_else(|| js_error(&format!("property '{name}' must be a string")))
}

fn get_number(value: &JsValue, field: Option<&str>) -> Result<f32, JsValue> {
    value
        .as_f64()
        .map(|number| number as f32)
        .ok_or_else(|| match field {
            Some(name) => js_error(&format!("property '{name}' must be a number")),
            None => js_error("value must be a number"),
        })
}

fn parse_vec3(value: &JsValue) -> Result<Vec3, JsValue> {
    let object: Object = value
        .clone()
        .dyn_into()
        .map_err(|_| js_error("vector must be an object"))?;
    let x = get_number(&get_property(&object, "x")?, Some("x"))?;
    let y = get_number(&get_property(&object, "y")?, Some("y"))?;
    let z = get_number(&get_property(&object, "z")?, Some("z"))?;
    Ok(Vec3::new(x, y, z))
}

fn parse_color(value: &JsValue) -> Result<Vec3, JsValue> {
    let object: Object = value
        .clone()
        .dyn_into()
        .map_err(|_| js_error("color must be an object"))?;
    let r = get_number(&get_property(&object, "r")?, Some("r"))?;
    let g = get_number(&get_property(&object, "g")?, Some("g"))?;
    let b = get_number(&get_property(&object, "b")?, Some("b"))?;
    Ok(Vec3::new(r / 255.0, g / 255.0, b / 255.0))
}

fn vec2_to_js(value: Vec2) -> JsValue {
    let object = Object::new();
    let _ = Reflect::set(
        &object,
        &JsValue::from_str("x"),
        &JsValue::from_f64(value.x as f64),
    );
    let _ = Reflect::set(
        &object,
        &JsValue::from_str("y"),
        &JsValue::from_f64(value.y as f64),
    );
    object.into()
}
