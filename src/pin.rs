//! 贴图悬浮窗口：像 Snipaste / Flameshot 贴图一样悬浮在屏幕上方，可拖拽、右键菜单操作。
//!
//! 使用普通无边框置顶窗口：位置由创建时传入（选区全局坐标），拖动交给窗口管理器。
//! - X11（含 Wayland 会话下的 XWayland）：`Utility` 窗口类型，不进入任务栏 / Alt+Tab。
//! - Windows：跳过任务栏的工具窗口，置顶。
//! - macOS：无边框置顶窗口。
//! - 原生 Wayland：无通用置顶与绝对定位，降级为普通无边框窗口（技术方案 §1.8）。

use std::path::PathBuf;

use egui::Color32;
use egui::Rect;
use egui::TextureHandle;
use image::RgbaImage;
use winit::dpi::PhysicalPosition;
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::Window;
use winit::window::WindowAttributes;
use winit::window::WindowId;
use winit::window::WindowLevel;

use crate::config::format_filename;
use crate::graphics::RenderedWindow;
use crate::i18n::Language;
use crate::i18n::t;
use crate::platform;

pub struct PinWindow {
    rendered: RenderedWindow,
    image: RgbaImage,
    texture: Option<TextureHandle>,
    close_requested: bool,
    topmost: bool,
    /// X11（含 XWayland）下自行移动：不依赖 WM 的 `_NET_WM_MOVERESIZE`，
    /// 与 Flameshot 贴图一致。
    manual_drag: bool,
    /// 手动拖拽时指针在窗口内的抓取偏移（物理像素）。
    drag_offset: Option<egui::Vec2>,
    language: Language,
    filename_template: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PinAction {
    Save,
    Destroy,
}

impl PinWindow {
    pub fn create(
        event_loop: &ActiveEventLoop,
        image: RgbaImage,
        position: PhysicalPosition<i32>,
        language: Language,
        filename_template: String,
    ) -> anyhow::Result<Self> {
        let (w, h) = image.dimensions();
        // Wayland 后端会忽略窗口层级；但 Wayland 会话下若走 XWayland（X11）后端，
        // 置顶仍然生效。原生 Wayland 无法通用置顶。
        let topmost = !platform::is_wayland() || platform::use_x11_backend();
        let level = if topmost {
            WindowLevel::AlwaysOnTop
        } else {
            WindowLevel::Normal
        };
        if !topmost {
            log::warn!("原生 Wayland 不支持通用置顶，贴图窗口降级为普通无边框窗口");
        }

        let mut attributes = WindowAttributes::default()
            .with_title("ScreenScan 贴图")
            .with_decorations(false)
            .with_resizable(false)
            .with_visible(false)
            .with_window_level(level)
            .with_position(position)
            .with_inner_size(PhysicalSize::new(w, h));

        #[cfg(target_os = "linux")]
        {
            use winit::platform::x11::WindowAttributesExtX11 as _;
            use winit::platform::x11::WindowType;
            // Utility 窗口通常不进入任务栏 / Alt+Tab，同时仍能被聚焦、接收输入。
            attributes = attributes.with_x11_window_type(vec![WindowType::Utility]);
        }

        #[cfg(target_os = "windows")]
        {
            use winit::platform::windows::WindowAttributesExtWindows as _;
            attributes = attributes.with_skip_taskbar(true);
        }

        let rendered = unsafe { RenderedWindow::create(event_loop, attributes)? };
        rendered.window().set_visible(true);
        rendered.window().focus_window();
        rendered.window().request_redraw();

        Ok(Self {
            rendered,
            image,
            texture: None,
            close_requested: false,
            topmost,
            manual_drag: platform::use_x11_backend(),
            drag_offset: None,
            language,
            filename_template,
        })
    }

    pub fn window_id(&self) -> WindowId {
        self.rendered.window_id()
    }

    /// 返回 `true` 表示贴图窗口应被关闭。
    pub fn handle_window_event(&mut self, event: &WindowEvent) -> bool {
        use winit::event::WindowEvent::*;

        match event {
            CloseRequested | Destroyed => return true,
            RedrawRequested => {
                self.redraw();
                return self.close_requested;
            }
            Resized(size) => self.rendered.resize(*size),
            Focused(false) if self.topmost => {
                // 其他应用可能用 HWND_TOP 静默摘掉置顶，重新断言
                self.rendered
                    .window()
                    .set_window_level(WindowLevel::AlwaysOnTop);
            }
            _ => {}
        }

        if self.rendered.on_window_event(event) {
            self.rendered.window().request_redraw();
        }
        false
    }

    fn redraw(&mut self) {
        let window = self.rendered.window().clone();
        let image = &self.image;
        let texture = &mut self.texture;
        let close_requested = &mut self.close_requested;
        let manual_drag = self.manual_drag;
        let drag_offset = &mut self.drag_offset;
        let language = self.language;
        let filename_template = &self.filename_template;
        self.rendered.render(|ui| {
            pin_ui(
                ui,
                image,
                texture,
                close_requested,
                &window,
                manual_drag,
                drag_offset,
                language,
                filename_template,
            );
        });
    }
}

#[allow(clippy::too_many_arguments)]
fn pin_ui(
    ui: &mut egui::Ui,
    image: &RgbaImage,
    texture: &mut Option<TextureHandle>,
    close_requested: &mut bool,
    window: &Window,
    manual_drag: bool,
    drag_offset: &mut Option<egui::Vec2>,
    language: Language,
    filename_template: &str,
) {
    let ctx = ui.ctx().clone();
    let rect = ctx.viewport_rect();

    // Snipaste 习惯：Esc 直接销毁贴图。
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        *close_requested = true;
    }

    if texture.is_none() {
        let color_image = egui::ColorImage::from_rgba_unmultiplied(
            [image.width() as usize, image.height() as usize],
            image.as_raw(),
        );
        *texture =
            Some(ctx.load_texture("pin_image", color_image, egui::TextureOptions::default()));
    }
    if let Some(tex) = texture {
        let uv = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
        ui.painter().image(tex.id(), rect, uv, Color32::WHITE);
    }

    let response = ui.interact(
        rect,
        ui.scope_id().with("pin_surface"),
        egui::Sense::click_and_drag(),
    );

    let mut action = None;
    response.context_menu(|ui| {
        ui.add_enabled(
            false,
            egui::Button::new(format!(
                "{}: {} × {} px",
                t(language, "pin.size"),
                image.width(),
                image.height()
            )),
        );
        ui.separator();
        if ui.button(t(language, "pin.save")).clicked() {
            action = Some(PinAction::Save);
            ui.close();
        }
        if ui.button(t(language, "pin.destroy")).clicked() {
            action = Some(PinAction::Destroy);
            ui.close();
        }
    });

    match action {
        Some(PinAction::Save) => {
            if let Some(path) = choose_folder_and_save(image, filename_template, language) {
                log::info!("贴图已保存: {}", path.display());
            }
        }
        Some(PinAction::Destroy) => {
            log::info!("贴图已销毁");
            *close_requested = true;
        }
        None => {}
    }

    // 用 egui 的拖拽判定替代原始按下事件：菜单弹层会拦截点击，
    // 不会在点击菜单项时误触发窗口拖动。
    if manual_drag {
        // X11 下自己移动窗口，不依赖 WM 是否实现 `_NET_WM_MOVERESIZE`。
        let scale = window.scale_factor() as f32;
        if response.drag_started_by(egui::PointerButton::Primary) {
            *drag_offset = ctx
                .input(|i| i.pointer.latest_pos())
                .map(|p| egui::vec2(p.x * scale, p.y * scale));
        }
        if response.dragged_by(egui::PointerButton::Primary)
            && let (Some(offset), Some(pointer), Ok(position)) = (
                *drag_offset,
                ctx.input(|i| i.pointer.latest_pos()),
                window.outer_position(),
            )
        {
            window.set_outer_position(drag_position(position, pointer, scale, offset));
        }
        if response.drag_stopped() {
            *drag_offset = None;
        }
    } else if response.drag_started_by(egui::PointerButton::Primary) {
        let _ = window.drag_window();
    }
}

/// 手动拖拽的移动目标（物理像素）：指针全局位置减去按下时的抓取偏移。
fn drag_position(
    window_position: PhysicalPosition<i32>,
    pointer: egui::Pos2,
    scale: f32,
    grab_offset: egui::Vec2,
) -> PhysicalPosition<i32> {
    let global = egui::vec2(
        window_position.x as f32 + pointer.x * scale,
        window_position.y as f32 + pointer.y * scale,
    );
    PhysicalPosition::new(
        (global.x - grab_offset.x).round() as i32,
        (global.y - grab_offset.y).round() as i32,
    )
}

/// 选择目录后按文件名模板写出图片（模板省略扩展名时补 `.png`）。
fn choose_folder_and_save(
    image: &RgbaImage,
    filename_template: &str,
    language: Language,
) -> Option<PathBuf> {
    let dir = rfd::FileDialog::new()
        .set_title(t(language, "pin.choose_dir"))
        .pick_folder()?;

    let mut name = format_filename(filename_template);
    if std::path::Path::new(&name).extension().is_none() {
        name.push_str(".png");
    }

    let path = dir.join(name);
    match image.save(&path) {
        Ok(()) => Some(path),
        Err(e) => {
            log::error!("贴图保存失败 {}: {e}", path.display());
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drag_keeps_grab_offset() {
        let grab = egui::vec2(10.0, 20.0);
        assert_eq!(
            drag_position(
                PhysicalPosition::new(100, 100),
                egui::pos2(10.0, 20.0),
                1.0,
                grab
            ),
            PhysicalPosition::new(100, 100)
        );
        assert_eq!(
            drag_position(
                PhysicalPosition::new(100, 100),
                egui::pos2(30.0, 40.0),
                1.0,
                grab
            ),
            PhysicalPosition::new(120, 120)
        );
        assert_eq!(
            drag_position(
                PhysicalPosition::new(100, 100),
                egui::pos2(15.0, 20.0),
                2.0,
                grab
            ),
            PhysicalPosition::new(120, 120)
        );
    }
}
