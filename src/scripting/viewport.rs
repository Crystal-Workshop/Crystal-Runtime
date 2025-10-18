use std::sync::Arc;

/// Provides viewport dimensions for Lua scripts.
pub trait ViewportProvider: Send + Sync {
    fn viewport_size(&self) -> (u32, u32);
}

/// Simple viewport that always reports the same resolution.
#[derive(Debug, Clone, Copy)]
pub struct StaticViewport {
    pub width: u32,
    pub height: u32,
}

impl StaticViewport {
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

impl ViewportProvider for StaticViewport {
    fn viewport_size(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

impl<T> ViewportProvider for Arc<T>
where
    T: ViewportProvider + ?Sized,
{
    fn viewport_size(&self) -> (u32, u32) {
        (**self).viewport_size()
    }
}
