//! 覆盖层：每个显示器一个全屏窗口，显示冻结截图 + 蒙层 + 选区。

use std::sync::Arc;

use egui_glow::winit::EguiGlow;
use winit::dpi::PhysicalPosition;
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::WindowAttributes;
use winit::window::WindowId;
use winit::window::WindowLevel;

use crate::capture::CapturedMonitor;
use crate::graphics::GlWindow;

/// 覆盖层状态机（后续里程碑逐步启用各状态）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Idle,
    Selecting,
    Selected,
    Annotating,
    Done,
}

/// 单个覆盖层窗口的可变 UI 状态。
pub struct OverlayState {
    pub screen: CapturedMonitor,
    pub texture: Option<egui::TextureHandle>,
    pub scale: f32,
    pub state: State,
    pub close_requested: bool,
}

impl OverlayState {
    fn ui(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let screen_rect = ctx.viewport_rect();

        // 懒上传冻结截图纹理
        if self.texture.is_none() {
            let img = &self.screen.image;
            let color_image = egui::ColorImage::from_rgba_unmultiplied(
                [img.width() as usize, img.height() as usize],
                img.as_raw(),
            );
            self.texture =
                Some(ctx.load_texture("screenshot", color_image, egui::TextureOptions::default()));
        }

        if let Some(tex) = &self.texture {
            let full_uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
            ui.painter()
                .image(tex.id(), screen_rect, full_uv, egui::Color32::WHITE);
        }

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.close_requested = true;
        }
    }
}

/// 覆盖层窗口（GL 上下文 + egui 渲染 + 状态）。
pub struct Overlay {
    gl: GlWindow,
    glow: Arc<glow::Context>,
    egui_glow: EguiGlow,
    state: OverlayState,
}

impl Overlay {
    pub fn create(event_loop: &ActiveEventLoop, screen: CapturedMonitor) -> anyhow::Result<Self> {
        let attributes = WindowAttributes::default()
            .with_title("ScreenScan")
            .with_decorations(false)
            .with_resizable(false)
            .with_visible(false)
            .with_window_level(WindowLevel::AlwaysOnTop)
            .with_position(PhysicalPosition::new(screen.monitor.x, screen.monitor.y))
            .with_inner_size(PhysicalSize::new(
                screen.monitor.width,
                screen.monitor.height,
            ));

        let gl = unsafe { GlWindow::new(event_loop, attributes)? };

        let glow = unsafe {
            Arc::new(glow::Context::from_loader_function(|s| {
                let s = std::ffi::CString::new(s)
                    .expect("failed to build C string for GL proc address");
                gl.get_proc_address(&s)
            }))
        };

        let scale = gl.window().scale_factor() as f32;
        let egui_glow = EguiGlow::new(event_loop, glow.clone(), None, Some(scale), true);

        // egui 请求重绘时，唤醒本窗口
        let window = gl.window().clone();
        egui_glow
            .egui_ctx
            .set_request_repaint_callback(move |_info| {
                window.request_redraw();
            });

        let state = OverlayState {
            screen,
            texture: None,
            scale,
            state: State::Idle,
            close_requested: false,
        };

        gl.window().set_visible(true);
        gl.window().focus_window();
        gl.window().request_redraw();

        Ok(Self {
            gl,
            glow,
            egui_glow,
            state,
        })
    }

    pub fn window_id(&self) -> WindowId {
        self.gl.window().id()
    }

    /// 处理窗口事件；返回 `true` 表示本覆盖层应被关闭。
    pub fn handle_window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        event: &WindowEvent,
    ) -> bool {
        use winit::event::WindowEvent::*;

        if matches!(event, CloseRequested | Destroyed) {
            return true;
        }

        if matches!(event, RedrawRequested) {
            self.redraw();
            if self.state.close_requested {
                return true;
            }
            return false;
        }

        if let Resized(size) = event {
            self.gl.resize(*size);
        }

        let window = self.gl.window().clone();
        let response = self.egui_glow.on_window_event(&window, event);
        if response.repaint {
            window.request_redraw();
        }
        let _ = event_loop;

        false
    }

    fn redraw(&mut self) {
        let window = self.gl.window().clone();
        self.egui_glow.run(&window, |ui| self.state.ui(ui));

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
