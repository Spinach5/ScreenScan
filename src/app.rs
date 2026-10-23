use std::collections::HashMap;
use std::path::PathBuf;
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

use crate::autostart;
use crate::capture;
use crate::capture::CapturedMonitor;
use crate::config::ConfigStore;
use crate::config::TrayIconSetting;
use crate::font;
use crate::font::FontEntry;
use crate::history;
use crate::i18n::t;
use crate::overlay::Overlay;
use crate::pin::PinWindow;
use crate::platform;
use crate::settings::Effect as SettingsEffect;
use crate::settings::SettingsWindow;
use crate::tray;
use crate::tray::Tray;
use crate::util;

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
    /// 托盘菜单点击（携带菜单项 id）。
    TrayAction(String),
    /// 系统字体扫描完成。
    FontsScanned(Vec<FontEntry>),
    /// 自定义托盘图标文件选择结果。
    TrayIconChosen(Option<PathBuf>),
    /// 配置文件目录选择结果。
    ConfigDirChosen(Option<PathBuf>),
}

pub struct App {
    proxy: EventLoopProxy<UserEvent>,
    overlays: HashMap<WindowId, Overlay>,
    pins: HashMap<WindowId, PinWindow>,
    /// 鼠标所在的「活动」覆盖层（亮），其余覆盖层暗。
    active_overlay: Option<WindowId>,
    /// 去抖：待确认的活动窗口（连续两次命中才切换）。
    pending_active: Option<WindowId>,
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
    store: ConfigStore,
    tray: Option<Tray>,
    settings: Option<SettingsWindow>,
    fonts: Vec<FontEntry>,
    fonts_scanning: bool,
    /// `--preferences`：启动后直接打开设置窗口。
    show_settings_on_start: bool,
}

impl App {
    pub fn new(
        proxy: EventLoopProxy<UserEvent>,
        one_shot: bool,
        show_settings_on_start: bool,
        store: ConfigStore,
    ) -> Self {
        Self {
            proxy,
            overlays: HashMap::new(),
            pins: HashMap::new(),
            active_overlay: None,
            pending_active: None,
            capturing: false,
            started: false,
            exit_deadline: None,
            one_shot,
            resident: false,
            hotkey_manager: None,
            capture_hotkey: None,
            quit_hotkey: None,
            store,
            tray: None,
            settings: None,
            fonts: Vec::new(),
            fonts_scanning: false,
            show_settings_on_start,
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
        let filename_template = self.store.config.effective_filename_format().to_owned();

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
            match Overlay::create(
                event_loop,
                screen,
                self.proxy.clone(),
                filename_template.clone(),
            ) {
                Ok(overlay) => {
                    let id = overlay.window_id();
                    self.overlays.insert(id, overlay);
                }
                Err(e) => log::error!("failed to create overlay: {e}"),
            }
        }

        // 初始活动屏：优先根据全局指针位置判定；否则主显示器。
        let pointer = platform::pointer_position();
        if let Some((x, y)) = pointer {
            log::debug!("initial pointer at ({x},{y})");
        }
        let initial = pointer
            .and_then(|(x, y)| {
                self.overlays
                    .values()
                    .find(|o| o.contains_global_point(x, y))
                    .map(|o| o.window_id())
            })
            .or_else(|| {
                self.overlays
                    .values()
                    .find(|o| o.is_primary())
                    .map(|o| o.window_id())
            })
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
        log::debug!("active overlay -> {active:?}");
        for (id, overlay) in self.overlays.iter_mut() {
            overlay.set_active(Some(*id) == active);
            overlay.request_redraw();
        }
    }

    /// 鼠标进入某覆盖层时切换活动屏；拖拽中不切换，避免中断操作。
    fn maybe_activate(&mut self, window_id: WindowId) {
        if self.active_overlay == Some(window_id) {
            self.pending_active = None;
            return;
        }
        // 去抖：偶发的伪造 enter 只命中一次，不触发切换
        if self.pending_active != Some(window_id) {
            self.pending_active = Some(window_id);
            return;
        }
        self.pending_active = None;

        if let Some(active) = self.active_overlay {
            if self.overlays.get(&active).is_some_and(|o| o.is_dragging()) {
                return;
            }
        }
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

    fn create_tray(&mut self) {
        match Tray::new(
            self.store.config.language,
            &self.store.config.tray_icon,
            self.proxy.clone(),
        ) {
            Ok(tray) => self.tray = Some(tray),
            Err(e) => log::error!("创建托盘图标失败: {e}"),
        }
    }

    fn open_settings(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(settings) = &self.settings {
            settings.focus();
            return;
        }
        match SettingsWindow::create(event_loop, self.store.config.language) {
            Ok(settings) => self.settings = Some(settings),
            Err(e) => log::error!("创建设置窗口失败: {e}"),
        }
    }

    fn handle_tray_action(&mut self, action: &str, event_loop: &ActiveEventLoop) {
        match action {
            tray::ACTION_CAPTURE => self.trigger_capture(),
            tray::ACTION_PIN => log::info!("贴图流程入口（预留）"),
            tray::ACTION_CLEAR => {
                let lang = self.store.config.language;
                match history::clear() {
                    Ok(()) => self.set_status(t(lang, "status.history_cleared").to_owned()),
                    Err(e) => log::warn!("清空截图记录失败: {e}"),
                }
            }
            tray::ACTION_PREFERENCES => self.open_settings(event_loop),
            tray::ACTION_QUIT => event_loop.exit(),
            other => log::warn!("未知托盘菜单项: {other}"),
        }
    }

    fn set_status(&mut self, message: String) {
        if let Some(settings) = &mut self.settings {
            settings.set_status(message);
            settings.request_redraw();
        }
    }

    fn save_config(&mut self) {
        if let Err(e) = self.store.save() {
            let lang = self.store.config.language;
            log::error!("保存配置失败: {e}");
            self.set_status(format!("{}: {e}", t(lang, "status.save_failed")));
        }
    }

    fn scan_fonts(&mut self) {
        if self.fonts_scanning {
            return;
        }
        self.fonts_scanning = true;
        let proxy = self.proxy.clone();
        std::thread::spawn(move || {
            let fonts = font::scan_system_fonts();
            let _ = proxy.send_event(UserEvent::FontsScanned(fonts));
        });
        if let Some(settings) = &mut self.settings {
            settings.request_redraw();
        }
    }

    fn pick_tray_icon(&self) {
        let proxy = self.proxy.clone();
        std::thread::spawn(move || {
            let path = rfd::FileDialog::new()
                .set_title("选择托盘图标")
                .add_filter("Image", &["png", "jpg", "jpeg", "webp", "bmp", "gif"])
                .pick_file();
            let _ = proxy.send_event(UserEvent::TrayIconChosen(path));
        });
    }

    fn pick_config_dir(&self) {
        let proxy = self.proxy.clone();
        std::thread::spawn(move || {
            let dir = rfd::FileDialog::new()
                .set_title("选择配置文件存储位置")
                .pick_folder();
            let _ = proxy.send_event(UserEvent::ConfigDirChosen(dir));
        });
    }

    fn apply_tray_icon(&mut self, setting: TrayIconSetting) {
        let result = match &self.tray {
            Some(tray) => tray.set_icon(&setting),
            None => Ok(()),
        };
        let lang = self.store.config.language;
        match result {
            Ok(()) => {
                self.store.config.tray_icon = setting;
                if let Some(settings) = &mut self.settings {
                    settings.invalidate_preview();
                    settings.set_status(t(lang, "status.icon_updated"));
                    settings.request_redraw();
                }
                self.save_config();
            }
            Err(e) => {
                log::error!("更新托盘图标失败: {e}");
                self.set_status(format!("{}: {e}", t(lang, "status.icon_failed")));
            }
        }
    }

    fn apply_settings_effects(&mut self, effects: Vec<SettingsEffect>) {
        let mut save = false;
        for effect in effects {
            match effect {
                SettingsEffect::Save => save = true,
                SettingsEffect::ScanFonts => self.scan_fonts(),
                SettingsEffect::FontSelected(choice) => {
                    let lang = self.store.config.language;
                    let valid = match &choice {
                        None => true,
                        Some(choice) => match std::fs::read(&choice.path) {
                            Ok(_) => true,
                            Err(e) => {
                                log::warn!("读取字体失败 {}: {e}", choice.path.display());
                                self.set_status(t(lang, "status.font_failed").to_owned());
                                false
                            }
                        },
                    };
                    if valid {
                        font::set_choice(choice.clone());
                        self.store.config.font = choice;
                        if let Some(settings) = &mut self.settings {
                            settings.reinstall_fonts();
                        }
                        self.set_status(t(lang, "status.font_updated").to_owned());
                        save = true;
                    }
                }
                SettingsEffect::TrayIcon(setting) => self.apply_tray_icon(setting),
                SettingsEffect::PickTrayIcon => self.pick_tray_icon(),
                SettingsEffect::PickConfigDir => self.pick_config_dir(),
                SettingsEffect::Language(lang) => {
                    self.store.config.language = lang;
                    if let Some(tray) = &mut self.tray {
                        tray.set_language(lang);
                    }
                    if let Some(settings) = &mut self.settings {
                        settings.set_language(lang);
                    }
                    save = true;
                }
                SettingsEffect::Autostart(enabled) => {
                    let lang = self.store.config.language;
                    if let Err(e) = autostart::set(enabled) {
                        log::error!("设置开机启动失败: {e}");
                        self.store.config.autostart = !enabled;
                        self.set_status(format!("{}: {e}", t(lang, "status.autostart_failed")));
                    }
                    save = true;
                }
                SettingsEffect::LogLevel => {
                    log::set_max_level(self.store.config.log_level.filter());
                    save = true;
                }
                SettingsEffect::OpenLogDir => {
                    if let Err(e) = crate::logging::open_log_dir() {
                        let lang = self.store.config.language;
                        log::warn!("打开日志目录失败: {e}");
                        self.set_status(format!("{}: {e}", t(lang, "status.open_log_failed")));
                    }
                }
            }
        }
        if save {
            self.save_config();
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

        if self.one_shot {
            self.setup_hotkeys(event_loop);
            self.trigger_capture();
            return;
        }

        // 无参数启动：不显示主窗口，直接驻留托盘。
        self.resident = true;
        self.create_tray();
        self.setup_hotkeys(event_loop);
        if self.show_settings_on_start {
            self.open_settings(event_loop);
        }
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
            UserEvent::TrayAction(action) => self.handle_tray_action(&action, event_loop),
            UserEvent::FontsScanned(fonts) => {
                self.fonts = fonts;
                self.fonts_scanning = false;
                if let Some(settings) = &mut self.settings {
                    settings.request_redraw();
                }
            }
            UserEvent::TrayIconChosen(Some(path)) => {
                self.apply_tray_icon(TrayIconSetting::Custom(path));
            }
            UserEvent::TrayIconChosen(None) => {
                if let Some(settings) = &mut self.settings {
                    settings.request_redraw();
                }
            }
            UserEvent::ConfigDirChosen(Some(dir)) => {
                let lang = self.store.config.language;
                match self.store.migrate_to_folder(&dir) {
                    Ok(new_path) => {
                        let display = util::path::display_path(&new_path);
                        log::info!("配置文件已迁移到 {display}");
                        self.set_status(format!("{} {display}", t(lang, "status.migrated")));
                    }
                    Err(e) => {
                        log::error!("迁移配置文件失败: {e}");
                        self.set_status(format!("{}: {e}", t(lang, "status.save_failed")));
                    }
                }
                if let Some(settings) = &mut self.settings {
                    settings.request_redraw();
                }
            }
            UserEvent::ConfigDirChosen(None) => {
                if let Some(settings) = &mut self.settings {
                    settings.request_redraw();
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
        if self
            .settings
            .as_ref()
            .is_some_and(|settings| settings.window_id() == window_id)
        {
            if matches!(&event, WindowEvent::RedrawRequested) {
                let display_path = self.store.display_path();
                let fonts_scanning = self.fonts_scanning;
                let fonts = &self.fonts;
                let settings = self.settings.as_mut().expect("settings window exists");
                let effects =
                    settings.redraw(&mut self.store.config, &display_path, fonts, fonts_scanning);
                self.apply_settings_effects(effects);
            } else {
                let close = self
                    .settings
                    .as_mut()
                    .is_some_and(|settings| settings.handle_window_event(&event));
                if close {
                    self.settings = None;
                }
            }
            return;
        }

        if self.overlays.contains_key(&window_id) {
            // 用「指针真实坐标是否落在本窗口内」判定活动屏：
            // 窗口映射/全屏过程中可能收到伪造的 CursorEntered，其坐标会越界。
            let mut activate = false;
            if let WindowEvent::CursorMoved { position, .. } = &event {
                if let Some(overlay) = self.overlays.get(&window_id) {
                    let size = overlay.window_inner_size();
                    activate = position.x >= 0.0
                        && position.y >= 0.0
                        && position.x < size.width as f64 + 0.5
                        && position.y < size.height as f64 + 0.5;
                }
            }
            if activate {
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
