//! 每个覆盖层窗口对应的 OpenGL 上下文 + surface 封装。
//!
//! 多窗口架构下，每个窗口拥有独立的 glutin 上下文，绘制前需 `make_current`。

use std::ffi::CStr;
use std::ffi::c_void;
use std::num::NonZeroU32;
use std::sync::Arc;

use glutin::context::NotCurrentGlContext as _;
use glutin::context::PossiblyCurrentContext;
use glutin::context::PossiblyCurrentGlContext as _;
use glutin::display::Display;
use glutin::display::GetGlDisplay as _;
use glutin::display::GlDisplay as _;
use glutin::prelude::GlSurface as _;
use glutin::surface::Surface;
use glutin::surface::WindowSurface;
use winit::dpi::PhysicalSize;
use winit::event_loop::ActiveEventLoop;
use winit::raw_window_handle::HasWindowHandle as _;
use winit::window::Window;
use winit::window::WindowAttributes;

pub struct GlWindow {
    window: Arc<Window>,
    gl_context: PossiblyCurrentContext,
    gl_display: Display,
    gl_surface: Surface<WindowSurface>,
}

impl GlWindow {
    /// # Safety
    /// 创建原生 OpenGL 上下文，需要调用方保证在主线程执行。
    pub unsafe fn new(
        event_loop: &ActiveEventLoop,
        attributes: WindowAttributes,
    ) -> anyhow::Result<Self> {
        let config_template = glutin::config::ConfigTemplateBuilder::new()
            .prefer_hardware_accelerated(None)
            .with_depth_size(0)
            .with_stencil_size(0)
            .with_transparency(false);

        let (mut window, gl_config) = glutin_winit::DisplayBuilder::new()
            .with_preference(glutin_winit::ApiPreference::FallbackEgl)
            .with_window_attributes(Some(attributes.clone()))
            .build(event_loop, config_template, |mut configs| {
                configs.next().expect("no matching OpenGL config found")
            })
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        let gl_display = gl_config.display();

        let raw_window_handle = window
            .as_ref()
            .and_then(|w| w.window_handle().ok())
            .map(|h| h.as_raw());

        let context_attributes =
            glutin::context::ContextAttributesBuilder::new().build(raw_window_handle);
        let fallback_context_attributes = glutin::context::ContextAttributesBuilder::new()
            .with_context_api(glutin::context::ContextApi::Gles(None))
            .build(raw_window_handle);

        let not_current_gl_context = unsafe {
            gl_display
                .create_context(&gl_config, &context_attributes)
                .unwrap_or_else(|_| {
                    gl_display
                        .create_context(&gl_config, &fallback_context_attributes)
                        .expect("failed to create GL context even with fallback attributes")
                })
        };

        let window = window.take().unwrap_or_else(|| {
            glutin_winit::finalize_window(event_loop, attributes, &gl_config)
                .expect("failed to finalize window")
        });

        let (width, height): (u32, u32) = window.inner_size().into();
        let width = NonZeroU32::new(width).unwrap_or(NonZeroU32::MIN);
        let height = NonZeroU32::new(height).unwrap_or(NonZeroU32::MIN);

        let surface_attributes = glutin::surface::SurfaceAttributesBuilder::<WindowSurface>::new()
            .build(
                window
                    .window_handle()
                    .map_err(|e| anyhow::anyhow!("{e}"))?
                    .as_raw(),
                width,
                height,
            );

        let gl_surface = unsafe {
            gl_display
                .create_window_surface(&gl_config, &surface_attributes)
                .map_err(|e| anyhow::anyhow!("{e}"))?
        };

        let gl_context = not_current_gl_context
            .make_current(&gl_surface)
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        gl_surface
            .set_swap_interval(
                &gl_context,
                glutin::surface::SwapInterval::Wait(NonZeroU32::MIN),
            )
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        Ok(Self {
            window: Arc::new(window),
            gl_context,
            gl_display,
            gl_surface,
        })
    }

    pub fn window(&self) -> &Arc<Window> {
        &self.window
    }

    /// 绘制前把本窗口的上下文设为当前（多窗口各自独立上下文）。
    pub fn make_current(&self) {
        self.gl_context
            .make_current(&self.gl_surface)
            .expect("failed to make GL context current");
    }

    pub fn resize(&self, size: PhysicalSize<u32>) {
        self.gl_surface.resize(
            &self.gl_context,
            NonZeroU32::new(size.width).unwrap_or(NonZeroU32::MIN),
            NonZeroU32::new(size.height).unwrap_or(NonZeroU32::MIN),
        );
    }

    pub fn swap_buffers(&self) {
        self.gl_surface
            .swap_buffers(&self.gl_context)
            .expect("failed to swap buffers");
    }

    pub fn get_proc_address(&self, addr: &CStr) -> *const c_void {
        self.gl_display.get_proc_address(addr)
    }
}
