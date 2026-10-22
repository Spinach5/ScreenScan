use image::RgbaImage;

use super::ActionOutcome;

/// 弹出系统保存对话框并写出图片。
///
/// `FileDialog::save_file` 为阻塞调用，在覆盖层线程执行；
/// 保存成功后覆盖层随即关闭，故不影响交互。
pub fn save(rgba: &RgbaImage) -> ActionOutcome {
    let path = rfd::FileDialog::new()
        .add_filter("PNG", &["png"])
        .add_filter("JPEG", &["jpg", "jpeg"])
        .add_filter("WebP", &["webp"])
        .set_file_name("screenshot.png")
        .save_file();

    let Some(path) = path else {
        return ActionOutcome::Failed("已取消保存".into());
    };

    match rgba.save(&path) {
        Ok(()) => ActionOutcome::Saved(path),
        Err(e) => ActionOutcome::Failed(format!("保存失败: {e}")),
    }
}
