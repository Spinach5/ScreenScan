//! 选区：以物理像素存储，与 xcap 输出同空间。

use egui::Pos2;
use egui::Rect;

/// 选区最小尺寸（物理像素）。
pub const MIN_SELECTION: f32 = 8.0;

#[derive(Debug, Clone, Copy)]
pub struct Selection {
    /// 物理像素坐标。
    pub rect: Rect,
}

impl Selection {
    /// 由拖拽的起点/当前点构造（自动规范化为 min/max）。
    pub fn new(anchor: Pos2, current: Pos2) -> Self {
        Self {
            rect: Rect::from_two_pos(anchor, current),
        }
    }

    pub fn is_valid(&self) -> bool {
        self.rect.width() >= MIN_SELECTION && self.rect.height() >= MIN_SELECTION
    }
}
