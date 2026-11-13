//! `config.ini` 的读写、迁移与设置项定义。

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use egui::Key;
use egui::Modifiers;
use global_hotkey::hotkey::Code;
use global_hotkey::hotkey::HotKey;
use global_hotkey::hotkey::Modifiers as GlobalModifiers;

use crate::font::FontChoice;
use crate::i18n::Language;
use crate::i18n::t;
use crate::util::path;

pub const DEFAULT_FILENAME_FORMAT: &str = "screenscan_$yyyy-MM-dd_HH-mm-ss$.png";

/// 可配置的键盘快捷键；`enabled == false` 表示用户清除了绑定。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shortcut {
    pub key: Key,
    pub modifiers: Modifiers,
    pub enabled: bool,
}

impl Shortcut {
    pub const fn new(modifiers: Modifiers, key: Key) -> Self {
        Self {
            key,
            modifiers,
            enabled: true,
        }
    }

    pub const fn disabled() -> Self {
        Self {
            key: Key::Escape,
            modifiers: Modifiers::NONE,
            enabled: false,
        }
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
            enabled: true,
        })
    }

    pub fn to_ini(self) -> String {
        if !self.enabled {
            return String::new();
        }
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
        if !self.enabled {
            return String::new();
        }
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
        if !self.enabled {
            return false;
        }
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

    /// 转换为系统级快捷键；不支持的按键返回 `None`。
    pub fn to_global(self) -> Option<HotKey> {
        if !self.enabled {
            return None;
        }
        let mut modifiers = GlobalModifiers::empty();
        if self.modifiers.mac_cmd {
            modifiers |= GlobalModifiers::META;
        } else if self.modifiers.ctrl || self.modifiers.command {
            modifiers |= GlobalModifiers::CONTROL;
        }
        if self.modifiers.shift {
            modifiers |= GlobalModifiers::SHIFT;
        }
        if self.modifiers.alt {
            modifiers |= GlobalModifiers::ALT;
        }
        let code = global_code(self.key)?;
        Some(HotKey::new(
            (!modifiers.is_empty()).then_some(modifiers),
            code,
        ))
    }
}

fn global_code(key: Key) -> Option<Code> {
    Some(match key {
        Key::A => Code::KeyA,
        Key::B => Code::KeyB,
        Key::C => Code::KeyC,
        Key::D => Code::KeyD,
        Key::E => Code::KeyE,
        Key::F => Code::KeyF,
        Key::G => Code::KeyG,
        Key::H => Code::KeyH,
        Key::I => Code::KeyI,
        Key::J => Code::KeyJ,
        Key::K => Code::KeyK,
        Key::L => Code::KeyL,
        Key::M => Code::KeyM,
        Key::N => Code::KeyN,
        Key::O => Code::KeyO,
        Key::P => Code::KeyP,
        Key::Q => Code::KeyQ,
        Key::R => Code::KeyR,
        Key::S => Code::KeyS,
        Key::T => Code::KeyT,
        Key::U => Code::KeyU,
        Key::V => Code::KeyV,
        Key::W => Code::KeyW,
        Key::X => Code::KeyX,
        Key::Y => Code::KeyY,
        Key::Z => Code::KeyZ,
        Key::Num0 => Code::Digit0,
        Key::Num1 => Code::Digit1,
        Key::Num2 => Code::Digit2,
        Key::Num3 => Code::Digit3,
        Key::Num4 => Code::Digit4,
        Key::Num5 => Code::Digit5,
        Key::Num6 => Code::Digit6,
        Key::Num7 => Code::Digit7,
        Key::Num8 => Code::Digit8,
        Key::Num9 => Code::Digit9,
        Key::F1 => Code::F1,
        Key::F2 => Code::F2,
        Key::F3 => Code::F3,
        Key::F4 => Code::F4,
        Key::F5 => Code::F5,
        Key::F6 => Code::F6,
        Key::F7 => Code::F7,
        Key::F8 => Code::F8,
        Key::F9 => Code::F9,
        Key::F10 => Code::F10,
        Key::F11 => Code::F11,
        Key::F12 => Code::F12,
        Key::F13 => Code::F13,
        Key::F14 => Code::F14,
        Key::F15 => Code::F15,
        Key::F16 => Code::F16,
        Key::F17 => Code::F17,
        Key::F18 => Code::F18,
        Key::F19 => Code::F19,
        Key::F20 => Code::F20,
        Key::F21 => Code::F21,
        Key::F22 => Code::F22,
        Key::F23 => Code::F23,
        Key::F24 => Code::F24,
        Key::ArrowUp => Code::ArrowUp,
        Key::ArrowDown => Code::ArrowDown,
        Key::ArrowLeft => Code::ArrowLeft,
        Key::ArrowRight => Code::ArrowRight,
        Key::Space => Code::Space,
        Key::Enter => Code::Enter,
        Key::Tab => Code::Tab,
        Key::Escape => Code::Escape,
        Key::Backspace => Code::Backspace,
        Key::Insert => Code::Insert,
        Key::Delete => Code::Delete,
        Key::Home => Code::Home,
        Key::End => Code::End,
        Key::PageUp => Code::PageUp,
        Key::PageDown => Code::PageDown,
        Key::Minus => Code::Minus,
        Key::Equals => Code::Equal,
        Key::Comma => Code::Comma,
        Key::Period => Code::Period,
        Key::Slash => Code::Slash,
        Key::Semicolon => Code::Semicolon,
        Key::Quote => Code::Quote,
        Key::Backslash => Code::Backslash,
        Key::Backtick => Code::Backquote,
        Key::OpenBracket => Code::BracketLeft,
        Key::CloseBracket => Code::BracketRight,
        _ => return None,
    })
}

/// 覆盖层内可重绑定的动作。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShortcutAction {
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    Copy,
    Save,
    SelectAll,
    Pin,
    Undo,
    Redo,
    Scan,
}

impl ShortcutAction {
    pub const ALL: [Self; 11] = [
        Self::MoveUp,
        Self::MoveDown,
        Self::MoveLeft,
        Self::MoveRight,
        Self::Copy,
        Self::Save,
        Self::SelectAll,
        Self::Pin,
        Self::Undo,
        Self::Redo,
        Self::Scan,
    ];

    pub fn config_key(self) -> &'static str {
        match self {
            Self::MoveUp => "move_up",
            Self::MoveDown => "move_down",
            Self::MoveLeft => "move_left",
            Self::MoveRight => "move_right",
            Self::Copy => "copy",
            Self::Save => "save",
            Self::SelectAll => "select_all",
            Self::Pin => "pin",
            Self::Undo => "undo",
            Self::Redo => "redo",
            Self::Scan => "scan",
        }
    }

    pub fn default_shortcut(self) -> Shortcut {
        match self {
            Self::MoveUp => Shortcut::new(Modifiers::NONE, Key::ArrowUp),
            Self::MoveDown => Shortcut::new(Modifiers::NONE, Key::ArrowDown),
            Self::MoveLeft => Shortcut::new(Modifiers::NONE, Key::ArrowLeft),
            Self::MoveRight => Shortcut::new(Modifiers::NONE, Key::ArrowRight),
            Self::Copy => Shortcut::new(Modifiers::CTRL, Key::C),
            Self::Save => Shortcut::new(Modifiers::CTRL, Key::S),
            Self::SelectAll => Shortcut::new(Modifiers::CTRL, Key::A),
            Self::Pin => Shortcut::new(Modifiers::CTRL, Key::P),
            Self::Undo => Shortcut::new(Modifiers::CTRL, Key::Z),
            Self::Redo => Shortcut::new(Modifiers::CTRL, Key::Y),
            Self::Scan => Shortcut::new(Modifiers::CTRL, Key::D),
        }
    }

    pub fn label(self, lang: Language) -> &'static str {
        match self {
            Self::MoveUp => t(lang, "shortcuts.move_up"),
            Self::MoveDown => t(lang, "shortcuts.move_down"),
            Self::MoveLeft => t(lang, "shortcuts.move_left"),
            Self::MoveRight => t(lang, "shortcuts.move_right"),
            Self::Copy => t(lang, "shortcuts.copy"),
            Self::Save => t(lang, "shortcuts.save"),
            Self::SelectAll => t(lang, "shortcuts.select_all"),
            Self::Pin => t(lang, "shortcuts.pin"),
            Self::Undo => t(lang, "shortcuts.undo"),
            Self::Redo => t(lang, "shortcuts.redo"),
            Self::Scan => t(lang, "shortcuts.scan"),
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
    pub save: Shortcut,
    pub select_all: Shortcut,
    pub pin: Shortcut,
    pub undo: Shortcut,
    pub redo: Shortcut,
    pub scan: Shortcut,
}

impl Default for Shortcuts {
    fn default() -> Self {
        Self {
            move_up: ShortcutAction::MoveUp.default_shortcut(),
            move_down: ShortcutAction::MoveDown.default_shortcut(),
            move_left: ShortcutAction::MoveLeft.default_shortcut(),
            move_right: ShortcutAction::MoveRight.default_shortcut(),
            copy: ShortcutAction::Copy.default_shortcut(),
            save: ShortcutAction::Save.default_shortcut(),
            select_all: ShortcutAction::SelectAll.default_shortcut(),
            pin: ShortcutAction::Pin.default_shortcut(),
            undo: ShortcutAction::Undo.default_shortcut(),
            redo: ShortcutAction::Redo.default_shortcut(),
            scan: ShortcutAction::Scan.default_shortcut(),
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
            ShortcutAction::Save => self.save,
            ShortcutAction::SelectAll => self.select_all,
            ShortcutAction::Pin => self.pin,
            ShortcutAction::Undo => self.undo,
            ShortcutAction::Redo => self.redo,
            ShortcutAction::Scan => self.scan,
        }
    }

    pub fn set(&mut self, action: ShortcutAction, shortcut: Shortcut) {
        match action {
            ShortcutAction::MoveUp => self.move_up = shortcut,
            ShortcutAction::MoveDown => self.move_down = shortcut,
            ShortcutAction::MoveLeft => self.move_left = shortcut,
            ShortcutAction::MoveRight => self.move_right = shortcut,
            ShortcutAction::Copy => self.copy = shortcut,
            ShortcutAction::Save => self.save = shortcut,
            ShortcutAction::SelectAll => self.select_all = shortcut,
            ShortcutAction::Pin => self.pin = shortcut,
            ShortcutAction::Undo => self.undo = shortcut,
            ShortcutAction::Redo => self.redo = shortcut,
            ShortcutAction::Scan => self.scan = shortcut,
        }
    }
}

/// 常驻模式下由系统注册的全局快捷键。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlobalAction {
    Capture,
    CaptureCopy,
    CustomCapture,
    Pin,
    TogglePins,
    NextPinGroup,
    ToggleClickThrough,
    Quit,
}

impl GlobalAction {
    pub const ALL: [Self; 8] = [
        Self::Capture,
        Self::CaptureCopy,
        Self::CustomCapture,
        Self::Pin,
        Self::TogglePins,
        Self::NextPinGroup,
        Self::ToggleClickThrough,
        Self::Quit,
    ];

    pub fn config_key(self) -> &'static str {
        match self {
            Self::Capture => "capture",
            Self::CaptureCopy => "capture_copy",
            Self::CustomCapture => "custom_capture",
            Self::Pin => "pin",
            Self::TogglePins => "toggle_pins",
            Self::NextPinGroup => "next_pin_group",
            Self::ToggleClickThrough => "toggle_click_through",
            Self::Quit => "quit",
        }
    }

    pub fn default_shortcut(self) -> Shortcut {
        match self {
            Self::Capture => Shortcut::new(Modifiers::CTRL.plus(Modifiers::SHIFT), Key::S),
            Self::CaptureCopy => Shortcut::new(Modifiers::NONE, Key::F3),
            Self::CustomCapture => Shortcut::new(Modifiers::NONE, Key::F4),
            Self::Pin => Shortcut::new(Modifiers::NONE, Key::F2),
            Self::TogglePins => Shortcut::new(Modifiers::NONE, Key::F6),
            Self::NextPinGroup => Shortcut::new(Modifiers::NONE, Key::F7),
            Self::ToggleClickThrough => Shortcut::new(Modifiers::NONE, Key::F8),
            Self::Quit => Shortcut::new(Modifiers::CTRL.plus(Modifiers::SHIFT), Key::Q),
        }
    }

    pub fn label(self, lang: Language) -> &'static str {
        match self {
            Self::Capture => t(lang, "global.capture"),
            Self::CaptureCopy => t(lang, "global.capture_copy"),
            Self::CustomCapture => t(lang, "global.custom_capture"),
            Self::Pin => t(lang, "global.pin"),
            Self::TogglePins => t(lang, "global.toggle_pins"),
            Self::NextPinGroup => t(lang, "global.next_pin_group"),
            Self::ToggleClickThrough => t(lang, "global.toggle_click_through"),
            Self::Quit => t(lang, "global.quit"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlobalShortcuts {
    values: [Shortcut; GlobalAction::ALL.len()],
}

impl Default for GlobalShortcuts {
    fn default() -> Self {
        let mut values = [Shortcut::disabled(); GlobalAction::ALL.len()];
        for action in GlobalAction::ALL {
            values[action as usize] = action.default_shortcut();
        }
        Self { values }
    }
}

impl GlobalShortcuts {
    pub fn get(self, action: GlobalAction) -> Shortcut {
        self.values[action as usize]
    }

    pub fn set(&mut self, action: GlobalAction, shortcut: Shortcut) {
        self.values[action as usize] = shortcut;
    }
}

/// 贴图鼠标操作的输入来源。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseInput {
    Left,
    Middle,
    Right,
    WheelUp,
    WheelDown,
}

impl MouseInput {
    pub const ALL: [Self; 5] = [
        Self::Left,
        Self::Middle,
        Self::Right,
        Self::WheelUp,
        Self::WheelDown,
    ];

    pub fn is_wheel(self) -> bool {
        matches!(self, Self::WheelUp | Self::WheelDown)
    }

    pub fn ini_value(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Middle => "middle",
            Self::Right => "right",
            Self::WheelUp => "wheel_up",
            Self::WheelDown => "wheel_down",
        }
    }

    pub fn label(self, lang: Language) -> &'static str {
        match self {
            Self::Left => t(lang, "mouse.input_left"),
            Self::Middle => t(lang, "mouse.input_middle"),
            Self::Right => t(lang, "mouse.input_right"),
            Self::WheelUp => t(lang, "mouse.input_wheel_up"),
            Self::WheelDown => t(lang, "mouse.input_wheel_down"),
        }
    }
}

/// 贴图鼠标操作的触发方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseGesture {
    Click,
    DoubleClick,
    Drag,
    Scroll,
}

impl MouseGesture {
    pub fn ini_value(self) -> &'static str {
        match self {
            Self::Click => "click",
            Self::DoubleClick => "double_click",
            Self::Drag => "drag",
            Self::Scroll => "scroll",
        }
    }

    pub fn label(self, lang: Language) -> &'static str {
        match self {
            Self::Click => t(lang, "mouse.gesture_click"),
            Self::DoubleClick => t(lang, "mouse.gesture_double_click"),
            Self::Drag => t(lang, "mouse.gesture_drag"),
            Self::Scroll => t(lang, "mouse.gesture_scroll"),
        }
    }
}

/// 贴图窗口上的鼠标动作；顺序即冲突优先级。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinMouseAction {
    ZoomIn,
    ZoomOut,
    Move,
    Copy,
    Save,
    Close,
    ResetZoom,
    ToggleClickThrough,
}

impl PinMouseAction {
    pub const ALL: [Self; 8] = [
        Self::ZoomIn,
        Self::ZoomOut,
        Self::Move,
        Self::Copy,
        Self::Save,
        Self::Close,
        Self::ResetZoom,
        Self::ToggleClickThrough,
    ];

    pub fn config_key(self) -> &'static str {
        match self {
            Self::ZoomIn => "zoom_in",
            Self::ZoomOut => "zoom_out",
            Self::Move => "move",
            Self::Copy => "copy",
            Self::Save => "save",
            Self::Close => "close",
            Self::ResetZoom => "reset_zoom",
            Self::ToggleClickThrough => "toggle_click_through",
        }
    }

    pub fn default_binding(self) -> MouseBinding {
        let binding = |modifiers, input, gesture| MouseBinding::new(modifiers, input, gesture);
        match self {
            Self::ZoomIn => binding(Modifiers::NONE, MouseInput::WheelUp, MouseGesture::Scroll),
            Self::ZoomOut => binding(Modifiers::NONE, MouseInput::WheelDown, MouseGesture::Scroll),
            Self::Move => binding(Modifiers::NONE, MouseInput::Middle, MouseGesture::Drag),
            Self::Copy => binding(Modifiers::CTRL, MouseInput::Left, MouseGesture::Click),
            Self::Save => binding(Modifiers::CTRL, MouseInput::Middle, MouseGesture::Click),
            Self::Close => binding(Modifiers::NONE, MouseInput::Left, MouseGesture::DoubleClick),
            Self::ResetZoom => binding(Modifiers::NONE, MouseInput::Middle, MouseGesture::Click),
            Self::ToggleClickThrough => binding(
                Modifiers::CTRL.plus(Modifiers::SHIFT),
                MouseInput::Middle,
                MouseGesture::Click,
            ),
        }
    }

    pub fn label(self, lang: Language) -> &'static str {
        match self {
            Self::ZoomIn => t(lang, "mouse.zoom_in"),
            Self::ZoomOut => t(lang, "mouse.zoom_out"),
            Self::Move => t(lang, "mouse.move"),
            Self::Copy => t(lang, "mouse.copy"),
            Self::Save => t(lang, "mouse.save"),
            Self::Close => t(lang, "mouse.close"),
            Self::ResetZoom => t(lang, "mouse.reset_zoom"),
            Self::ToggleClickThrough => t(lang, "mouse.toggle_click_through"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MouseBinding {
    pub modifiers: Modifiers,
    pub input: MouseInput,
    pub gesture: MouseGesture,
    pub enabled: bool,
}

impl MouseBinding {
    pub const fn new(modifiers: Modifiers, input: MouseInput, gesture: MouseGesture) -> Self {
        Self {
            modifiers,
            input,
            gesture,
            enabled: true,
        }
    }

    pub const fn disabled() -> Self {
        Self {
            modifiers: Modifiers::NONE,
            input: MouseInput::Left,
            gesture: MouseGesture::Click,
            enabled: false,
        }
    }

    /// 解析 `Ctrl+wheel_up+scroll` 形式；无法识别时返回 `None`。
    pub fn parse(text: &str) -> Option<Self> {
        let mut modifiers = Modifiers::NONE;
        let mut input = None;
        let mut gesture = None;
        for part in text
            .split('+')
            .map(str::trim)
            .filter(|part| !part.is_empty())
        {
            match part.to_ascii_lowercase().as_str() {
                "ctrl" | "control" => {
                    modifiers.ctrl = true;
                    modifiers.command = true;
                }
                "shift" => modifiers.shift = true,
                "alt" | "option" => modifiers.alt = true,
                "left" => input = Some(MouseInput::Left),
                "middle" => input = Some(MouseInput::Middle),
                "right" => input = Some(MouseInput::Right),
                "wheel_up" => input = Some(MouseInput::WheelUp),
                "wheel_down" => input = Some(MouseInput::WheelDown),
                "click" => gesture = Some(MouseGesture::Click),
                "double_click" | "double" => gesture = Some(MouseGesture::DoubleClick),
                "drag" => gesture = Some(MouseGesture::Drag),
                "scroll" => gesture = Some(MouseGesture::Scroll),
                _ => return None,
            }
        }
        Some(Self {
            modifiers,
            input: input?,
            gesture: gesture?,
            enabled: true,
        })
    }

    pub fn to_ini(self) -> String {
        if !self.enabled {
            return String::new();
        }
        let mut parts = Vec::with_capacity(4);
        if self.modifiers.ctrl || self.modifiers.command {
            parts.push("Ctrl");
        }
        if self.modifiers.shift {
            parts.push("Shift");
        }
        if self.modifiers.alt {
            parts.push("Alt");
        }
        parts.push(self.input.ini_value());
        parts.push(self.gesture.ini_value());
        parts.join("+")
    }

    pub fn matches_modifiers(self, modifiers: Modifiers) -> bool {
        self.enabled && self.modifiers.matches_exact(modifiers)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PinMouseBindings {
    values: [MouseBinding; PinMouseAction::ALL.len()],
}

impl Default for PinMouseBindings {
    fn default() -> Self {
        let mut values = [MouseBinding::disabled(); PinMouseAction::ALL.len()];
        for action in PinMouseAction::ALL {
            values[action as usize] = action.default_binding();
        }
        Self { values }
    }
}

impl PinMouseBindings {
    pub fn get(self, action: PinMouseAction) -> MouseBinding {
        self.values[action as usize]
    }

    pub fn set(&mut self, action: PinMouseAction, binding: MouseBinding) {
        self.values[action as usize] = binding;
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
    pub global_shortcuts: GlobalShortcuts,
    pub pin_mouse: PinMouseBindings,
    /// 自定义截屏的倒计时（秒）。
    pub capture_delay: u32,
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
            global_shortcuts: GlobalShortcuts::default(),
            pin_mouse: PinMouseBindings::default(),
            capture_delay: 3,
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

        let shortcut = |section: &str, key: &str, default: Shortcut| match get(section, key) {
            Some(value) if value.trim().is_empty() => Shortcut::disabled(),
            Some(value) => Shortcut::parse(value).unwrap_or(default),
            None => default,
        };
        let mut shortcuts = Shortcuts::default();
        for action in ShortcutAction::ALL {
            shortcuts.set(
                action,
                shortcut("shortcuts", action.config_key(), action.default_shortcut()),
            );
        }
        let mut global_shortcuts = GlobalShortcuts::default();
        for action in GlobalAction::ALL {
            global_shortcuts.set(
                action,
                shortcut(
                    "global_shortcuts",
                    action.config_key(),
                    action.default_shortcut(),
                ),
            );
        }
        let mut pin_mouse = PinMouseBindings::default();
        for action in PinMouseAction::ALL {
            if let Some(value) = get("pin_mouse", action.config_key()) {
                let binding = if value.trim().is_empty() {
                    MouseBinding::disabled()
                } else {
                    MouseBinding::parse(value).unwrap_or(action.default_binding())
                };
                pin_mouse.set(action, binding);
            }
        }

        Self {
            autostart: parse_bool(get("general", "autostart"), false),
            auto_backup: parse_bool(get("general", "auto_backup"), false),
            font,
            tray_icon,
            log_level: LogLevel::parse(get("output", "log_level").unwrap_or("info")),
            filename_format,
            language: Language::parse(get("appearance", "language").unwrap_or("zh-CN")),
            shortcuts,
            global_shortcuts,
            pin_mouse,
            capture_delay: get("general", "capture_delay")
                .and_then(|value| value.parse().ok())
                .unwrap_or(3)
                .min(60),
        }
    }

    fn to_ini(&self) -> String {
        let mut out = String::new();
        out.push_str("[general]\n");
        out.push_str(&format!("autostart={}\n", self.autostart));
        out.push_str(&format!("auto_backup={}\n", self.auto_backup));
        out.push_str(&format!("capture_delay={}\n", self.capture_delay));
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
        out.push_str("\n[global_shortcuts]\n");
        for action in GlobalAction::ALL {
            out.push_str(&format!(
                "{}={}\n",
                action.config_key(),
                self.global_shortcuts.get(action).to_ini()
            ));
        }
        out.push_str("\n[pin_mouse]\n");
        for action in PinMouseAction::ALL {
            out.push_str(&format!(
                "{}={}\n",
                action.config_key(),
                self.pin_mouse.get(action).to_ini()
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

    #[test]
    fn disabled_shortcut_roundtrip() {
        let mut config = Config::default();
        config
            .global_shortcuts
            .set(GlobalAction::Pin, Shortcut::disabled());
        let parsed = Config::from_ini(&config.to_ini());
        assert!(!parsed.global_shortcuts.get(GlobalAction::Pin).enabled);
        assert!(parsed.global_shortcuts.get(GlobalAction::Capture).enabled);
    }

    #[test]
    fn global_shortcuts_accept_common_keys() {
        for key in [Key::F1, Key::F12, Key::S, Key::Num3, Key::ArrowUp] {
            let shortcut = Shortcut::new(Modifiers::CTRL, key);
            assert!(shortcut.to_global().is_some(), "{key:?}");
        }
        assert!(
            Shortcut::new(Modifiers::NONE, Key::Copy)
                .to_global()
                .is_none()
        );
    }

    #[test]
    fn mouse_binding_roundtrip() {
        let binding = MouseBinding::parse("Ctrl+Shift+wheel_up+scroll").expect("valid binding");
        assert_eq!(binding.input, MouseInput::WheelUp);
        assert_eq!(binding.gesture, MouseGesture::Scroll);
        assert!(binding.modifiers.ctrl && binding.modifiers.shift);
        assert_eq!(binding.to_ini(), "Ctrl+Shift+wheel_up+scroll");
        assert!(MouseBinding::parse("Ctrl+left").is_none());
    }

    #[test]
    fn ini_roundtrip_keeps_mouse_and_global_settings() {
        let mut config = Config::default();
        config.pin_mouse.set(
            PinMouseAction::Close,
            MouseBinding::new(Modifiers::ALT, MouseInput::Right, MouseGesture::Click),
        );
        config
            .global_shortcuts
            .set(GlobalAction::Quit, Shortcut::disabled());
        config
            .pin_mouse
            .set(PinMouseAction::Move, MouseBinding::disabled());
        config.capture_delay = 7;

        let parsed = Config::from_ini(&config.to_ini());
        assert_eq!(
            parsed.pin_mouse.get(PinMouseAction::Close),
            config.pin_mouse.get(PinMouseAction::Close)
        );
        assert!(!parsed.pin_mouse.get(PinMouseAction::Move).enabled);
        assert!(!parsed.global_shortcuts.get(GlobalAction::Quit).enabled);
        assert_eq!(parsed.capture_delay, 7);
    }
}
