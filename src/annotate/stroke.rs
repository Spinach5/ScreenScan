//! 标注工具与笔画模型。

use egui::Color32;
use egui::Pos2;

/// 涂鸦使用的固定线宽（逻辑点）。
pub const SCRIBBLE_WIDTH: f32 = 2.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolKind {
    Brush,
    Scribble,
    Eraser,
}

impl ToolKind {
    pub fn label(self) -> &'static str {
        match self {
            ToolKind::Brush => "画笔",
            ToolKind::Scribble => "涂鸦",
            ToolKind::Eraser => "橡皮擦",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tool {
    /// 画笔：指定颜色 + 粗细的实线。
    Brush { color: Color32, width: f32 },
    /// 涂鸦：自由手绘（固定线宽）。
    Scribble { color: Color32 },
    /// 橡皮擦：按笔画删除。
    Eraser { width: f32 },
}

impl Tool {
    pub fn color(&self) -> Color32 {
        match self {
            Tool::Brush { color, .. } | Tool::Scribble { color } => *color,
            Tool::Eraser { .. } => Color32::TRANSPARENT,
        }
    }

    pub fn width(&self) -> f32 {
        match self {
            Tool::Brush { width, .. } | Tool::Eraser { width } => *width,
            Tool::Scribble { .. } => SCRIBBLE_WIDTH,
        }
    }
}

/// 一笔标注。点序列存逻辑坐标（渲染时无需换算，导出时乘 scale）。
#[derive(Debug, Clone, PartialEq)]
pub struct Stroke {
    pub tool: Tool,
    pub color: Color32,
    pub width: f32,
    pub points: Vec<Pos2>,
}

impl Stroke {
    pub fn new(tool: Tool) -> Self {
        Self {
            tool,
            color: tool.color(),
            width: tool.width(),
            points: Vec::new(),
        }
    }
}
