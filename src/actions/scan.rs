//! 条码/二维码识别：rxing 解码 + 预处理（原图 → 反色 → 2x 放大）。

use std::collections::HashSet;

use image::RgbaImage;
use rxing::BarcodeFormat;
use rxing::BinaryBitmap;
use rxing::DecodeHints;
use rxing::MultiFormatReader;
use rxing::RGBLuminanceSource;
use rxing::Reader;
use rxing::common::HybridBinarizer;

/// 阻塞调用，应在后台线程执行。
pub fn decode(rgba: &RgbaImage) -> anyhow::Result<String> {
    let (w, h) = rgba.dimensions();
    let pixels: Vec<u32> = rgba
        .pixels()
        .map(|p| ((p[0] as u32) << 16) | ((p[1] as u32) << 8) | (p[2] as u32))
        .collect();

    // 尝试 1：原图
    if let Some(text) = try_decode(&pixels, w, h) {
        return Ok(text);
    }

    // 尝试 2：反色
    let inverted: Vec<u32> = pixels.iter().map(|p| p ^ 0x00FF_FFFF).collect();
    if let Some(text) = try_decode(&inverted, w, h) {
        return Ok(text);
    }

    // 尝试 3：2x 最近邻放大
    let (up, uw, uh) = upscale2x(&pixels, w, h);
    if let Some(text) = try_decode(&up, uw, uh) {
        return Ok(text);
    }

    anyhow::bail!("未识别到条码")
}

fn try_decode(pixels: &[u32], width: u32, height: u32) -> Option<String> {
    let source =
        RGBLuminanceSource::new_with_width_height_pixels(width as usize, height as usize, pixels)
            .ok()?;
    let mut bitmap = BinaryBitmap::new(HybridBinarizer::new(source));
    let hints = decode_hints();
    let mut reader = MultiFormatReader::default();
    reader
        .decode_with_hints(&mut bitmap, &hints)
        .ok()
        .map(|result| result.getText().to_string())
}

fn decode_hints() -> DecodeHints {
    let formats: HashSet<BarcodeFormat> = [
        BarcodeFormat::QR_CODE,
        BarcodeFormat::DATA_MATRIX,
        BarcodeFormat::PDF_417,
        BarcodeFormat::AZTEC,
        BarcodeFormat::CODE_128,
        BarcodeFormat::CODE_39,
        BarcodeFormat::EAN_13,
    ]
    .into_iter()
    .collect();

    DecodeHints {
        TryHarder: Some(true),
        PossibleFormats: Some(formats),
        ..Default::default()
    }
}

/// 最近邻 2x 放大（像素打包为 0xRRGGBB）。
fn upscale2x(pixels: &[u32], w: u32, h: u32) -> (Vec<u32>, u32, u32) {
    let uw = w * 2;
    let uh = h * 2;
    let mut out = vec![0u32; (uw * uh) as usize];
    for y in 0..h {
        for x in 0..w {
            let p = pixels[(y * w + x) as usize];
            let row0 = (y * 2) * uw;
            let row1 = (y * 2 + 1) * uw;
            out[(row0 + x * 2) as usize] = p;
            out[(row0 + x * 2 + 1) as usize] = p;
            out[(row1 + x * 2) as usize] = p;
            out[(row1 + x * 2 + 1) as usize] = p;
        }
    }
    (out, uw, uh)
}
