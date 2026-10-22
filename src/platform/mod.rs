/// 检测当前是否运行在 Wayland 会话下。
///
/// Wayland 无法绝对定位窗口、无法置顶、不支持全局快捷键，
/// 相关能力需要降级（见技术方案 §7.2）。
pub fn is_wayland() -> bool {
    let wayland_display = std::env::var("WAYLAND_DISPLAY").unwrap_or_default();
    let session_type = std::env::var("XDG_SESSION_TYPE").unwrap_or_default();
    !wayland_display.is_empty() || session_type.eq_ignore_ascii_case("wayland")
}

/// 是否使用 X11（XWayland）后端。
///
/// Mutter 等 Wayland 合成器无法可靠地把窗口放到指定输出；
/// 存在 XWayland（`DISPLAY`）时改用 X11 后端，可用绝对坐标精确覆盖各显示器。
pub fn use_x11_backend() -> bool {
    cfg!(target_os = "linux") && std::env::var_os("DISPLAY").is_some()
}

/// 查询 X11 全局指针位置（物理像素）。仅在 Linux/XWayland 下可用。
#[cfg(target_os = "linux")]
pub fn pointer_position() -> Option<(i32, i32)> {
    use x11_dl::xlib::Xlib;

    if std::env::var_os("DISPLAY").is_none() {
        return None;
    }
    let xlib = Xlib::open().ok()?;
    unsafe {
        let display = (xlib.XOpenDisplay)(std::ptr::null());
        if display.is_null() {
            return None;
        }
        let root = (xlib.XDefaultRootWindow)(display);
        let (mut root_ret, mut child) = (0, 0);
        let (mut root_x, mut root_y) = (0, 0);
        let (mut win_x, mut win_y) = (0, 0);
        let mut mask = 0;
        let ok = (xlib.XQueryPointer)(
            display,
            root,
            &mut root_ret,
            &mut child,
            &mut root_x,
            &mut root_y,
            &mut win_x,
            &mut win_y,
            &mut mask,
        );
        (xlib.XCloseDisplay)(display);
        if ok != 0 {
            Some((root_x, root_y))
        } else {
            None
        }
    }
}

#[cfg(not(target_os = "linux"))]
pub fn pointer_position() -> Option<(i32, i32)> {
    None
}
