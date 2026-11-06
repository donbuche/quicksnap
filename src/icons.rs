//! Simple vector icons drawn with lines and shapes (no artwork is copied).

use egui::{pos2, vec2, Color32, Painter, Pos2, Rect, Shape, Stroke};

#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Icon {
    Select,
    Back,
    Arrow,
    Rect,
    Line,
    Ellipse,
    Pen,
    Text,
    Bubble,
    Blur,
    Crop,
    Rotate,
    Resize,
    Undo,
    Redo,
    // settings tabs
    General,
    Advanced,
    Appearance,
    Hotkeys,
    Fonts,
    // home
    CaptureArea,
    Fullscreen,
    Timer,
    Video,
    Clipboard,
    Folder,
    Image,
}

fn p(r: Rect, x: f32, y: f32) -> Pos2 {
    pos2(r.min.x + x * r.width(), r.min.y + y * r.height())
}

pub fn paint(painter: &Painter, r: Rect, icon: Icon, c: Color32) {
    let w = (r.width() / 12.0).max(1.4);
    let s = Stroke::new(w, c);
    match icon {
        Icon::Select => {
            painter.add(Shape::convex_polygon(
                vec![p(r, 0.3, 0.14), p(r, 0.3, 0.8), p(r, 0.45, 0.65), p(r, 0.72, 0.62)],
                c,
                Stroke::NONE,
            ));
            painter.line_segment([p(r, 0.47, 0.64), p(r, 0.62, 0.9)], Stroke::new(w * 1.6, c));
        }
        Icon::Back => {
            painter.add(Shape::line(vec![p(r, 0.62, 0.2), p(r, 0.32, 0.5), p(r, 0.62, 0.8)], Stroke::new(w * 1.3, c)));
        }
        Icon::Arrow => {
            painter.line_segment([p(r, 0.2, 0.8), p(r, 0.72, 0.28)], s);
            painter.add(Shape::convex_polygon(
                vec![p(r, 0.82, 0.18), p(r, 0.5, 0.26), p(r, 0.74, 0.5)],
                c,
                Stroke::NONE,
            ));
        }
        Icon::Rect => {
            painter.rect_stroke(Rect::from_min_max(p(r, 0.18, 0.25), p(r, 0.82, 0.75)), 1.0, s, egui::StrokeKind::Middle);
        }
        Icon::Line => {
            painter.line_segment([p(r, 0.2, 0.8), p(r, 0.8, 0.2)], s);
        }
        Icon::Ellipse => {
            let c0 = r.center();
            let pts: Vec<Pos2> = (0..40)
                .map(|i| {
                    let t = i as f32 / 40.0 * std::f32::consts::TAU;
                    pos2(c0.x + r.width() * 0.33 * t.cos(), c0.y + r.height() * 0.25 * t.sin())
                })
                .collect();
            painter.add(Shape::closed_line(pts, s));
        }
        Icon::Pen => {
            let pts: Vec<Pos2> = (0..=20)
                .map(|i| {
                    let t = i as f32 / 20.0;
                    p(r, 0.18 + t * 0.64, 0.55 - (t * 6.0).sin() * 0.18)
                })
                .collect();
            painter.add(Shape::line(pts, s));
        }
        Icon::Text => {
            painter.line_segment([p(r, 0.25, 0.22), p(r, 0.75, 0.22)], Stroke::new(w * 1.4, c));
            painter.line_segment([p(r, 0.5, 0.22), p(r, 0.5, 0.8)], Stroke::new(w * 1.4, c));
        }
        Icon::Bubble => {
            painter.rect_stroke(Rect::from_min_max(p(r, 0.18, 0.2), p(r, 0.82, 0.6)), 3.0, s, egui::StrokeKind::Middle);
            painter.add(Shape::line(vec![p(r, 0.35, 0.6), p(r, 0.25, 0.82), p(r, 0.5, 0.6)], s));
        }
        Icon::Blur => {
            for i in 0..3 {
                for j in 0..3 {
                    let a = if (i + j) % 2 == 0 { 255 } else { 110 };
                    let cc = Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a);
                    let cell = Rect::from_min_size(
                        p(r, 0.2 + i as f32 * 0.2, 0.2 + j as f32 * 0.2),
                        vec2(r.width() * 0.19, r.height() * 0.19),
                    );
                    painter.rect_filled(cell, 0.0, cc);
                }
            }
        }
        Icon::Crop => {
            painter.add(Shape::line(vec![p(r, 0.3, 0.12), p(r, 0.3, 0.7), p(r, 0.88, 0.7)], s));
            painter.add(Shape::line(vec![p(r, 0.12, 0.3), p(r, 0.7, 0.3), p(r, 0.7, 0.88)], s));
        }
        Icon::Rotate | Icon::Undo | Icon::Redo => {
            let c0 = r.center();
            let rad = r.width() * 0.28;
            let (a0, a1, flip) = match icon {
                Icon::Rotate => (-80.0f32, 200.0f32, false),
                Icon::Undo => (200.0, 340.0, true),
                _ => (200.0, 340.0, false),
            };
            let pts: Vec<Pos2> = (0..=24)
                .map(|i| {
                    let a = (a0 + (a1 - a0) * i as f32 / 24.0).to_radians();
                    let x = if flip { -a.cos() } else { a.cos() };
                    pos2(c0.x + rad * x, c0.y + rad * a.sin() + if icon == Icon::Rotate { 0.0 } else { rad * 0.6 })
                })
                .collect();
            let end = *pts.last().unwrap();
            let prev = pts[pts.len() - 3];
            painter.add(Shape::line(pts, s));
            let d = (end - prev).normalized();
            let n = vec2(-d.y, d.x);
            let hs = r.width() * 0.14;
            painter.add(Shape::convex_polygon(
                vec![end + d * hs, end - d * hs * 0.4 + n * hs, end - d * hs * 0.4 - n * hs],
                c,
                Stroke::NONE,
            ));
        }
        Icon::Resize => {
            painter.rect_stroke(Rect::from_min_max(p(r, 0.15, 0.4), p(r, 0.6, 0.85)), 1.0, s, egui::StrokeKind::Middle);
            painter.line_segment([p(r, 0.45, 0.55), p(r, 0.85, 0.15)], s);
            painter.add(Shape::line(vec![p(r, 0.6, 0.15), p(r, 0.85, 0.15), p(r, 0.85, 0.4)], s));
        }
        Icon::General => {
            // a switch
            let track = Rect::from_min_max(p(r, 0.12, 0.32), p(r, 0.88, 0.68));
            painter.rect_stroke(track, track.height() / 2.0, s, egui::StrokeKind::Middle);
            painter.circle_filled(p(r, 0.68, 0.5), track.height() * 0.32, c);
        }
        Icon::Advanced => {
            for (i, k) in [0.3f32, 0.7, 0.45].iter().enumerate() {
                let y = 0.25 + i as f32 * 0.25;
                painter.line_segment([p(r, 0.12, y), p(r, 0.88, y)], s);
                painter.circle_filled(p(r, 0.12 + 0.76 * k, y), r.width() * 0.08, c);
            }
        }
        Icon::Appearance => {
            let c0 = r.center();
            let pts_top: Vec<Pos2> = (0..=20)
                .map(|i| {
                    let t = i as f32 / 20.0;
                    pos2(r.min.x + r.width() * (0.1 + 0.8 * t), c0.y - (t * std::f32::consts::PI).sin() * r.height() * 0.28)
                })
                .collect();
            let pts_bot: Vec<Pos2> = pts_top.iter().map(|q| pos2(q.x, 2.0 * c0.y - q.y)).collect();
            painter.add(Shape::line(pts_top, s));
            painter.add(Shape::line(pts_bot, s));
            painter.circle_filled(c0, r.width() * 0.13, c);
        }
        Icon::Hotkeys => {
            let kb = Rect::from_min_max(p(r, 0.08, 0.25), p(r, 0.92, 0.78));
            painter.rect_stroke(kb, 3.0, s, egui::StrokeKind::Middle);
            for row in 0..2 {
                for col in 0..4 {
                    let q = p(r, 0.22 + col as f32 * 0.19, 0.4 + row as f32 * 0.15);
                    painter.rect_filled(Rect::from_center_size(q, vec2(r.width() * 0.09, r.height() * 0.07)), 0.0, c);
                }
            }
            painter.line_segment([p(r, 0.3, 0.68), p(r, 0.7, 0.68)], s);
        }
        Icon::Fonts => {
            painter.text(r.center(), egui::Align2::CENTER_CENTER, "Aa", egui::FontId::proportional(r.height() * 0.62), c);
        }
        Icon::CaptureArea => {
            let rr = Rect::from_min_max(p(r, 0.15, 0.15), p(r, 0.85, 0.85));
            let dash = Stroke::new(w, c);
            for (a, b) in [
                (rr.left_top(), rr.right_top()),
                (rr.right_top(), rr.right_bottom()),
                (rr.right_bottom(), rr.left_bottom()),
                (rr.left_bottom(), rr.left_top()),
            ] {
                painter.add(Shape::dashed_line(&[a, b], dash, w * 2.0, w * 1.6));
            }
            painter.line_segment([p(r, 0.5, 0.35), p(r, 0.5, 0.65)], s);
            painter.line_segment([p(r, 0.35, 0.5), p(r, 0.65, 0.5)], s);
        }
        Icon::Fullscreen => {
            painter.rect_stroke(Rect::from_min_max(p(r, 0.1, 0.2), p(r, 0.9, 0.72)), 2.0, s, egui::StrokeKind::Middle);
            painter.line_segment([p(r, 0.5, 0.72), p(r, 0.5, 0.85)], s);
            painter.line_segment([p(r, 0.3, 0.85), p(r, 0.7, 0.85)], s);
        }
        Icon::Timer => {
            painter.circle_stroke(p(r, 0.5, 0.55), r.width() * 0.32, s);
            painter.line_segment([p(r, 0.5, 0.55), p(r, 0.5, 0.35)], s);
            painter.line_segment([p(r, 0.5, 0.55), p(r, 0.64, 0.62)], s);
            painter.line_segment([p(r, 0.42, 0.12), p(r, 0.58, 0.12)], s);
        }
        Icon::Video => {
            painter.rect_stroke(Rect::from_min_max(p(r, 0.1, 0.28), p(r, 0.66, 0.72)), 2.0, s, egui::StrokeKind::Middle);
            painter.add(Shape::convex_polygon(vec![p(r, 0.68, 0.5), p(r, 0.9, 0.3), p(r, 0.9, 0.7)], c, Stroke::NONE));
        }
        Icon::Clipboard => {
            painter.rect_stroke(Rect::from_min_max(p(r, 0.2, 0.18), p(r, 0.8, 0.88)), 2.0, s, egui::StrokeKind::Middle);
            painter.rect_filled(Rect::from_min_max(p(r, 0.36, 0.1), p(r, 0.64, 0.24)), 1.0, c);
        }
        Icon::Folder => {
            painter.add(Shape::closed_line(
                vec![p(r, 0.1, 0.25), p(r, 0.4, 0.25), p(r, 0.47, 0.35), p(r, 0.9, 0.35), p(r, 0.9, 0.8), p(r, 0.1, 0.8)],
                s,
            ));
        }
        Icon::Image => {
            painter.rect_stroke(Rect::from_min_max(p(r, 0.12, 0.2), p(r, 0.88, 0.8)), 2.0, s, egui::StrokeKind::Middle);
            painter.add(Shape::line(vec![p(r, 0.16, 0.72), p(r, 0.4, 0.45), p(r, 0.58, 0.62), p(r, 0.7, 0.52), p(r, 0.84, 0.7)], s));
            painter.circle_filled(p(r, 0.66, 0.36), r.width() * 0.06, c);
        }
    }
}
