use std::borrow::Cow;

use arboard::Clipboard;
use arboard::ImageData;
use image::RgbaImage;

use super::ActionOutcome;

/// 把 RGBA 图像写入系统剪贴板。
///
/// 注意：X11 下剪贴板内容归进程所有，进程退出后若无剪贴板管理器
/// （clipman/copyq 等），粘贴目标可能拿不到数据（技术方案 §1.4）。
pub fn copy(rgba: &RgbaImage) -> ActionOutcome {
    let mut clipboard = match Clipboard::new() {
        Ok(c) => c,
        Err(e) => return ActionOutcome::Failed(format!("剪贴板不可用: {e}")),
    };

    let image = ImageData {
        width: rgba.width() as usize,
        height: rgba.height() as usize,
        bytes: Cow::Borrowed(rgba.as_raw()),
    };

    match clipboard.set_image(image) {
        Ok(()) => ActionOutcome::Copied,
        Err(e) => ActionOutcome::Failed(format!("复制失败: {e}")),
    }
}
