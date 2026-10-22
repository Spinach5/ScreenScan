use std::collections::HashMap;

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::event_loop::EventLoopProxy;
use winit::window::WindowId;

use crate::capture;
use crate::capture::CapturedMonitor;
use crate::overlay::Overlay;

/// 跨线程送回主线程的事件。
pub enum UserEvent {
    /// 后台抓屏完成。
    Captured(Vec<CapturedMonitor>),
}

pub struct App {
    proxy: EventLoopProxy<UserEvent>,
    overlays: HashMap<WindowId, Overlay>,
    capturing: bool,
}

impl App {
    pub fn new(proxy: EventLoopProxy<UserEvent>) -> Self {
        Self {
            proxy,
            overlays: HashMap::new(),
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
            match Overlay::create(event_loop, screen) {
                Ok(overlay) => {
                    let id = overlay.window_id();
                    self.overlays.insert(id, overlay);
                }
                Err(e) => log::error!("failed to create overlay: {e}"),
            }
        }
        if self.overlays.is_empty() {
            event_loop.exit();
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
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(overlay) = self.overlays.get_mut(&window_id) else {
            return;
        };
        let close = overlay.handle_window_event(event_loop, &event);
        if close {
            self.overlays.remove(&window_id);
            if self.overlays.is_empty() {
                event_loop.exit();
            }
        }
    }
}
