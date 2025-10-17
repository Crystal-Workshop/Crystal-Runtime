use std::sync::Arc;

use anyhow::{anyhow, Result};
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

use crate::{CGameArchive, SceneObject};

use super::common::{CameraParams, LightParams};

/// Minimal renderer backed by a 2D canvas for WebAssembly builds.
pub struct Renderer {
    canvas: HtmlCanvasElement,
    context: CanvasRenderingContext2d,
    archive: Arc<CGameArchive>,
    size: (u32, u32),
}

impl Renderer {
    /// Creates a renderer that draws into the provided HTML canvas element.
    pub async fn new(canvas: HtmlCanvasElement, archive: Arc<CGameArchive>) -> Result<Self> {
        let context = canvas
            .get_context("2d")
            .map_err(|err| anyhow!("failed to query canvas context: {err:?}"))?
            .ok_or_else(|| anyhow!("canvas does not support 2d context"))?
            .dyn_into::<CanvasRenderingContext2d>()
            .map_err(|_| anyhow!("failed to cast canvas context"))?;

        let size = (canvas.width(), canvas.height());
        Ok(Self {
            canvas,
            context,
            archive,
            size,
        })
    }

    /// Updates the canvas dimensions to match the browser layout.
    pub fn resize(&mut self, new_size: (u32, u32)) {
        if new_size.0 == 0 || new_size.1 == 0 {
            return;
        }
        self.size = new_size;
        self.canvas.set_width(new_size.0);
        self.canvas.set_height(new_size.1);
    }

    /// Updates the cached camera state. The 2D renderer currently ignores it but keeps the API.
    pub fn update_globals(&self, _camera: &CameraParams, _light: &LightParams) {}

    /// Renders the current scene snapshot using a simple orthographic projection.
    pub fn render(&mut self, objects: &[SceneObject]) -> Result<(), wasm_bindgen::JsValue> {
        self.clear_background();

        let count = objects.len() as f64;
        let width = self.size.0 as f64;
        let height = self.size.1 as f64;
        let bar_width = if count > 0.0 { width / count } else { width };

        for (index, object) in objects.iter().enumerate() {
            let hue = (index as f64 * 47.0) % 360.0;
            let color = format!("hsl({hue}, 60%, 55%)");
            self.context.set_fill_style(&color.into());
            let x = index as f64 * bar_width;
            let bar_height = (object.scale.length() * 25.0).clamp(10.0, height * 0.9);
            self.context
                .fill_rect(x, height - bar_height - 5.0, bar_width - 4.0, bar_height);
        }

        self.context.set_fill_style(&"white".into());
        let summary = format!(
            "Objects: {}  Lights: {}  Archive v{}",
            objects.len(),
            objects.iter().filter(|o| o.object_type == "light").count(),
            self.archive.version(),
        );
        let _ = self.context.fill_text(&summary, 10.0, 24.0);

        Ok(())
    }

    fn clear_background(&self) {
        self.context.set_fill_style(&"#06060a".into());
        self.context
            .fill_rect(0.0, 0.0, self.size.0 as f64, self.size.1 as f64);
    }
}
