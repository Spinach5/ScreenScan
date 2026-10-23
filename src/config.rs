//! `config.ini` 的读写、迁移与设置项定义。

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;

use crate::font::FontChoice;
use crate::i18n::Language;
use crate::i18n::t;
use crate::util::path;

pub const DEFAULT_FILENAME_FORMAT: &str = "screenscan_$yyyy-MM-dd_HH-mm-ss$.png";

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

        Self {
            autostart: parse_bool(get("general", "autostart"), false),
            auto_backup: parse_bool(get("general", "auto_backup"), false),
            font,
            tray_icon,
            log_level: LogLevel::parse(get("output", "log_level").unwrap_or("info")),
            filename_format,
            language: Language::parse(get("appearance", "language").unwrap_or("zh-CN")),
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
}
