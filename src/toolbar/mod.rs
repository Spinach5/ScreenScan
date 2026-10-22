//! 工具栏：以 egui `Area` 叠加在覆盖层同一视口内（不建独立窗口）。

use egui::Color32;
use egui::Id;
use egui::Order;
use egui::Pos2;
use egui::Rect;
use egui::Slider;
use egui::Vec2;

use crate::annotate::ToolKind;

/// 工具栏触发的动作。
#[derive(Debug, Default, Clone, Copy)]
pub struct ToolbarAction {
    pub copy: bool,
    pub save: bool,
    pub pin: bool,
    pub scan: bool,
    pub set_kind: Option<ToolKind>,
    pub set_color: Option<Color32>,
    pub set_width: Option<f32>,
    pub undo: bool,
    pub redo: bool,
}

/// 工具栏尺寸估算（用于定位与越界翻转）。
const ESTIMATED_SIZE: Vec2 = Vec2::new(650.0, 44.0);
const GAP: f32 = 8.0;
const MARGIN: f32 = 4.0;

/// 在选区下方居中显示工具栏；下方越界则翻到选区上方。
#[allow(clippy::too_many_arguments)]
pub fn show(
    ctx: &egui::Context,
    selection: Rect,
    screen_rect: Rect,
    tool_kind: ToolKind,
    color: Color32,
    width: f32,
    can_undo: bool,
    can_redo: bool,
) -> ToolbarAction {
    let pos = position(selection, screen_rect);

    let mut action = ToolbarAction::default();
    let mut color_edit = color;
    let mut width_edit = width;

    egui::Area::new(Id::unique("screenscan_toolbar"))
        .order(Order::Foreground)
        .fixed_pos(pos)
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.horizontal(|ui| {
                    tool_button(ui, &mut action, tool_kind, ToolKind::Brush);
                    tool_button(ui, &mut action, tool_kind, ToolKind::Scribble);
                    tool_button(ui, &mut action, tool_kind, ToolKind::Eraser);

                    ui.separator();

                    if ui.color_edit_button_srgba(&mut color_edit).changed() {
                        action.set_color = Some(color_edit);
                    }
                    if ui
                        .add(Slider::new(&mut width_edit, 1.0..=30.0).suffix("px"))
                        .changed()
                    {
                        action.set_width = Some(width_edit);
                    }

                    ui.separator();

                    if ui
                        .add_enabled(can_undo, egui::Button::new("撤销"))
                        .clicked()
                    {
                        action.undo = true;
                    }
                    if ui
                        .add_enabled(can_redo, egui::Button::new("重做"))
                        .clicked()
                    {
                        action.redo = true;
                    }

                    ui.separator();

                    if ui.button("复制").clicked() {
                        action.copy = true;
                    }
                    if ui.button("保存").clicked() {
                        action.save = true;
                    }
                    if ui.button("贴图").clicked() {
                        action.pin = true;
                    }
                    if ui.button("扫描").clicked() {
                        action.scan = true;
                    }
                });
            });
        });

    action
}

fn tool_button(ui: &mut egui::Ui, action: &mut ToolbarAction, current: ToolKind, target: ToolKind) {
    if ui
        .selectable_label(current == target, target.label())
        .clicked()
    {
        action.set_kind = Some(target);
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
