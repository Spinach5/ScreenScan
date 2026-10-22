/// 检测当前是否运行在 Wayland 会话下。
///
/// Wayland 无法绝对定位窗口、无法置顶、不支持全局快捷键，
/// 相关能力需要降级（见技术方案 §7.2）。
pub fn is_wayland() -> bool {
    let wayland_display = std::env::var("WAYLAND_DISPLAY").unwrap_or_default();
    let session_type = std::env::var("XDG_SESSION_TYPE").unwrap_or_default();
    !wayland_display.is_empty() || session_type.eq_ignore_ascii_case("wayland")
}
