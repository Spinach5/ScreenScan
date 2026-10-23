//! 界面语言的简单字典（托盘菜单与设置窗口）。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    ZhCn,
    En,
}

impl Language {
    pub const ALL: [Self; 2] = [Self::ZhCn, Self::En];

    pub fn code(self) -> &'static str {
        match self {
            Self::ZhCn => "zh-CN",
            Self::En => "en-US",
        }
    }

    pub fn native_name(self) -> &'static str {
        match self {
            Self::ZhCn => "简体中文",
            Self::En => "English",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "en" | "en-us" | "en_us" | "english" => Self::En,
            _ => Self::ZhCn,
        }
    }
}

pub fn t(lang: Language, key: &str) -> &'static str {
    match (lang, key) {
        (Language::ZhCn, "tray.capture") => "截屏",
        (Language::En, "tray.capture") => "Capture",
        (Language::ZhCn, "tray.pin") => "贴图",
        (Language::En, "tray.pin") => "Pin",
        (Language::ZhCn, "tray.clear") => "清空截图记录",
        (Language::En, "tray.clear") => "Clear Screenshot History",
        (Language::ZhCn, "tray.preferences") => "首选项",
        (Language::En, "tray.preferences") => "Preferences",
        (Language::ZhCn, "tray.quit") => "退出",
        (Language::En, "tray.quit") => "Quit",

        (Language::ZhCn, "settings.title") => "ScreenScan 首选项",
        (Language::En, "settings.title") => "ScreenScan Preferences",
        (Language::ZhCn, "tab.general") => "常规",
        (Language::En, "tab.general") => "General",
        (Language::ZhCn, "tab.interface") => "界面",
        (Language::En, "tab.interface") => "Interface",
        (Language::ZhCn, "tab.output") => "输出",
        (Language::En, "tab.output") => "Output",
        (Language::ZhCn, "tab.language") => "语言",
        (Language::En, "tab.language") => "Language",
        (Language::ZhCn, "tab.update") => "更新",
        (Language::En, "tab.update") => "Updates",
        (Language::ZhCn, "tab.about") => "关于",
        (Language::En, "tab.about") => "About",

        (Language::ZhCn, "general.autostart") => "开机启动",
        (Language::En, "general.autostart") => "Launch at login",
        (Language::ZhCn, "general.autostart_hint") => "登录系统后自动在托盘驻留",
        (Language::En, "general.autostart_hint") => "Start in the system tray after you sign in",
        (Language::ZhCn, "general.backup") => "自动备份",
        (Language::En, "general.backup") => "Automatic backup",
        (Language::ZhCn, "general.backup_hint") => "保存设置前将上一版配置备份为 config.ini.bak",
        (Language::En, "general.backup_hint") => {
            "Back up the previous config to config.ini.bak before saving"
        }
        (Language::ZhCn, "general.config_path") => "配置文件存储位置",
        (Language::En, "general.config_path") => "Config file location",
        (Language::ZhCn, "general.browse") => "浏览…",
        (Language::En, "general.browse") => "Browse…",
        (Language::ZhCn, "general.migrate_hint") => {
            "更改位置时会把当前配置迁移到新目录下的 config.ini"
        }
        (Language::En, "general.migrate_hint") => {
            "Changing the location migrates the current config to config.ini in the new folder"
        }

        (Language::ZhCn, "interface.font") => "界面字体",
        (Language::En, "interface.font") => "Interface font",
        (Language::ZhCn, "interface.font_default") => "默认字体",
        (Language::En, "interface.font_default") => "Default font",
        (Language::ZhCn, "interface.font_hint") => "点击选择系统中已安装的字体，立即生效",
        (Language::En, "interface.font_hint") => {
            "Click to pick an installed system font. Changes apply immediately."
        }
        (Language::ZhCn, "interface.font_picker") => "选择界面字体",
        (Language::En, "interface.font_picker") => "Choose Interface Font",
        (Language::ZhCn, "interface.font_search") => "搜索字体…",
        (Language::En, "interface.font_search") => "Search fonts…",
        (Language::ZhCn, "interface.font_scanning") => "正在扫描系统字体…",
        (Language::En, "interface.font_scanning") => "Scanning system fonts…",
        (Language::ZhCn, "interface.font_empty") => "未在系统中找到可用字体",
        (Language::En, "interface.font_empty") => "No usable system fonts found",
        (Language::ZhCn, "interface.tray_icon") => "托盘图标",
        (Language::En, "interface.tray_icon") => "Tray icon",
        (Language::ZhCn, "interface.tray_default") => "使用默认图标",
        (Language::En, "interface.tray_default") => "Use default icon",
        (Language::ZhCn, "interface.tray_custom") => "自定义图标",
        (Language::En, "interface.tray_custom") => "Custom icon",
        (Language::ZhCn, "interface.tray_choose") => "选择图片…",
        (Language::En, "interface.tray_choose") => "Choose image…",
        (Language::ZhCn, "interface.tray_replace") => "更换图片…",
        (Language::En, "interface.tray_replace") => "Replace image…",
        (Language::ZhCn, "interface.tray_hint") => {
            "支持 PNG / JPG / WebP / BMP / GIF，建议使用方形图片"
        }
        (Language::En, "interface.tray_hint") => {
            "PNG / JPG / WebP / BMP / GIF. A square image works best."
        }
        (Language::ZhCn, "interface.tray_preview_failed") => "无法预览该图片",
        (Language::En, "interface.tray_preview_failed") => "Unable to preview this image",

        (Language::ZhCn, "output.log_level") => "日志级别",
        (Language::En, "output.log_level") => "Log level",
        (Language::ZhCn, "output.log_off") => "无",
        (Language::En, "output.log_off") => "Off",
        (Language::ZhCn, "output.log_debug") => "调试",
        (Language::En, "output.log_debug") => "Debug",
        (Language::ZhCn, "output.log_info") => "普通",
        (Language::En, "output.log_info") => "Normal",
        (Language::ZhCn, "output.log_error") => "错误",
        (Language::En, "output.log_error") => "Errors only",
        (Language::ZhCn, "output.log_level_hint") => "日志级别同时作用于终端与日志文件",
        (Language::En, "output.log_level_hint") => {
            "The log level applies to both the terminal and log files"
        }
        (Language::ZhCn, "output.log_file") => "日志文件",
        (Language::En, "output.log_file") => "Log files",
        (Language::ZhCn, "output.open_log_folder") => "打开日志目录",
        (Language::En, "output.open_log_folder") => "Open log folder",
        (Language::ZhCn, "output.log_hint") => "按天写入，自动保留最近 14 天",
        (Language::En, "output.log_hint") => {
            "Written daily; the last 14 days are retained automatically"
        }
        (Language::ZhCn, "output.filename") => "保存的文件名",
        (Language::En, "output.filename") => "Saved file name",
        (Language::ZhCn, "output.filename_hint") => {
            "用 $…$ 包裹时间格式，例如 $yyyy-MM-dd_HH-mm-ss$"
        }
        (Language::En, "output.filename_hint") => {
            "Wrap time tokens in $…$, e.g. $yyyy-MM-dd_HH-mm-ss$"
        }
        (Language::ZhCn, "output.preview") => "预览",
        (Language::En, "output.preview") => "Preview",
        (Language::ZhCn, "output.restore") => "恢复默认",
        (Language::En, "output.restore") => "Restore default",

        (Language::ZhCn, "language.current") => "当前界面语言",
        (Language::En, "language.current") => "Current language",
        (Language::ZhCn, "language.hint") => "点击选择语言，切换后立即生效",
        (Language::En, "language.hint") => "Click to choose a language. Changes apply immediately.",
        (Language::ZhCn, "language.picker") => "选择语言",
        (Language::En, "language.picker") => "Choose Language",

        (Language::ZhCn, "update.placeholder") => "功能开发中",
        (Language::En, "update.placeholder") => "Coming soon",
        (Language::ZhCn, "update.detail") => "后续版本将支持检查更新、显示更新日志与一键升级。",
        (Language::En, "update.detail") => {
            "Future releases will support update checks, release notes and one-click upgrades."
        }
        (Language::ZhCn, "about.version") => "版本",
        (Language::En, "about.version") => "Version",
        (Language::ZhCn, "about.copyright") => "© 2026 ScreenScan · 版权所有",
        (Language::En, "about.copyright") => "© 2026 ScreenScan. All rights reserved.",
        (Language::ZhCn, "about.placeholder") => "关于页面占位内容",
        (Language::En, "about.placeholder") => "About page placeholder",

        (Language::ZhCn, "common.close") => "关闭",
        (Language::En, "common.close") => "Close",
        (Language::ZhCn, "common.apply_immediately") => "所有修改即时生效",
        (Language::En, "common.apply_immediately") => "All changes apply immediately",

        (Language::ZhCn, "status.saved") => "设置已保存",
        (Language::En, "status.saved") => "Settings saved",
        (Language::ZhCn, "status.migrated") => "配置已迁移到",
        (Language::En, "status.migrated") => "Config migrated to",
        (Language::ZhCn, "status.font_updated") => "界面字体已更新",
        (Language::En, "status.font_updated") => "Interface font updated",
        (Language::ZhCn, "status.font_failed") => "字体加载失败，已保留原字体",
        (Language::En, "status.font_failed") => "Failed to load the font; kept the previous one",
        (Language::ZhCn, "status.icon_updated") => "托盘图标已更新",
        (Language::En, "status.icon_updated") => "Tray icon updated",
        (Language::ZhCn, "status.icon_failed") => "托盘图标更新失败",
        (Language::En, "status.icon_failed") => "Failed to update the tray icon",
        (Language::ZhCn, "status.autostart_failed") => "开机启动设置失败",
        (Language::En, "status.autostart_failed") => "Failed to update launch-at-login",
        (Language::ZhCn, "status.save_failed") => "设置保存失败",
        (Language::En, "status.save_failed") => "Failed to save settings",
        (Language::ZhCn, "status.open_log_failed") => "无法打开日志目录",
        (Language::En, "status.open_log_failed") => "Failed to open the log folder",
        (Language::ZhCn, "status.history_cleared") => "截图记录已清空",
        (Language::En, "status.history_cleared") => "Screenshot history cleared",
        (Language::ZhCn, "status.capture_placeholder") => "截图流程入口（预留）",
        (Language::En, "status.capture_placeholder") => "Capture entry point (reserved)",
        (Language::ZhCn, "status.pin_placeholder") => "贴图流程入口（预留）",
        (Language::En, "status.pin_placeholder") => "Pin entry point (reserved)",

        _ => "",
    }
}
