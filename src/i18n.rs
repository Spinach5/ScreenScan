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

        (Language::ZhCn, "pin.size") => "尺寸",
        (Language::En, "pin.size") => "Size",
        (Language::ZhCn, "pin.save") => "保存…",
        (Language::En, "pin.save") => "Save…",
        (Language::ZhCn, "pin.destroy") => "销毁",
        (Language::En, "pin.destroy") => "Destroy",
        (Language::ZhCn, "pin.choose_dir") => "选择贴图保存目录",
        (Language::En, "pin.choose_dir") => "Choose a folder for the pinned image",
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
        (Language::ZhCn, "tab.control") => "控制",
        (Language::En, "tab.control") => "Control",
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

        (Language::ZhCn, "shortcuts.move_up") => "向上移动选区",
        (Language::En, "shortcuts.move_up") => "Move selection up",
        (Language::ZhCn, "shortcuts.move_down") => "向下移动选区",
        (Language::En, "shortcuts.move_down") => "Move selection down",
        (Language::ZhCn, "shortcuts.move_left") => "向左移动选区",
        (Language::En, "shortcuts.move_left") => "Move selection left",
        (Language::ZhCn, "shortcuts.move_right") => "向右移动选区",
        (Language::En, "shortcuts.move_right") => "Move selection right",
        (Language::ZhCn, "shortcuts.copy") => "复制选区",
        (Language::En, "shortcuts.copy") => "Copy selection",
        (Language::ZhCn, "shortcuts.save") => "保存选区",
        (Language::En, "shortcuts.save") => "Save selection",
        (Language::ZhCn, "shortcuts.select_all") => "选中全屏",
        (Language::En, "shortcuts.select_all") => "Select entire screen",
        (Language::ZhCn, "shortcuts.pin") => "贴图",
        (Language::En, "shortcuts.pin") => "Pin selection",
        (Language::ZhCn, "shortcuts.undo") => "撤销标注",
        (Language::En, "shortcuts.undo") => "Undo annotation",
        (Language::ZhCn, "shortcuts.redo") => "重做标注",
        (Language::En, "shortcuts.redo") => "Redo annotation",
        (Language::ZhCn, "shortcuts.scan") => "识别条码",
        (Language::En, "shortcuts.scan") => "Scan barcode",
        (Language::ZhCn, "shortcuts.recording") => "请按下按键…",
        (Language::En, "shortcuts.recording") => "Press keys…",
        (Language::ZhCn, "shortcuts.unset") => "未设置",
        (Language::En, "shortcuts.unset") => "Not set",

        (Language::ZhCn, "control.global") => "全局快捷键",
        (Language::En, "control.global") => "Global shortcuts",
        (Language::ZhCn, "control.app") => "应用内快捷键",
        (Language::En, "control.app") => "In-app shortcuts",
        (Language::ZhCn, "control.internal") => "内部快捷键",
        (Language::En, "control.internal") => "Internal shortcuts",
        (Language::ZhCn, "control.mouse") => "鼠标",
        (Language::En, "control.mouse") => "Mouse",
        (Language::ZhCn, "control.global_hint") => {
            "常驻托盘时生效。Wayland 会话不支持全局快捷键，请改用系统快捷键绑定 screenscan -c"
        }
        (Language::En, "control.global_hint") => {
            "Active while running in the tray. Wayland does not support global hotkeys; bind `screenscan -c` in your desktop settings instead."
        }
        (Language::ZhCn, "control.app_hint") => {
            "在截屏选区界面内生效。点击绑定后按下新组合，再点确认按钮生效；× 清除绑定"
        }
        (Language::En, "control.app_hint") => {
            "Active in the capture overlay. Click a binding, press a new combination, then press the check button; × clears it."
        }
        (Language::ZhCn, "control.mouse_hint") => {
            "作用于贴图窗口。修改后点确认按钮生效；如有冲突，由上至下优先匹配；× 可清除绑定"
        }
        (Language::En, "control.mouse_hint") => {
            "Applies to pinned images. Press the check button to apply; conflicts resolve top to bottom; × clears a binding."
        }
        (Language::ZhCn, "control.restore") => "恢复默认",
        (Language::En, "control.restore") => "Restore defaults",
        (Language::ZhCn, "control.delay") => "自定义截屏延迟",
        (Language::En, "control.delay") => "Custom capture delay",
        (Language::ZhCn, "control.seconds") => "秒",
        (Language::En, "control.seconds") => "s",

        (Language::ZhCn, "global.capture") => "截屏",
        (Language::En, "global.capture") => "Capture",
        (Language::ZhCn, "global.capture_copy") => "截屏并自动复制",
        (Language::En, "global.capture_copy") => "Capture and copy",
        (Language::ZhCn, "global.custom_capture") => "自定义截屏（延迟）",
        (Language::En, "global.custom_capture") => "Custom capture (delayed)",
        (Language::ZhCn, "global.pin") => "贴图（剪贴板图片）",
        (Language::En, "global.pin") => "Pin clipboard image",
        (Language::ZhCn, "global.toggle_pins") => "隐藏/显示所有贴图",
        (Language::En, "global.toggle_pins") => "Hide/show all pins",
        (Language::ZhCn, "global.next_pin_group") => "切换到另一贴图组",
        (Language::En, "global.next_pin_group") => "Switch pin group",
        (Language::ZhCn, "global.toggle_click_through") => "开/关鼠标穿透",
        (Language::En, "global.toggle_click_through") => "Toggle click-through",
        (Language::ZhCn, "global.quit") => "退出程序",
        (Language::En, "global.quit") => "Quit",

        (Language::ZhCn, "mouse.zoom_in") => "放大贴图",
        (Language::En, "mouse.zoom_in") => "Zoom in",
        (Language::ZhCn, "mouse.zoom_out") => "缩小贴图",
        (Language::En, "mouse.zoom_out") => "Zoom out",
        (Language::ZhCn, "mouse.move") => "移动贴图",
        (Language::En, "mouse.move") => "Move pin",
        (Language::ZhCn, "mouse.copy") => "复制贴图",
        (Language::En, "mouse.copy") => "Copy pin",
        (Language::ZhCn, "mouse.save") => "保存贴图",
        (Language::En, "mouse.save") => "Save pin",
        (Language::ZhCn, "mouse.close") => "关闭贴图",
        (Language::En, "mouse.close") => "Close pin",
        (Language::ZhCn, "mouse.reset_zoom") => "重置缩放",
        (Language::En, "mouse.reset_zoom") => "Reset zoom",
        (Language::ZhCn, "mouse.toggle_click_through") => "开/关鼠标穿透",
        (Language::En, "mouse.toggle_click_through") => "Toggle click-through",
        (Language::ZhCn, "mouse.input_left") => "左键",
        (Language::En, "mouse.input_left") => "Left",
        (Language::ZhCn, "mouse.input_middle") => "中键",
        (Language::En, "mouse.input_middle") => "Middle",
        (Language::ZhCn, "mouse.input_right") => "右键",
        (Language::En, "mouse.input_right") => "Right",
        (Language::ZhCn, "mouse.input_wheel_up") => "滚轮上",
        (Language::En, "mouse.input_wheel_up") => "Wheel up",
        (Language::ZhCn, "mouse.input_wheel_down") => "滚轮下",
        (Language::En, "mouse.input_wheel_down") => "Wheel down",
        (Language::ZhCn, "mouse.gesture_click") => "单击",
        (Language::En, "mouse.gesture_click") => "Click",
        (Language::ZhCn, "mouse.gesture_double_click") => "双击",
        (Language::En, "mouse.gesture_double_click") => "Double-click",
        (Language::ZhCn, "mouse.gesture_drag") => "拖拽",
        (Language::En, "mouse.gesture_drag") => "Drag",
        (Language::ZhCn, "mouse.gesture_scroll") => "滚动",
        (Language::En, "mouse.gesture_scroll") => "Scroll",
        (Language::ZhCn, "mouse.modifier_none") => "无",
        (Language::En, "mouse.modifier_none") => "None",

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
        (Language::ZhCn, "common.cancel") => "取消",
        (Language::En, "common.cancel") => "Cancel",
        (Language::ZhCn, "common.clear") => "清除",
        (Language::En, "common.clear") => "Clear",
        (Language::ZhCn, "common.confirm") => "确认",
        (Language::En, "common.confirm") => "Confirm",
        (Language::ZhCn, "common.apply_immediately") => {
            "快捷键与鼠标操作确认后生效，其余修改即时保存"
        }
        (Language::En, "common.apply_immediately") => {
            "Shortcuts and mouse bindings apply after confirmation; other changes save immediately"
        }

        (Language::ZhCn, "permission.title") => "允许 ScreenScan 截屏",
        (Language::En, "permission.title") => "Allow ScreenScan to Capture",
        (Language::ZhCn, "permission.body") => {
            "系统需要你授权 ScreenScan 截取屏幕。完成一次授权后，即可直接从托盘截屏，无需再次确认。"
        }
        (Language::En, "permission.body") => {
            "Your system needs permission for ScreenScan to capture the screen. \
             After granting it once, you can capture directly from the tray."
        }
        (Language::ZhCn, "permission.hint") => "该权限可随时在系统设置的隐私页面中更改。",
        (Language::En, "permission.hint") => {
            "You can change this permission at any time in your system's privacy settings."
        }
        (Language::ZhCn, "permission.grant") => "授权并截屏",
        (Language::En, "permission.grant") => "Allow and Capture",
        (Language::ZhCn, "permission.pending") => "正在等待系统授权…",
        (Language::En, "permission.pending") => "Waiting for system permission…",
        (Language::ZhCn, "permission.failed") => "未获得截屏权限，请重试",
        (Language::En, "permission.failed") => {
            "Screenshot permission was not granted. Please try again."
        }

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

        _ => "",
    }
}
