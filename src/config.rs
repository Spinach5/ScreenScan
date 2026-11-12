//! `config.ini` 的读写、迁移与设置项定义。

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use egui::Key;
use egui::Modifiers;

use crate::font::FontChoice;
use crate::i18n::Language;
use crate::i18n::t;
use crate::util::path;

pub const DEFAULT_FILENAME_FORMAT: &str = "screenscan_$yyyy-MM-dd_HH-mm-ss$.png";

/// 覆盖层内可配置的键盘快捷键。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shortcut {
    pub key: Key,
    pub modifiers: Modifiers,
}

impl Shortcut {
    pub const fn new(modifiers: Modifiers, key: Key) -> Self {
        Self { key, modifiers }
    }

    /// 解析 `Ctrl+Shift+C` 形式；无法识别时返回 `None`。
    pub fn parse(text: &str) -> Option<Self> {
        let mut modifiers = Modifiers::NONE;
        let mut key = None;
        for part in text
            .split('+')
            .map(str::trim)
            .filter(|part| !part.is_empty())
        {
            match part.to_ascii_lowercase().as_str() {
                "ctrl" | "control" => {
                    modifiers.ctrl = true;
                    // 让配置在 Windows/Linux 与 macOS 之间通用。
                    modifiers.command = true;
                }
                "shift" => modifiers.shift = true,
                "alt" | "option" => modifiers.alt = true,
                "cmd" | "command" | "super" | "win" | "meta" => modifiers.mac_cmd = true,
                _ => key = Key::from_name(part),
            }
        }
        Some(Self {
            key: key?,
            modifiers,
        })
    }

    pub fn to_ini(self) -> String {
        let mut out = String::new();
        if self.modifiers.mac_cmd {
            out.push_str("Cmd+");
        } else if self.modifiers.ctrl || self.modifiers.command {
            out.push_str("Ctrl+");
        }
        if self.modifiers.shift {
            out.push_str("Shift+");
        }
        if self.modifiers.alt {
            out.push_str("Alt+");
        }
        out.push_str(self.key.name());
        out
    }

    pub fn display(self) -> String {
        let key = match self.key {
            Key::ArrowUp => "↑",
            Key::ArrowDown => "↓",
            Key::ArrowLeft => "←",
            Key::ArrowRight => "→",
            key => key.name(),
        };
        let mut parts = Vec::with_capacity(4);
        if self.modifiers.mac_cmd {
            parts.push("Cmd");
        } else if self.modifiers.ctrl || self.modifiers.command {
            parts.push("Ctrl");
        }
        if self.modifiers.shift {
            parts.push("Shift");
        }
        if self.modifiers.alt {
            parts.push("Alt");
        }
        parts.push(key);
        parts.join("+")
    }

    pub fn matches_event(self, event: &egui::Event) -> bool {
        match event {
            egui::Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } => *key == self.key && modifiers.matches_exact(self.modifiers),
            _ => false,
        }
    }
}

/// 覆盖层内可重绑定的动作。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShortcutAction {
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    Copy,
    SelectAll,
    Pin,
}

impl ShortcutAction {
    pub const ALL: [Self; 7] = [
        Self::MoveUp,
        Self::MoveDown,
        Self::MoveLeft,
        Self::MoveRight,
        Self::Copy,
        Self::SelectAll,
        Self::Pin,
    ];

    pub fn config_key(self) -> &'static str {
        match self {
            Self::MoveUp => "move_up",
            Self::MoveDown => "move_down",
            Self::MoveLeft => "move_left",
            Self::MoveRight => "move_right",
            Self::Copy => "copy",
            Self::SelectAll => "select_all",
            Self::Pin => "pin",
        }
    }

    pub fn default_shortcut(self) -> Shortcut {
        match self {
            Self::MoveUp => Shortcut::new(Modifiers::NONE, Key::ArrowUp),
            Self::MoveDown => Shortcut::new(Modifiers::NONE, Key::ArrowDown),
            Self::MoveLeft => Shortcut::new(Modifiers::NONE, Key::ArrowLeft),
            Self::MoveRight => Shortcut::new(Modifiers::NONE, Key::ArrowRight),
            Self::Copy => Shortcut::new(Modifiers::CTRL, Key::C),
            Self::SelectAll => Shortcut::new(Modifiers::CTRL, Key::A),
            Self::Pin => Shortcut::new(Modifiers::CTRL, Key::P),
        }
    }

    pub fn label(self, lang: Language) -> &'static str {
        match self {
            Self::MoveUp => t(lang, "shortcuts.move_up"),
            Self::MoveDown => t(lang, "shortcuts.move_down"),
            Self::MoveLeft => t(lang, "shortcuts.move_left"),
            Self::MoveRight => t(lang, "shortcuts.move_right"),
            Self::Copy => t(lang, "shortcuts.copy"),
            Self::SelectAll => t(lang, "shortcuts.select_all"),
            Self::Pin => t(lang, "shortcuts.pin"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shortcuts {
    pub move_up: Shortcut,
    pub move_down: Shortcut,
    pub move_left: Shortcut,
    pub move_right: Shortcut,
    pub copy: Shortcut,
    pub select_all: Shortcut,
    pub pin: Shortcut,
}

impl Default for Shortcuts {
    fn default() -> Self {
        Self {
            move_up: ShortcutAction::MoveUp.default_shortcut(),
            move_down: ShortcutAction::MoveDown.default_shortcut(),
            move_left: ShortcutAction::MoveLeft.default_shortcut(),
            move_right: ShortcutAction::MoveRight.default_shortcut(),
            copy: ShortcutAction::Copy.default_shortcut(),
            select_all: ShortcutAction::SelectAll.default_shortcut(),
            pin: ShortcutAction::Pin.default_shortcut(),
        }
    }
}

impl Shortcuts {
    pub fn get(self, action: ShortcutAction) -> Shortcut {
        match action {
            ShortcutAction::MoveUp => self.move_up,
            ShortcutAction::MoveDown => self.move_down,
            ShortcutAction::MoveLeft => self.move_left,
            ShortcutAction::MoveRight => self.move_right,
            ShortcutAction::Copy => self.copy,
            ShortcutAction::SelectAll => self.select_all,
            ShortcutAction::Pin => self.pin,
        }
    }

    pub fn set(&mut self, action: ShortcutAction, shortcut: Shortcut) {
        match action {
            ShortcutAction::MoveUp => self.move_up = shortcut,
            ShortcutAction::MoveDown => self.move_down = shortcut,
            ShortcutAction::MoveLeft => self.move_left = shortcut,
            ShortcutAction::MoveRight => self.move_right = shortcut,
            ShortcutAction::Copy => self.copy = shortcut,
            ShortcutAction::SelectAll => self.select_all = shortcut,
            ShortcutAction::Pin => self.pin = shortcut,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Off,
    Debug,
    Info,
    Error,
}

impl LogLevel {
    pub const ALL: [Self; 4] = [Self::Off, Self::Debug, Self::Info, Self::Error];

    pub fn ini_value(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Error => "error",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "off" | "none" | "无" => Self::Off,
            "debug" | "调试" => Self::Debug,
            "error" | "错误" => Self::Error,
            _ => Self::Info,
        }
    }

    pub fn filter(self) -> log::LevelFilter {
        match self {
            Self::Off => log::LevelFilter::Off,
            Self::Debug => log::LevelFilter::Debug,
            Self::Info => log::LevelFilter::Info,
            Self::Error => log::LevelFilter::Error,
        }
    }

    pub fn label(self, lang: Language) -> &'static str {
        match self {
            Self::Off => t(lang, "output.log_off"),
            Self::Debug => t(lang, "output.log_debug"),
            Self::Info => t(lang, "output.log_info"),
            Self::Error => t(lang, "output.log_error"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrayIconSetting {
    Default,
    Custom(PathBuf),
}

impl TrayIconSetting {
    pub fn custom_path(&self) -> Option<&Path> {
        match self {
            Self::Default => None,
            Self::Custom(path) => Some(path),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub autostart: bool,
    pub auto_backup: bool,
    pub font: Option<FontChoice>,
    pub tray_icon: TrayIconSetting,
    pub log_level: LogLevel,
    pub filename_format: String,
    pub language: Language,
    pub shortcuts: Shortcuts,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            autostart: false,
            auto_backup: false,
            font: None,
            tray_icon: TrayIconSetting::Default,
            log_level: LogLevel::Info,
            filename_format: DEFAULT_FILENAME_FORMAT.to_owned(),
            language: Language::ZhCn,
            shortcuts: Shortcuts::default(),
        }
    }
}

impl Config {
    fn from_ini(text: &str) -> Self {
        let ini = parse_ini(text);
        let get = |section: &str, key: &str| {
            ini.get(section)
                .and_then(|values| values.get(key))
                .map(String::as_str)
        };

        let font = get("interface", "font_path")
            .filter(|value| !value.trim().is_empty())
            .map(|value| FontChoice {
                family: get("interface", "font_family")
                    .unwrap_or_default()
                    .to_owned(),
                path: path::expand_tilde(value),
                index: get("interface", "font_index")
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(0),
            });

        let tray_icon = match get("interface", "tray_icon") {
            Some("custom") => get("interface", "tray_icon_path")
                .filter(|value| !value.trim().is_empty())
                .map(|value| TrayIconSetting::Custom(path::expand_tilde(value)))
                .unwrap_or(TrayIconSetting::Default),
            _ => TrayIconSetting::Default,
        };

        let filename_format = get("output", "filename_format")
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(DEFAULT_FILENAME_FORMAT)
            .to_owned();

        let shortcut = |action: ShortcutAction| {
            get("shortcuts", action.config_key())
                .and_then(Shortcut::parse)
                .unwrap_or(action.default_shortcut())
        };
        let shortcuts = Shortcuts {
            move_up: shortcut(ShortcutAction::MoveUp),
            move_down: shortcut(ShortcutAction::MoveDown),
            move_left: shortcut(ShortcutAction::MoveLeft),
            move_right: shortcut(ShortcutAction::MoveRight),
            copy: shortcut(ShortcutAction::Copy),
            select_all: shortcut(ShortcutAction::SelectAll),
            pin: shortcut(ShortcutAction::Pin),
        };

        Self {
            autostart: parse_bool(get("general", "autostart"), false),
            auto_backup: parse_bool(get("general", "auto_backup"), false),
            font,
            tray_icon,
            log_level: LogLevel::parse(get("output", "log_level").unwrap_or("info")),
            filename_format,
            language: Language::parse(get("appearance", "language").unwrap_or("zh-CN")),
            shortcuts,
        }
    }

    fn to_ini(&self) -> String {
        let mut out = String::new();
        out.push_str("[general]\n");
        out.push_str(&format!("autostart={}\n", self.autostart));
        out.push_str(&format!("auto_backup={}\n", self.auto_backup));
        out.push_str("\n[interface]\n");
        if let Some(font) = &self.font {
            out.push_str(&format!("font_path={}\n", font.path.display()));
            out.push_str(&format!("font_index={}\n", font.index));
            out.push_str(&format!("font_family={}\n", font.family));
        }
        match &self.tray_icon {
            TrayIconSetting::Default => out.push_str("tray_icon=default\n"),
            TrayIconSetting::Custom(path) => {
                out.push_str("tray_icon=custom\n");
                out.push_str(&format!("tray_icon_path={}\n", path.display()));
            }
        }
        out.push_str("\n[output]\n");
        out.push_str(&format!("log_level={}\n", self.log_level.ini_value()));
        out.push_str(&format!("filename_format={}\n", self.filename_format));
        out.push_str("\n[appearance]\n");
        out.push_str(&format!("language={}\n", self.language.code()));
        out.push_str("\n[shortcuts]\n");
        for action in ShortcutAction::ALL {
            out.push_str(&format!(
                "{}={}\n",
                action.config_key(),
                self.shortcuts.get(action).to_ini()
            ));
        }
        out
    }

    pub fn effective_filename_format(&self) -> &str {
        if self.filename_format.trim().is_empty() {
            DEFAULT_FILENAME_FORMAT
        } else {
            &self.filename_format
        }
    }

    pub fn preview_filename(&self) -> String {
        format_filename(self.effective_filename_format())
    }
}

/// 读取配置文件；默认位置为 `~/.screenscan/config.ini`。
/// 若默认文件里记录了 `config_path`，则改为加载该自定义位置。
pub struct ConfigStore {
    pub config: Config,
    pub path: PathBuf,
    pub bootstrap_path: PathBuf,
}

impl ConfigStore {
    pub fn load() -> Self {
        let bootstrap_path = default_config_path();
        let mut path = bootstrap_path.clone();

        if let Ok(text) = fs::read_to_string(&bootstrap_path) {
            let ini = parse_ini(&text);
            if let Some(custom) = ini
                .get("storage")
                .and_then(|values| values.get("config_path"))
                .filter(|value| !value.trim().is_empty())
            {
                let custom = path::expand_tilde(custom);
                if custom != bootstrap_path && custom.is_file() {
                    path = custom;
                }
            }
        }

        let config = fs::read_to_string(&path)
            .map(|text| Config::from_ini(&text))
            .unwrap_or_default();

        Self {
            config,
            path,
            bootstrap_path,
        }
    }

    pub fn display_path(&self) -> String {
        path::display_path(&self.path)
    }

    pub fn save(&mut self) -> Result<()> {
        self.write_to(&self.path.clone(), true)
    }

    fn write_to(&self, path: &Path, backup: bool) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("创建配置目录失败: {}", parent.display()))?;
        }
        if backup && self.config.auto_backup && path.is_file() {
            let bak = path.with_extension("ini.bak");
            if let Err(e) = fs::copy(path, &bak) {
                log::warn!("备份配置失败 {}: {e}", bak.display());
            }
        }
        fs::write(path, self.config.to_ini())
            .with_context(|| format!("写入配置失败: {}", path.display()))?;
        if path != self.bootstrap_path {
            write_bootstrap_pointer(&self.bootstrap_path, path)?;
        }
        Ok(())
    }

    /// 将当前配置迁移到新目录下的 `config.ini`。
    pub fn migrate_to_folder(&mut self, folder: &Path) -> Result<PathBuf> {
        let new_path = folder.join("config.ini");
        if new_path == self.path {
            return Ok(new_path);
        }
        self.write_to(&new_path, true)?;
        if self.path != self.bootstrap_path
            && self.path.is_file()
            && let Err(e) = fs::remove_file(&self.path)
        {
            log::warn!("删除旧配置文件失败 {}: {e}", self.path.display());
        }
        self.path = new_path.clone();
        write_bootstrap_pointer(&self.bootstrap_path, &new_path)?;
        Ok(new_path)
    }
}

pub fn default_config_path() -> PathBuf {
    path::home_dir().join(".screenscan").join("config.ini")
}

fn write_bootstrap_pointer(bootstrap: &Path, target: &Path) -> Result<()> {
    if let Some(parent) = bootstrap.parent() {
        fs::create_dir_all(parent)?;
    }
    let contents = format!("[storage]\nconfig_path={}\n", target.display());
    fs::write(bootstrap, contents)
        .with_context(|| format!("写入配置位置记录失败: {}", bootstrap.display()))?;
    Ok(())
}

type Ini = BTreeMap<String, BTreeMap<String, String>>;

fn parse_ini(text: &str) -> Ini {
    let mut ini = Ini::new();
    let mut section = String::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        if let Some(name) = line
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'))
        {
            section = name.trim().to_ascii_lowercase();
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            ini.entry(section.clone())
                .or_default()
                .insert(key.trim().to_ascii_lowercase(), value.trim().to_owned());
        }
    }
    ini
}

fn parse_bool(value: Option<&str>, default: bool) -> bool {
    match value.map(str::trim) {
        Some("true" | "1" | "yes" | "on") => true,
        Some("false" | "0" | "no" | "off") => false,
        _ => default,
    }
}

/// 展开 `$...$` 中的时间占位符，例如
/// `screenscan_$yyyy-MM-dd_HH-mm-ss$.png`。
pub fn format_filename(template: &str) -> String {
    let template = if template.trim().is_empty() {
        DEFAULT_FILENAME_FORMAT
    } else {
        template
    };
    let now = chrono::Local::now();
    let mut out = String::with_capacity(template.len() + 16);
    let mut rest = template;

    while let Some(start) = rest.find('$') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        match after.find('$') {
            Some(end) => {
                let pattern = strftime_pattern(&after[..end]);
                out.push_str(&now.format(&pattern).to_string());
                rest = &after[end + 1..];
            }
            None => {
                out.push_str(&rest[start..]);
                rest = "";
                break;
            }
        }
    }
    out.push_str(rest);
    out
}

fn strftime_pattern(pattern: &str) -> String {
    pattern
        .replace("SSS", "%3f")
        .replace("yyyy", "%Y")
        .replace("yy", "%y")
        .replace("MM", "%m")
        .replace("dd", "%d")
        .replace("HH", "%H")
        .replace("mm", "%M")
        .replace("ss", "%S")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ini_roundtrip_keeps_values() {
        let config = Config {
            autostart: true,
            auto_backup: true,
            log_level: LogLevel::Debug,
            language: Language::En,
            tray_icon: TrayIconSetting::Custom(PathBuf::from("/tmp/icon.png")),
            filename_format: "shot_$yyyy$.png".to_owned(),
            ..Default::default()
        };

        let parsed = Config::from_ini(&config.to_ini());
        assert!(parsed.autostart);
        assert!(parsed.auto_backup);
        assert_eq!(parsed.log_level, LogLevel::Debug);
        assert_eq!(parsed.language, Language::En);
        assert_eq!(
            parsed.tray_icon,
            TrayIconSetting::Custom(PathBuf::from("/tmp/icon.png"))
        );
        assert_eq!(parsed.filename_format, "shot_$yyyy$.png");
    }

    #[test]
    fn default_filename_matches_expected_shape() {
        let name = format_filename(DEFAULT_FILENAME_FORMAT);
        assert!(name.starts_with("screenscan_"));
        assert!(name.ends_with(".png"));
        assert_eq!(name.len(), "screenscan_2026-10-23_20-56-50.png".len());
    }

    #[test]
    fn empty_template_falls_back_to_default() {
        let name = format_filename("");
        assert!(name.starts_with("screenscan_"));
    }

    #[test]
    fn parse_log_level_aliases() {
        assert_eq!(LogLevel::parse("off"), LogLevel::Off);
        assert_eq!(LogLevel::parse("错误"), LogLevel::Error);
        assert_eq!(LogLevel::parse("unknown"), LogLevel::Info);
    }

    #[test]
    fn shortcut_roundtrip_and_display() {
        let shortcut = Shortcut::parse("Ctrl+Shift+A").expect("valid shortcut");
        assert_eq!(shortcut.key, Key::A);
        assert_eq!(shortcut.to_ini(), "Ctrl+Shift+A");
        assert_eq!(shortcut.display(), "Ctrl+Shift+A");

        let up = Shortcut::parse("Up").expect("valid arrow key");
        assert_eq!(up.key, Key::ArrowUp);
        assert_eq!(up.display(), "↑");
        assert!(Shortcut::parse("NotAKey").is_none());
    }

    #[test]
    fn shortcut_matches_only_with_exact_modifiers() {
        let key_event = |modifiers| egui::Event::Key {
            key: Key::C,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        };
        let copy = Shortcuts::default().copy;
        assert!(copy.matches_event(&key_event(Modifiers::CTRL.plus(Modifiers::COMMAND))));
        assert!(!copy.matches_event(&key_event(Modifiers::NONE)));
        assert!(!copy.matches_event(&key_event(Modifiers::CTRL.plus(Modifiers::SHIFT))));
    }

    #[test]
    fn ini_roundtrip_keeps_custom_shortcut() {
        let mut config = Config::default();
        config.shortcuts.pin = Shortcut::new(Modifiers::ALT, Key::P);
        let parsed = Config::from_ini(&config.to_ini());
        assert_eq!(parsed.shortcuts.pin, config.shortcuts.pin);
        assert_eq!(parsed.shortcuts.copy.key, Key::C);
    }
}
