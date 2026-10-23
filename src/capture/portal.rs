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

use crate::capture::CaptureFailure;

#[derive(DeserializeDict, Type, Debug)]
#[zvariant(signature = "dict")]
struct ScreenshotResponse {
    /// 失败时 results 为空字典，因此必须是 Option，否则会在检查响应码前先报
    /// "missing field `uri`"，掩盖真正的失败原因。
    uri: Option<String>,
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
    let uri = response
        .uri
        .ok_or_else(|| anyhow::anyhow!("portal screenshot response has no uri"))?;
    let path = uri_to_path(&uri)?;
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
        1 => Err(anyhow::Error::new(CaptureFailure::Cancelled)),
        other => {
            // 典型原因是系统授权对话框无法显示（请求方不是焦点应用），
            // 由上层引导用户完成一次授权后重试。
            log::warn!("portal screenshot failed (response code {other})");
            Err(anyhow::Error::new(CaptureFailure::PermissionDenied))
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use zbus::zvariant::serialized::Context;

    fn parse_response(code: u32, uri: Option<&'static str>) -> (u32, ScreenshotResponse) {
        let mut results: HashMap<String, Value<'static>> = HashMap::new();
        if let Some(uri) = uri {
            results.insert("uri".to_owned(), Value::from(uri));
        }

        let data =
            zbus::zvariant::to_bytes(Context::new_dbus(zbus::zvariant::LE, 0), &(code, results))
                .expect("serialize portal response");
        let (response, _) = data
            .deserialize::<(u32, ScreenshotResponse)>()
            .expect("deserialize portal response");
        response
    }

    #[test]
    fn reads_uri_from_successful_response() {
        let (code, response) = parse_response(0, Some("file:///tmp/shot.png"));
        assert_eq!(code, 0);
        assert_eq!(response.uri.as_deref(), Some("file:///tmp/shot.png"));
    }

    #[test]
    fn tolerates_empty_results_on_failure() {
        // 授权失败时 results 为空字典；此前会误报 "missing field `uri`"。
        let (code, response) = parse_response(2, None);
        assert_eq!(code, 2);
        assert!(response.uri.is_none());
    }
}
