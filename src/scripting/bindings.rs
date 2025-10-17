use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use glam::{Vec2, Vec3};
use mlua::{
    FromLua, IntoLua, Lua, MultiValue, Result as LuaResult, Table, UserData, UserDataFields,
    UserDataMethods, Value, Variadic,
};

use crate::data_model::DataModel;
use crate::input::InputState;

use super::native::ViewportProvider;

pub(super) struct ScriptContext {
    pub data_model: DataModel,
    pub input_state: Arc<InputState>,
    pub viewport: Arc<dyn ViewportProvider + Send + Sync>,
    pub running: Arc<AtomicBool>,
}

impl ScriptContext {
    pub fn new(
        data_model: DataModel,
        input_state: Arc<InputState>,
        viewport: Arc<dyn ViewportProvider + Send + Sync>,
        running: Arc<AtomicBool>,
    ) -> Self {
        Self {
            data_model,
            input_state,
            viewport,
            running,
        }
    }
}

impl Clone for ScriptContext {
    fn clone(&self) -> Self {
        Self {
            data_model: self.data_model.clone(),
            input_state: Arc::clone(&self.input_state),
            viewport: Arc::clone(&self.viewport),
            running: Arc::clone(&self.running),
        }
    }
}

pub(super) fn register_globals(lua: &Lua, context: &ScriptContext) -> LuaResult<()> {
    println!("Registering Globals");
    register_print(lua)?;
    register_wait(lua, Arc::clone(&context.running))?;
    register_datatypes(lua)?;
    register_scene(lua, context)?;
    register_service(lua, context)?;
    register_screen(lua, context)?;
    Ok(())
}

fn register_datatypes(lua: &Lua) -> LuaResult<()> {
    let vector3 = lua.create_table()?;
    vector3.set(
        "new",
        lua.create_function(|_, (x, y, z): (f32, f32, f32)| {
            Ok(LuaVector3::new(Vec3::new(x, y, z)))
        })?,
    )?;
    lua.globals().set("Vector3", vector3)?;

    let color3 = lua.create_table()?;
    color3.set(
        "new",
        lua.create_function(|_, (r, g, b): (f32, f32, f32)| Ok(LuaColor3::from_rgb(r, g, b)))?,
    )?;
    lua.globals().set("Color3", color3)?;

    Ok(())
}

fn register_print(lua: &Lua) -> LuaResult<()> {
    println!("Registering print with a script");
    let print = lua.create_function(|lua, values: Variadic<Value>| {
        println!("Print Called");
        let mut out = Vec::new();
        for value in values.iter() {
            let text = match value {
                Value::Nil => "nil".to_string(),
                Value::Boolean(b) => b.to_string(),
                Value::String(s) => s.to_str()?.to_string(),
                _ => match lua.coerce_string(value.clone())? {
                    Some(s) => s.to_str()?.to_string(),
                    None => format!("{:?}", value),
                },
            };
            out.push(text);
        }
        println!("[Lua] {}", out.join("\t"));
        Ok(())
    })?;
    lua.globals().set("print", print)?;
    Ok(())
}

fn register_wait(lua: &Lua, running: Arc<AtomicBool>) -> LuaResult<()> {
    let wait_running = Arc::clone(&running);
    let wait = lua.create_function(move |_, millis: Option<u64>| {
        let mut remaining = millis.unwrap_or(0);
        if remaining == 0 {
            std::thread::yield_now();
            return Ok(());
        }
        const CHUNK: u64 = 10;
        while remaining > 0 {
            if !wait_running.load(Ordering::Acquire) {
                return Err(mlua::Error::RuntimeError("wait interrupted".into()));
            }
            let sleep = remaining.min(CHUNK);
            std::thread::sleep(Duration::from_millis(sleep));
            remaining -= sleep;
        }
        Ok(())
    })?;
    lua.globals().set("wait", wait)?;
    Ok(())
}

fn register_scene(lua: &Lua, context: &ScriptContext) -> LuaResult<()> {
    let globals = lua.globals();
    let table = lua.create_table()?;

    // Clone context for __index lookup
    let get_context = context.clone();

    // Create the __index function
    let index_fn = lua.create_function(move |lua, (_scene, key): (Table, String)| {
        if key.is_empty() || get_context.data_model.get(&key).is_none() {
            return Ok(Value::Nil);
        }
        let object = PlaceObject::new(get_context.data_model.clone(), key);
        let userdata = lua.create_userdata(object)?;
        Ok(Value::UserData(userdata))
    })?;

    let metatable = lua.create_table()?;
    metatable.set("__index", index_fn)?;
    table.set_metatable(Some(metatable));

    let get_fn_context = context.clone();
    let get_fn = lua.create_function(move |lua, args: MultiValue| {
        let key = args
            .iter()
            .find_map(|value| match value {
                Value::String(s) => Some(s.to_str().map(|s| s.to_string())),
                _ => None,
            })
            .transpose()?;

        let Some(key) = key else {
            return Err(mlua::Error::FromLuaConversionError {
                from: "value",
                to: "string",
                message: Some("expected object name".into()),
            });
        };

        if key.is_empty() || get_fn_context.data_model.get(&key).is_none() {
            return Ok(Value::Nil);
        }
        let object = PlaceObject::new(get_fn_context.data_model.clone(), key);
        let userdata = lua.create_userdata(object)?;
        Ok(Value::UserData(userdata))
    })?;
    table.set("get", get_fn)?;

    // Keep your existing `names` function
    let names_context = context.clone();
    let names = lua.create_function(move |lua, ()| {
        let names: Vec<String> = names_context
            .data_model
            .all_objects()
            .into_iter()
            .map(|object| object.name)
            .collect();
        let result = lua.create_table_with_capacity(names.len(), 0)?;
        for (index, name) in names.into_iter().enumerate() {
            result.set(index + 1, name)?;
        }
        Ok::<_, mlua::Error>(result)
    })?;
    table.set("names", names)?;

    globals.set("scene", table.clone())?;
    globals.set("place", table)?;
    Ok(())
}

fn register_service(lua: &Lua, context: &ScriptContext) -> LuaResult<()> {
    let globals = lua.globals();
    let service = lua.create_table()?;
    let input_table = lua.create_table()?;

    let input_state = Arc::clone(&context.input_state);
    let get_key_down = lua.create_function(move |_, args: MultiValue| {
        if let Some(name) = string_argument(&args)? {
            Ok(input_state.is_key_down_by_name(&name))
        } else {
            Ok(false)
        }
    })?;
    input_table.set("GetKeyDown", get_key_down)?;

    let input_state = Arc::clone(&context.input_state);
    let get_mouse_position = lua.create_function(move |lua, _args: MultiValue| {
        let pos = input_state.mouse_position();
        LuaVec2(pos).into_lua(lua)
    })?;
    input_table.set("GetMousePosition", get_mouse_position)?;

    service.set("input", input_table)?;
    globals.set("service", service)?;
    Ok(())
}

fn register_screen(lua: &Lua, context: &ScriptContext) -> LuaResult<()> {
    let globals = lua.globals();
    let screen = lua.create_table()?;
    let viewport = Arc::clone(&context.viewport);
    let get_viewport_size = lua.create_function(move |lua, _args: MultiValue| {
        let (width, height) = viewport.viewport_size();
        LuaVec2(Vec2::new(width as f32, height as f32)).into_lua(lua)
    })?;
    screen.set("GetViewportSize", get_viewport_size)?;
    globals.set("screen", screen)?;
    Ok(())
}

fn string_argument(values: &MultiValue) -> LuaResult<Option<String>> {
    for value in values.iter() {
        if let Value::String(s) = value {
            return Ok(Some(s.to_str()?.to_string()));
        }
    }
    Ok(None)
}

struct PlaceObject {
    data_model: DataModel,
    name: String,
}

impl PlaceObject {
    fn new(data_model: DataModel, name: String) -> Self {
        Self { data_model, name }
    }
}

impl UserData for PlaceObject {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("name", |_, this| Ok(this.name.clone()));
        fields.add_field_method_get("position", |lua, this| {
            if let Some(object) = this.data_model.get(&this.name) {
                Ok(Value::UserData(
                    lua.create_userdata(LuaVector3::new(object.position))?,
                ))
            } else {
                Ok(Value::Nil)
            }
        });
        fields.add_field_method_get("rotation", |lua, this| {
            if let Some(object) = this.data_model.get(&this.name) {
                Ok(Value::UserData(
                    lua.create_userdata(LuaVector3::new(object.rotation))?,
                ))
            } else {
                Ok(Value::Nil)
            }
        });
        fields.add_field_method_get("scale", |lua, this| {
            if let Some(object) = this.data_model.get(&this.name) {
                Ok(Value::UserData(
                    lua.create_userdata(LuaVector3::new(object.scale))?,
                ))
            } else {
                Ok(Value::Nil)
            }
        });
        fields.add_field_method_get("color", |lua, this| {
            if let Some(object) = this.data_model.get(&this.name) {
                Ok(Value::UserData(lua.create_userdata(
                    LuaColor3::from_normalized(object.color),
                )?))
            } else {
                Ok(Value::Nil)
            }
        });
        fields.add_field_method_get("fov", |_, this| {
            Ok(this.data_model.get(&this.name).map(|object| object.fov))
        });
        fields.add_field_method_get("intensity", |_, this| {
            Ok(this
                .data_model
                .get(&this.name)
                .map(|object| object.intensity))
        });

        fields.add_field_method_set("position", |_, this, value: LuaVector3| {
            this.data_model.set_position(&this.name, value.as_vec3());
            Ok(())
        });
        fields.add_field_method_set("rotation", |_, this, value: LuaVector3| {
            this.data_model.set_rotation(&this.name, value.as_vec3());
            Ok(())
        });
        fields.add_field_method_set("scale", |_, this, value: LuaVector3| {
            this.data_model.set_scale(&this.name, value.as_vec3());
            Ok(())
        });
        fields.add_field_method_set("color", |_, this, value: LuaColor3| {
            this.data_model.set_color(&this.name, value.as_vec3());
            Ok(())
        });
        fields.add_field_method_set("fov", |_, this, value: f32| {
            this.data_model.set_fov(&this.name, value);
            Ok(())
        });
        fields.add_field_method_set("intensity", |_, this, value: f32| {
            this.data_model.set_intensity(&this.name, value);
            Ok(())
        });
    }

    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(_methods: &mut M) {}
}

#[derive(Debug, Clone, Copy)]
struct LuaVector3(Vec3);

impl LuaVector3 {
    fn new(inner: Vec3) -> Self {
        Self(inner)
    }

    fn as_vec3(self) -> Vec3 {
        self.0
    }
}

impl UserData for LuaVector3 {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("X", |_, this| Ok(this.0.x));
        fields.add_field_method_get("Y", |_, this| Ok(this.0.y));
        fields.add_field_method_get("Z", |_, this| Ok(this.0.z));
        fields.add_field_method_get("x", |_, this| Ok(this.0.x));
        fields.add_field_method_get("y", |_, this| Ok(this.0.y));
        fields.add_field_method_get("z", |_, this| Ok(this.0.z));
    }
}

impl<'lua> FromLua<'lua> for LuaVector3 {
    fn from_lua(value: Value<'lua>, _: &'lua Lua) -> LuaResult<Self> {
        match value {
            Value::Table(table) => Ok(Self(table_to_vec3(&table)?)),
            Value::UserData(ud) => ud.borrow::<LuaVector3>().map(|vec| *vec),
            _ => Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "Vector3",
                message: Some("expected Vector3 userdata or table".into()),
            }),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct LuaColor3(Vec3);

impl LuaColor3 {
    fn from_rgb(r: f32, g: f32, b: f32) -> Self {
        Self(Vec3::new(r, g, b) / 255.0)
    }

    fn from_normalized(color: Vec3) -> Self {
        Self(color)
    }

    fn as_vec3(self) -> Vec3 {
        self.0
    }
}

impl UserData for LuaColor3 {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("R", |_, this| Ok(this.0.x * 255.0));
        fields.add_field_method_get("G", |_, this| Ok(this.0.y * 255.0));
        fields.add_field_method_get("B", |_, this| Ok(this.0.z * 255.0));
        fields.add_field_method_get("r", |_, this| Ok(this.0.x * 255.0));
        fields.add_field_method_get("g", |_, this| Ok(this.0.y * 255.0));
        fields.add_field_method_get("b", |_, this| Ok(this.0.z * 255.0));
    }
}

impl<'lua> FromLua<'lua> for LuaColor3 {
    fn from_lua(value: Value<'lua>, _: &'lua Lua) -> LuaResult<Self> {
        match value {
            Value::Table(table) => Ok(Self(table_to_vec3(&table)?)),
            Value::UserData(ud) => {
                if let Ok(color) = ud.borrow::<LuaColor3>() {
                    Ok(*color)
                } else if let Ok(vec) = ud.borrow::<LuaVector3>() {
                    Ok(Self(vec.as_vec3()))
                } else {
                    Err(mlua::Error::FromLuaConversionError {
                        from: "userdata",
                        to: "Color3",
                        message: Some("unexpected userdata".into()),
                    })
                }
            }
            _ => Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "Color3",
                message: Some("expected Color3 userdata or table".into()),
            }),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct LuaVec2(Vec2);

impl<'lua> IntoLua<'lua> for LuaVec2 {
    fn into_lua(self, lua: &'lua Lua) -> LuaResult<Value<'lua>> {
        let table = lua.create_table()?;
        table.set("x", self.0.x)?;
        table.set("y", self.0.y)?;
        Ok(Value::Table(table))
    }
}

impl<'lua> FromLua<'lua> for LuaVec2 {
    fn from_lua(value: Value<'lua>, _: &'lua Lua) -> LuaResult<Self> {
        match value {
            Value::Table(table) => Ok(Self(table_to_vec2(&table)?)),
            _ => Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "Vector2",
                message: Some("expected table".into()),
            }),
        }
    }
}

fn table_to_vec3(table: &Table) -> LuaResult<Vec3> {
    Ok(Vec3::new(
        table_component(table, "x", 1)?,
        table_component(table, "y", 2)?,
        table_component(table, "z", 3)?,
    ))
}

fn table_to_vec2(table: &Table) -> LuaResult<Vec2> {
    Ok(Vec2::new(
        table_component(table, "x", 1)?,
        table_component(table, "y", 2)?,
    ))
}

fn table_component(table: &Table, key: &str, index: i32) -> LuaResult<f32> {
    if let Ok(value) = table.get::<_, f32>(key) {
        return Ok(value);
    }
    table.get::<_, f32>(index)
}

#[cfg(test)]
mod tests {
    use super::super::native::{StaticViewport, ViewportProvider};
    use super::*;
    use crate::data_model::DataModel;
    use crate::input::{InputState, KeyCode, MouseButton, NamedKey};
    use crate::scene::SceneObject;
    use glam::{Vec2, Vec3};
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    #[test]
    fn place_object_getters_and_setters_update_data_model() {
        let lua = Lua::new();
        let object = SceneObject {
            name: "Cube".into(),
            ..SceneObject::default()
        };
        let model = DataModel::from_objects(vec![object]);
        let input = Arc::new(InputState::new());
        let viewport: Arc<dyn ViewportProvider + Send + Sync> =
            Arc::new(StaticViewport::new(640, 480));
        let running = Arc::new(AtomicBool::new(true));
        let context = ScriptContext::new(model.clone(), input, viewport, running);
        register_globals(&lua, &context).unwrap();

        let (pos_x, color_y, names_len): (f32, f32, i64) = lua
            .load(
                r#"
                local cube = place.get("Cube")
                assert(cube ~= nil, "cube should exist")
                cube.position = Vector3.new(1.0, 2.0, 3.0)
                cube.color = Color3.new(128, 64, 0)
                local names = place.names()
                local color = cube.color
                local position = cube.position
                return position.X, color.G, #names
            "#,
            )
            .eval()
            .unwrap();

        assert!((pos_x - 1.0).abs() < f32::EPSILON);
        assert!((color_y - 64.0).abs() < f32::EPSILON);
        assert_eq!(names_len, 1);

        let updated = model.get("Cube").unwrap();
        assert_eq!(updated.position, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(updated.color, Vec3::new(128.0 / 255.0, 64.0 / 255.0, 0.0));
    }

    #[test]
    fn service_tables_report_input_and_viewport_state() {
        let lua = Lua::new();
        let model = DataModel::new();
        let input = Arc::new(InputState::new());
        input.set_key_down(KeyCode::Named(NamedKey::Space));
        input.set_mouse_button_down(MouseButton::new(1));
        input.set_mouse_position(Vec2::new(640.0, 360.0));
        let viewport: Arc<dyn ViewportProvider + Send + Sync> =
            Arc::new(StaticViewport::new(1920, 1080));
        let running = Arc::new(AtomicBool::new(true));
        let context = ScriptContext::new(model, Arc::clone(&input), viewport, running);
        register_globals(&lua, &context).unwrap();

        let (space_down, mouse_down, mouse_x, mouse_y, width, height, unknown): (
            bool,
            bool,
            f32,
            f32,
            f32,
            f32,
            bool,
        ) = lua
            .load(
                r#"
                local mouse = service.input.GetMousePosition()
                local viewport = screen.GetViewportSize()
                return service.input.GetKeyDown("Space"),
                       service.input.GetKeyDown("Mouse2"),
                       mouse.x, mouse.y,
                       viewport.x, viewport.y,
                       service.input.GetKeyDown("Unknown")
            "#,
            )
            .eval()
            .unwrap();

        assert!(space_down);
        assert!(mouse_down);
        assert_eq!(mouse_x, 640.0);
        assert_eq!(mouse_y, 360.0);
        assert_eq!(width, 1920.0);
        assert_eq!(height, 1080.0);
        assert!(!unknown);
    }

    #[test]
    fn wait_function_reports_stop_request() {
        let lua = Lua::new();
        let model = DataModel::new();
        let input = Arc::new(InputState::new());
        let viewport: Arc<dyn ViewportProvider + Send + Sync> =
            Arc::new(StaticViewport::new(800, 600));
        let running = Arc::new(AtomicBool::new(false));
        let context = ScriptContext::new(model, input, viewport, Arc::clone(&running));
        register_globals(&lua, &context).unwrap();

        let (ok, message): (bool, String) = lua
            .load(
                r#"
                local success, err = pcall(function()
                    wait(20)
                end)
                if success then
                    return true, ""
                else
                    return false, tostring(err)
                end
            "#,
            )
            .eval()
            .unwrap();

        assert!(!ok);
        assert!(message.contains("wait interrupted"));
    }
}
