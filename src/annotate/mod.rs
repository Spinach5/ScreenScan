//! 标注引擎：笔画列表 + 撤销/重做 + 按笔画擦除。

pub mod render;
pub mod stroke;

use egui::Pos2;

pub use stroke::Stroke;
pub use stroke::Tool;
pub use stroke::ToolKind;

/// 一次可撤销的编辑。
enum BoardChange {
    Added(Stroke),
    Erased(Vec<Stroke>),
}

#[derive(Default)]
pub struct AnnotationBoard {
    pub strokes: Vec<Stroke>,
    undo_stack: Vec<BoardChange>,
    redo_stack: Vec<BoardChange>,
}

impl AnnotationBoard {
    /// 提交一笔（画笔/涂鸦）。
    pub fn push(&mut self, stroke: Stroke) {
        self.strokes.push(stroke.clone());
        self.undo_stack.push(BoardChange::Added(stroke));
        self.redo_stack.clear();
    }

    /// 移除与路径相交的笔画并返回之（不记录历史，供橡皮擦实时预览）。
    pub fn remove_intersecting(&mut self, path: &[Pos2], width: f32) -> Vec<Stroke> {
        if path.is_empty() {
            return Vec::new();
        }
        let removed: Vec<Stroke> = self
            .strokes
            .iter()
            .filter(|s| polyline_distance(&s.points, path) <= (width + s.width) / 2.0)
            .cloned()
            .collect();
        if !removed.is_empty() {
            self.strokes.retain(|s| !removed.contains(s));
        }
        removed
    }

    /// 记录一次擦除（撤销时恢复这些笔画）。
    pub fn record_erase(&mut self, removed: Vec<Stroke>) {
        if removed.is_empty() {
            return;
        }
        self.undo_stack.push(BoardChange::Erased(removed));
        self.redo_stack.clear();
    }

    pub fn undo(&mut self) -> bool {
        match self.undo_stack.pop() {
            Some(BoardChange::Added(stroke)) => {
                remove_stroke(&mut self.strokes, &stroke);
                self.redo_stack.push(BoardChange::Added(stroke));
                true
            }
            Some(BoardChange::Erased(removed)) => {
                self.strokes.extend(removed.iter().cloned());
                self.redo_stack.push(BoardChange::Erased(removed));
                true
            }
            None => false,
        }
    }

    pub fn redo(&mut self) -> bool {
        match self.redo_stack.pop() {
            Some(BoardChange::Added(stroke)) => {
                self.strokes.push(stroke.clone());
                self.undo_stack.push(BoardChange::Added(stroke));
                true
            }
            Some(BoardChange::Erased(removed)) => {
                for s in &removed {
                    remove_stroke(&mut self.strokes, s);
                }
                self.undo_stack.push(BoardChange::Erased(removed));
                true
            }
            None => false,
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }
}

fn remove_stroke(strokes: &mut Vec<Stroke>, target: &Stroke) {
    if let Some(pos) = strokes.iter().position(|s| s == target) {
        strokes.remove(pos);
    }
}

/// 两条折线间的最小距离（用于橡皮擦命中判定）。
fn polyline_distance(a: &[Pos2], b: &[Pos2]) -> f32 {
    let sa = to_segments(a);
    let sb = to_segments(b);
    if sa.is_empty() || sb.is_empty() {
        return f32::INFINITY;
    }
    let mut min = f32::INFINITY;
    for (a1, a2) in &sa {
        for (b1, b2) in &sb {
            min = min.min(segment_distance(*a1, *a2, *b1, *b2));
        }
    }
    min
}

fn to_segments(points: &[Pos2]) -> Vec<(Pos2, Pos2)> {
    if points.len() < 2 {
        // 单点视为零长度线段
        points.iter().map(|p| (*p, *p)).collect()
    } else {
        points.windows(2).map(|w| (w[0], w[1])).collect()
    }
}

fn point_segment_distance(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let len2 = ab.length_sq();
    if len2 <= f32::EPSILON {
        return (p - a).length();
    }
    let t = ((p - a).dot(ab) / len2).clamp(0.0, 1.0);
    (p - (a + ab * t)).length()
}

fn segment_distance(a1: Pos2, a2: Pos2, b1: Pos2, b2: Pos2) -> f32 {
    if segments_intersect(a1, a2, b1, b2) {
        return 0.0;
    }
    point_segment_distance(a1, b1, b2)
        .min(point_segment_distance(a2, b1, b2))
        .min(point_segment_distance(b1, a1, a2))
        .min(point_segment_distance(b2, a1, a2))
}

fn orient(a: Pos2, b: Pos2, c: Pos2) -> f32 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

fn segments_intersect(a1: Pos2, a2: Pos2, b1: Pos2, b2: Pos2) -> bool {
    let d1 = orient(b1, b2, a1);
    let d2 = orient(b1, b2, a2);
    let d3 = orient(a1, a2, b1);
    let d4 = orient(a1, a2, b2);
    ((d1 > 0.0 && d2 < 0.0) || (d1 < 0.0 && d2 > 0.0))
        && ((d3 > 0.0 && d4 < 0.0) || (d3 < 0.0 && d4 > 0.0))
}
