//! 跨平台路径辅助。

use std::path::Path;
use std::path::PathBuf;

pub fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn expand_tilde(value: &str) -> PathBuf {
    if value == "~" {
        return home_dir();
    }
    match value
        .strip_prefix("~/")
        .or_else(|| value.strip_prefix("~\\"))
    {
        Some(rest) => home_dir().join(rest),
        None => PathBuf::from(value),
    }
}

/// 用户界面中展示的路径：位于主目录下时缩写为 `~/...`。
pub fn display_path(path: &Path) -> String {
    let home = home_dir();
    match path.strip_prefix(&home) {
        Ok(rest) if !rest.as_os_str().is_empty() => format!("~/{}", rest.display()),
        Ok(_) => "~".to_owned(),
        Err(_) => path.display().to_string(),
    }
}
