#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(not(target_arch = "wasm32"))]
use std::any::Any;
#[cfg(not(target_arch = "wasm32"))]
use std::env;
#[cfg(not(target_arch = "wasm32"))]
use std::fmt;
#[cfg(not(target_arch = "wasm32"))]
use std::panic::{self, AssertUnwindSafe};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
use anyhow::{anyhow, Context, Result};
#[cfg(not(target_arch = "wasm32"))]
use glam::Vec2;
#[cfg(not(target_arch = "wasm32"))]
use log::info;
#[cfg(not(target_arch = "wasm32"))]
use parking_lot::RwLock;
#[cfg(not(target_arch = "wasm32"))]
use pollster::block_on;
#[cfg(not(target_arch = "wasm32"))]
use winit::dpi::LogicalSize;
#[cfg(not(target_arch = "wasm32"))]
use winit::event::{
    ElementState, Event, KeyboardInput, MouseButton as WinitMouseButton, WindowEvent,
};
#[cfg(not(target_arch = "wasm32"))]
use winit::event_loop::{ControlFlow, EventLoop};
#[cfg(not(target_arch = "wasm32"))]
use winit::platform::run_return::EventLoopExtRunReturn;
#[cfg(not(target_arch = "wasm32"))]
use winit::window::WindowBuilder;

#[cfg(not(target_arch = "wasm32"))]
use crystal_runtime::{
    app::{
        camera_from_objects, light_from_objects, map_keycode, map_mouse_button, print_final_state,
    },
    CGameArchive, DataModel, InputState, LuaScriptManager, Renderer, Scene, StaticViewport,
    ViewportProvider,
};

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    env_logger::init();
    if let Err(err) = run() {
        eprintln!("Error: {err:?}");
        std::process::exit(1);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn run() -> Result<()> {
    let options = CliOptions::parse()?;
    let archive = Arc::new(
        CGameArchive::open(&options.path)
            .with_context(|| format!("failed to open archive {}", options.path))?,
    );
    let scene = Scene::from_xml(archive.scene_xml()).context("failed to parse scene XML")?;

    println!(
        "Loaded scene with {} objects ({} lights)",
        scene.objects.len(),
        scene.lights.len()
    );
    for object in &scene.objects {
        println!(" - {} ({})", object.name, object.object_type);
    }

    let model = DataModel::from_objects(scene.objects.clone());
    let input = Arc::new(InputState::new());

    if options.summary_only {
        run_headless(archive, model, input, options.run_scripts)
    } else {
        let headless_archive = Arc::clone(&archive);
        let headless_model = model.clone();
        let headless_input = Arc::clone(&input);
        match run_interactive(archive, model, input, options.run_scripts) {
            Ok(()) => Ok(()),
            Err(err) => {
                if err.downcast_ref::<WindowInitError>().is_some() {
                    eprintln!(
                        "{err}. Falling back to --summary-only mode (set DISPLAY or install X11 libs to enable rendering)."
                    );
                    run_headless(
                        headless_archive,
                        headless_model,
                        headless_input,
                        options.run_scripts,
                    )
                } else {
                    Err(err)
                }
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn run_headless(
    archive: Arc<CGameArchive>,
    model: DataModel,
    input: Arc<InputState>,
    run_scripts: bool,
) -> Result<()> {
    if run_scripts {
        println!("Starting Lua scripts...");
        let viewport: Arc<dyn ViewportProvider + Send + Sync> =
            Arc::new(StaticViewport::new(1280, 720));
        let mut manager = LuaScriptManager::new(
            Arc::clone(&archive),
            model.clone(),
            Arc::clone(&input),
            viewport,
        );
        let count = manager.start().context("failed to launch scripts")?;
        println!("Launched {count} script(s)");
        manager.wait().context("script execution failed")?;
    }

    print_final_state(&model);
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn run_interactive(
    archive: Arc<CGameArchive>,
    model: DataModel,
    input: Arc<InputState>,
    run_scripts: bool,
) -> Result<()> {
    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(|_| {}));
    let event_loop = panic::catch_unwind(AssertUnwindSafe(EventLoop::new));
    panic::set_hook(default_hook);
    let event_loop =
        event_loop.map_err(|panic| WindowInitError::from_panic("event loop", panic))?;
    let window = Arc::new(
        WindowBuilder::new()
            .with_title("Crystal Runtime")
            .with_inner_size(LogicalSize::new(1280.0, 720.0))
            .build(&event_loop)
            .map_err(|err| WindowInitError::from_error("window", err))?,
    );

    let renderer = block_on(Renderer::new(Arc::clone(&window), Arc::clone(&archive)))?;
    let viewport = Arc::new(WindowViewport::new(
        window.inner_size().width,
        window.inner_size().height,
    ));
    let viewport_provider: Arc<dyn ViewportProvider + Send + Sync> = viewport.clone();

    let script_manager = if run_scripts {
        println!("Starting Lua scripts...");
        let mut manager = LuaScriptManager::new(
            Arc::clone(&archive),
            model.clone(),
            Arc::clone(&input),
            viewport_provider,
        );
        let count = manager.start().context("failed to launch scripts")?;
        println!("Launched {count} script(s)");
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

    let mut event_loop = event_loop;
    event_loop.run_return(|event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        if let Err(err) = app.process_event(&event, control_flow) {
            app.last_error = Some(err);
            control_flow.set_exit();
        }
    });

    app.shutdown();

    if let Some(err) = app.last_error {
        return Err(err);
    }

    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
struct AppState {
    renderer: Renderer,
    data_model: DataModel,
    input: Arc<InputState>,
    viewport: Arc<WindowViewport>,
    script_manager: Option<LuaScriptManager>,
    last_error: Option<anyhow::Error>,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug)]
struct WindowInitError {
    message: String,
}

#[cfg(not(target_arch = "wasm32"))]
impl WindowInitError {
    fn from_panic(stage: &str, panic: Box<dyn Any + Send>) -> Self {
        Self {
            message: format!("failed to initialize {stage}: {}", panic_message(panic)),
        }
    }

    fn from_error(stage: &str, err: impl fmt::Display) -> Self {
        Self {
            message: format!("failed to initialize {stage}: {err}"),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl fmt::Display for WindowInitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl std::error::Error for WindowInitError {}

#[cfg(not(target_arch = "wasm32"))]
fn panic_message(panic: Box<dyn Any + Send>) -> String {
    match panic.downcast::<String>() {
        Ok(msg) => *msg,
        Err(panic) => match panic.downcast::<&'static str>() {
            Ok(msg) => (*msg).to_string(),
            Err(_) => "unknown panic".into(),
        },
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl AppState {
    fn process_event(&mut self, event: &Event<()>, control_flow: &mut ControlFlow) -> Result<()> {
        match event {
            Event::WindowEvent { event, window_id } if *window_id == self.renderer.window_id() => {
                match event {
                    WindowEvent::CloseRequested => {
                        control_flow.set_exit();
                    }
                    WindowEvent::Resized(size) => {
                        self.renderer.resize(*size);
                        self.viewport.update(size.width, size.height);
                    }
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        self.renderer.resize(**new_inner_size);
                        self.viewport
                            .update(new_inner_size.width, new_inner_size.height);
                    }
                    WindowEvent::KeyboardInput { input, .. } => {
                        self.handle_keyboard(input);
                    }
                    WindowEvent::MouseInput { state, button, .. } => {
                        self.handle_mouse_button(*state, *button);
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        let pos = Vec2::new(position.x as f32, position.y as f32);
                        self.input.set_mouse_position(pos);
                    }
                    _ => {}
                }
            }
            Event::RedrawRequested(window_id) if *window_id == self.renderer.window_id() => {
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
                            return Err(anyhow!("GPU is out of memory"));
                        }
                        wgpu::SurfaceError::Timeout => {
                            info!("Surface timeout; retrying next frame");
                        }
                    }
                }
            }
            Event::MainEventsCleared => {
                self.renderer.window().request_redraw();
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

    fn handle_keyboard(&self, input: &KeyboardInput) {
        let Some(keycode) = input.virtual_keycode.and_then(map_keycode) else {
            return;
        };
        match input.state {
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
                eprintln!("Error stopping scripts: {err:?}");
            }
        }
        print_final_state(&self.data_model);
    }
}

#[cfg(not(target_arch = "wasm32"))]
struct CliOptions {
    path: String,
    run_scripts: bool,
    summary_only: bool,
}

#[cfg(not(target_arch = "wasm32"))]
impl CliOptions {
    fn parse() -> Result<Self> {
        let mut args = env::args().skip(1);
        let Some(path) = args.next() else {
            return Err(anyhow!(
                "Usage: crystal-runtime <scene.cgame> [--run-scripts] [--summary-only]"
            ));
        };
        let mut run_scripts = false;
        let mut summary_only = false;
        for arg in args {
            match arg.as_str() {
                "--run-scripts" => run_scripts = true,
                "--summary-only" => summary_only = true,
                other => {
                    return Err(anyhow!(
                        "Unknown argument: {other}. Expected --run-scripts or --summary-only"
                    ));
                }
            }
        }
        Ok(Self {
            path,
            run_scripts,
            summary_only,
        })
    }
}

#[derive(Debug)]
#[cfg(not(target_arch = "wasm32"))]
struct WindowViewport {
    size: RwLock<(u32, u32)>,
}

#[cfg(not(target_arch = "wasm32"))]
impl WindowViewport {
    fn new(width: u32, height: u32) -> Self {
        Self {
            size: RwLock::new((width, height)),
        }
    }

    fn update(&self, width: u32, height: u32) {
        *self.size.write() = (width.max(1), height.max(1));
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl ViewportProvider for WindowViewport {
    fn viewport_size(&self) -> (u32, u32) {
        *self.size.read()
    }
}
