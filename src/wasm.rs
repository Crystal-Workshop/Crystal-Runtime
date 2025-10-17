#![cfg(target_arch = "wasm32")]

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use glam::{Mat4, Vec3};
use parking_lot::RwLock;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{window, HtmlCanvasElement};

use js_sys::Uint8Array;

use crate::input::wasm::WasmInputHandler;
use crate::render::{CameraParams, LightParams, Renderer};
use crate::{CGameArchive, DataModel, InputState, Scene, SceneObject, ViewportProvider};

#[wasm_bindgen(start)]
pub fn init_logging() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub struct WasmApp {
    inner: Rc<RefCell<AppState>>,
}

#[wasm_bindgen]
impl WasmApp {
    #[wasm_bindgen(constructor)]
    pub async fn new(canvas_id: String, archive_bytes: Uint8Array) -> Result<WasmApp, JsValue> {
        let archive_data = archive_bytes.to_vec();
        let archive = Arc::new(
            CGameArchive::from_bytes(&canvas_id, archive_data)
                .map_err(|err| JsValue::from_str(&err.to_string()))?,
        );

        let scene = Scene::from_xml(archive.scene_xml())
            .map_err(|err| JsValue::from_str(&format!("failed to parse scene XML: {err}")))?;
        let model = DataModel::from_objects(scene.objects.clone());
        let input = Arc::new(InputState::new());

        let window = window().ok_or_else(|| JsValue::from_str("window not available"))?;
        let document = window
            .document()
            .ok_or_else(|| JsValue::from_str("document not available"))?;
        let canvas = document
            .get_element_by_id(&canvas_id)
            .ok_or_else(|| JsValue::from_str("canvas element not found"))?
            .dyn_into::<HtmlCanvasElement>()
            .map_err(|_| JsValue::from_str("element is not a canvas"))?;

        let renderer = Renderer::new(canvas.clone(), Arc::clone(&archive))
            .await
            .map_err(|err| JsValue::from_str(&err.to_string()))?;
        let viewport = Arc::new(CanvasViewport::new(canvas.width(), canvas.height()));

        let input_handler = WasmInputHandler::attach(&canvas, Arc::clone(&input))
            .map_err(|err| JsValue::from_str(&err.to_string()))?;

        let state = AppState {
            archive,
            renderer,
            data_model: model,
            input,
            viewport,
            _input_handler: input_handler,
            animation_closure: None,
        };

        Ok(Self {
            inner: Rc::new(RefCell::new(state)),
        })
    }

    pub fn start(&self) -> Result<(), JsValue> {
        schedule_animation_loop(Rc::clone(&self.inner))
            .map_err(|err| JsValue::from_str(&err.to_string()))
    }
}

struct AppState {
    archive: Arc<CGameArchive>,
    renderer: Renderer,
    data_model: DataModel,
    input: Arc<InputState>,
    viewport: Arc<CanvasViewport>,
    _input_handler: WasmInputHandler,
    animation_closure: Option<Closure<dyn FnMut()>>,
}

impl AppState {
    fn render_frame(&mut self) -> Result<()> {
        let objects = self.data_model.all_objects();
        let aspect = if self.viewport.height() == 0 {
            1.0
        } else {
            self.viewport.width() as f32 / self.viewport.height() as f32
        };
        let camera = camera_from_objects(&objects, aspect);
        let light = light_from_objects(&objects);
        self.renderer.update_globals(&camera, &light);
        self.renderer.render(&objects).map_err(|err| {
            let message = err
                .as_string()
                .unwrap_or_else(|| "unknown canvas error".to_string());
            anyhow!("render failed: {message}")
        })?;
        Ok(())
    }
}

fn schedule_animation_loop(app: Rc<RefCell<AppState>>) -> Result<()> {
    let window = window().ok_or_else(|| anyhow!("window not available"))?;
    let mut state = app.borrow_mut();
    let app_clone = Rc::clone(&app);

    let closure = Closure::wrap(Box::new(move || {
        if let Err(err) = app_clone.borrow_mut().render_frame() {
            web_sys::console::error_1(&JsValue::from_str(&err.to_string()));
        }
        if let Err(err) = schedule_animation_loop(Rc::clone(&app_clone)) {
            web_sys::console::error_1(&JsValue::from_str(&err.to_string()));
        }
    }) as Box<dyn FnMut()>);

    window
        .request_animation_frame(closure.as_ref().unchecked_ref())
        .map_err(|err| anyhow!("requestAnimationFrame failed: {err:?}"))?;

    state.animation_closure = Some(closure);
    Ok(())
}

#[derive(Debug)]
struct CanvasViewport {
    size: RwLock<(u32, u32)>,
}

impl CanvasViewport {
    fn new(width: u32, height: u32) -> Self {
        Self {
            size: RwLock::new((width.max(1), height.max(1))),
        }
    }

    fn width(&self) -> u32 {
        self.size.read().0
    }

    fn height(&self) -> u32 {
        self.size.read().1
    }
}

impl ViewportProvider for CanvasViewport {
    fn viewport_size(&self) -> (u32, u32) {
        *self.size.read()
    }
}

fn camera_from_objects(objects: &[SceneObject], aspect: f32) -> CameraParams {
    let default_position = Vec3::new(0.0, 2.0, 6.0);
    let default_target = Vec3::ZERO;
    let (position, rotation, fov) = objects
        .iter()
        .find(|o| o.object_type == "camera")
        .map(|camera| (camera.position, camera.rotation, camera.fov))
        .unwrap_or((default_position, Vec3::ZERO, 60.0));

    let rotation_matrix = Mat4::from_rotation_z(rotation.z.to_radians())
        * Mat4::from_rotation_y(rotation.y.to_radians())
        * Mat4::from_rotation_x(rotation.x.to_radians());
    let forward = (rotation_matrix * Vec3::new(0.0, 0.0, -1.0).extend(0.0)).truncate();
    let up = (rotation_matrix * Vec3::Y.extend(0.0)).truncate();
    let target = if forward.length_squared() > f32::EPSILON {
        position + forward.normalize()
    } else {
        default_target
    };
    let view = Mat4::look_at_rh(position, target, up);
    let projection = Mat4::perspective_rh_gl(fov.to_radians(), aspect.max(0.01), 0.1, 100.0);
    CameraParams {
        view_proj: projection * view,
        position,
    }
}

fn light_from_objects(objects: &[SceneObject]) -> LightParams {
    objects
        .iter()
        .find(|o| o.object_type == "light")
        .map(|light| LightParams {
            position: light.position,
            color: light.color,
            intensity: light.intensity.max(0.1),
        })
        .unwrap_or(LightParams {
            position: Vec3::new(3.0, 5.0, -3.0),
            color: Vec3::splat(1.0),
            intensity: 1.0,
        })
}
