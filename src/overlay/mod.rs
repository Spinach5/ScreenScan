//! 覆盖层：每个显示器一个全屏窗口，显示冻结截图 + 蒙层 + 选区。

mod render;
mod selection;

use egui::Pos2;
use egui::Rect;
use egui::Vec2;
use winit::dpi::PhysicalPosition;
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::event_loop::EventLoopProxy;
use winit::window::Fullscreen;
use winit::window::WindowAttributes;
use winit::window::WindowId;
use winit::window::WindowLevel;

use crate::actions;
use crate::actions::ActionOutcome;
use crate::annotate;
use crate::annotate::AnnotationBoard;
use crate::annotate::Stroke;
use crate::annotate::Tool;
use crate::annotate::ToolKind;
use crate::annotate::render as annotate_render;
use crate::app::UserEvent;
use crate::capture::CapturedMonitor;
use crate::graphics::RenderedWindow;
use crate::toolbar;
use crate::util;
use selection::HitArea;
use selection::MIN_SELECTION;
use selection::Selection;

/// 控制点半径（逻辑点）。
const HANDLE_R_PT: f32 = 4.0;
/// 橡皮擦默认线宽（逻辑点）。
const DEFAULT_WIDTH: f32 = 3.0;
/// 判定「单击」而非「拖动」的位移阈值（物理像素）。
const DRAG_THRESHOLD: f32 = 4.0;

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
    /// 是否为鼠标所在的「活动」显示器（亮）；否则整屏变暗。
    pub active: bool,
    /// 选区（物理像素）。
    pub selection: Option<Selection>,
    /// 拖拽起点（物理像素）。
    pub anchor: Option<Pos2>,
    /// 本次按下后是否已产生有效拖动（用于区分单击/拖动）。
    pub drag_moved: bool,
    /// 当前缩放/平移拖拽。
    pub drag: Option<Drag>,
    /// 标注笔画列表 + 撤销/重做。
    pub board: AnnotationBoard,
    /// 当前工具类型、颜色、线宽（逻辑点）。
    pub tool_kind: ToolKind,
    pub color: egui::Color32,
    pub width: f32,
    /// 正在绘制的一笔。
    pub drawing: Option<Stroke>,
    /// 橡皮擦本次拖拽累计移除的笔画（用于单条撤销记录）。
    pub erased_accum: Vec<Stroke>,
    pub close_requested: bool,
    /// 本覆盖层窗口 id（用于回传异步结果路由）。
    pub window_id: WindowId,
    /// 扫描等操作的提示文本。
    pub status: Option<String>,
    /// 是否正在后台扫描。
    pub scanning: bool,
    /// 回主线程的事件代理（贴图窗口创建等）。
    pub proxy: EventLoopProxy<UserEvent>,
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

        // 非活动显示器：整屏暗色蒙层，不显示选区/工具栏/尺寸
        if !self.active {
            painter.rect_filled(screen_rect, 0.0, render::DIM);
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.close_requested = true;
            }
            return;
        }

        // 2) 工具栏（叠加在同一视口，foreground 层）
        let mut action = toolbar::ToolbarAction::default();
        if let Some(sel_pts) = self.selection.map(|s| self.physical_rect_to_points(s.rect)) {
            if matches!(self.state, State::Selected | State::Annotating) {
                action = toolbar::show(
                    &ctx,
                    sel_pts,
                    screen_rect,
                    self.tool_kind,
                    self.color,
                    self.width,
                    self.board.can_undo(),
                    self.board.can_redo(),
                );
            }
        }

        // 3) 处理输入。指针落在任意 egui 前景层（工具栏/颜色弹窗）时跳过交互，
        //    但正在拖拽或绘制时不跳过，避免指针划过工具栏导致操作中断。
        let pointer_over_ui = ctx.input(|i| i.pointer.latest_pos()).is_some_and(|p| {
            ctx.layer_id_at(p)
                .is_some_and(|layer| layer != egui::LayerId::background())
        });
        if !(pointer_over_ui && self.drag.is_none() && self.drawing.is_none()) {
            self.handle_input(&ctx);
        }

        // 4) 蒙层 + 选区挖空
        let sel_pts = self.selection.map(|s| self.physical_rect_to_points(s.rect));
        if let Some(tex) = tex {
            render::draw_mask(&painter, tex, screen_rect, sel_pts);
        }

        // 5) 选区边框 + 控制点 + 尺寸标签 + 标注
        if let Some(sel_pts) = sel_pts {
            render::draw_border(&painter, sel_pts);
            if self.state == State::Selected {
                render::draw_handles(&painter, sel_pts, HANDLE_R_PT);
            }
            if let Some(selection) = self.selection {
                let label = format!(
                    "{} x {}px",
                    selection.rect.width().round() as u32,
                    selection.rect.height().round() as u32
                );
                render::draw_size_label(&painter, sel_pts, screen_rect, &label);
            }
            annotate_render::draw_strokes(&painter, &self.board.strokes, sel_pts);
            if let Some(drawing) = &self.drawing {
                annotate_render::draw_strokes(&painter, std::slice::from_ref(drawing), sel_pts);
            }
        }

        // 6) 应用工具栏动作
        self.apply_toolbar(action);

        // 6.1) 状态提示（toast）
        if let Some(status) = &self.status {
            let toast = egui::Rect::from_center_size(
                egui::pos2(screen_rect.center().x, screen_rect.min.y + 36.0),
                egui::vec2(280.0, 34.0),
            );
            painter.rect_filled(toast, 6.0, egui::Color32::from_black_alpha(210));
            painter.text(
                toast.center(),
                egui::Align2::CENTER_CENTER,
                status,
                egui::FontId::proportional(16.0),
                egui::Color32::WHITE,
            );
        }

        // 7) Esc：绘制中退回选区调整，其余状态退出
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            if self.state == State::Annotating {
                self.state = State::Selected;
            } else {
                self.close_requested = true;
            }
        }
    }

    fn apply_toolbar(&mut self, action: toolbar::ToolbarAction) {
        if let Some(kind) = action.set_kind {
            self.tool_kind = kind;
            self.state = State::Annotating;
            self.drag = None;
            self.drawing = None;
        }
        if let Some(color) = action.set_color {
            self.color = color;
        }
        if let Some(width) = action.set_width {
            self.width = width;
        }
        if action.undo {
            self.board.undo();
        }
        if action.redo {
            self.board.redo();
        }
        if action.copy || action.save {
            self.perform_action(action.copy);
        }
        if action.pin {
            self.perform_pin();
        }
        if action.scan {
            self.perform_scan();
        }
    }

    /// 后台扫描选区内的条码/二维码，结果经 UserEvent 回主线程。
    fn perform_scan(&mut self) {
        if self.scanning {
            return;
        }
        let Some(selection) = self.selection else {
            return;
        };
        let (base, origin) = util::image::crop(&self.screen.image, selection.rect);
        let composed = annotate_render::compose(&base, &self.board.strokes, origin, self.scale);

        self.scanning = true;
        self.status = Some("扫描中...".into());

        let proxy = self.proxy.clone();
        let window_id = self.window_id;
        std::thread::spawn(move || {
            let text = actions::scan::decode(&composed).map_err(|e| e.to_string());
            let _ = proxy.send_event(UserEvent::ScanResult { window_id, text });
        });
    }

    /// 接收后台扫描结果（主线程调用）。
    pub fn set_scan_result(&mut self, result: Result<String, String>) {
        self.scanning = false;
        match result {
            Ok(text) => {
                let message = match actions::copy_text(&text) {
                    Ok(()) => "已复制到剪贴板".to_string(),
                    Err(e) => format!("复制失败: {e}"),
                };
                log::info!("扫描结果: {text}");
                self.status = Some(message);
            }
            Err(e) => {
                log::info!("扫描失败: {e}");
                self.status = Some("未识别到条码".to_string());
            }
        }
    }

    /// 请求贴图：合成选区后把图片交给主线程创建贴图窗口。
    fn perform_pin(&mut self) {
        let Some(selection) = self.selection else {
            return;
        };
        let (base, origin) = util::image::crop(&self.screen.image, selection.rect);
        let composed = annotate_render::compose(&base, &self.board.strokes, origin, self.scale);
        match self.proxy.send_event(UserEvent::Pin(composed)) {
            Ok(()) => log::info!("请求创建贴图窗口"),
            Err(e) => log::warn!("贴图事件发送失败: {e}"),
        }
    }

    fn current_tool(&self) -> Tool {
        match self.tool_kind {
            ToolKind::Brush => Tool::Brush {
                color: self.color,
                width: self.width,
            },
            ToolKind::Scribble => Tool::Scribble { color: self.color },
            ToolKind::Eraser => Tool::Eraser { width: self.width },
        }
    }

    /// 执行复制/保存：裁剪选区并合成标注。
    fn perform_action(&mut self, is_copy: bool) {
        let Some(selection) = self.selection else {
            return;
        };
        let (base, origin) = util::image::crop(&self.screen.image, selection.rect);
        let composed = annotate_render::compose(&base, &self.board.strokes, origin, self.scale);
        let outcome = if is_copy {
            actions::copy(&composed)
        } else {
            actions::save(&composed)
        };

        match outcome {
            ActionOutcome::Copied => {
                log::info!("已复制到剪贴板");
                self.finish();
            }
            ActionOutcome::Saved(path) => {
                log::info!("已保存: {}", path.display());
                self.finish();
            }
            ActionOutcome::Failed(msg) => {
                log::warn!("动作失败: {msg}");
            }
        }
    }

    fn finish(&mut self) {
        self.state = State::Done;
        self.close_requested = true;
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

        // 右键：绘制中撤销上一笔；其余恢复默认全屏选区
        if secondary_pressed {
            if self.state == State::Annotating {
                self.board.undo();
            } else {
                self.reset_to_default();
            }
            return;
        }

        match self.state {
            State::Idle => {
                if primary_pressed {
                    if let Some(p) = pos {
                        self.begin_press(p);
                    }
                }
            }
            State::Selecting => {
                if let (Some(anchor), Some(p)) = (self.anchor, pos) {
                    if (p - anchor).length() > DRAG_THRESHOLD {
                        self.drag_moved = true;
                    }
                    if self.drag_moved {
                        self.selection = Some(Selection::new(anchor, p));
                    }
                }
            }
            State::Annotating => {
                self.handle_draw(pointer, primary_pressed, primary_down, primary_released);
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
                                // 选区外按下：准备重新拖选；仅单击则保持当前选区
                                self.begin_press(p);
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

                if primary_released {
                    self.drag = None;
                }
            }
            _ => {}
        }

        // 选区收尾（兼容按下与释放落在同一帧的快速单击）：
        // - 拖动过：松开即确认（方案 1）
        // - 未拖动：确认为默认全屏 / 当前选区
        if self.state == State::Selecting && primary_released {
            if self.drag_moved {
                if self.selection.is_some_and(|s| s.is_valid()) {
                    self.state = State::Selected;
                } else {
                    self.reset_to_default();
                }
            } else {
                if self.selection.is_none() {
                    self.selection = Some(self.default_selection());
                }
                self.state = State::Selected;
            }
            self.anchor = None;
            self.drag_moved = false;
        }
    }

    fn begin_press(&mut self, pos: Pos2) {
        self.anchor = Some(pos);
        self.drag = None;
        self.drag_moved = false;
        self.state = State::Selecting;
    }

    /// 覆盖本显示器全屏的默认选区（物理像素）。
    fn default_selection(&self) -> Selection {
        let (w, h) = self.screen.image.dimensions();
        Selection {
            rect: Rect::from_min_max(Pos2::ZERO, Pos2::new(w as f32, h as f32)),
        }
    }

    fn reset_to_default(&mut self) {
        self.selection = Some(self.default_selection());
        self.anchor = None;
        self.drag = None;
        self.drag_moved = false;
        self.drawing = None;
        self.erased_accum.clear();
        self.state = State::Idle;
    }

    pub fn is_dragging(&self) -> bool {
        self.state == State::Selecting || self.drag.is_some() || self.drawing.is_some()
    }

    /// 绘制状态下的输入处理。`pointer` 为逻辑点（笔画坐标空间）。
    fn handle_draw(&mut self, pointer: Option<Pos2>, pressed: bool, down: bool, released: bool) {
        let is_eraser = self.tool_kind == ToolKind::Eraser;

        if pressed && self.drawing.is_none() {
            let scale = self.scale;
            let inside = match (pointer, self.selection) {
                (Some(pt), Some(sel)) => sel.rect.contains(egui::pos2(pt.x * scale, pt.y * scale)),
                _ => false,
            };
            if inside {
                if let Some(pt) = pointer {
                    let mut stroke = Stroke::new(self.current_tool());
                    stroke.points.push(pt);
                    self.drawing = Some(stroke);
                    self.erased_accum.clear();
                }
            }
        }

        if down && self.drawing.is_some() {
            if let Some(pt) = pointer {
                let should_add = self
                    .drawing
                    .as_ref()
                    .and_then(|d| d.points.last())
                    .is_none_or(|last| (*last - pt).length() > 0.5);
                if should_add {
                    let width = {
                        let d = self.drawing.as_mut().expect("drawing stroke exists");
                        d.points.push(pt);
                        d.width
                    };
                    if is_eraser {
                        let n = self.drawing.as_ref().map(|d| d.points.len()).unwrap_or(0);
                        if n >= 2 {
                            let d = self.drawing.as_ref().expect("drawing stroke exists");
                            let segment = [d.points[n - 2], d.points[n - 1]];
                            let removed = self.board.remove_intersecting(&segment, width);
                            self.erased_accum.extend(removed);
                        }
                    }
                }
            }
        }

        if released {
            if let Some(drawing) = self.drawing.take() {
                if is_eraser {
                    let removed = std::mem::take(&mut self.erased_accum);
                    self.board.record_erase(removed);
                } else if !drawing.points.is_empty() {
                    self.board.push(drawing);
                }
            }
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
    rendered: RenderedWindow,
    state: OverlayState,
    /// 首帧后是否已向目标显示器发起全屏请求。
    placed: bool,
    target_monitor: Option<winit::monitor::MonitorHandle>,
}

impl Overlay {
    pub fn create(
        event_loop: &ActiveEventLoop,
        screen: CapturedMonitor,
        proxy: EventLoopProxy<UserEvent>,
    ) -> anyhow::Result<Self> {
        // 按名称（优先）或位置/尺寸匹配 winit 显示器，使用「按显示器全屏」
        // 让覆盖层精确铺满对应屏幕（Wayland 无法绝对定位时同样有效）。
        let name = screen.monitor.name.clone();
        let want = (
            screen.monitor.x,
            screen.monitor.y,
            screen.monitor.width,
            screen.monitor.height,
        );
        let handle = event_loop
            .available_monitors()
            .find(|m| m.name().as_deref() == Some(name.as_str()))
            .or_else(|| {
                event_loop.available_monitors().find(|m| {
                    m.position().x == want.0
                        && m.position().y == want.1
                        && m.size().width == want.2
                        && m.size().height == want.3
                })
            });

        let base = WindowAttributes::default()
            .with_title("ScreenScan")
            .with_decorations(false)
            .with_resizable(false)
            .with_visible(false)
            .with_window_level(WindowLevel::AlwaysOnTop);
        let attributes = match &handle {
            Some(monitor) => {
                base.with_fullscreen(Some(Fullscreen::Borderless(Some(monitor.clone()))))
            }
            None => base
                .with_position(PhysicalPosition::new(screen.monitor.x, screen.monitor.y))
                .with_inner_size(PhysicalSize::new(
                    screen.monitor.width,
                    screen.monitor.height,
                )),
        };

        let rendered = unsafe { RenderedWindow::create(event_loop, attributes)? };
        let scale = rendered.scale();

        let (iw, ih) = screen.image.dimensions();
        let default_selection = Selection {
            rect: Rect::from_min_max(Pos2::ZERO, Pos2::new(iw as f32, ih as f32)),
        };

        let state = OverlayState {
            screen,
            texture: None,
            scale,
            state: State::Idle,
            active: false,
            selection: Some(default_selection),
            anchor: None,
            drag_moved: false,
            drag: None,
            board: AnnotationBoard::default(),
            tool_kind: ToolKind::Brush,
            color: annotate::render::DEFAULT_COLOR,
            width: DEFAULT_WIDTH,
            drawing: None,
            erased_accum: Vec::new(),
            close_requested: false,
            window_id: rendered.window_id(),
            status: None,
            scanning: false,
            proxy,
        };

        rendered.window().set_visible(true);
        // 部分 Wayland 合成器需在窗口映射后再请求「按显示器全屏」。
        if let Some(monitor) = &handle {
            rendered
                .window()
                .set_fullscreen(Some(Fullscreen::Borderless(Some(monitor.clone()))));
        }
        rendered.window().focus_window();
        rendered.window().request_redraw();

        Ok(Self {
            rendered,
            state,
            placed: false,
            target_monitor: handle,
        })
    }

    pub fn window_id(&self) -> WindowId {
        self.rendered.window_id()
    }

    pub fn request_redraw(&self) {
        self.rendered.window().request_redraw();
    }

    pub fn set_active(&mut self, active: bool) {
        self.state.active = active;
    }

    pub fn is_primary(&self) -> bool {
        self.state.screen.monitor.is_primary
    }

    pub fn is_dragging(&self) -> bool {
        self.state.is_dragging()
    }

    pub fn set_scan_result(&mut self, result: Result<String, String>) {
        self.state.set_scan_result(result);
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
            self.rendered.resize(*size);
        }

        if self.rendered.on_window_event(event) {
            self.rendered.window().request_redraw();
        }
        let _ = event_loop;

        false
    }

    fn redraw(&mut self) {
        self.rendered.render(|ui| self.state.ui(ui));
        self.rendered.window().set_visible(true);

        if !self.placed {
            self.placed = true;
            // 首帧缓冲区已提交（surface 已映射），此时再按目标输出全屏，
            // 部分 Wayland 合成器（如 Mutter）才会真正把窗口放到指定显示器。
            if let Some(monitor) = &self.target_monitor {
                self.rendered
                    .window()
                    .set_fullscreen(Some(Fullscreen::Borderless(Some(monitor.clone()))));
            }
            log::debug!(
                "overlay[{}] inner={:?}",
                self.state.screen.monitor.name,
                self.rendered.window().inner_size()
            );
        }
    }
}
