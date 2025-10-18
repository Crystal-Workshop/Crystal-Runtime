#![cfg(target_arch = "wasm32")]

use std::sync::Arc;

use glam::Vec2;
use parking_lot::RwLock;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, Event, KeyEvent, MouseButton as WinitMouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::platform::web::{EventLoopExtWebSys, WindowAttributesExtWebSys};
use winit::window::Window;

use crate::app::{
    camera_from_objects, light_from_objects, map_keycode, map_mouse_button, print_final_state,
};
use crate::{
    CGameArchive, DataModel, InputState, LuaScriptManager, Renderer, Scene, ViewportProvider,
};

#[wasm_bindgen]
pub async fn run(
    canvas_id: String,
    archive_bytes: js_sys::Uint8Array,
    run_scripts: bool,
) -> Result<(), JsValue> {
    console_error_panic_hook::set_once();

    let bytes = archive_bytes.to_vec();
    let archive = Arc::new(
        CGameArchive::from_bytes("wasm-scene", bytes)
            .map_err(|err| JsValue::from_str(&format!("failed to load archive: {err}")))?,
    );

    let scene = Scene::from_xml(archive.scene_xml())
        .map_err(|err| JsValue::from_str(&format!("failed to parse scene XML: {err}")))?;
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("missing window"))?;
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("missing document"))?;
    let element = document
        .get_element_by_id(&canvas_id)
        .ok_or_else(|| JsValue::from_str("canvas element not found"))?;
    let canvas: web_sys::HtmlCanvasElement = element
        .dyn_into()
        .map_err(|_| JsValue::from_str("element is not a canvas"))?;

    let event_loop = EventLoop::new()
        .map_err(|err| JsValue::from_str(&format!("failed to create event loop: {err}")))?;
    #[allow(deprecated)]
    let window = Arc::new(
        event_loop
            .create_window(
                Window::default_attributes()
                    .with_canvas(Some(canvas))
                    .with_title("Crystal Runtime")
                    .with_inner_size(LogicalSize::new(1280.0, 720.0)),
            )
            .map_err(|err| JsValue::from_str(&format!("window error: {err}")))?,
    );

    let renderer = Renderer::new(Arc::clone(&window), Arc::clone(&archive))
        .await
        .map_err(|err| JsValue::from_str(&format!("renderer error: {err}")))?;

    let viewport = Arc::new(WebViewport::new(
        window.inner_size().width,
        window.inner_size().height,
    ));
    let viewport_provider: Arc<dyn ViewportProvider + Send + Sync> = viewport.clone();

    let input = Arc::new(InputState::new());
    let data_model = DataModel::from_objects(scene.objects.clone());

    let script_manager = if run_scripts {
        let mut manager = LuaScriptManager::new(
            Arc::clone(&archive),
            data_model.clone(),
            Arc::clone(&input),
            viewport_provider,
        );
        let count = manager
            .start()
            .map_err(|err| JsValue::from_str(&format!("failed to launch scripts: {err}")))?;
        if count > 0 {
            log_to_console(&format!(
                "Lua scripts unavailable in wasm build (skipped {count})."
            ));
        }
        Some(manager)
    } else {
        None
    };

    log_scene_summary(&scene);

    let mut app = WebAppState {
        renderer,
        data_model,
        input,
        viewport,
        script_manager,
    };

    #[allow(deprecated)]
    event_loop.spawn(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Poll);
        if let Err(err) = app.process_event(&event, elwt) {
            log_to_console(&format!("Error: {err}"));
            elwt.exit();
        }
    });

    Ok(())
}

fn log_scene_summary(scene: &Scene) {
    let summary = format!(
        "Loaded scene with {} objects ({} lights)",
        scene.objects.len(),
        scene.lights.len()
    );
    log_to_console(&summary);
    for object in &scene.objects {
        log_to_console(&format!(" - {} ({})", object.name, object.object_type));
    }
}

fn log_to_console(message: &str) {
    web_sys::console::log_1(&JsValue::from_str(message));
}

struct WebAppState {
    renderer: Renderer,
    data_model: DataModel,
    input: Arc<InputState>,
    viewport: Arc<WebViewport>,
    script_manager: Option<LuaScriptManager>,
}

impl WebAppState {
    fn process_event(&mut self, event: &Event<()>, elwt: &ActiveEventLoop) -> Result<(), String> {
        match event {
            Event::WindowEvent { event, window_id } if *window_id == self.renderer.window_id() => {
                match event {
                    WindowEvent::CloseRequested => elwt.exit(),
                    WindowEvent::Resized(size) => {
                        self.renderer.resize(*size);
                        self.viewport.update(size.width, size.height);
                    }
                    WindowEvent::ScaleFactorChanged { .. } => {
                        let size = self.renderer.window().inner_size();
                        self.renderer.resize(size);
                        self.viewport.update(size.width, size.height);
                    }
                    WindowEvent::KeyboardInput { event, .. } => self.handle_keyboard(event),
                    WindowEvent::MouseInput { state, button, .. } => {
                        self.handle_mouse_button(*state, *button)
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        let pos = Vec2::new(position.x as f32, position.y as f32);
                        self.input.set_mouse_position(pos);
                    }
                    WindowEvent::RedrawRequested => {
                        let objects = self.data_model.all_objects();
                        let aspect = self.renderer_aspect();
                        let camera = camera_from_objects(&objects, aspect);
                        let light = light_from_objects(&objects);
                        self.renderer.update_globals(&camera, &light);
                        if let Err(err) = self.renderer.render(&objects) {
                            match err {
                                wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated => {
                                    let size = self.renderer.window().inner_size();
                                    self.renderer.resize(size);
                                }
                                wgpu::SurfaceError::OutOfMemory => {
                                    return Err("GPU is out of memory".to_string());
                                }
                                wgpu::SurfaceError::Timeout => {
                                    log_to_console("Surface timeout; retrying next frame");
                                }
                                wgpu::SurfaceError::Other => {
                                    log_to_console(
                                        "Surface reported an unknown error; retrying next frame",
                                    );
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            Event::AboutToWait => {
                self.renderer.window().request_redraw();
            }
            Event::LoopExiting => {
                self.shutdown();
            }
            _ => {}
        }
        Ok(())
    }

    fn renderer_aspect(&self) -> f32 {
        let size = self.renderer.window().inner_size();
        if size.height == 0 {
            1.0
        } else {
            size.width as f32 / size.height as f32
        }
    }

    fn handle_keyboard(&self, event: &KeyEvent) {
        let Some(keycode) = map_keycode(&event.physical_key) else {
            return;
        };
        if event.repeat {
            return;
        }
        match event.state {
            ElementState::Pressed => self.input.set_key_down(keycode),
            ElementState::Released => self.input.set_key_up(keycode),
        }
    }

    fn handle_mouse_button(&self, state: ElementState, button: WinitMouseButton) {
        let button = map_mouse_button(button);
        match state {
            ElementState::Pressed => self.input.set_mouse_button_down(button),
            ElementState::Released => self.input.set_mouse_button_up(button),
        }
    }

    fn shutdown(&mut self) {
        if let Some(manager) = self.script_manager.as_mut() {
            if let Err(err) = manager.stop() {
                log_to_console(&format!("Error stopping scripts: {err}"));
            }
        }
        print_final_state(&self.data_model);
    }
}

#[derive(Debug)]
struct WebViewport {
    size: RwLock<(u32, u32)>,
}

impl WebViewport {
    fn new(width: u32, height: u32) -> Self {
        Self {
            size: RwLock::new((width, height)),
        }
    }

    fn update(&self, width: u32, height: u32) {
        *self.size.write() = (width.max(1), height.max(1));
    }
}

impl ViewportProvider for WebViewport {
    fn viewport_size(&self) -> (u32, u32) {
        *self.size.read()
    }
}
