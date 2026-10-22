use std::collections::HashMap;

use image::RgbaImage;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::event_loop::EventLoopProxy;
use winit::window::WindowId;

use crate::capture;
use crate::capture::CapturedMonitor;
use crate::overlay::Overlay;
use crate::pin::PinWindow;

/// 跨线程送回主线程的事件。
pub enum UserEvent {
    /// 后台抓屏完成。
    Captured(Vec<CapturedMonitor>),
    /// 请求创建贴图窗口（图片为选区合成结果）。
    Pin(RgbaImage),
    /// 后台扫描完成，路由回对应覆盖层。
    ScanResult {
        window_id: WindowId,
        text: Result<String, String>,
    },
}

pub struct App {
    proxy: EventLoopProxy<UserEvent>,
    overlays: HashMap<WindowId, Overlay>,
    pins: HashMap<WindowId, PinWindow>,
    capturing: bool,
}

impl App {
    pub fn new(proxy: EventLoopProxy<UserEvent>) -> Self {
        Self {
            proxy,
            overlays: HashMap::new(),
            pins: HashMap::new(),
            capturing: false,
        }
    }

    fn trigger_capture(&mut self) {
        if self.capturing {
            return;
        }
        self.capturing = true;
        let proxy = self.proxy.clone();
        std::thread::spawn(move || match capture::capture_all() {
            Ok(screens) => {
                let _ = proxy.send_event(UserEvent::Captured(screens));
            }
            Err(e) => log::error!("capture failed: {e}"),
        });
    }

    fn open_overlays(&mut self, event_loop: &ActiveEventLoop, screens: Vec<CapturedMonitor>) {
        self.capturing = false;
        for screen in screens {
            match Overlay::create(event_loop, screen, self.proxy.clone()) {
                Ok(overlay) => {
                    let id = overlay.window_id();
                    self.overlays.insert(id, overlay);
                }
                Err(e) => log::error!("failed to create overlay: {e}"),
            }
        }
        if self.overlays.is_empty() && self.pins.is_empty() {
            event_loop.exit();
        }
    }

    fn open_pin(&mut self, event_loop: &ActiveEventLoop, image: RgbaImage) {
        match PinWindow::create(event_loop, image) {
            Ok(pin) => {
                let id = pin.window_id();
                self.pins.insert(id, pin);
                // 贴图窗口创建成功后，结束本次截图会话
                self.overlays.clear();
            }
            Err(e) => log::error!("failed to create pin window: {e}"),
        }
    }
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {
        self.trigger_capture();
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::Captured(screens) => self.open_overlays(event_loop, screens),
            UserEvent::Pin(image) => self.open_pin(event_loop, image),
            UserEvent::ScanResult { window_id, text } => {
                if let Some(overlay) = self.overlays.get_mut(&window_id) {
                    overlay.set_scan_result(text);
                    overlay.request_redraw();
                }
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if let Some(overlay) = self.overlays.get_mut(&window_id) {
            let close = overlay.handle_window_event(event_loop, &event);
            if close {
                // Esc / 动作完成对整个截图会话生效：关闭所有屏幕的覆盖层
                self.overlays.clear();
                if self.pins.is_empty() {
                    event_loop.exit();
                }
            }
            return;
        }

        if let Some(pin) = self.pins.get_mut(&window_id) {
            let close = pin.handle_window_event(&event);
            if close {
                self.pins.remove(&window_id);
                if self.pins.is_empty() && self.overlays.is_empty() {
                    event_loop.exit();
                }
            }
        }
    }
}
