//! 工具栏：以 egui `Area` 叠加在覆盖层同一视口内（不建独立窗口）。

use egui::Id;
use egui::Order;
use egui::Pos2;
use egui::Rect;
use egui::Vec2;

/// 工具栏触发的动作。
#[derive(Debug, Default, Clone, Copy)]
pub struct ToolbarAction {
    pub copy: bool,
    pub save: bool,
}

/// 工具栏展示结果。
pub struct ToolbarResponse {
    pub action: ToolbarAction,
    /// 指针是否落在工具栏上（用于避免与选区交互冲突）。
    pub contains_pointer: bool,
}

/// 工具栏尺寸估算（用于定位与越界翻转）。
const ESTIMATED_SIZE: Vec2 = Vec2::new(150.0, 40.0);
const GAP: f32 = 8.0;
const MARGIN: f32 = 4.0;

/// 在选区下方居中显示工具栏；下方越界则翻到选区上方。
pub fn show(ctx: &egui::Context, selection: Rect, screen_rect: Rect) -> ToolbarResponse {
    let pos = position(selection, screen_rect);

    let mut action = ToolbarAction::default();
    let inner = egui::Area::new(Id::unique("screenscan_toolbar"))
        .order(Order::Foreground)
        .fixed_pos(pos)
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("复制").clicked() {
                        action.copy = true;
                    }
                    if ui.button("保存").clicked() {
                        action.save = true;
                    }
                });
            });
        });

    ToolbarResponse {
        action,
        contains_pointer: inner.response.contains_pointer(),
    }
}

fn position(selection: Rect, screen_rect: Rect) -> Pos2 {
    let mut x = selection.center().x - ESTIMATED_SIZE.x / 2.0;
    let mut y = selection.max.y + GAP;

    // 下方越界 → 翻到上方
    if y + ESTIMATED_SIZE.y > screen_rect.max.y {
        y = selection.min.y - GAP - ESTIMATED_SIZE.y;
    }

    x = x.clamp(
        screen_rect.min.x + MARGIN,
        (screen_rect.max.x - ESTIMATED_SIZE.x - MARGIN).max(screen_rect.min.x + MARGIN),
    );
    y = y.clamp(
        screen_rect.min.y + MARGIN,
        (screen_rect.max.y - ESTIMATED_SIZE.y - MARGIN).max(screen_rect.min.y + MARGIN),
    );

    Pos2::new(x, y)
}
