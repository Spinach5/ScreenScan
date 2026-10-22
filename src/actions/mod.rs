//! 动作执行层：复制 / 保存 / 贴图 / 扫描。

pub mod copy;
pub mod save;

use std::path::PathBuf;

use image::RgbaImage;

pub enum ActionOutcome {
    Copied,
    Saved(PathBuf),
    Pinned,
    Scanned(String),
    Failed(String),
}

pub fn copy(rgba: &RgbaImage) -> ActionOutcome {
    copy::copy(rgba)
}

pub fn save(rgba: &RgbaImage) -> ActionOutcome {
    save::save(rgba)
}
