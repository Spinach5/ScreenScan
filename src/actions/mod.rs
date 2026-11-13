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

pub fn save(rgba: &RgbaImage, default_name: &str) -> ActionOutcome {
    save::save(rgba, default_name)
}

/// 把文本写入剪贴板（扫描结果用）。
pub fn copy_text(text: &str) -> anyhow::Result<()> {
    let mut clipboard = arboard::Clipboard::new().map_err(|e| anyhow::anyhow!("{e}"))?;
    clipboard.set_text(text).map_err(|e| anyhow::anyhow!("{e}"))
}

/// 读取剪贴板中的图片（全局「贴图」快捷键用）。
pub fn read_clipboard_image() -> anyhow::Result<RgbaImage> {
    let mut clipboard = arboard::Clipboard::new().map_err(|e| anyhow::anyhow!("{e}"))?;
    let image = clipboard.get_image().map_err(|e| anyhow::anyhow!("{e}"))?;
    RgbaImage::from_raw(
        image.width as u32,
        image.height as u32,
        image.bytes.into_owned(),
    )
    .ok_or_else(|| anyhow::anyhow!("剪贴板图片数据无效"))
}
