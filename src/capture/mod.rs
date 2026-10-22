use image::RgbaImage;
use xcap::Monitor;

/// 单个显示器的几何信息（物理像素坐标）。
#[derive(Debug, Clone)]
pub struct MonitorInfo {
    pub id: u32,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale_factor: f32,
    pub is_primary: bool,
}

/// 抓取到的单个显示器画面（冻结截图）。
pub struct CapturedMonitor {
    pub monitor: MonitorInfo,
    pub image: RgbaImage,
}

pub fn list_monitors() -> anyhow::Result<Vec<MonitorInfo>> {
    let monitors = Monitor::all().map_err(|e| anyhow::anyhow!("{e}"))?;
    monitors
        .into_iter()
        .map(|m| {
            Ok(MonitorInfo {
                id: m.id().map_err(|e| anyhow::anyhow!("{e}"))?,
                name: m.name().unwrap_or_default(),
                x: m.x().map_err(|e| anyhow::anyhow!("{e}"))?,
                y: m.y().map_err(|e| anyhow::anyhow!("{e}"))?,
                width: m.width().map_err(|e| anyhow::anyhow!("{e}"))?,
                height: m.height().map_err(|e| anyhow::anyhow!("{e}"))?,
                scale_factor: m.scale_factor().map_err(|e| anyhow::anyhow!("{e}"))?,
                is_primary: m.is_primary().map_err(|e| anyhow::anyhow!("{e}"))?,
            })
        })
        .collect()
}

/// 阻塞调用，应在后台线程执行。
pub fn capture_monitor(info: &MonitorInfo) -> anyhow::Result<CapturedMonitor> {
    let m = Monitor::from_point(info.x, info.y).map_err(|e| anyhow::anyhow!("{e}"))?;
    let image = m.capture_image().map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(CapturedMonitor {
        monitor: info.clone(),
        image,
    })
}

/// 阻塞调用，应在后台线程执行。
pub fn capture_all() -> anyhow::Result<Vec<CapturedMonitor>> {
    let monitors = list_monitors()?;
    monitors.into_iter().map(|m| capture_monitor(&m)).collect()
}
