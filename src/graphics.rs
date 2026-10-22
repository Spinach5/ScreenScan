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

/// 组装好的窗口：glutin 上下文 + glow + egui 渲染，供覆盖层与贴图窗口共用。
pub struct RenderedWindow {
    gl: GlWindow,
    glow: Arc<glow::Context>,
    egui_glow: egui_glow::winit::EguiGlow,
}

impl RenderedWindow {
    /// # Safety
    /// 创建原生 OpenGL 上下文，需要调用方保证在主线程执行。
    pub unsafe fn create(
        event_loop: &ActiveEventLoop,
        attributes: WindowAttributes,
    ) -> anyhow::Result<Self> {
        let gl = unsafe { GlWindow::new(event_loop, attributes)? };

        let glow = unsafe {
            Arc::new(glow::Context::from_loader_function(|s| {
                let s = std::ffi::CString::new(s)
                    .expect("failed to build C string for GL proc address");
                gl.get_proc_address(&s)
            }))
        };

        let scale = gl.window().scale_factor() as f32;
        let egui_glow =
            egui_glow::winit::EguiGlow::new(event_loop, glow.clone(), None, Some(scale), true);

        // egui 请求重绘时，唤醒本窗口
        let window = gl.window().clone();
        egui_glow
            .egui_ctx
            .set_request_repaint_callback(move |_info| {
                window.request_redraw();
            });

        Ok(Self {
            gl,
            glow,
            egui_glow,
        })
    }

    pub fn window(&self) -> &Arc<Window> {
        self.gl.window()
    }

    pub fn window_id(&self) -> winit::window::WindowId {
        self.gl.window().id()
    }

    pub fn scale(&self) -> f32 {
        self.gl.window().scale_factor() as f32
    }

    pub fn resize(&self, size: PhysicalSize<u32>) {
        self.gl.resize(size);
    }

    /// 把窗口事件喂给 egui；返回是否需要重绘。
    pub fn on_window_event(&mut self, event: &winit::event::WindowEvent) -> bool {
        let window = self.gl.window().clone();
        let response = self.egui_glow.on_window_event(&window, event);
        response.repaint
    }

    /// 跑一帧 UI：`run_ui` 构建界面，随后清屏、绘制、交换缓冲。
    pub fn render(&mut self, run_ui: impl FnMut(&mut egui::Ui)) {
        let window = self.gl.window().clone();
        self.egui_glow.run(&window, run_ui);

        self.gl.make_current();
        unsafe {
            use glow::HasContext as _;
            self.glow.clear_color(0.0, 0.0, 0.0, 1.0);
            self.glow.clear(glow::COLOR_BUFFER_BIT);
        }

        self.egui_glow.paint(&window);
        self.gl.swap_buffers();
        self.gl.window().set_visible(true);
    }
}
