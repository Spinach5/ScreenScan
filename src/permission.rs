//! 截屏授权引导窗口。
//!
//! Wayland 门户只有在请求方是当前焦点应用时才允许弹出系统授权对话框；
//! 托盘驻留进程没有窗口，因此首次截图会失败。这里显示一个小窗口，用户点击
//! 授权后由本窗口获得焦点再发起门户请求，系统对话框才能正常显示。

use egui::Color32;
use egui::RichText;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::WindowAttributes;
use winit::window::WindowId;
use winit::window::WindowLevel;

use crate::graphics::RenderedWindow;
use crate::i18n::Language;
use crate::i18n::t;

pub enum Effect {
    Grant,
    Cancel,
}

pub struct PermissionWindow {
    rendered: RenderedWindow,
    pending: bool,
    status: Option<String>,
}

impl PermissionWindow {
    pub fn create(event_loop: &ActiveEventLoop, lang: Language) -> anyhow::Result<Self> {
        let attributes = WindowAttributes::default()
            .with_title(t(lang, "permission.title"))
            .with_inner_size(LogicalSize::new(460.0, 260.0))
            .with_resizable(false)
            .with_window_level(WindowLevel::AlwaysOnTop);

        let rendered = unsafe { RenderedWindow::create(event_loop, attributes)? };

        if let Ok(image) = crate::tray::default_icon_image() {
            let (width, height) = image.dimensions();
            if let Ok(icon) = winit::window::Icon::from_rgba(image.into_raw(), width, height) {
                rendered.window().set_window_icon(Some(icon));
            }
        }

        rendered.window().set_visible(true);
        rendered.window().focus_window();
        rendered.window().request_redraw();

        Ok(Self {
            rendered,
            pending: false,
            status: None,
        })
    }

    pub fn window_id(&self) -> WindowId {
        self.rendered.window_id()
    }

    pub fn request_redraw(&self) {
        self.rendered.window().request_redraw();
    }

    pub fn focus(&self) {
        self.rendered.window().focus_window();
    }

    pub fn set_pending(&mut self, pending: bool) {
        self.pending = pending;
        if pending {
            self.status = None;
        }
    }

    pub fn set_status(&mut self, status: impl Into<String>) {
        self.status = Some(status.into());
        self.pending = false;
    }

    /// 处理非 RedrawRequested 事件；返回 `true` 表示窗口应关闭。
    pub fn handle_window_event(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::CloseRequested | WindowEvent::Destroyed => return true,
            WindowEvent::Resized(size) => self.rendered.resize(*size),
            _ => {}
        }

        if self.rendered.on_window_event(event) {
            self.request_redraw();
        }
        false
    }

    pub fn redraw(&mut self, lang: Language) -> Option<Effect> {
        let pending = self.pending;
        let status = self.status.clone();
        let mut effect = None;

        self.rendered.render(|ui| {
            let background = ui.visuals().panel_fill;
            ui.painter().rect_filled(ui.max_rect(), 0.0, background);

            egui::Frame::new()
                .fill(background)
                .inner_margin(egui::Margin::same(18))
                .show(ui, |ui| {
                    ui.set_min_size(ui.available_size());
                    ui.heading(t(lang, "permission.title"));
                    ui.add_space(10.0);
                    ui.label(t(lang, "permission.body"));
                    ui.add_space(6.0);
                    ui.label(RichText::new(t(lang, "permission.hint")).small().weak());

                    if let Some(status) = status.as_deref() {
                        ui.add_space(8.0);
                        ui.colored_label(Color32::from_rgb(220, 80, 80), status);
                    }

                    ui.add_space(14.0);
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(!pending, egui::Button::new(t(lang, "permission.grant")))
                            .clicked()
                        {
                            effect = Some(Effect::Grant);
                        }
                        if ui
                            .add_enabled(!pending, egui::Button::new(t(lang, "common.cancel")))
                            .clicked()
                        {
                            effect = Some(Effect::Cancel);
                        }
                        if pending {
                            ui.spinner();
                            ui.label(t(lang, "permission.pending"));
                        }
                    });
                });
        });

        effect
    }
}
