//! 图像工具：裁剪、合成。

use egui::Pos2;
use egui::Rect;
use image::RgbaImage;

/// 按物理像素矩形裁剪（自动取整并钳制在图像范围内）。
///
/// 返回裁剪结果与裁剪区域左上角的物理像素原点（取整后）。
pub fn crop(img: &RgbaImage, rect: Rect) -> (RgbaImage, Pos2) {
    let (w, h) = img.dimensions();
    let x0 = rect.min.x.floor().clamp(0.0, w as f32) as u32;
    let y0 = rect.min.y.floor().clamp(0.0, h as f32) as u32;
    let x1 = rect.max.x.ceil().clamp(0.0, w as f32) as u32;
    let y1 = rect.max.y.ceil().clamp(0.0, h as f32) as u32;
    let cw = (x1 - x0).max(1);
    let ch = (y1 - y0).max(1);
    let cropped = image::imageops::crop_imm(img, x0, y0, cw, ch).to_image();
    (cropped, Pos2::new(x0 as f32, y0 as f32))
}
