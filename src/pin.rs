//! 贴图悬浮窗口：独立无边框窗口显示合成图，可拖拽、右键菜单操作。
//!
//! - Windows/macOS/X11（含 XWayland）：置顶；焦点丢失时重新断言置顶。
//! - 原生 Wayland：无通用置顶，降级为普通无边框窗口（技术方案 §1.8）。

use std::path::PathBuf;

use egui::Color32;
use egui::Rect;
use egui::TextureHandle;
use image::RgbaImage;
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
        language: Language,
        filename_template: String,
    ) -> anyhow::Result<Self> {
        let (w, h) = image.dimensions();
        // Wayland 后端会忽略窗口层级；但 Wayland 会话下若走 XWayland（X11）后端，
        // 置顶仍然生效。
        let topmost = !platform::is_wayland() || platform::use_x11_backend();
        let level = if topmost {
            WindowLevel::AlwaysOnTop
        } else {
            WindowLevel::Normal
        };
        if !topmost {
            log::warn!("原生 Wayland 不支持通用置顶，贴图窗口降级为普通无边框窗口");
        }

        let attributes = WindowAttributes::default()
            .with_title("ScreenScan 贴图")
            .with_decorations(false)
            .with_resizable(false)
            .with_visible(false)
            .with_window_level(level)
            .with_inner_size(PhysicalSize::new(w, h));

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
        let language = self.language;
        let filename_template = &self.filename_template;
        self.rendered.render(|ui| {
            pin_ui(
                ui,
                image,
                texture,
                close_requested,
                &window,
                language,
                filename_template,
            );
        });
    }
}

fn pin_ui(
    ui: &mut egui::Ui,
    image: &RgbaImage,
    texture: &mut Option<TextureHandle>,
    close_requested: &mut bool,
    window: &Window,
    language: Language,
    filename_template: &str,
) {
    let ctx = ui.ctx().clone();
    let rect = ctx.viewport_rect();

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

    // 菜单关闭后才允许开始拖拽，避免点击菜单项时同时拖动窗口。
    let menu_open = ctx.any_popup_open();

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
        Some(PinAction::Destroy) => *close_requested = true,
        None => {}
    }

    let pressed = ctx.input(|i| i.pointer.primary_pressed());
    if pressed && !menu_open {
        let _ = window.drag_window();
    }
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
