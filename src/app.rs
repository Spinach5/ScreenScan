use std::collections::HashMap;
use std::time::Duration;
use std::time::Instant;

use global_hotkey::GlobalHotKeyEvent;
use global_hotkey::GlobalHotKeyManager;
use global_hotkey::HotKeyState;
use global_hotkey::hotkey::Code;
use global_hotkey::hotkey::HotKey;
use global_hotkey::hotkey::Modifiers;
use image::RgbaImage;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::event_loop::ControlFlow;
use winit::event_loop::EventLoopProxy;
use winit::window::WindowId;

use crate::capture;
use crate::capture::CapturedMonitor;
use crate::overlay::Overlay;
use crate::pin::PinWindow;
use crate::platform;

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
    /// 抓屏失败。
    CaptureFailed(String),
    /// 全局快捷键事件。
    Hotkey { id: u32, pressed: bool },
}

pub struct App {
    proxy: EventLoopProxy<UserEvent>,
    overlays: HashMap<WindowId, Overlay>,
    pins: HashMap<WindowId, PinWindow>,
    /// 鼠标所在的「活动」覆盖层（亮），其余覆盖层暗。
    active_overlay: Option<WindowId>,
    capturing: bool,
    started: bool,
    /// 延迟退出时刻（一次性模式：留出剪贴板管理器抓取内容的时间）。
    exit_deadline: Option<Instant>,
    /// 执行一次截图后退出（`-c` / Wayland）。
    one_shot: bool,
    /// 常驻等待全局快捷键。
    resident: bool,
    hotkey_manager: Option<GlobalHotKeyManager>,
    capture_hotkey: Option<u32>,
    quit_hotkey: Option<u32>,
}

impl App {
    pub fn new(proxy: EventLoopProxy<UserEvent>, one_shot: bool) -> Self {
        Self {
            proxy,
            overlays: HashMap::new(),
            pins: HashMap::new(),
            active_overlay: None,
            capturing: false,
            started: false,
            exit_deadline: None,
            one_shot,
            resident: false,
            hotkey_manager: None,
            capture_hotkey: None,
            quit_hotkey: None,
        }
    }

    fn trigger_capture(&mut self) {
        if self.capturing || !self.overlays.is_empty() {
            return;
        }
        self.capturing = true;
        let proxy = self.proxy.clone();
        std::thread::spawn(move || match capture::capture_all() {
            Ok(screens) => {
                let _ = proxy.send_event(UserEvent::Captured(screens));
            }
            Err(e) => {
                let _ = proxy.send_event(UserEvent::CaptureFailed(e.to_string()));
            }
        });
    }

    fn open_overlays(&mut self, event_loop: &ActiveEventLoop, screens: Vec<CapturedMonitor>) {
        self.capturing = false;

        for screen in screens {
            log::info!(
                "overlay: monitor {} ({}) {}x{} @ ({},{}) scale {}",
                screen.monitor.id,
                screen.monitor.name,
                screen.monitor.width,
                screen.monitor.height,
                screen.monitor.x,
                screen.monitor.y,
                screen.monitor.scale_factor
            );
            match Overlay::create(event_loop, screen, self.proxy.clone()) {
                Ok(overlay) => {
                    let id = overlay.window_id();
                    self.overlays.insert(id, overlay);
                }
                Err(e) => log::error!("failed to create overlay: {e}"),
            }
        }

        // 初始活动屏：默认主显示器；鼠标进入其他屏幕后会立即切换。
        let initial = self
            .overlays
            .values()
            .find(|o| o.is_primary())
            .map(|o| o.window_id())
            .or_else(|| self.overlays.keys().next().copied());
        self.set_active_window(initial);

        if !self.resident && self.overlays.is_empty() && self.pins.is_empty() {
            self.schedule_exit(event_loop);
        }
    }

    /// 切换活动覆盖层：活动层亮，其余暗。
    fn set_active_window(&mut self, active: Option<WindowId>) {
        if self.active_overlay == active {
            return;
        }
        self.active_overlay = active;
        for (id, overlay) in self.overlays.iter_mut() {
            overlay.set_active(Some(*id) == active);
            overlay.request_redraw();
        }
    }

    /// 鼠标进入某覆盖层时切换活动屏；拖拽中不切换，避免中断操作。
    fn maybe_activate(&mut self, window_id: WindowId) {
        if self.active_overlay == Some(window_id) {
            return;
        }
        if let Some(active) = self.active_overlay {
            if self.overlays.get(&active).is_some_and(|o| o.is_dragging()) {
                return;
            }
        }
        log::debug!("active overlay -> {:?}", window_id);
        self.set_active_window(Some(window_id));
    }

    /// 一次性模式退出前稍作延迟，给剪贴板管理器抓取内容的时间
    /// （X11 剪贴板内容随进程退出而失效）。
    fn schedule_exit(&mut self, event_loop: &ActiveEventLoop) {
        let deadline = Instant::now() + Duration::from_millis(500);
        self.exit_deadline = Some(deadline);
        event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
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

    fn setup_hotkeys(&mut self, event_loop: &ActiveEventLoop) {
        if self.one_shot || platform::is_wayland() {
            log::info!(
                "一次性模式或 Wayland：不注册全局快捷键。\
                 可让桌面环境把快捷键绑定到 `screenscan -c`。"
            );
            return;
        }

        match GlobalHotKeyManager::new() {
            Ok(manager) => {
                let capture_key =
                    HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyS);
                let quit_key = HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyQ);

                if let Err(e) = manager.register(capture_key) {
                    log::warn!("注册截图快捷键失败: {e}");
                }
                if let Err(e) = manager.register(quit_key) {
                    log::warn!("注册退出快捷键失败: {e}");
                }

                self.capture_hotkey = Some(capture_key.id());
                self.quit_hotkey = Some(quit_key.id());
                self.hotkey_manager = Some(manager);
                self.resident = true;

                // 快捷键事件在独立线程产生，转成 UserEvent 唤醒主事件循环
                let receiver = GlobalHotKeyEvent::receiver().clone();
                let proxy = self.proxy.clone();
                std::thread::spawn(move || {
                    while let Ok(event) = receiver.recv() {
                        let pressed = event.state == HotKeyState::Pressed;
                        let msg = UserEvent::Hotkey {
                            id: event.id,
                            pressed,
                        };
                        if proxy.send_event(msg).is_err() {
                            break;
                        }
                    }
                });

                log::info!("全局快捷键: Ctrl+Shift+S 截图 / Ctrl+Shift+Q 退出");
            }
            Err(e) => {
                log::warn!("初始化全局快捷键失败，退回一次性模式: {e}");
                let _ = event_loop;
            }
        }
    }
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.started {
            return;
        }
        self.started = true;
        self.setup_hotkeys(event_loop);
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
            UserEvent::CaptureFailed(message) => {
                self.capturing = false;
                log::error!("capture failed: {message}");
                if !self.resident && self.overlays.is_empty() && self.pins.is_empty() {
                    self.schedule_exit(event_loop);
                }
            }
            UserEvent::Hotkey { id, pressed } => {
                if !pressed {
                    return;
                }
                if Some(id) == self.quit_hotkey {
                    log::info!("收到退出快捷键");
                    event_loop.exit();
                } else if Some(id) == self.capture_hotkey {
                    self.trigger_capture();
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
        if self.overlays.contains_key(&window_id) {
            // 鼠标进入/在屏幕上移动时，将其设为活动屏（亮），其余变暗
            if matches!(
                event,
                WindowEvent::CursorEntered { .. } | WindowEvent::CursorMoved { .. }
            ) {
                self.maybe_activate(window_id);
            }

            let Some(overlay) = self.overlays.get_mut(&window_id) else {
                return;
            };
            let close = overlay.handle_window_event(event_loop, &event);
            if close {
                // Esc / 动作完成对整个截图会话生效：关闭所有屏幕的覆盖层
                self.overlays.clear();
                self.active_overlay = None;
                if !self.resident && self.pins.is_empty() {
                    self.schedule_exit(event_loop);
                }
            }
            return;
        }

        if let Some(pin) = self.pins.get_mut(&window_id) {
            let close = pin.handle_window_event(&event);
            if close {
                self.pins.remove(&window_id);
                if !self.resident && self.pins.is_empty() && self.overlays.is_empty() {
                    self.schedule_exit(event_loop);
                }
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(deadline) = self.exit_deadline {
            if Instant::now() >= deadline {
                event_loop.exit();
            } else {
                event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
            }
        }
    }
}
