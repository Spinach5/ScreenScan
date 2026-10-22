mod app;
mod capture;
mod graphics;
mod overlay;
mod platform;

use winit::event_loop::EventLoop;

use crate::app::{App, UserEvent};

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let session = if platform::is_wayland() {
        "wayland"
    } else if std::env::var("DISPLAY").is_ok() {
        "x11"
    } else {
        "unknown"
    };
    log::info!("session type: {session}");

    let event_loop = EventLoop::<UserEvent>::with_user_event().build()?;
    let proxy = event_loop.create_proxy();
    let mut app = App::new(proxy);
    event_loop.run_app(&mut app)?;
    Ok(())
}
