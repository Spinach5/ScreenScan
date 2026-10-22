//! 加载系统中文字体，补齐 egui 默认字体（Ubuntu/Hack/Emoji）缺失的 CJK 字形。

use std::sync::Arc;
use std::sync::OnceLock;

use egui::FontData;
use egui::FontDefinitions;
use egui::FontFamily;

/// 按优先级尝试的系统中文字体（覆盖 Linux / macOS / Windows）。
/// 单字体文件优先，避免 TTC 集合索引差异。
const CANDIDATES: &[&str] = &[
    // Linux
    "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf",
    "/usr/share/fonts/truetype/winfonts/simhei.ttf",
    "/usr/share/fonts/truetype/winfonts/NotoSansSC-VF.ttf",
    "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
    "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
    // macOS
    "/System/Library/Fonts/PingFang.ttc",
    "/System/Library/Fonts/STHeiti Light.ttc",
    "/System/Library/Fonts/Hiragino Sans GB.ttc",
    // Windows
    "C:\\Windows\\Fonts\\msyh.ttc",
    "C:\\Windows\\Fonts\\msyh.ttf",
    "C:\\Windows\\Fonts\\simhei.ttf",
    "C:\\Windows\\Fonts\\simsun.ttc",
];

fn cjk_font() -> Option<&'static Arc<FontData>> {
    static FONT: OnceLock<Option<Arc<FontData>>> = OnceLock::new();
    FONT.get_or_init(|| {
        for path in CANDIDATES {
            match std::fs::read(path) {
                Ok(bytes) => {
                    log::info!("加载中文字体: {path}");
                    return Some(Arc::new(FontData::from_owned(bytes)));
                }
                Err(_) => continue,
            }
        }
        log::warn!("未找到系统中文字体，中文界面文字可能无法显示");
        None
    })
    .as_ref()
}

/// 在 egui Context 上安装中文字体，作为默认字体的回退。
pub fn install(ctx: &egui::Context) {
    let Some(font) = cjk_font() else {
        return;
    };
    let mut defs = FontDefinitions::default();
    defs.font_data.insert("cjk".to_owned(), font.clone());
    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        defs.families
            .entry(family)
            .or_default()
            .push("cjk".to_owned());
    }
    ctx.set_fonts(defs);
}
