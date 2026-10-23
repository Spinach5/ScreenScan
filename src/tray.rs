//! 系统托盘：菜单、语言更新与图标切换。
//!
//! Linux 使用 tray-icon 的 ksni（StatusNotifierItem）后端，不依赖 GTK。

use std::path::Path;

use anyhow::Context;
use anyhow::Result;
use image::RgbaImage;
use tray_icon::Icon;
use tray_icon::TrayIcon;
use tray_icon::TrayIconBuilder;
use tray_icon::menu::Menu;
use tray_icon::menu::MenuEvent;
use tray_icon::menu::MenuItem;
use tray_icon::menu::PredefinedMenuItem;
use winit::event_loop::EventLoopProxy;

use crate::app::UserEvent;
use crate::config::TrayIconSetting;
use crate::i18n::Language;
use crate::i18n::t;

pub const ACTION_CAPTURE: &str = "screenscan.capture";
pub const ACTION_PIN: &str = "screenscan.pin";
pub const ACTION_CLEAR: &str = "screenscan.clear";
pub const ACTION_PREFERENCES: &str = "screenscan.preferences";
pub const ACTION_QUIT: &str = "screenscan.quit";

const DEFAULT_ICON: &[u8] = include_bytes!("../assets/tray-icon.png");

pub struct Tray {
    tray: TrayIcon,
    capture: MenuItem,
    pin: MenuItem,
    clear: MenuItem,
    preferences: MenuItem,
    quit: MenuItem,
    language: Language,
}

impl Tray {
    pub fn new(
        language: Language,
        icon: &TrayIconSetting,
        proxy: EventLoopProxy<UserEvent>,
    ) -> Result<Self> {
        // 菜单事件在托盘线程产生，转发到 winit 事件循环。
        // 使用通道线程而不是 set_event_handler：后者要求回调 Send + Sync，
        // 而 macOS 上的 EventLoopProxy 并非 Sync。
        let receiver = MenuEvent::receiver().clone();
        std::thread::spawn(move || {
            while let Ok(event) = receiver.recv() {
                let action = event.id().as_ref().to_owned();
                if proxy.send_event(UserEvent::TrayAction(action)).is_err() {
                    break;
                }
            }
        });

        let capture = MenuItem::with_id(ACTION_CAPTURE, t(language, "tray.capture"), true, None);
        let pin = MenuItem::with_id(ACTION_PIN, t(language, "tray.pin"), true, None);
        let clear = MenuItem::with_id(ACTION_CLEAR, t(language, "tray.clear"), true, None);
        let preferences = MenuItem::with_id(
            ACTION_PREFERENCES,
            t(language, "tray.preferences"),
            true,
            None,
        );
        let quit = MenuItem::with_id(ACTION_QUIT, t(language, "tray.quit"), true, None);

        let separator1 = PredefinedMenuItem::separator();
        let separator2 = PredefinedMenuItem::separator();
        let menu = Menu::new();
        menu.append_items(&[
            &capture,
            &pin,
            &clear,
            &separator1,
            &preferences,
            &separator2,
            &quit,
        ])
        .map_err(|e| anyhow::anyhow!("构建托盘菜单失败: {e}"))?;

        let icon = load_icon(icon).or_else(|e| {
            log::warn!("自定义托盘图标不可用，回退到默认图标: {e}");
            load_icon(&TrayIconSetting::Default)
        })?;

        let tray = TrayIconBuilder::new()
            .with_id("screenscan")
            .with_tooltip("ScreenScan")
            .with_menu(Box::new(menu))
            .with_icon(icon)
            .with_menu_on_left_click(true)
            .with_menu_on_right_click(true)
            .build()
            .map_err(|e| anyhow::anyhow!("创建托盘图标失败: {e}"))?;

        Ok(Self {
            tray,
            capture,
            pin,
            clear,
            preferences,
            quit,
            language,
        })
    }

    pub fn set_language(&mut self, language: Language) {
        if self.language == language {
            return;
        }
        self.language = language;
        self.capture.set_text(t(language, "tray.capture"));
        self.pin.set_text(t(language, "tray.pin"));
        self.clear.set_text(t(language, "tray.clear"));
        self.preferences.set_text(t(language, "tray.preferences"));
        self.quit.set_text(t(language, "tray.quit"));
    }

    pub fn set_icon(&self, setting: &TrayIconSetting) -> Result<()> {
        self.tray
            .set_icon(Some(load_icon(setting)?))
            .map_err(|e| anyhow::anyhow!("更新托盘图标失败: {e}"))
    }
}

fn load_icon(setting: &TrayIconSetting) -> Result<Icon> {
    let image = match setting {
        TrayIconSetting::Default => default_icon_image()?,
        TrayIconSetting::Custom(path) => load_custom_icon(path)?,
    };
    icon_from_rgba(image)
}

pub fn default_icon_image() -> Result<RgbaImage> {
    image::load_from_memory(DEFAULT_ICON)
        .map(|image| image.to_rgba8())
        .context("内置托盘图标损坏")
}

fn load_custom_icon(path: &Path) -> Result<RgbaImage> {
    let image = image::open(path)
        .with_context(|| format!("读取图标失败: {}", path.display()))?
        .to_rgba8();

    let (width, height) = image.dimensions();
    if width > 128 || height > 128 {
        let scale = 64.0 / width.max(height) as f32;
        let width = (width as f32 * scale).round().max(1.0) as u32;
        let height = (height as f32 * scale).round().max(1.0) as u32;
        return Ok(image::imageops::resize(
            &image,
            width,
            height,
            image::imageops::FilterType::Lanczos3,
        ));
    }
    Ok(image)
}

fn icon_from_rgba(image: RgbaImage) -> Result<Icon> {
    let (width, height) = image.dimensions();
    Icon::from_rgba(image.into_raw(), width, height)
        .map_err(|e| anyhow::anyhow!("图标格式无效: {e}"))
}
