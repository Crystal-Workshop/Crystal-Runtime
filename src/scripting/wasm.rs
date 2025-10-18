use std::fmt::Write as _;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use futures::future::{AbortHandle, Abortable, Aborted};
use futures::lock::Mutex as AsyncMutex;
use glam::Vec3;
use gloo_timers::future::TimeoutFuture;
use serde::Deserialize;
use serde_json::Value;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{spawn_local, JsFuture};

use crate::archive::{ArchiveFileEntry, CGameArchive};
use crate::data_model::DataModel;
use crate::input::{InputState, KeyCode, MouseButton, NamedKey};

use super::viewport::ViewportProvider;

#[wasm_bindgen(module = "/src/js/luau_shim.js")]
extern "C" {
    #[wasm_bindgen(catch, js_name = executeLuau)]
    fn js_execute_luau(source: &str, chunk: &str) -> Result<js_sys::Promise, JsValue>;
}

#[derive(Debug, Deserialize)]
struct ScriptChange {
    object: String,
    field: String,
    value: Value,
}

#[derive(Debug, Deserialize)]
struct RawScriptResult {
    changes: Vec<ScriptChange>,
    wait: Option<f64>,
    finished: Option<bool>,
}

struct ScriptResult {
    changes: Vec<ScriptChange>,
    wait: u32,
    finished: bool,
}

/// Manages Lua scripts for the WebAssembly build.
pub struct LuaScriptManager {
    archive: Arc<CGameArchive>,
    data_model: DataModel,
    input_state: Arc<InputState>,
    viewport: Arc<dyn ViewportProvider + Send + Sync>,
    running: Arc<AtomicBool>,
    active_tasks: Arc<AtomicUsize>,
    execution_lock: Arc<AsyncMutex<()>>,
    tasks: Vec<ScriptTask>,
    launched: usize,
}

struct ScriptTask {
    abort_handle: AbortHandle,
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
            active_tasks: Arc::new(AtomicUsize::new(0)),
            execution_lock: Arc::new(AsyncMutex::new(())),
            tasks: Vec::new(),
            launched: 0,
        }
    }

    pub async fn start(&mut self) -> Result<usize> {
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

        self.active_tasks.store(0, Ordering::Release);
        self.running.store(true, Ordering::Release);
        let mut launched = 0;
        for entry in entries {
            let script_body = self
                .extract_script_source(&entry)
                .with_context(|| format!("failed to extract {}", entry.name))?;

            let running = Arc::clone(&self.running);
            let active_tasks = Arc::clone(&self.active_tasks);
            let data_model = self.data_model.clone();
            let input_state = Arc::clone(&self.input_state);
            let viewport = Arc::clone(&self.viewport);
            let lock = Arc::clone(&self.execution_lock);
            let chunk_name = entry.name.clone();

            let (abort_handle, abort_registration) = AbortHandle::new_pair();
            active_tasks.fetch_add(1, Ordering::AcqRel);
            let task_future = {
                let script_body = script_body;
                async move {
                    let mut finished = false;
                    let mut last_error: Option<anyhow::Error> = None;
                    while running.load(Ordering::Acquire) && !finished {
                        let payload = match build_script_payload(
                            &data_model,
                            &input_state,
                            viewport.as_ref(),
                            &script_body,
                            &chunk_name,
                        ) {
                            Ok(payload) => payload,
                            Err(err) => {
                                last_error = Some(err);
                                break;
                            }
                        };

                        let result = {
                            let _guard = lock.lock().await;
                            let outcome = execute_script(&payload, &chunk_name).await;
                            drop(_guard);
                            outcome
                        };

                        match result {
                            Ok(script_result) => {
                                for change in script_result.changes {
                                    apply_change(&data_model, change)
                                        .map_err(|err| anyhow!("{chunk_name}: {err}"))?;
                                }

                                finished = script_result.finished;

                                if running.load(Ordering::Acquire) && !finished {
                                    if script_result.wait > 0 {
                                        TimeoutFuture::new(script_result.wait).await;
                                    } else {
                                        TimeoutFuture::new(0).await;
                                    }
                                }
                            }
                            Err(err) => {
                                last_error = Some(err);
                                break;
                            }
                        }
                    }

                    if let Some(err) = last_error {
                        Err(err)
                    } else {
                        Ok(())
                    }
                }
            };

            let name = entry.name.clone();
            let running_flag = Arc::clone(&self.running);
            let active_counter = Arc::clone(&self.active_tasks);
            spawn_local(async move {
                let outcome = Abortable::new(task_future, abort_registration).await;
                match outcome {
                    Ok(Ok(())) => {}
                    Ok(Err(err)) => {
                        log_script_error(&name, &err.to_string());
                    }
                    Err(Aborted) => {}
                }
                finish_task(&active_counter, &running_flag);
            });

            self.tasks.push(ScriptTask { abort_handle });
            launched += 1;
        }

        self.launched = launched;
        Ok(launched)
    }

    pub fn wait(&mut self) -> Result<()> {
        Ok(())
    }

    pub fn stop(&mut self) -> Result<()> {
        self.running.store(false, Ordering::Release);
        for task in self.tasks.drain(..) {
            task.abort_handle.abort();
        }
        Ok(())
    }
}

fn apply_change(data_model: &DataModel, change: ScriptChange) -> Result<()> {
    match change.field.as_str() {
        "position" => {
            let vec = parse_vec3(&change.value)?;
            if !data_model.set_position(&change.object, vec) {
                return Err(anyhow!("unknown object {}", change.object));
            }
        }
        "rotation" => {
            let vec = parse_vec3(&change.value)?;
            if !data_model.set_rotation(&change.object, vec) {
                return Err(anyhow!("unknown object {}", change.object));
            }
        }
        "scale" => {
            let vec = parse_vec3(&change.value)?;
            if !data_model.set_scale(&change.object, vec) {
                return Err(anyhow!("unknown object {}", change.object));
            }
        }
        "color" => {
            let vec = parse_vec3(&change.value)?;
            if !data_model.set_color(&change.object, vec) {
                return Err(anyhow!("unknown object {}", change.object));
            }
        }
        "fov" => {
            let value = parse_f32(&change.value)?;
            if !data_model.set_fov(&change.object, value) {
                return Err(anyhow!("unknown object {}", change.object));
            }
        }
        "intensity" => {
            let value = parse_f32(&change.value)?;
            if !data_model.set_intensity(&change.object, value) {
                return Err(anyhow!("unknown object {}", change.object));
            }
        }
        other => return Err(anyhow!("unsupported field {other}")),
    }
    Ok(())
}

fn build_script_payload(
    data_model: &DataModel,
    input_state: &InputState,
    viewport: &dyn ViewportProvider,
    script: &str,
    chunk: &str,
) -> Result<String> {
    let mut payload = String::new();
    writeln!(&mut payload, "local __chunk_name = {}", luau_string(chunk))?;
    emit_object_table(&mut payload, data_model)?;
    emit_input_snapshot(&mut payload, input_state);
    emit_viewport(&mut payload, viewport);
    payload.push_str(LUAU_HELPERS);
    payload.push_str("\nlocal function __host_script()\n");
    payload.push_str(&indent_script(script));
    payload.push_str("\nend\n__host_emit_result(__host_run_script(__chunk_name, __host_script, __objects, __object_order, __input, __viewport))\n");
    Ok(payload)
}

fn emit_object_table(buffer: &mut String, data_model: &DataModel) -> Result<()> {
    let objects = data_model.all_objects();
    buffer.push_str("local __objects = {\n");
    for object in &objects {
        writeln!(buffer, "  [{}] = {{", luau_string(&object.name))?;
        writeln!(buffer, "    name = {},", luau_string(&object.name))?;
        writeln!(buffer, "    type = {},", luau_string(&object.object_type))?;
        if let Some(mesh) = &object.mesh {
            writeln!(buffer, "    mesh = {},", luau_string(mesh))?;
        }
        writeln!(
            buffer,
            "    position = {},",
            luau_vec3_literal(object.position)
        )?;
        writeln!(
            buffer,
            "    rotation = {},",
            luau_vec3_literal(object.rotation)
        )?;
        writeln!(buffer, "    scale = {},", luau_vec3_literal(object.scale))?;
        writeln!(buffer, "    color = {},", luau_vec3_literal(object.color))?;
        writeln!(buffer, "    fov = {},", luau_number(object.fov))?;
        writeln!(buffer, "    intensity = {}", luau_number(object.intensity))?;
        buffer.push_str("  },\n");
    }
    buffer.push_str("}\n");

    buffer.push_str("local __object_order = {\n");
    for object in &objects {
        writeln!(buffer, "  {},", luau_string(&object.name))?;
    }
    buffer.push_str("}\n");
    Ok(())
}

fn emit_input_snapshot(buffer: &mut String, input_state: &InputState) {
    let keys = collect_key_names(input_state);
    let buttons = collect_mouse_buttons(input_state);
    let mouse = input_state.mouse_position();
    buffer.push_str("local __input = {\n");
    buffer.push_str("  keys = {\n");
    for name in keys {
        buffer.push_str("    [");
        buffer.push_str(&luau_string(&name));
        buffer.push_str("] = true,\n");
    }
    buffer.push_str("  },\n");
    buffer.push_str("  buttons = {\n");
    for name in buttons {
        buffer.push_str("    [");
        buffer.push_str(&luau_string(&name));
        buffer.push_str("] = true,\n");
    }
    buffer.push_str("  },\n");
    writeln!(
        buffer,
        "  mouse = {{ x = {}, y = {} }},\n",
        luau_number(mouse.x),
        luau_number(mouse.y)
    )
    .unwrap();
    buffer.push_str("}\n");
}

fn emit_viewport(buffer: &mut String, viewport: &dyn ViewportProvider) {
    let (width, height) = viewport.viewport_size();
    writeln!(
        buffer,
        "local __viewport = {{ width = {}, height = {} }}\n",
        width, height
    )
    .unwrap();
}

fn indent_script(script: &str) -> String {
    let mut indented = String::new();
    if script.is_empty() {
        return indented;
    }
    for line in script.lines() {
        indented.push_str("    ");
        indented.push_str(line);
        indented.push('\n');
    }
    indented
}

async fn execute_script(payload: &str, chunk: &str) -> Result<ScriptResult> {
    let promise = js_execute_luau(payload, chunk).map_err(js_error)?;
    let value = JsFuture::from(promise)
        .await
        .map_err(js_error)?
        .as_string()
        .ok_or_else(|| anyhow!("Luau runtime did not return a result"))?;
    let raw: RawScriptResult = serde_json::from_str(&value)
        .map_err(|err| anyhow!("failed to parse Luau result: {err}"))?;
    let wait = raw.wait.unwrap_or(0.0).max(0.0);
    let wait = wait.min(u32::MAX as f64) as u32;
    Ok(ScriptResult {
        changes: raw.changes,
        wait,
        finished: raw.finished.unwrap_or(false),
    })
}

impl Drop for LuaScriptManager {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

impl LuaScriptManager {
    fn extract_script_source(&self, entry: &ArchiveFileEntry) -> Result<String> {
        let source = self.archive.extract_entry(entry)?;
        String::from_utf8(source).map_err(|err| anyhow!("{} is not UTF-8: {err}", entry.name))
    }
}

fn parse_vec3(value: &Value) -> Result<Vec3> {
    let obj = value
        .as_object()
        .ok_or_else(|| anyhow!("expected object for vector"))?;
    let x = obj
        .get("x")
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow!("missing vector component x"))? as f32;
    let y = obj
        .get("y")
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow!("missing vector component y"))? as f32;
    let z = obj
        .get("z")
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow!("missing vector component z"))? as f32;
    Ok(Vec3::new(x, y, z))
}

fn parse_f32(value: &Value) -> Result<f32> {
    value
        .as_f64()
        .map(|v| v as f32)
        .ok_or_else(|| anyhow!("expected number"))
}

fn luau_vec3_literal(vec: Vec3) -> String {
    format!(
        "{{ x = {}, y = {}, z = {} }}",
        luau_number(vec.x),
        luau_number(vec.y),
        luau_number(vec.z)
    )
}

fn luau_number(value: f32) -> String {
    if value.fract() == 0.0 {
        format!("{:.1}", value)
    } else {
        format!("{:.6}", value)
    }
}

fn luau_string(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len() + 2);
    escaped.push('"');
    for ch in value.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            c => escaped.push(c),
        }
    }
    escaped.push('"');
    escaped
}

fn collect_key_names(input: &InputState) -> Vec<String> {
    let mut result = Vec::new();
    for key in input_snapshot_keys(input) {
        match key {
            KeyCode::Named(named) => result.extend(named_key_aliases(named)),
            KeyCode::Character(ch) => result.push(ch.to_string()),
            KeyCode::Digit(digit) => result.push(((digit + b'0') as char).to_string()),
            KeyCode::Function(index) => result.push(format!("F{}", index)),
        }
    }
    result
}

fn collect_mouse_buttons(input: &InputState) -> Vec<String> {
    let mut names = Vec::new();
    for button in input_snapshot_mouse_buttons(input) {
        let index = button.index() + 1;
        if index == 1 {
            names.push("Mouse".to_string());
        }
        names.push(format!("Mouse{}", index));
    }
    names
}

fn input_snapshot_keys(input: &InputState) -> Vec<KeyCode> {
    let mut keys = Vec::new();
    for named in ALL_NAMED_KEYS.iter().copied() {
        if input.is_key_down(KeyCode::Named(named)) {
            keys.push(KeyCode::Named(named));
        }
    }
    for ch in 'A'..='Z' {
        if input.is_key_down(KeyCode::Character(ch)) {
            keys.push(KeyCode::Character(ch));
        }
    }
    for digit in 0..=9 {
        if input.is_key_down(KeyCode::Digit(digit)) {
            keys.push(KeyCode::Digit(digit));
        }
    }
    for function in 1..=12 {
        if input.is_key_down(KeyCode::Function(function)) {
            keys.push(KeyCode::Function(function));
        }
    }
    keys
}

fn input_snapshot_mouse_buttons(input: &InputState) -> Vec<MouseButton> {
    let mut buttons = Vec::new();
    for index in 0..3 {
        let button = MouseButton::new(index);
        if input.is_mouse_button_down(button) {
            buttons.push(button);
        }
    }
    buttons
}

fn named_key_aliases(key: NamedKey) -> Vec<String> {
    use NamedKey::*;
    match key {
        Space => vec!["Space".to_string()],
        Enter => vec!["Enter".to_string(), "Return".to_string()],
        Tab => vec!["Tab".to_string()],
        Left => vec!["Left".to_string()],
        Right => vec!["Right".to_string()],
        Up => vec!["Up".to_string()],
        Down => vec!["Down".to_string()],
        Escape => vec!["Escape".to_string(), "Esc".to_string()],
        Backspace => vec!["Backspace".to_string()],
        Home => vec!["Home".to_string()],
        End => vec!["End".to_string()],
        PageUp => vec!["PageUp".to_string()],
        PageDown => vec!["PageDown".to_string()],
        LeftShift => vec!["LeftShift".to_string(), "LShift".to_string()],
        RightShift => vec!["RightShift".to_string(), "RShift".to_string()],
        LeftCtrl => vec!["LeftCtrl".to_string(), "LControl".to_string()],
        RightCtrl => vec!["RightCtrl".to_string(), "RControl".to_string()],
        LeftAlt => vec!["LeftAlt".to_string(), "LAlt".to_string()],
        RightAlt => vec!["RightAlt".to_string(), "RAlt".to_string()],
    }
}

const ALL_NAMED_KEYS: [NamedKey; 19] = [
    NamedKey::Space,
    NamedKey::Enter,
    NamedKey::Tab,
    NamedKey::Left,
    NamedKey::Right,
    NamedKey::Up,
    NamedKey::Down,
    NamedKey::Escape,
    NamedKey::Backspace,
    NamedKey::Home,
    NamedKey::End,
    NamedKey::PageUp,
    NamedKey::PageDown,
    NamedKey::LeftShift,
    NamedKey::RightShift,
    NamedKey::LeftCtrl,
    NamedKey::RightCtrl,
    NamedKey::LeftAlt,
    NamedKey::RightAlt,
];

fn js_error(err: JsValue) -> anyhow::Error {
    if let Some(message) = err.as_string() {
        anyhow!(message)
    } else {
        anyhow!("JavaScript error: {:?}", err)
    }
}

fn log_script_error(chunk: &str, message: &str) {
    let formatted = format!("Luau script {} failed: {}", chunk, message);
    web_sys::console::error_1(&JsValue::from_str(&formatted));
}

fn finish_task(active: &Arc<AtomicUsize>, running: &Arc<AtomicBool>) {
    if active.fetch_sub(1, Ordering::AcqRel) == 1 {
        running.store(false, Ordering::Release);
    }
}

const LUAU_HELPERS: &str = r#"
__host_runtime = __host_runtime or {}
local __host_current_state = nil

local function __host_get_state(chunk)
    local state = __host_runtime[chunk]
    if not state then
        state = {
            thread = nil,
            objects = {},
            object_order = {},
            input = { keys = {}, buttons = {}, mouse = { x = 0, y = 0 } },
            viewport = { width = 0, height = 0 },
            changes = {},
            object_cache = {},
        }
        __host_runtime[chunk] = state
    end
    state.changes = {}
    state.object_cache = state.object_cache or {}
    return state
end

local function __host_set_current(state)
    __host_current_state = state
end

local function __copy_vec3(v)
    return { x = v.x, y = v.y, z = v.z }
end

local function __copy_color(v)
    return { x = v.x, y = v.y, z = v.z }
end

local function __record_change(name, field, value)
    local state = __host_current_state
    if not state then
        return
    end
    local changes = state.changes
    changes[#changes + 1] = { object = name, field = field, value = value }
end

Vector3 = Vector3 or {}
function Vector3.new(x, y, z)
    return { X = x, Y = y, Z = z, x = x, y = y, z = z }
end

Vector2 = Vector2 or {}
function Vector2.new(x, y)
    return { X = x, Y = y, x = x, y = y }
end

Color3 = Color3 or {}
function Color3.new(r, g, b)
    return { R = r, G = g, B = b, r = r, g = g, b = b }
end

local function __to_vec3(value)
    if type(value) ~= "table" then
        return nil
    end
    local x = value.x or value.X
    local y = value.y or value.Y
    local z = value.z or value.Z
    if type(x) ~= "number" or type(y) ~= "number" or type(z) ~= "number" then
        return nil
    end
    return { x = x, y = y, z = z }
end

local function __to_color3(value)
    if type(value) ~= "table" then
        return nil
    end
    local r = value.r or value.R
    local g = value.g or value.G
    local b = value.b or value.B
    if type(r) ~= "number" or type(g) ~= "number" or type(b) ~= "number" then
        return nil
    end
    return { x = r / 255, y = g / 255, z = b / 255 }
end

local function __wrap_object(name)
    local state = __host_current_state
    if not state then
        return nil
    end
    if state.object_cache[name] then
        return state.object_cache[name]
    end
    local proxy = {}
    local meta = {}
    function meta.__index(_, key)
        local data = state.objects[name]
        if not data then
            return nil
        end
        if key == "name" then
            return data.name
        elseif key == "position" then
            return Vector3.new(data.position.x, data.position.y, data.position.z)
        elseif key == "rotation" then
            return Vector3.new(data.rotation.x, data.rotation.y, data.rotation.z)
        elseif key == "scale" then
            return Vector3.new(data.scale.x, data.scale.y, data.scale.z)
        elseif key == "color" then
            return Color3.new(data.color.x * 255, data.color.y * 255, data.color.z * 255)
        elseif key == "fov" then
            return data.fov
        elseif key == "intensity" then
            return data.intensity
        elseif key == "type" then
            return data.type
        elseif key == "mesh" then
            return data.mesh
        end
        return rawget(proxy, key)
    end
    function meta.__newindex(_, key, value)
        local data = state.objects[name]
        if not data then
            return
        end
        if key == "position" then
            local vec = __to_vec3(value)
            if vec then
                data.position = vec
                __record_change(name, "position", __copy_vec3(vec))
            end
        elseif key == "rotation" then
            local vec = __to_vec3(value)
            if vec then
                data.rotation = vec
                __record_change(name, "rotation", __copy_vec3(vec))
            end
        elseif key == "scale" then
            local vec = __to_vec3(value)
            if vec then
                data.scale = vec
                __record_change(name, "scale", __copy_vec3(vec))
            end
        elseif key == "color" then
            local col = __to_color3(value)
            if col then
                data.color = col
                __record_change(name, "color", __copy_color(col))
            end
        elseif key == "fov" and type(value) == "number" then
            data.fov = value
            __record_change(name, "fov", value)
        elseif key == "intensity" and type(value) == "number" then
            data.intensity = value
            __record_change(name, "intensity", value)
        else
            rawset(proxy, key, value)
        end
    end
    state.object_cache[name] = setmetatable(proxy, meta)
    return state.object_cache[name]
end

scene = scene or {}
local scene_meta = getmetatable(scene) or {}
function scene_meta.__index(_, key)
    return __wrap_object(key)
end
setmetatable(scene, scene_meta)

function scene.get(name)
    return __wrap_object(name)
end

function scene.names()
    local state = __host_current_state
    if not state then
        return {}
    end
    local result = {}
    for index, name in ipairs(state.object_order) do
        result[index] = name
    end
    return result
end

place = scene

service = service or {}
service.input = service.input or {}

local function __normalize_name(name)
    if type(name) ~= "string" then
        return nil
    end
    return name
end

function service.input:GetKeyDown(name)
    name = __normalize_name(name)
    if not name then
        return false
    end
    local state = __host_current_state
    if not state then
        return false
    end
    return state.input.keys[name] or false
end

function service.input:GetMousePosition()
    local state = __host_current_state
    if not state then
        return Vector2.new(0, 0)
    end
    return Vector2.new(state.input.mouse.x, state.input.mouse.y)
end

screen = screen or {}
function screen:GetViewportSize()
    local state = __host_current_state
    if not state then
        return Vector2.new(0, 0)
    end
    return Vector2.new(state.viewport.width, state.viewport.height)
end

function wait(duration)
    duration = tonumber(duration) or 0
    if duration < 0 then
        duration = 0
    end
    return coroutine.yield(duration)
end

local function __escape(str)
    str = string.gsub(str, "\\", "\\\\")
    str = string.gsub(str, '"', '\\"')
    str = string.gsub(str, "\n", "\\n")
    str = string.gsub(str, "\r", "\\r")
    str = string.gsub(str, "\t", "\\t")
    return str
end

local function __encode(value)
    local kind = type(value)
    if kind == "string" then
        return '"' .. __escape(value) .. '"'
    elseif kind == "number" or kind == "boolean" then
        return tostring(value)
    elseif kind == "table" then
        local is_array = (#value > 0)
        local parts = {}
        if is_array then
            for i = 1, #value do
                parts[i] = __encode(value[i])
            end
            return "[" .. table.concat(parts, ",") .. "]"
        else
            for k, v in pairs(value) do
                parts[#parts + 1] = '"' .. __escape(tostring(k)) .. '":' .. __encode(v)
            end
            return "{" .. table.concat(parts, ",") .. "}"
        end
    end
    return "null"
end

local function __host_emit_result(result)
    print("__HOST_RESULT__:" .. __encode(result))
end

local function __host_run_script(chunk, script_fn, objects, order, input, viewport)
    local state = __host_get_state(chunk)
    state.objects = objects
    state.object_order = order
    state.input = input
    state.viewport = viewport
    __host_set_current(state)

    if not state.thread or coroutine.status(state.thread) == "dead" then
        state.thread = coroutine.create(function()
            local ok, err = pcall(script_fn)
            if not ok then
                error(err)
            end
        end)
    end

    local thread = state.thread
    if not thread or coroutine.status(thread) == "dead" then
        state.thread = nil
        return { changes = state.changes, wait = 0, finished = true }
    end

    local ok, wait_time = coroutine.resume(thread, 0)
    if not ok then
        state.thread = nil
        error(wait_time)
    end

    local finished = coroutine.status(thread) == "dead"
    local wait_ms = 0
    if finished then
        state.thread = nil
    else
        wait_ms = tonumber(wait_time) or 0
        if wait_ms < 0 then
            wait_ms = 0
        end
    end

    return { changes = state.changes, wait = wait_ms, finished = finished }
end
"#;
