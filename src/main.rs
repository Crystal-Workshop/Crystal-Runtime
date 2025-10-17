use std::any::Any;
use std::env;
use std::fmt;
use std::panic::{self, AssertUnwindSafe};
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use glam::{Mat4, Vec2, Vec3};
use log::info;
use parking_lot::RwLock;
use pollster::block_on;
use winit::dpi::LogicalSize;
use winit::event::{
    ElementState, Event, KeyboardInput, MouseButton as WinitMouseButton, WindowEvent,
};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::platform::run_return::EventLoopExtRunReturn;
use winit::window::WindowBuilder;

use crystal_runtime::{
    CGameArchive, CameraParams, DataModel, InputState, KeyCode, LightParams, LuaScriptManager,
    NamedKey, Renderer, Scene, SceneObject, StaticViewport, ViewportProvider,
};

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    env_logger::init();
    if let Err(err) = run() {
        eprintln!("Error: {err:?}");
        std::process::exit(1);
    }
}

#[cfg(target_arch = "wasm32")]
fn main() {}

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

struct AppState {
    renderer: Renderer,
    data_model: DataModel,
    input: Arc<InputState>,
    viewport: Arc<WindowViewport>,
    script_manager: Option<LuaScriptManager>,
    last_error: Option<anyhow::Error>,
}

#[derive(Debug)]
struct WindowInitError {
    message: String,
}

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

impl fmt::Display for WindowInitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for WindowInitError {}

fn panic_message(panic: Box<dyn Any + Send>) -> String {
    match panic.downcast::<String>() {
        Ok(msg) => *msg,
        Err(panic) => match panic.downcast::<&'static str>() {
            Ok(msg) => (*msg).to_string(),
            Err(_) => "unknown panic".into(),
        },
    }
}

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
        let index = match button {
            WinitMouseButton::Left => 0,
            WinitMouseButton::Right => 1,
            WinitMouseButton::Middle => 2,
            WinitMouseButton::Other(value) => value,
        } as u8;
        let button = crystal_runtime::MouseButton::new(index);
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

fn print_final_state(model: &DataModel) {
    println!("Final object states:");
    for object in model.all_objects() {
        println!(
            " - {} pos=({:.2}, {:.2}, {:.2}) color=({:.2}, {:.2}, {:.2})",
            object.name,
            object.position.x,
            object.position.y,
            object.position.z,
            object.color.x,
            object.color.y,
            object.color.z
        );
    }
}

fn map_keycode(code: winit::event::VirtualKeyCode) -> Option<KeyCode> {
    use winit::event::VirtualKeyCode as Key;
    Some(match code {
        Key::Space => KeyCode::Named(NamedKey::Space),
        Key::Return => KeyCode::Named(NamedKey::Enter),
        Key::Tab => KeyCode::Named(NamedKey::Tab),
        Key::Left => KeyCode::Named(NamedKey::Left),
        Key::Right => KeyCode::Named(NamedKey::Right),
        Key::Up => KeyCode::Named(NamedKey::Up),
        Key::Down => KeyCode::Named(NamedKey::Down),
        Key::Escape => KeyCode::Named(NamedKey::Escape),
        Key::Back => KeyCode::Named(NamedKey::Backspace),
        Key::Home => KeyCode::Named(NamedKey::Home),
        Key::End => KeyCode::Named(NamedKey::End),
        Key::PageUp => KeyCode::Named(NamedKey::PageUp),
        Key::PageDown => KeyCode::Named(NamedKey::PageDown),
        Key::LShift => KeyCode::Named(NamedKey::LeftShift),
        Key::RShift => KeyCode::Named(NamedKey::RightShift),
        Key::LControl => KeyCode::Named(NamedKey::LeftCtrl),
        Key::RControl => KeyCode::Named(NamedKey::RightCtrl),
        Key::LAlt => KeyCode::Named(NamedKey::LeftAlt),
        Key::RAlt => KeyCode::Named(NamedKey::RightAlt),
        Key::Key0 => KeyCode::Digit(0),
        Key::Key1 => KeyCode::Digit(1),
        Key::Key2 => KeyCode::Digit(2),
        Key::Key3 => KeyCode::Digit(3),
        Key::Key4 => KeyCode::Digit(4),
        Key::Key5 => KeyCode::Digit(5),
        Key::Key6 => KeyCode::Digit(6),
        Key::Key7 => KeyCode::Digit(7),
        Key::Key8 => KeyCode::Digit(8),
        Key::Key9 => KeyCode::Digit(9),
        Key::A => KeyCode::Character('A'),
        Key::B => KeyCode::Character('B'),
        Key::C => KeyCode::Character('C'),
        Key::D => KeyCode::Character('D'),
        Key::E => KeyCode::Character('E'),
        Key::F => KeyCode::Character('F'),
        Key::G => KeyCode::Character('G'),
        Key::H => KeyCode::Character('H'),
        Key::I => KeyCode::Character('I'),
        Key::J => KeyCode::Character('J'),
        Key::K => KeyCode::Character('K'),
        Key::L => KeyCode::Character('L'),
        Key::M => KeyCode::Character('M'),
        Key::N => KeyCode::Character('N'),
        Key::O => KeyCode::Character('O'),
        Key::P => KeyCode::Character('P'),
        Key::Q => KeyCode::Character('Q'),
        Key::R => KeyCode::Character('R'),
        Key::S => KeyCode::Character('S'),
        Key::T => KeyCode::Character('T'),
        Key::U => KeyCode::Character('U'),
        Key::V => KeyCode::Character('V'),
        Key::W => KeyCode::Character('W'),
        Key::X => KeyCode::Character('X'),
        Key::Y => KeyCode::Character('Y'),
        Key::Z => KeyCode::Character('Z'),
        Key::F1 => KeyCode::Function(1),
        Key::F2 => KeyCode::Function(2),
        Key::F3 => KeyCode::Function(3),
        Key::F4 => KeyCode::Function(4),
        Key::F5 => KeyCode::Function(5),
        Key::F6 => KeyCode::Function(6),
        Key::F7 => KeyCode::Function(7),
        Key::F8 => KeyCode::Function(8),
        Key::F9 => KeyCode::Function(9),
        Key::F10 => KeyCode::Function(10),
        Key::F11 => KeyCode::Function(11),
        Key::F12 => KeyCode::Function(12),
        _ => return None,
    })
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

struct CliOptions {
    path: String,
    run_scripts: bool,
    summary_only: bool,
}

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
struct WindowViewport {
    size: RwLock<(u32, u32)>,
}

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

impl ViewportProvider for WindowViewport {
    fn viewport_size(&self) -> (u32, u32) {
        *self.size.read()
    }
}
