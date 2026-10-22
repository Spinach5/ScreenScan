//! 通过 xdg-desktop-portal 一次性抓取整个虚拟桌面（Wayland）。
//!
//! 逐个显示器调用 `Monitor::capture_image` 会为每块屏各触发一次 portal
//! 截图（GNOME 每次都会闪一下）。这里改为只调用一次 Screenshot，再按各
//! 显示器的矩形切分：闪光从 N 次降为 1 次，且所有屏来自同一时刻。

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use image::RgbaImage;
use zbus::blocking::Connection;
use zbus::blocking::Proxy;
use zbus::zvariant::DeserializeDict;
use zbus::zvariant::Type;
use zbus::zvariant::Value;

#[derive(DeserializeDict, Type, Debug)]
#[zvariant(signature = "dict")]
struct ScreenshotResponse {
    uri: String,
}

/// 一次性抓取整个虚拟桌面（阻塞，应在后台线程调用）。
pub fn capture_desktop() -> anyhow::Result<RgbaImage> {
    let conn = Connection::session().map_err(|e| anyhow::anyhow!("{e}"))?;
    let portal = Proxy::new(
        &conn,
        "org.freedesktop.portal.Desktop",
        "/org/freedesktop/portal/desktop",
        "org.freedesktop.portal.Screenshot",
    )
    .map_err(|e| anyhow::anyhow!("{e}"))?;

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let token = format!("screenscan{nanos}");

    let request = request_proxy(&conn, &token)?;
    let receiver = wait_response::<ScreenshotResponse>(&request);

    let mut options: HashMap<&str, Value> = HashMap::new();
    options.insert("handle_token", Value::from(&token));
    options.insert("modal", Value::from(false));
    options.insert("interactive", Value::from(false));

    portal
        .call_method("Screenshot", &("", options))
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let response = receiver.recv().map_err(|e| anyhow::anyhow!("{e}"))??;
    let path = uri_to_path(&response.uri)?;
    let image = image::open(&path)
        .map_err(|e| anyhow::anyhow!("{e}"))?
        .to_rgba8();
    let _ = std::fs::remove_file(&path);
    Ok(image)
}

fn request_proxy(conn: &Connection, token: &str) -> anyhow::Result<Proxy<'static>> {
    let unique = conn
        .unique_name()
        .ok_or_else(|| anyhow::anyhow!("no DBus unique name"))?
        .trim_start_matches(':')
        .replace('.', "_");
    let path = format!("/org/freedesktop/portal/desktop/request/{unique}/{token}");
    Proxy::new(
        conn,
        "org.freedesktop.portal.Desktop",
        path,
        "org.freedesktop.portal.Request",
    )
    .map_err(|e| anyhow::anyhow!("{e}"))
}

fn wait_response<T>(request: &Proxy<'static>) -> Receiver<anyhow::Result<T>>
where
    T: for<'de> serde::Deserialize<'de> + Type + Send + Sync + 'static,
{
    let (tx, rx) = std::sync::mpsc::channel();
    let request = request.clone();
    std::thread::spawn(move || {
        let _ = tx.send(wait_response_inner::<T>(&request));
    });
    rx
}

fn wait_response_inner<T>(request: &Proxy<'_>) -> anyhow::Result<T>
where
    T: for<'de> serde::Deserialize<'de> + Type,
{
    let mut stream = request
        .receive_signal("Response")
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let message = stream
        .next()
        .ok_or_else(|| anyhow::anyhow!("portal response stream ended"))?;
    let body = message.body();
    let (code, results): (u32, T) = body.deserialize().map_err(|e| anyhow::anyhow!("{e}"))?;
    match code {
        0 => Ok(results),
        1 => Err(anyhow::anyhow!("portal screenshot cancelled")),
        other => Err(anyhow::anyhow!("portal screenshot failed (code {other})")),
    }
}

fn uri_to_path(uri: &str) -> anyhow::Result<PathBuf> {
    let encoded = uri
        .strip_prefix("file://")
        .ok_or_else(|| anyhow::anyhow!("unexpected screenshot uri: {uri}"))?;
    Ok(PathBuf::from(percent_decode(encoded)))
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(hi), Some(lo)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2])) {
                out.push(hi * 16 + lo);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}
