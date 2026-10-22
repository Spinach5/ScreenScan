//! 动作执行层：复制 / 保存 / 贴图 / 扫描。

pub mod copy;
pub mod save;
pub mod scan;

use std::path::PathBuf;

use image::RgbaImage;

pub enum ActionOutcome {
    Copied,
    Saved(PathBuf),
    Failed(String),
}

pub fn copy(rgba: &RgbaImage) -> ActionOutcome {
    copy::copy(rgba)
}

pub fn save(rgba: &RgbaImage) -> ActionOutcome {
    save::save(rgba)
}

/// 把文本写入剪贴板（扫描结果用）。
pub fn copy_text(text: &str) -> anyhow::Result<()> {
    let mut clipboard = arboard::Clipboard::new().map_err(|e| anyhow::anyhow!("{e}"))?;
    clipboard.set_text(text).map_err(|e| anyhow::anyhow!("{e}"))
}
