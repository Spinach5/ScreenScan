use image::RgbaImage;
use xcap::Monitor;

#[cfg(target_os = "linux")]
mod portal;

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

    // Wayland：优先一次抓整块虚拟桌面再切分，避免每屏各触发一次 portal 闪光。
    #[cfg(target_os = "linux")]
    if crate::platform::is_wayland() {
        match portal::capture_desktop().and_then(|desktop| slice_desktop(&desktop, &monitors)) {
            Ok(captured) if captured.len() == monitors.len() => {
                log::info!(
                    "captured {} monitors in a single portal screenshot",
                    captured.len()
                );
                return Ok(captured);
            }
            Ok(_) => log::warn!("single-shot capture produced incomplete monitors"),
            Err(e) => log::warn!("single-shot desktop capture failed: {e}"),
        }
    }

    // 回退：逐显示器抓取。
    monitors.into_iter().map(|m| capture_monitor(&m)).collect()
}

/// 把整块虚拟桌面按显示器矩形切分。
///
/// portal 返回的图像可能是逻辑尺寸或物理尺寸，这里用「图像宽度 / 逻辑总宽度」
/// 推出统一缩放比，兼容 scale=1 与整体缩放两种情形。
#[cfg(target_os = "linux")]
fn slice_desktop(
    desktop: &RgbaImage,
    monitors: &[MonitorInfo],
) -> anyhow::Result<Vec<CapturedMonitor>> {
    let (dw, dh) = desktop.dimensions();
    if dw == 0 || dh == 0 || monitors.is_empty() {
        anyhow::bail!("empty desktop image");
    }

    let min_x = monitors.iter().map(|m| m.x).min().unwrap_or(0);
    let min_y = monitors.iter().map(|m| m.y).min().unwrap_or(0);
    let total_w = monitors
        .iter()
        .map(|m| m.x + m.width as i32)
        .max()
        .unwrap_or(0)
        - min_x;
    let total_h = monitors
        .iter()
        .map(|m| m.y + m.height as i32)
        .max()
        .unwrap_or(0)
        - min_y;
    if total_w <= 0 || total_h <= 0 {
        anyhow::bail!("invalid monitor layout");
    }

    let sx = dw as f64 / total_w as f64;
    let sy = dh as f64 / total_h as f64;

    let mut out = Vec::with_capacity(monitors.len());
    for m in monitors {
        let x = (((m.x - min_x) as f64) * sx).round() as i64;
        let y = (((m.y - min_y) as f64) * sy).round() as i64;
        let w = ((m.width as f64) * sx).round() as i64;
        let h = ((m.height as f64) * sy).round() as i64;

        let x = x.clamp(0, dw as i64 - 1) as u32;
        let y = y.clamp(0, dh as i64 - 1) as u32;
        let cw = w.clamp(1, dw as i64 - x as i64) as u32;
        let ch = h.clamp(1, dh as i64 - y as i64) as u32;

        let image = image::imageops::crop_imm(desktop, x, y, cw, ch).to_image();
        log::debug!(
            "desktop {}x{} -> monitor {} crop ({},{},{},{})",
            dw,
            dh,
            m.name,
            x,
            y,
            cw,
            ch
        );
        out.push(CapturedMonitor {
            monitor: m.clone(),
            image,
        });
    }
    Ok(out)
}
