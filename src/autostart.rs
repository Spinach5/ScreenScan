//! 开机启动（Linux: XDG autostart / Windows: 注册表 Run / macOS: LaunchAgent）。

use std::path::Path;
use std::path::PathBuf;

use anyhow::Result;

use crate::util::path;

pub fn set(enabled: bool) -> Result<()> {
    set_impl(enabled)
}

#[cfg(target_os = "linux")]
fn set_impl(enabled: bool) -> Result<()> {
    let path = config_home().join("autostart").join("screenscan.desktop");
    if !enabled {
        if path.is_file() {
            std::fs::remove_file(&path)?;
        }
        return Ok(());
    }

    let exe = std::env::current_exe()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let contents = format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=ScreenScan\n\
         Comment=Capture, annotate and pin screenshots\n\
         Exec={}\n\
         Terminal=false\n\
         X-GNOME-Autostart-enabled=true\n",
        desktop_exec(&exe)
    );
    std::fs::write(&path, contents)?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn config_home() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .unwrap_or_else(|| path::home_dir().join(".config"))
}

#[cfg(target_os = "linux")]
fn desktop_exec(exe: &Path) -> String {
    let value = exe.display().to_string();
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

#[cfg(target_os = "macos")]
fn set_impl(enabled: bool) -> Result<()> {
    let path = path::home_dir().join("Library/LaunchAgents/com.spinach5.screenscan.plist");
    if !enabled {
        if path.is_file() {
            std::fs::remove_file(&path)?;
        }
        return Ok(());
    }

    let exe = std::env::current_exe()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let contents = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key><string>com.spinach5.screenscan</string>
  <key>ProgramArguments</key>
  <array><string>{}</string></array>
  <key>RunAtLoad</key><true/>
</dict>
</plist>
"#,
        xml_escape(&exe.display().to_string())
    );
    std::fs::write(&path, contents)?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(target_os = "windows")]
fn set_impl(enabled: bool) -> Result<()> {
    const KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
    let status = if enabled {
        let exe = std::env::current_exe()?;
        let value = format!("\"{}\"", exe.display());
        std::process::Command::new("reg")
            .args([
                "add",
                KEY,
                "/v",
                "ScreenScan",
                "/t",
                "REG_SZ",
                "/d",
                &value,
                "/f",
            ])
            .status()?
    } else {
        std::process::Command::new("reg")
            .args(["delete", KEY, "/v", "ScreenScan", "/f"])
            .status()?
    };
    if enabled && !status.success() {
        anyhow::bail!("reg add 退出码: {status}");
    }
    // 关闭时注册表值可能本来就不存在，忽略失败。
    Ok(())
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn set_impl(_enabled: bool) -> Result<()> {
    anyhow::bail!("当前平台不支持开机启动")
}
