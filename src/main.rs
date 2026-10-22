mod actions;
mod annotate;
mod app;
mod capture;
mod font;
mod graphics;
mod overlay;
mod pin;
mod platform;
mod toolbar;
mod util;

use winit::event_loop::EventLoop;

use crate::app::{App, UserEvent};

fn main() -> anyhow::Result<()> {
    // 默认抑制 libwayshot_xcap 在部分合成器上的 wl_output 版本告警与 zbus 握手噪音
    env_logger::Builder::from_env(
        env_logger::Env::default()
            .default_filter_or("warn,ScreenScan=info,libwayshot_xcap=off,zbus=warn,tracing=warn"),
    )
    .init();

    let session = if platform::is_wayland() {
        "wayland"
    } else if std::env::var("DISPLAY").is_ok() {
        "x11"
    } else {
        "unknown"
    };
    log::info!("session type: {session}");

    // -c/--capture/--once：执行一次截图后退出（供桌面环境绑定快捷键，Wayland 推荐）
    let one_shot =
        std::env::args().any(|arg| matches!(arg.as_str(), "-c" | "--capture" | "--once"));

    let mut builder = EventLoop::<UserEvent>::with_user_event();

    // Wayland（Mutter）下无法可靠地把覆盖层窗口放到指定显示器；
    // 有 XWayland 时优先使用 X11 后端，以便按显示器绝对定位。
    #[cfg(target_os = "linux")]
    if platform::use_x11_backend() {
        use winit::platform::x11::EventLoopBuilderExtX11;
        builder.with_x11();
        log::info!("using winit X11 backend (XWayland) for deterministic multi-monitor placement");
    }

    let event_loop = builder.build()?;
    let proxy = event_loop.create_proxy();
    let mut app = App::new(proxy, one_shot);
    event_loop.run_app(&mut app)?;
    Ok(())
}
