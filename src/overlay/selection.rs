//! 选区：以物理像素存储，与 xcap 输出同空间。含命中测试、缩放、平移。

use egui::Pos2;
use egui::Rect;
use egui::Vec2;

/// 选区最小尺寸（物理像素）。
pub const MIN_SELECTION: f32 = 8.0;

/// 命中区域：8 个缩放方向 + 内部平移 + 无。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitArea {
    N,
    S,
    E,
    W,
    NE,
    NW,
    SE,
    SW,
    Move,
    None,
}

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

    /// 命中优先级：控制点(角) > 边 > 内部。
    pub fn hit_test(&self, pos: Pos2, handle_r: f32) -> HitArea {
        let r = self.rect;
        let near_left = (pos.x - r.min.x).abs() <= handle_r;
        let near_right = (pos.x - r.max.x).abs() <= handle_r;
        let near_top = (pos.y - r.min.y).abs() <= handle_r;
        let near_bottom = (pos.y - r.max.y).abs() <= handle_r;
        let within_x = pos.x >= r.min.x - handle_r && pos.x <= r.max.x + handle_r;
        let within_y = pos.y >= r.min.y - handle_r && pos.y <= r.max.y + handle_r;

        if near_left && near_top {
            return HitArea::NW;
        }
        if near_right && near_top {
            return HitArea::NE;
        }
        if near_left && near_bottom {
            return HitArea::SW;
        }
        if near_right && near_bottom {
            return HitArea::SE;
        }
        if near_left && within_y {
            return HitArea::W;
        }
        if near_right && within_y {
            return HitArea::E;
        }
        if near_top && within_x {
            return HitArea::N;
        }
        if near_bottom && within_x {
            return HitArea::S;
        }
        if r.contains(pos) {
            return HitArea::Move;
        }
        HitArea::None
    }

    /// 按命中方向缩放；受最小尺寸与屏幕边界约束。
    pub fn resize(&mut self, hit: HitArea, pos: Pos2, min_size: f32, bounds: Rect) {
        let mut r = self.rect;
        // 不会因浮点误差出现 min > max 的 clamp（否则会 panic）
        let clamp = |v: f32, lo: f32, hi: f32| v.clamp(lo, hi.max(lo));
        match hit {
            HitArea::N => r.min.y = clamp(pos.y, bounds.min.y, r.max.y - min_size),
            HitArea::S => r.max.y = clamp(pos.y, r.min.y + min_size, bounds.max.y),
            HitArea::W => r.min.x = clamp(pos.x, bounds.min.x, r.max.x - min_size),
            HitArea::E => r.max.x = clamp(pos.x, r.min.x + min_size, bounds.max.x),
            HitArea::NW => {
                r.min.x = clamp(pos.x, bounds.min.x, r.max.x - min_size);
                r.min.y = clamp(pos.y, bounds.min.y, r.max.y - min_size);
            }
            HitArea::NE => {
                r.max.x = clamp(pos.x, r.min.x + min_size, bounds.max.x);
                r.min.y = clamp(pos.y, bounds.min.y, r.max.y - min_size);
            }
            HitArea::SW => {
                r.min.x = clamp(pos.x, bounds.min.x, r.max.x - min_size);
                r.max.y = clamp(pos.y, r.min.y + min_size, bounds.max.y);
            }
            HitArea::SE => {
                r.max.x = clamp(pos.x, r.min.x + min_size, bounds.max.x);
                r.max.y = clamp(pos.y, r.min.y + min_size, bounds.max.y);
            }
            HitArea::Move | HitArea::None => {}
        }
        self.rect = r;
    }

    /// 平移；保持尺寸并钳制在屏幕边界内。
    pub fn move_by(&mut self, delta: Vec2, bounds: Rect) {
        let mut r = self.rect.translate(delta);
        if r.min.x < bounds.min.x {
            r = r.translate(Vec2::new(bounds.min.x - r.min.x, 0.0));
        }
        if r.max.x > bounds.max.x {
            r = r.translate(Vec2::new(bounds.max.x - r.max.x, 0.0));
        }
        if r.min.y < bounds.min.y {
            r = r.translate(Vec2::new(0.0, bounds.min.y - r.min.y));
        }
        if r.max.y > bounds.max.y {
            r = r.translate(Vec2::new(0.0, bounds.max.y - r.max.y));
        }
        self.rect = r;
    }
}
