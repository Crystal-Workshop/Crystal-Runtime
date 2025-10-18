#![cfg(target_arch = "wasm32")]

use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use glam::Vec2;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::{window, HtmlCanvasElement};
use winit::dpi::LogicalSize;
use winit::event::{
    ElementState, Event, KeyboardInput, MouseButton as WinitMouseButton, WindowEvent,
};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::platform::web::{EventLoopExtWebSys, WindowExtWebSys};
use winit::window::WindowBuilder;

use crate::app::{camera_from_objects, light_from_objects, print_final_state, WindowViewport};
use crate::input::{map_virtual_keycode, mouse_button_from_winit, InputState};
use crate::{CGameArchive, DataModel, LuaScriptManager, Renderer, Scene};

#[wasm_bindgen(start)]
pub fn bootstrap() {
    console_error_panic_hook::set_once();
    let _ = wasm_logger::init(wasm_logger::Config::default());
}

#[wasm_bindgen]
pub async fn run_scene(bytes: js_sys::Uint8Array, run_scripts: bool) -> Result<(), JsValue> {
    let data = bytes.to_vec();
    spawn_local(async move {
        if let Err(err) = run_runtime(data, run_scripts).await {
            log::error!("runtime error: {err:?}");
        }
    });
    Ok(())
}

async fn run_runtime(bytes: Vec<u8>, run_scripts: bool) -> Result<()> {
    let archive = Arc::new(CGameArchive::from_bytes(bytes)?);
    let scene = Scene::from_xml(archive.scene_xml()).context("failed to parse scene XML")?;

    log::info!(
        "Loaded scene with {} objects ({} lights)",
        scene.objects.len(),
        scene.lights.len()
    );
    for object in &scene.objects {
        log::info!(" - {} ({})", object.name, object.object_type);
    }

    let model = DataModel::from_objects(scene.objects.clone());
    let input = Arc::new(InputState::new());

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Crystal Runtime")
        .with_inner_size(LogicalSize::new(1280.0, 720.0))
        .build(&event_loop)
        .map_err(|err| anyhow!("failed to create window: {err}"))?;

    attach_canvas(&window)?;

    let renderer = Renderer::new(window, Arc::clone(&archive)).await?;
    let viewport = Arc::new(WindowViewport::new(1280, 720));
    let viewport_provider: Arc<dyn crate::scripting::ViewportProvider + Send + Sync> =
        viewport.clone();

    let script_manager = if run_scripts {
        log::info!("Starting Lua scripts...");
        let mut manager = LuaScriptManager::new(
            Arc::clone(&archive),
            model.clone(),
            Arc::clone(&input),
            viewport_provider,
        );
        let count = manager.start().context("failed to launch scripts")?;
        log::info!("Launched {count} script(s)");
        Some(manager)
    } else {
        None
    };

    let mut app = AppState {
        renderer,
        data_model: model,
        input,
        viewport,
        script_manager,
        last_error: None,
    };

    event_loop.spawn(move |event, _target, control_flow| {
        *control_flow = ControlFlow::Poll;
        if let Err(err) = app.process_event(event, control_flow) {
            log::error!("event processing error: {err:?}");
            app.last_error = Some(err);
            control_flow.set_exit();
        }
    });

    Ok(())
}

fn attach_canvas(winit_window: &winit::window::Window) -> Result<()> {
    let canvas: HtmlCanvasElement = winit_window.canvas();
    canvas.set_width(1280);
    canvas.set_height(720);
    let document = window()
        .and_then(|win| win.document())
        .ok_or_else(|| anyhow!("document not available"))?;
    let body = document
        .body()
        .ok_or_else(|| anyhow!("document has no body element"))?;
    if !body.contains(Some(canvas.as_ref())) {
        body.append_child(&canvas)
            .map_err(|err| anyhow!("failed to append canvas: {:?}", err))?;
    }
    Ok(())
}

struct AppState {
    renderer: Renderer,
    data_model: DataModel,
    input: Arc<InputState>,
    viewport: Arc<WindowViewport>,
    script_manager: Option<LuaScriptManager>,
    last_error: Option<anyhow::Error>,
}

impl AppState {
    fn process_event(&mut self, event: Event<()>, control_flow: &mut ControlFlow) -> Result<()> {
        match event {
            Event::WindowEvent { event, window_id } if window_id == self.renderer.window_id() => {
                match event {
                    WindowEvent::CloseRequested => control_flow.set_exit(),
                    WindowEvent::Resized(size) => {
                        self.renderer.resize(size);
                        self.viewport.update(size.width, size.height);
                    }
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        let size = *new_inner_size;
                        self.renderer.resize(size);
                        self.viewport
                            .update(new_inner_size.width, new_inner_size.height);
                    }
                    WindowEvent::KeyboardInput { input, .. } => {
                        self.handle_keyboard(input);
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        let pos = Vec2::new(position.x as f32, position.y as f32);
                        self.input.set_mouse_position(pos);
                    }
                    WindowEvent::MouseInput { state, button, .. } => {
                        self.handle_mouse_button(state, button);
                    }
                    _ => {}
                }
            }
            Event::RedrawRequested(window_id) if window_id == self.renderer.window_id() => {
                self.draw_frame()?;
            }
            Event::MainEventsCleared => {
                if let Some(manager) = self.script_manager.as_mut() {
                    if let Err(err) = manager.update() {
                        self.last_error = Some(err);
                        control_flow.set_exit();
                    }
                }
                self.renderer.window().request_redraw();
            }
            Event::LoopDestroyed => {
                self.shutdown();
                if let Some(err) = self.last_error.take() {
                    log::error!("application error: {err:?}");
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn draw_frame(&mut self) -> Result<()> {
        let objects = self.data_model.all_objects();
        let aspect = self.renderer_aspect();
        let camera = camera_from_objects(&objects, aspect);
        let light = light_from_objects(&objects);
        self.renderer.update_globals(&camera, &light);
        match self.renderer.render(&objects) {
            Ok(()) => Ok(()),
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                let size = self.renderer.window().inner_size();
                self.renderer.resize(size);
                Ok(())
            }
            Err(wgpu::SurfaceError::OutOfMemory) => Err(anyhow!("GPU is out of memory")),
            Err(wgpu::SurfaceError::Timeout) => {
                log::warn!("Surface timeout; retrying next frame");
                Ok(())
            }
        }
    }

    fn renderer_aspect(&self) -> f32 {
        let size = self.renderer.window().inner_size();
        if size.height == 0 {
            1.0
        } else {
            size.width as f32 / size.height as f32
        }
    }

    fn handle_keyboard(&self, input: KeyboardInput) {
        let Some(keycode) = input.virtual_keycode.and_then(map_virtual_keycode) else {
            return;
        };
        match input.state {
            ElementState::Pressed => self.input.set_key_down(keycode),
            ElementState::Released => self.input.set_key_up(keycode),
        }
    }

    fn handle_mouse_button(&self, state: ElementState, button: WinitMouseButton) {
        let button = mouse_button_from_winit(button);
        match state {
            ElementState::Pressed => self.input.set_mouse_button_down(button),
            ElementState::Released => self.input.set_mouse_button_up(button),
        }
    }

    fn shutdown(&mut self) {
        if let Some(manager) = self.script_manager.as_mut() {
            if let Err(err) = manager.stop() {
                log::error!("Error stopping scripts: {err:?}");
            }
        }
        print_final_state(&self.data_model);
    }
}
