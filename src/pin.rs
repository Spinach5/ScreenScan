//! 贴图悬浮窗口：独立无边框窗口显示合成图，可拖拽、可关闭。
//!
//! - Windows/macOS/X11：置顶；Windows 上焦点丢失时重新断言置顶。
//! - Wayland：无通用置顶，降级为普通无边框窗口（技术方案 §1.8）。

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

use crate::graphics::RenderedWindow;
use crate::platform;

const CLOSE_BTN: f32 = 22.0;
const EDGE: f32 = 4.0;

pub struct PinWindow {
    rendered: RenderedWindow,
    image: RgbaImage,
    texture: Option<TextureHandle>,
    close_requested: bool,
    topmost: bool,
}

impl PinWindow {
    pub fn create(event_loop: &ActiveEventLoop, image: RgbaImage) -> anyhow::Result<Self> {
        let (w, h) = image.dimensions();
        let topmost = !platform::is_wayland();
        let level = if topmost {
            WindowLevel::AlwaysOnTop
        } else {
            WindowLevel::Normal
        };

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
        self.rendered.render(|ui| {
            pin_ui(ui, image, texture, close_requested, &window);
        });
    }
}

fn pin_ui(
    ui: &mut egui::Ui,
    image: &RgbaImage,
    texture: &mut Option<TextureHandle>,
    close_requested: &mut bool,
    window: &Window,
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

    // 关闭按钮（右上角）
    let btn_rect = Rect::from_min_size(
        egui::pos2(rect.max.x - CLOSE_BTN - EDGE, rect.min.y + EDGE),
        egui::vec2(CLOSE_BTN, CLOSE_BTN),
    );
    if ui.put(btn_rect, egui::Button::new("X")).clicked() {
        *close_requested = true;
    }

    // 点击非按钮区域拖动窗口
    let (pressed, pointer) = ctx.input(|i| (i.pointer.primary_pressed(), i.pointer.latest_pos()));
    if pressed {
        if let Some(p) = pointer {
            if !btn_rect.contains(p) {
                let _ = window.drag_window();
            }
        }
    }
}
