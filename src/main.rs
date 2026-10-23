mod actions;
mod annotate;
mod app;
mod autostart;
mod capture;
mod config;
mod font;
mod graphics;
mod history;
mod i18n;
mod logging;
mod overlay;
mod pin;
mod platform;
mod settings;
mod toolbar;
mod tray;
mod util;

use winit::event_loop::EventLoop;

use crate::app::{App, UserEvent};
use crate::config::ConfigStore;
use crate::util::path;

fn main() -> anyhow::Result<()> {
    let mut store = ConfigStore::load();
    logging::init(store.config.log_level);
    install_panic_hook();
    font::set_choice(store.config.font.clone());

    // 首次启动落盘一份默认配置，便于用户直接编辑与备份。
    if !store.path.is_file()
        && let Err(e) = store.save()
    {
        log::warn!("初始化配置文件失败: {e}");
    }

    log::info!(
        "ScreenScan {} 启动 ({} {})",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    if let Ok(exe) = std::env::current_exe() {
        log::info!("可执行文件: {}", exe.display());
    }
    log::info!("配置文件: {}", store.display_path());
    log::info!("日志目录: {}", path::display_path(&logging::log_dir()));

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
    // --preferences：直接打开设置窗口（托盘不可用时的备用入口，也便于无托盘宿主测试）
    let show_settings = std::env::args().any(|arg| matches!(arg.as_str(), "--preferences"));

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
    let mut app = App::new(proxy, one_shot, show_settings, store);
    event_loop.run_app(&mut app)?;
    Ok(())
}

fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        log::error!("应用发生 panic: {info}");
        previous(info);
    }));
}
