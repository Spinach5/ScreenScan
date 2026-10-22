//! 覆盖层：每个显示器一个全屏窗口，显示冻结截图 + 蒙层 + 选区。

mod render;
mod selection;

use std::sync::Arc;

use egui::Pos2;
use egui::Rect;
use egui::Vec2;
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
use selection::HitArea;
use selection::MIN_SELECTION;
use selection::Selection;

/// 控制点半径（逻辑点）。
const HANDLE_R_PT: f32 = 4.0;

/// 覆盖层状态机。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Idle,
    Selecting,
    Selected,
    Annotating,
    Done,
}

/// 当前拖拽类型（物理像素空间）。
#[derive(Debug, Clone, Copy)]
pub enum Drag {
    Resize(HitArea),
    Move { offset: Vec2 },
}

/// 单个覆盖层窗口的可变 UI 状态。
pub struct OverlayState {
    pub screen: CapturedMonitor,
    pub texture: Option<egui::TextureHandle>,
    pub scale: f32,
    pub state: State,
    /// 选区（物理像素）。
    pub selection: Option<Selection>,
    /// 拖拽起点（物理像素）。
    pub anchor: Option<Pos2>,
    /// 当前缩放/平移拖拽。
    pub drag: Option<Drag>,
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

        let painter = ui.painter().clone();
        let tex = self.texture.as_ref().map(|t| t.id());

        // 1) 冻结截图铺满全窗
        if let Some(tex) = tex {
            render::draw_screenshot(&painter, tex, screen_rect);
        }

        // 2) 处理输入（更新选区/状态）
        self.handle_input(&ctx);

        // 3) 蒙层 + 选区挖空
        let sel_pts = self.selection.map(|s| self.physical_rect_to_points(s.rect));
        if let Some(tex) = tex {
            render::draw_mask(&painter, tex, screen_rect, sel_pts);
        }

        // 4) 选区边框 + 控制点
        if let Some(sel_pts) = sel_pts {
            render::draw_border(&painter, sel_pts);
            if self.state == State::Selected {
                render::draw_handles(&painter, sel_pts, HANDLE_R_PT);
            }
        }

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.close_requested = true;
        }
    }

    fn handle_input(&mut self, ctx: &egui::Context) {
        let (primary_pressed, primary_released, primary_down, secondary_pressed, pointer) = ctx
            .input(|i| {
                (
                    i.pointer.primary_pressed(),
                    i.pointer.primary_released(),
                    i.pointer.primary_down(),
                    i.pointer.secondary_pressed(),
                    i.pointer.latest_pos(),
                )
            });

        // 指针逻辑点 → 物理像素，并钳制在截图范围内
        let image_size = self.screen.image.dimensions();
        let bounds = Rect::from_min_size(
            Pos2::ZERO,
            Vec2::new(image_size.0 as f32, image_size.1 as f32),
        );
        let pos = pointer.map(|p| {
            egui::pos2(
                (p.x * self.scale).clamp(0.0, image_size.0 as f32),
                (p.y * self.scale).clamp(0.0, image_size.1 as f32),
            )
        });
        let handle_r = HANDLE_R_PT * self.scale;

        // 右键：取消当前选区
        if secondary_pressed {
            self.selection = None;
            self.anchor = None;
            self.drag = None;
            self.state = State::Idle;
            return;
        }

        match self.state {
            State::Idle => {
                if primary_pressed {
                    if let Some(p) = pos {
                        self.anchor = Some(p);
                        self.selection = Some(Selection::new(p, p));
                        self.state = State::Selecting;
                    }
                }
            }
            State::Selecting => {
                if let (Some(anchor), Some(p)) = (self.anchor, pos) {
                    self.selection = Some(Selection::new(anchor, p));
                }
            }
            State::Selected => {
                if primary_pressed {
                    if let (Some(p), Some(sel)) = (pos, self.selection) {
                        match sel.hit_test(p, handle_r) {
                            HitArea::Move => {
                                self.drag = Some(Drag::Move {
                                    offset: p - sel.rect.min,
                                });
                            }
                            HitArea::None => {
                                // 选区外按下：重新框选
                                self.anchor = Some(p);
                                self.selection = Some(Selection::new(p, p));
                                self.state = State::Selecting;
                            }
                            hit => {
                                self.drag = Some(Drag::Resize(hit));
                            }
                        }
                    }
                }

                if primary_down {
                    if let (Some(drag), Some(p), Some(sel)) =
                        (self.drag, pos, self.selection.as_mut())
                    {
                        match drag {
                            Drag::Resize(hit) => sel.resize(hit, p, MIN_SELECTION, bounds),
                            Drag::Move { offset } => {
                                let delta = (p - offset) - sel.rect.min;
                                sel.move_by(delta, bounds);
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        // 释放统一在此收尾，兼容「按下与释放落在同一帧」的快速点击
        if self.state == State::Selecting && primary_released {
            let valid = self.selection.is_some_and(|s| s.is_valid());
            if valid {
                self.state = State::Selected;
            } else {
                self.selection = None;
                self.state = State::Idle;
            }
            self.anchor = None;
        }

        if primary_released {
            self.drag = None;
        }
    }

    /// 物理像素矩形 → 逻辑点矩形。
    fn physical_rect_to_points(&self, rect: Rect) -> Rect {
        Rect::from_min_max(
            Pos2::new(rect.min.x / self.scale, rect.min.y / self.scale),
            Pos2::new(rect.max.x / self.scale, rect.max.y / self.scale),
        )
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
            selection: None,
            anchor: None,
            drag: None,
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
