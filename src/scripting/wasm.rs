use std::fmt::Write as _;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use glam::Vec3;
use serde::Deserialize;
use serde_json::Value;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

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

/// Manages Lua scripts for the WebAssembly build.
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

        self.running.store(true, Ordering::Release);
        let mut launched = 0;
        for entry in entries {
            if !self.running.load(Ordering::Acquire) {
                break;
            }
            let script = self.build_script_payload(&entry)?;
            self.execute_script(&script, &entry.name)
                .await
                .with_context(|| format!("failed to run {}", entry.name))?;
            launched += 1;
        }
        self.running.store(false, Ordering::Release);
        self.launched = launched;
        Ok(launched)
    }

    pub fn wait(&mut self) -> Result<()> {
        Ok(())
    }

    pub fn stop(&mut self) -> Result<()> {
        self.running.store(false, Ordering::Release);
        Ok(())
    }

    async fn execute_script(&self, payload: &str, chunk: &str) -> Result<()> {
        let promise = js_execute_luau(payload, chunk).map_err(js_error)?;
        let value = JsFuture::from(promise)
            .await
            .map_err(js_error)?
            .as_string()
            .ok_or_else(|| anyhow!("Luau runtime did not return a result"))?;
        let changes: Vec<ScriptChange> = serde_json::from_str(&value)
            .map_err(|err| anyhow!("failed to parse Luau changes: {err}"))?;
        for change in changes {
            self.apply_change(change)
                .map_err(|err| anyhow!("failed to apply change: {err}"))?;
        }
        Ok(())
    }

    fn apply_change(&self, change: ScriptChange) -> Result<()> {
        match change.field.as_str() {
            "position" => {
                let vec = parse_vec3(&change.value)?;
                if !self.data_model.set_position(&change.object, vec) {
                    return Err(anyhow!("unknown object {}", change.object));
                }
            }
            "rotation" => {
                let vec = parse_vec3(&change.value)?;
                if !self.data_model.set_rotation(&change.object, vec) {
                    return Err(anyhow!("unknown object {}", change.object));
                }
            }
            "scale" => {
                let vec = parse_vec3(&change.value)?;
                if !self.data_model.set_scale(&change.object, vec) {
                    return Err(anyhow!("unknown object {}", change.object));
                }
            }
            "color" => {
                let vec = parse_vec3(&change.value)?;
                if !self.data_model.set_color(&change.object, vec) {
                    return Err(anyhow!("unknown object {}", change.object));
                }
            }
            "fov" => {
                let value = parse_f32(&change.value)?;
                if !self.data_model.set_fov(&change.object, value) {
                    return Err(anyhow!("unknown object {}", change.object));
                }
            }
            "intensity" => {
                let value = parse_f32(&change.value)?;
                if !self.data_model.set_intensity(&change.object, value) {
                    return Err(anyhow!("unknown object {}", change.object));
                }
            }
            other => return Err(anyhow!("unsupported field {other}")),
        }
        Ok(())
    }

    fn build_script_payload(&self, entry: &ArchiveFileEntry) -> Result<String> {
        let source = self
            .archive
            .extract_entry(entry)
            .with_context(|| format!("failed to extract {}", entry.name))?;
        let script = String::from_utf8(source)
            .map_err(|err| anyhow!("{} is not UTF-8: {err}", entry.name))?;
        let mut payload = String::new();
        self.emit_object_table(&mut payload)?;
        self.emit_input_snapshot(&mut payload);
        self.emit_viewport(&mut payload);
        payload.push_str(LUAU_HELPERS);
        payload.push_str("\n");
        payload.push_str(&script);
        payload.push_str("\n__host_emit_changes()\n");
        Ok(payload)
    }

    fn emit_object_table(&self, buffer: &mut String) -> Result<()> {
        let objects = self.data_model.all_objects();
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

    fn emit_input_snapshot(&self, buffer: &mut String) {
        let keys = collect_key_names(&self.input_state);
        let buttons = collect_mouse_buttons(&self.input_state);
        let mouse = self.input_state.mouse_position();
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

    fn emit_viewport(&self, buffer: &mut String) {
        let (width, height) = self.viewport.viewport_size();
        writeln!(
            buffer,
            "local __viewport = {{ width = {}, height = {} }}\n",
            width, height
        )
        .unwrap();
    }
}

impl Drop for LuaScriptManager {
    fn drop(&mut self) {
        let _ = self.stop();
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

const LUAU_HELPERS: &str = r#"
local __changes = {}

local function __copy_vec3(v)
    return { x = v.x, y = v.y, z = v.z }
end

local function __copy_color(v)
    return { x = v.x, y = v.y, z = v.z }
end

local function __record_change(name, field, value)
    __changes[#__changes + 1] = { object = name, field = field, value = value }
end

local function __vector3_table(v)
    return { X = v.x, Y = v.y, Z = v.z, x = v.x, y = v.y, z = v.z }
end

local function __color3_table(v)
    local r = v.x * 255
    local g = v.y * 255
    local b = v.z * 255
    return { R = r, G = g, B = b, r = r, g = g, b = b }
end

Vector3 = {}
function Vector3.new(x, y, z)
    return { X = x, Y = y, Z = z, x = x, y = y, z = z }
end

Vector2 = {}
function Vector2.new(x, y)
    return { X = x, Y = y, x = x, y = y }
end

Color3 = {}
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

local __object_cache = {}
local function __wrap_object(name)
    if __object_cache[name] then
        return __object_cache[name]
    end
    local data = __objects[name]
    if not data then
        return nil
    end
    local proxy = {}
    local meta = {}
    function meta.__index(_, key)
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
    __object_cache[name] = setmetatable(proxy, meta)
    return __object_cache[name]
end

scene = {}
setmetatable(scene, {
    __index = function(_, key)
        return __wrap_object(key)
    end
})

function scene.get(name)
    return __wrap_object(name)
end

function scene.names()
    local result = {}
    for index, name in ipairs(__object_order) do
        result[index] = name
    end
    return result
end

place = scene

service = { input = {} }

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
    return __input.keys[name] or false
end

function service.input:GetMousePosition()
    return Vector2.new(__input.mouse.x, __input.mouse.y)
end

screen = {}
function screen:GetViewportSize()
    return Vector2.new(__viewport.width, __viewport.height)
end

function wait(_)
    return
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

function __host_emit_changes()
    local parts = {}
    for i, change in ipairs(__changes) do
        parts[i] = '{"object":' .. __encode(change.object)
            .. ',"field":' .. __encode(change.field)
            .. ',"value":' .. __encode(change.value) .. '}'
    end
    print("__CHANGES__:[" .. table.concat(parts, ",") .. "]")
end
"#;
