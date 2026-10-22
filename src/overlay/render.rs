//! 覆盖层渲染：冻结截图 + 暗色蒙层 + 选区挖空 + 边框。

use egui::Color32;
use egui::Painter;
use egui::Pos2;
use egui::Rect;
use egui::Stroke;
use egui::StrokeKind;
use egui::TextureId;

/// 蒙层颜色（半透明黑）。
pub const DIM: Color32 = Color32::from_black_alpha(140);
/// 选区边框颜色。
const BORDER: Color32 = Color32::WHITE;

/// 全窗铺满冻结截图。
pub fn draw_screenshot(painter: &Painter, tex: TextureId, screen_rect: Rect) {
    painter.image(tex, screen_rect, full_uv(), Color32::WHITE);
}

/// 绘制蒙层：选区外变暗，选区内贴回原图（挖空）。
///
/// `selection` 为逻辑点坐标（与 `screen_rect` 同空间）。
pub fn draw_mask(painter: &Painter, tex: TextureId, screen_rect: Rect, selection: Option<Rect>) {
    let Some(sel) = selection else {
        painter.rect_filled(screen_rect, 0.0, DIM);
        return;
    };

    // 1) 选区四周的暗色矩形
    painter.rect_filled(
        Rect::from_min_max(screen_rect.min, Pos2::new(screen_rect.max.x, sel.min.y)),
        0.0,
        DIM,
    );
    painter.rect_filled(
        Rect::from_min_max(Pos2::new(screen_rect.min.x, sel.max.y), screen_rect.max),
        0.0,
        DIM,
    );
    painter.rect_filled(
        Rect::from_min_max(
            Pos2::new(screen_rect.min.x, sel.min.y),
            Pos2::new(sel.min.x, sel.max.y),
        ),
        0.0,
        DIM,
    );
    painter.rect_filled(
        Rect::from_min_max(
            Pos2::new(sel.max.x, sel.min.y),
            Pos2::new(screen_rect.max.x, sel.max.y),
        ),
        0.0,
        DIM,
    );

    // 2) 选区贴回原图：uv 裁剪
    painter.image(tex, sel, uv_for(sel, screen_rect), Color32::WHITE);
}

/// 绘制选区边框。
pub fn draw_border(painter: &Painter, selection: Rect) {
    painter.rect_stroke(selection, 0.0, Stroke::new(1.0, BORDER), StrokeKind::Inside);
}

fn full_uv() -> Rect {
    Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0))
}

/// 将屏幕内子矩形换算为纹理 uv（冻结截图铺满 `screen_rect`）。
fn uv_for(sel: Rect, screen: Rect) -> Rect {
    let min = Pos2::new(
        (sel.min.x - screen.min.x) / screen.width(),
        (sel.min.y - screen.min.y) / screen.height(),
    );
    let max = Pos2::new(
        (sel.max.x - screen.min.x) / screen.width(),
        (sel.max.y - screen.min.y) / screen.height(),
    );
    Rect::from_min_max(min, max)
}
