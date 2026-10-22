//! 标注渲染：egui Painter 实时预览 + tiny-skia 导出光栅化。

use egui::Color32;
use egui::Painter;
use egui::Pos2;
use egui::Rect;
use egui::Stroke as EguiStroke;
use image::RgbaImage;
use tiny_skia::FillRule;
use tiny_skia::IntSize;
use tiny_skia::LineCap;
use tiny_skia::LineJoin;
use tiny_skia::Paint;
use tiny_skia::PathBuilder;
use tiny_skia::Pixmap;
use tiny_skia::Transform;

use super::Stroke;
use super::Tool;

/// 实时预览：把笔画画到 egui Painter，并裁剪在选区矩形内。
pub fn draw_strokes(painter: &Painter, strokes: &[Stroke], clip: Rect) {
    let painter = painter.with_clip_rect(clip);
    for stroke in strokes {
        draw_one(&painter, stroke);
    }
}

fn draw_one(painter: &Painter, stroke: &Stroke) {
    if matches!(stroke.tool, Tool::Eraser { .. }) || stroke.points.is_empty() {
        return;
    }
    if stroke.points.len() == 1 {
        painter.circle_filled(stroke.points[0], stroke.width / 2.0, stroke.color);
        return;
    }
    for w in stroke.points.windows(2) {
        painter.line_segment([w[0], w[1]], EguiStroke::new(stroke.width, stroke.color));
    }
    // 圆头衔接：在每个顶点补圆点
    for p in &stroke.points {
        painter.circle_filled(*p, stroke.width / 2.0, stroke.color);
    }
}

/// 导出合成：把笔画光栅化到已裁剪的选区图上。
///
/// `selection_min` 为裁剪区域左上角的物理像素坐标；笔画点为逻辑坐标，
/// 换算物理后减去该原点即得到裁剪图内坐标。
pub fn compose(base: &RgbaImage, strokes: &[Stroke], selection_min: Pos2, scale: f32) -> RgbaImage {
    let (w, h) = base.dimensions();
    let Some(size) = IntSize::from_wh(w, h) else {
        return base.clone();
    };
    let Some(mut pixmap) = Pixmap::from_vec(base.as_raw().clone(), size) else {
        return base.clone();
    };

    for stroke in strokes {
        rasterize_stroke(&mut pixmap, stroke, selection_min, scale);
    }

    match RgbaImage::from_raw(w, h, pixmap.data().to_vec()) {
        Some(img) => img,
        None => base.clone(),
    }
}

fn rasterize_stroke(pixmap: &mut Pixmap, stroke: &Stroke, origin: Pos2, scale: f32) {
    if matches!(stroke.tool, Tool::Eraser { .. }) || stroke.points.is_empty() {
        return;
    }

    let to_px = |p: Pos2| (p.x * scale - origin.x, p.y * scale - origin.y);

    let mut paint = Paint::default();
    paint.set_color_rgba8(
        stroke.color.r(),
        stroke.color.g(),
        stroke.color.b(),
        stroke.color.a(),
    );
    paint.anti_alias = true;

    if stroke.points.len() == 1 {
        let (x, y) = to_px(stroke.points[0]);
        let mut builder = PathBuilder::new();
        builder.push_circle(x, y, (stroke.width * scale / 2.0).max(0.5));
        if let Some(path) = builder.finish() {
            pixmap.fill_path(
                &path,
                &paint,
                FillRule::Winding,
                Transform::identity(),
                None,
            );
        }
        return;
    }

    let mut builder = PathBuilder::new();
    let (x0, y0) = to_px(stroke.points[0]);
    builder.move_to(x0, y0);
    for p in &stroke.points[1..] {
        let (x, y) = to_px(*p);
        builder.line_to(x, y);
    }

    let Some(path) = builder.finish() else {
        return;
    };

    let stroke_style = tiny_skia::Stroke {
        width: (stroke.width * scale).max(1.0),
        line_cap: LineCap::Round,
        line_join: LineJoin::Round,
        ..Default::default()
    };

    pixmap.stroke_path(&path, &paint, &stroke_style, Transform::identity(), None);
}

/// 供 UI 使用的默认颜色常量。
pub const DEFAULT_COLOR: Color32 = Color32::from_rgb(255, 60, 60);
