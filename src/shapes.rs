//! Annotation shapes. Each shape is turned into simple drawing primitives in
//! image-pixel space; the same primitives are drawn on screen (egui) and baked
//! into the exported picture (tiny-skia + ab_glyph), so both always match.

use ab_glyph::{Font, FontArc, PxScale, ScaleFont};
use egui::{pos2, vec2, Color32, Pos2, Rect, Vec2};
use image::RgbaImage;
use serde::{Deserialize, Serialize};

/// How text is drawn: plain colour, colour with a contrasting outline, or
/// contrasting text on a rounded box of the colour.
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum TextStyle {
    Plain,
    Outline,
    Box,
}

/// text_size_for_width(6.0): the default line width 6 gives 20 px.
pub const TEXT_SIZE_DEFAULT: f32 = 20.0;
pub const TEXT_SIZE_MIN: f32 = 10.0;
pub const TEXT_SIZE_MAX: f32 = 120.0;

#[derive(Clone, Debug)]
pub struct Shape {
    pub kind: Kind,
    pub color: Color32,
    pub width: f32,
}

#[derive(Clone, Debug)]
pub enum Kind {
    Arrow { a: Pos2, b: Pos2 },
    Line { a: Pos2, b: Pos2 },
    Rect { r: Rect, filled: bool, darken: bool },
    Ellipse { r: Rect, filled: bool },
    Pen { pts: Vec<Pos2>, highlighter: bool },
    Text { pos: Pos2, text: String, size: f32, style: TextStyle },
    Bubble { tail: Pos2, pos: Pos2, text: String, size: f32 },
    /// Hides what is underneath (blurs the picture inside `r`).
    Blur { r: Rect },
}

#[derive(Clone, Debug)]
pub enum Prim {
    Stroke { pts: Vec<Pos2>, width: f32, color: Color32, closed: bool },
    Fill { pts: Vec<Pos2>, color: Color32 },
    /// A filled polygon that may be concave but is star-shaped from pts[0].
    Poly { pts: Vec<Pos2>, color: Color32 },
    Text { pos: Pos2, text: String, size: f32, color: Color32 },
    DarkenOutside { r: Rect },
    /// The picture inside `r`, strongly blurred (see `redact_small`).
    Blur { r: Rect },
}


pub const SHADOW_ALPHA: u8 = 80;

fn shadow_offset(width: f32) -> Vec2 {
    let d = 1.0 + width * 0.25;
    vec2(d, d)
}

/// The font used for text annotations, with measuring helpers.
#[derive(Clone)]
pub struct AnnotFont {
    pub font: FontArc,
    pub bytes: std::sync::Arc<Vec<u8>>,
}

impl AnnotFont {
    pub fn load(path: Option<&str>) -> AnnotFont {
        let candidates: Vec<String> = path
            .map(|p| vec![p.to_string()])
            .unwrap_or_default()
            .into_iter()
            .chain([
                "/System/Library/Fonts/Supplemental/Arial Bold.ttf".to_string(),
                "/Library/Fonts/Arial Bold.ttf".to_string(),
            ])
            .collect();
        for c in candidates {
            if let Ok(b) = std::fs::read(&c) {
                if let Ok(f) = FontArc::try_from_vec(b.clone()) {
                    return AnnotFont { font: f, bytes: std::sync::Arc::new(b) };
                }
            }
        }
        let b = epaint_default_fonts::UBUNTU_LIGHT.to_vec();
        AnnotFont { font: FontArc::try_from_vec(b.clone()).expect("built-in font"), bytes: std::sync::Arc::new(b) }
    }

    /// egui sizes text by the em square while ab_glyph sizes it by
    /// ascent − descent; this ratio lets both draw the same size.
    pub fn em_ratio(&self) -> f32 {
        let upem = self.font.units_per_em().unwrap_or(1000.0);
        (self.font.height_unscaled() / upem).max(0.1)
    }

    pub fn line_height(&self, size: f32) -> f32 {
        let sf = self.font.as_scaled(PxScale::from(size));
        sf.ascent() - sf.descent() + sf.line_gap()
    }

    pub fn measure(&self, text: &str, size: f32) -> Vec2 {
        let sf = self.font.as_scaled(PxScale::from(size));
        let mut w: f32 = 0.0;
        let mut lines = 0;
        for line in text.split('\n') {
            lines += 1;
            let mut x = 0.0;
            let mut prev = None;
            for ch in line.chars() {
                let id = sf.glyph_id(ch);
                if let Some(p) = prev {
                    x += sf.kern(p, id);
                }
                x += sf.h_advance(id);
                prev = Some(id);
            }
            w = w.max(x);
        }
        vec2(w, lines.max(1) as f32 * self.line_height(size))
    }
}

fn ellipse_points(r: Rect, n: usize) -> Vec<Pos2> {
    let c = r.center();
    let (rx, ry) = (r.width() / 2.0, r.height() / 2.0);
    (0..n)
        .map(|i| {
            let t = i as f32 / n as f32 * std::f32::consts::TAU;
            pos2(c.x + rx * t.cos(), c.y + ry * t.sin())
        })
        .collect()
}

fn rect_points(r: Rect) -> Vec<Pos2> {
    vec![r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom()]
}

fn rounded_rect_points(r: Rect, radius: f32) -> Vec<Pos2> {
    let rad = radius.min(r.width() / 2.0).min(r.height() / 2.0);
    let mut pts = Vec::new();
    let corners = [
        (pos2(r.max.x - rad, r.min.y + rad), -90.0f32),
        (pos2(r.max.x - rad, r.max.y - rad), 0.0),
        (pos2(r.min.x + rad, r.max.y - rad), 90.0),
        (pos2(r.min.x + rad, r.min.y + rad), 180.0),
    ];
    for (c, start) in corners {
        for i in 0..=6 {
            let a = (start + i as f32 * 15.0).to_radians();
            pts.push(pos2(c.x + rad * a.cos(), c.y + rad * a.sin()));
        }
    }
    pts
}

/// Classic arrow: a straight shaft as thick as the line width ending in a
/// triangular head whose barbs sweep slightly back past the shaft.
/// Proportions, relative to the head length H: half-width 0.53 H, barbs
/// reach 0.14 H further back than where the shaft joins. H is 3 × width
/// (at least 10 px); short arrows scale the head down so the shaft shows.
pub const ARROW_MIN_LEN: f32 = 2.0;
pub const ARROW_HEAD_LEN: f32 = 3.0;
pub const ARROW_HEAD_MIN: f32 = 10.0;
pub const ARROW_HEAD_HALF: f32 = 0.53;
pub const ARROW_NOTCH: f32 = 0.14;

fn arrow_prims(a: Pos2, b: Pos2, w: f32, color: Color32) -> Vec<Prim> {
    let d = b - a;
    let len = d.length();
    if len < ARROW_MIN_LEN {
        return vec![];
    }
    let dir = d / len;
    let n = vec2(-dir.y, dir.x);
    let mut head = (ARROW_HEAD_LEN * w).max(ARROW_HEAD_MIN);
    let mut shaft = w / 2.0;
    // keep a bit of shaft visible on short arrows
    let room = len / 1.2;
    if head > room {
        let f = room / head;
        head *= f;
        shaft *= f.max(0.5);
    }
    let half = head * ARROW_HEAD_HALF;
    let join = head * (1.0 - ARROW_NOTCH);
    let at = |back: f32, side: f32| b - dir * back + n * side;
    // one outline, star-shaped from the tip
    let pts = vec![b, at(head, half), at(join, shaft), a + n * shaft, a - n * shaft, at(join, -shaft), at(head, -half)];
    vec![Prim::Poly { pts, color }]
}

pub fn is_light(c: Color32) -> bool {
    (c.r() as u32 * 299 + c.g() as u32 * 587 + c.b() as u32 * 114) / 1000 > 170
}

/// Text colour that reads well on top of `c` (used for the Box style).
pub fn contrast(c: Color32) -> Color32 {
    if is_light(c) { Color32::from_rgb(0x15, 0x15, 0x18) } else { Color32::WHITE }
}

/// Outline colour rule from the original's TextRenderer (0x10079a1a8):
/// black when red + green + blue (0–1 each) is above 2.82353, else white.
pub const OUTLINE_LIGHT_SUM: f32 = 2.82353;
pub fn outline_color(c: Color32) -> Color32 {
    let sum = (c.r() as f32 + c.g() as f32 + c.b() as f32) / 255.0;
    if sum > OUTLINE_LIGHT_SUM { Color32::BLACK } else { Color32::WHITE }
}

/// Outline thickness from the original (0x10079a9a4): max(1, width / 2.6).
pub const OUTLINE_DIV: f32 = 2.6;
pub fn outline_width(width: f32) -> f32 {
    (width / OUTLINE_DIV).max(1.0)
}

/// Font size from line width, from the original (0x10079a984, 0x10079add0,
/// 0x10079ae78): round(3.5 × width), at least 4, rounded down to even.
pub const TEXT_PER_WIDTH: f32 = 3.5;
pub const TEXT_MIN_SIZE: f32 = 4.0;
pub fn text_size_for_width(width: f32) -> f32 {
    let s = (TEXT_PER_WIDTH * width).round().max(TEXT_MIN_SIZE) as u32;
    (s & !1) as f32
}

/// Where the arrow of "Text with arrow" meets the text, from the original
/// (0x10076fd84): from the text's centre, 1.3 × the half-size towards the
/// pointed spot, on the axis (up / down / left / right) facing it.
pub const BUBBLE_ATTACH: f32 = 1.3;
pub fn bubble_attach(r: Rect, target: Pos2) -> Pos2 {
    let c = r.center();
    let (dx, dy) = (target.x - c.x, target.y - c.y);
    if dy >= 0.0 && dy >= dx.abs() {
        pos2(c.x, c.y + BUBBLE_ATTACH * r.height() / 2.0)
    } else if dy <= -dx.abs() {
        pos2(c.x, c.y - BUBBLE_ATTACH * r.height() / 2.0)
    } else if dx > 0.0 {
        pos2(c.x + BUBBLE_ATTACH * r.width() / 2.0, c.y)
    } else {
        pos2(c.x - BUBBLE_ATTACH * r.width() / 2.0, c.y)
    }
}

fn outlined_text(pos: Pos2, text: &str, size: f32, c: Color32, width: f32) -> Vec<Prim> {
    let o = outline_width(width);
    let oc = outline_color(c);
    let mut v: Vec<Prim> = (0..12)
        .map(|k| {
            let a = k as f32 / 12.0 * std::f32::consts::TAU;
            Prim::Text { pos: pos + vec2(a.cos(), a.sin()) * o, text: text.to_string(), size, color: oc }
        })
        .collect();
    v.push(Prim::Text { pos, text: text.to_string(), size, color: c });
    v
}

/// Rectangle covered by a text's letters (an empty text still gets a caret-sized box).
pub fn text_rect(font: &AnnotFont, pos: Pos2, text: &str, size: f32) -> Rect {
    let t = if text.is_empty() { " " } else { text };
    let m = font.measure(t, size);
    Rect::from_min_size(pos, vec2(m.x.max(size * 0.3), m.y))
}

fn seg_dist(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let t = if ab.length_sq() > 0.0 { ((p - a).dot(ab) / ab.length_sq()).clamp(0.0, 1.0) } else { 0.0 };
    p.distance(a + ab * t)
}

impl Shape {
    pub fn is_text(&self) -> bool {
        matches!(self.kind, Kind::Text { .. } | Kind::Bubble { .. })
    }

    pub fn text_size(&self) -> Option<f32> {
        match &self.kind {
            Kind::Text { size, .. } | Kind::Bubble { size, .. } => Some(*size),
            _ => None,
        }
    }

    /// Where the first letter of the text starts.
    pub fn text_origin(&self) -> Option<Pos2> {
        match &self.kind {
            Kind::Text { pos, .. } => Some(*pos),
            Kind::Bubble { pos, .. } => Some(*pos),
            _ => None,
        }
    }

    /// The colour the letters are drawn in.
    pub fn text_color(&self) -> Color32 {
        match &self.kind {
            Kind::Text { style: TextStyle::Box, .. } => contrast(self.color),
            _ => self.color,
        }
    }

    pub fn text_mut(&mut self) -> Option<&mut String> {
        match &mut self.kind {
            Kind::Text { text, .. } | Kind::Bubble { text, .. } => Some(text),
            _ => None,
        }
    }

    /// Bounding box in picture pixels.
    pub fn bounds(&self, font: &AnnotFont) -> Rect {
        let pad = self.width / 2.0 + 2.0;
        match &self.kind {
            Kind::Arrow { a, b } | Kind::Line { a, b } => Rect::from_two_pos(*a, *b).expand(pad + self.width),
            Kind::Rect { r, .. } | Kind::Ellipse { r, .. } => r.expand(pad),
            Kind::Blur { r } => *r,
            Kind::Pen { pts, highlighter } => {
                let w = if *highlighter { self.width * 3.0 } else { self.width };
                Rect::from_points(pts).expand(w / 2.0 + 2.0)
            }
            Kind::Text { pos, text, size, style } => {
                let r = text_rect(font, *pos, text, *size);
                if *style == TextStyle::Box { r.expand(size * 0.3) } else { r.expand(2.0) }
            }
            Kind::Bubble { tail, pos, text, size } => {
                text_rect(font, *pos, text, *size).expand(outline_width(self.width) + 2.0).union(Rect::from_min_max(*tail, *tail).expand(pad))
            }
        }
    }

    /// Is `p` on this shape? `tol` is the grab distance in picture pixels.
    pub fn hit(&self, p: Pos2, tol: f32, font: &AnnotFont) -> bool {
        let w = self.width / 2.0 + tol;
        match &self.kind {
            Kind::Arrow { a, b } | Kind::Line { a, b } => seg_dist(p, *a, *b) <= w + self.width,
            Kind::Rect { r, filled, .. } => {
                if *filled { r.expand(tol).contains(p) } else { r.expand(w).contains(p) && !r.shrink(w).contains(p) }
            }
            Kind::Ellipse { r, filled } => {
                let c = r.center();
                let (rx, ry) = (r.width() / 2.0, r.height() / 2.0);
                if rx < 1.0 || ry < 1.0 {
                    return r.expand(w).contains(p);
                }
                let d = (((p.x - c.x) / rx).powi(2) + ((p.y - c.y) / ry).powi(2)).sqrt();
                if *filled { d <= 1.0 + tol / rx.min(ry) } else { (d - 1.0).abs() * rx.min(ry) <= w }
            }
            Kind::Pen { pts, highlighter } => {
                let ww = if *highlighter { self.width * 1.5 + tol } else { w };
                pts.windows(2).any(|s| seg_dist(p, s[0], s[1]) <= ww) || pts.iter().any(|q| q.distance(p) <= ww)
            }
            Kind::Text { .. } => self.bounds(font).expand(tol * 0.5).contains(p),
            Kind::Blur { r } => r.expand(tol * 0.5).contains(p),
            Kind::Bubble { tail, pos, text, size } => {
                let r = text_rect(font, *pos, text, *size);
                r.expand(tol * 0.5).contains(p) || seg_dist(p, bubble_attach(r, *tail), *tail) <= w + self.width
            }
        }
    }

    /// Whether the selection shows a frame around the item. Lines and arrows
    /// only show their two end points.
    pub fn shows_frame(&self) -> bool {
        !matches!(self.kind, Kind::Arrow { .. } | Kind::Line { .. })
    }

    /// The grab points of a selected item, in picture pixels. `frame` is the
    /// selection frame around it.
    pub fn handles(&self, frame: Rect) -> Vec<(Handle, Pos2)> {
        let at_corners = |r: Rect| corners(r).into_iter().enumerate().map(|(i, c)| (Handle::Corner(i), c)).collect();
        match &self.kind {
            Kind::Arrow { a, b } | Kind::Line { a, b } => vec![(Handle::Start, *a), (Handle::End, *b)],
            Kind::Bubble { tail, .. } => vec![(Handle::End, *tail)],
            Kind::Rect { r, .. } | Kind::Ellipse { r, .. } | Kind::Blur { r } => at_corners(*r),
            _ => at_corners(frame),
        }
    }

    /// Reshapes `orig` (the item as it was when the drag began) by dragging
    /// handle `h` by `delta`. `frame` is the selection frame at that moment;
    /// `uniform` (⇧) keeps the proportions or snaps lines to 45°.
    pub fn drag_handle(&mut self, orig: &Shape, h: Handle, frame: Rect, delta: Vec2, uniform: bool) {
        match (&mut self.kind, &orig.kind, h) {
            (Kind::Arrow { a, b } | Kind::Line { a, b }, Kind::Arrow { a: oa, b: ob } | Kind::Line { a: oa, b: ob }, _) => {
                if h == Handle::Start {
                    let p = *oa + delta;
                    *a = if uniform { snap45(*ob, p) } else { p };
                } else {
                    let p = *ob + delta;
                    *b = if uniform { snap45(*oa, p) } else { p };
                }
            }
            (Kind::Bubble { tail, .. }, Kind::Bubble { tail: ot, pos, .. }, _) => {
                let p = *ot + delta;
                *tail = if uniform { snap45(*pos, p) } else { p };
            }
            (
                Kind::Rect { r, .. } | Kind::Ellipse { r, .. } | Kind::Blur { r },
                Kind::Rect { r: or, .. } | Kind::Ellipse { r: or, .. } | Kind::Blur { r: or },
                Handle::Corner(i),
            ) => {
                let c = corners(*or);
                let anchor = c[3 - i];
                let p = c[i] + delta;
                *r = Rect::from_two_pos(anchor, if uniform { square(anchor, p) } else { p });
            }
            (Kind::Pen { pts, .. }, Kind::Pen { pts: opts, .. }, Handle::Corner(i)) => {
                let (anchor, sx, sy) = corner_scale(frame, i, delta, uniform);
                *pts = opts.iter().map(|q| scale_about(*q, anchor, sx, sy)).collect();
            }
            (Kind::Text { pos, size, .. }, Kind::Text { pos: op, size: os, .. }, Handle::Corner(i)) => {
                // texts grow and shrink as a whole: the font size follows
                let (anchor, k, _) = corner_scale(frame, i, delta, true);
                *size = (os * k).clamp(TEXT_SIZE_MIN, TEXT_SIZE_MAX);
                let k = *size / os;
                *pos = scale_about(*op, anchor, k, k);
            }
            _ => {}
        }
    }

    pub fn translate(&mut self, d: Vec2) {
        match &mut self.kind {
            Kind::Arrow { a, b } | Kind::Line { a, b } => {
                *a += d;
                *b += d;
            }
            Kind::Rect { r, .. } | Kind::Ellipse { r, .. } | Kind::Blur { r } => *r = r.translate(d),
            Kind::Pen { pts, .. } => pts.iter_mut().for_each(|p| *p += d),
            Kind::Text { pos, .. } => *pos += d,
            Kind::Bubble { tail, pos, .. } => {
                *tail += d;
                *pos += d;
            }
        }
    }

    /// What to draw, with a soft shadow under it when `shadow` is on.
    pub fn prims(&self, font: &AnnotFont, shadow: bool) -> Vec<Prim> {
        let mut out = Vec::new();
        let body = self.body_prims(font);
        if shadow {
            let off = shadow_offset(self.width);
            let shade = Color32::from_rgba_unmultiplied(0, 0, 0, SHADOW_ALPHA);
            for p in &body {
                match p {
                    Prim::Stroke { pts, width, closed, .. } => out.push(Prim::Stroke {
                        pts: pts.iter().map(|q| *q + off).collect(),
                        width: *width,
                        color: shade,
                        closed: *closed,
                    }),
                    Prim::Fill { pts, .. } => {
                        out.push(Prim::Fill { pts: pts.iter().map(|q| *q + off).collect(), color: shade })
                    }
                    Prim::Poly { pts, .. } => {
                        out.push(Prim::Poly { pts: pts.iter().map(|q| *q + off).collect(), color: shade })
                    }
                    Prim::Text { pos, text, size, .. } => {
                        if !matches!(self.kind, Kind::Text { style: TextStyle::Outline, .. } | Kind::Bubble { .. }) {
                            out.push(Prim::Text { pos: *pos + off * 0.6, text: text.clone(), size: *size, color: shade })
                        }
                    }
                    Prim::DarkenOutside { .. } | Prim::Blur { .. } => {}
                }
            }
        }
        out.extend(body);
        out
    }

    fn body_prims(&self, font: &AnnotFont) -> Vec<Prim> {
        let c = self.color;
        let w = self.width;
        match &self.kind {
            Kind::Blur { r } => vec![Prim::Blur { r: *r }],
            Kind::Arrow { a, b } => arrow_prims(*a, *b, w, c),
            Kind::Line { a, b } => vec![Prim::Stroke { pts: vec![*a, *b], width: w, color: c, closed: false }],
            Kind::Rect { r, filled, darken } => {
                let mut v = Vec::new();
                if *darken {
                    v.push(Prim::DarkenOutside { r: *r });
                }
                if *filled {
                    v.push(Prim::Fill { pts: rect_points(*r), color: c });
                } else {
                    v.push(Prim::Stroke { pts: rect_points(*r), width: w, color: c, closed: true });
                }
                v
            }
            Kind::Ellipse { r, filled } => {
                let pts = ellipse_points(*r, 72);
                if *filled {
                    vec![Prim::Fill { pts, color: c }]
                } else {
                    vec![Prim::Stroke { pts, width: w, color: c, closed: true }]
                }
            }
            Kind::Pen { pts, highlighter } => {
                if *highlighter {
                    vec![Prim::Stroke {
                        pts: pts.clone(),
                        width: w * 3.0,
                        color: Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), 100),
                        closed: false,
                    }]
                } else {
                    vec![Prim::Stroke { pts: pts.clone(), width: w, color: c, closed: false }]
                }
            }
            Kind::Text { pos, text, size, style } => {
                let size = *size;
                match style {
                    TextStyle::Plain => vec![Prim::Text { pos: *pos, text: text.clone(), size, color: c }],
                    TextStyle::Outline => outlined_text(*pos, text, size, c, w),
                    TextStyle::Box => {
                        let r = text_rect(font, *pos, text, size).expand(size * 0.3);
                        vec![
                            Prim::Fill { pts: rounded_rect_points(r, size * 0.3), color: c },
                            Prim::Text { pos: *pos, text: text.clone(), size, color: contrast(c) },
                        ]
                    }
                }
            }
            Kind::Bubble { tail, pos, text, size } => {
                // "Text with arrow" in the original is an arrow plus a text
                // (picasso::Bubble holds an arrow_ and a text_).
                let r = text_rect(font, *pos, text, *size);
                let start = bubble_attach(r, *tail);
                let mut v = arrow_prims(start, *tail, w, c);
                v.extend(outlined_text(*pos, text, *size, c, w));
                v
            }
        }
    }
}

/// A grab point on a selected item.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Handle {
    Start,
    End,
    /// 0 top-left, 1 top-right, 2 bottom-left, 3 bottom-right
    Corner(usize),
}

/// Corners in `Handle::Corner` order; corner `i` faces corner `3 - i`.
pub fn corners(r: Rect) -> [Pos2; 4] {
    [r.left_top(), r.right_top(), r.left_bottom(), r.right_bottom()]
}

/// Scale factors for dragging corner `i` of `frame` by `delta` with the
/// opposite corner fixed. Never flips or collapses the item.
fn corner_scale(frame: Rect, i: usize, delta: Vec2, uniform: bool) -> (Pos2, f32, f32) {
    let c = corners(frame);
    let anchor = c[3 - i];
    let o = c[i] - anchor;
    let n = o + delta;
    let f = |n: f32, o: f32| if o.abs() > 1e-3 { (n / o).max(0.05) } else { 1.0 };
    let (sx, sy) = (f(n.x, o.x), f(n.y, o.y));
    if uniform {
        let s = (sx + sy) / 2.0;
        (anchor, s, s)
    } else {
        (anchor, sx, sy)
    }
}

fn scale_about(p: Pos2, anchor: Pos2, sx: f32, sy: f32) -> Pos2 {
    pos2(anchor.x + (p.x - anchor.x) * sx, anchor.y + (p.y - anchor.y) * sy)
}

/// Constrain a drag end point to 45° steps (⇧ for lines and arrows).
pub fn snap45(a: Pos2, b: Pos2) -> Pos2 {
    let d = b - a;
    let len = d.length();
    let ang = d.y.atan2(d.x);
    let step = std::f32::consts::FRAC_PI_4;
    let s = (ang / step).round() * step;
    a + vec2(s.cos(), s.sin()) * len
}

/// Make a drag rectangle square (⇧ for rectangle, ellipse, blur, crop).
pub fn square(a: Pos2, b: Pos2) -> Pos2 {
    let d = b - a;
    let s = d.x.abs().max(d.y.abs());
    a + vec2(s * d.x.signum(), s * d.y.signum())
}

// ---------------- baking into pixels ----------------

fn to_skia_color(c: Color32) -> tiny_skia::Color {
    let [r, g, b, a] = c.to_srgba_unmultiplied();
    tiny_skia::Color::from_rgba8(r, g, b, a)
}

fn path_of(pts: &[Pos2], closed: bool) -> Option<tiny_skia::Path> {
    let mut pb = tiny_skia::PathBuilder::new();
    let first = pts.first()?;
    pb.move_to(first.x, first.y);
    if pts.len() == 1 {
        pb.line_to(first.x + 0.01, first.y);
    }
    for p in &pts[1..] {
        pb.line_to(p.x, p.y);
    }
    if closed {
        pb.close();
    }
    pb.finish()
}

fn blend(px: &mut tiny_skia::Pixmap, x: i32, y: i32, c: Color32, coverage: f32) {
    if x < 0 || y < 0 || x >= px.width() as i32 || y >= px.height() as i32 {
        return;
    }
    let [r, g, b, a] = c.to_srgba_unmultiplied();
    let sa = (a as f32 / 255.0) * coverage.clamp(0.0, 1.0);
    let idx = (y as usize * px.width() as usize + x as usize) * 4;
    let d = px.data_mut();
    for (i, sc) in [r, g, b].iter().enumerate() {
        let v = *sc as f32 * sa + d[idx + i] as f32 * (1.0 - sa);
        d[idx + i] = v.round().clamp(0.0, 255.0) as u8;
    }
    let va = sa * 255.0 + d[idx + 3] as f32 * (1.0 - sa);
    d[idx + 3] = va.round().clamp(0.0, 255.0) as u8;
}

fn draw_text(px: &mut tiny_skia::Pixmap, font: &AnnotFont, pos: Pos2, text: &str, size: f32, c: Color32) {
    let scale = PxScale::from(size);
    let sf = font.font.as_scaled(scale);
    let lh = font.line_height(size);
    for (li, line) in text.split('\n').enumerate() {
        let mut x = pos.x;
        let base = pos.y + li as f32 * lh + sf.ascent();
        let mut prev = None;
        for ch in line.chars() {
            let id = sf.glyph_id(ch);
            if let Some(p) = prev {
                x += sf.kern(p, id);
            }
            let g = id.with_scale_and_position(scale, ab_glyph::point(x, base));
            if let Some(og) = font.font.outline_glyph(g) {
                let bb = og.px_bounds();
                og.draw(|gx, gy, cov| {
                    blend(px, bb.min.x as i32 + gx as i32, bb.min.y as i32 + gy as i32, c, cov);
                });
            }
            x += sf.h_advance(id);
            prev = Some(id);
        }
    }
}

/// Draws every shape onto a copy of the picture.
/// Draws the items into the picture; `shadows` adds a soft shadow under them.
pub fn bake(img: &RgbaImage, shapes: &[Shape], font: &AnnotFont, shadows: bool) -> RgbaImage {
    if shapes.is_empty() {
        return img.clone();
    }
    let (w, h) = img.dimensions();
    let Some(mut px) = tiny_skia::Pixmap::new(w, h) else { return img.clone() };
    {
        let d = px.data_mut();
        for (i, p) in img.pixels().enumerate() {
            let a = p[3] as u32;
            d[i * 4] = (p[0] as u32 * a / 255) as u8;
            d[i * 4 + 1] = (p[1] as u32 * a / 255) as u8;
            d[i * 4 + 2] = (p[2] as u32 * a / 255) as u8;
            d[i * 4 + 3] = p[3];
        }
    }
    let full = Rect::from_min_size(Pos2::ZERO, vec2(w as f32, h as f32));
    // the blurred picture, made only when some item needs it
    let mut blurred: Option<RgbaImage> = None;
    for s in shapes {
        for prim in s.prims(font, shadows) {
            match prim {
                Prim::Stroke { pts, width, color, closed } => {
                    if let Some(path) = path_of(&pts, closed) {
                        let mut paint = tiny_skia::Paint::default();
                        paint.set_color(to_skia_color(color));
                        paint.anti_alias = true;
                        let stroke = tiny_skia::Stroke {
                            width,
                            line_cap: tiny_skia::LineCap::Round,
                            line_join: tiny_skia::LineJoin::Round,
                            ..Default::default()
                        };
                        px.stroke_path(&path, &paint, &stroke, tiny_skia::Transform::identity(), None);
                    }
                }
                Prim::Fill { pts, color } | Prim::Poly { pts, color } => {
                    if let Some(path) = path_of(&pts, true) {
                        let mut paint = tiny_skia::Paint::default();
                        paint.set_color(to_skia_color(color));
                        paint.anti_alias = true;
                        px.fill_path(&path, &paint, tiny_skia::FillRule::Winding, tiny_skia::Transform::identity(), None);
                    }
                }
                Prim::Text { pos, text, size, color } => draw_text(&mut px, font, pos, &text, size, color),
                Prim::Blur { r } => {
                    let b = blurred.get_or_insert_with(|| {
                        image::imageops::resize(&redact_small(img), w, h, image::imageops::FilterType::Triangle)
                    });
                    let r = r.intersect(full);
                    let d = px.data_mut();
                    for y in (r.min.y.round() as u32)..(r.max.y.round() as u32).min(h) {
                        for x in (r.min.x.round() as u32)..(r.max.x.round() as u32).min(w) {
                            let p = b.get_pixel(x, y);
                            let a = p[3] as u32;
                            let i = ((y * w + x) * 4) as usize;
                            d[i..i + 4].copy_from_slice(&[
                                (p[0] as u32 * a / 255) as u8,
                                (p[1] as u32 * a / 255) as u8,
                                (p[2] as u32 * a / 255) as u8,
                                p[3],
                            ]);
                        }
                    }
                }
                Prim::DarkenOutside { r } => {
                    for part in outside_parts(full, r) {
                        if let Some(rr) = tiny_skia::Rect::from_ltrb(part.min.x, part.min.y, part.max.x, part.max.y) {
                            let mut paint = tiny_skia::Paint::default();
                            paint.set_color(tiny_skia::Color::from_rgba8(0, 0, 0, DARKEN_ALPHA));
                            px.fill_rect(rr, &paint, tiny_skia::Transform::identity(), None);
                        }
                    }
                }
            }
        }
    }
    let mut out = RgbaImage::new(w, h);
    for (i, p) in px.pixels().iter().enumerate() {
        let c = p.demultiply();
        out.as_mut()[i * 4..i * 4 + 4].copy_from_slice(&[c.red(), c.green(), c.blue(), c.alpha()]);
    }
    out
}

pub const DARKEN_ALPHA: u8 = 128;

/// The four bands of `full` that lie outside `r`.
pub fn outside_parts(full: Rect, r: Rect) -> Vec<Rect> {
    let r = r.intersect(full);
    vec![
        Rect::from_min_max(full.min, pos2(full.max.x, r.min.y)),
        Rect::from_min_max(pos2(full.min.x, r.max.y), full.max),
        Rect::from_min_max(pos2(full.min.x, r.min.y), pos2(r.min.x, r.max.y)),
        Rect::from_min_max(pos2(r.max.x, r.min.y), pos2(full.max.x, r.max.y)),
    ]
    .into_iter()
    .filter(|p| p.width() > 0.0 && p.height() > 0.0)
    .collect()
}

/// Blur a region of the picture in place (three box-blur passes; radius estimated).
/// The drop shadow around an exported picture (the editor's Shadow toggle):
/// a soft black shadow, slightly lower than the picture, on a transparent
/// margin, like the shadow macOS gives window screenshots. Its softness
/// grows with the picture (σ = 2 % of the longer side, 12–40 px).
pub const DROP_SHADOW_ALPHA: f32 = 0.55;
pub const DROP_SHADOW_SIGMA: f32 = 0.02;
pub const DROP_SHADOW_SIGMA_MIN: f32 = 12.0;
pub const DROP_SHADOW_SIGMA_MAX: f32 = 40.0;
pub const DROP_SHADOW_DROP: f32 = 0.6;
pub const DROP_SHADOW_MARGIN: f32 = 2.5;

/// σ, downward offset and margin of the drop shadow for a w × h picture.
pub fn drop_shadow_geometry(w: f32, h: f32) -> (f32, f32, f32) {
    let sigma = (w.max(h) * DROP_SHADOW_SIGMA).clamp(DROP_SHADOW_SIGMA_MIN, DROP_SHADOW_SIGMA_MAX);
    let drop = (sigma * DROP_SHADOW_DROP).round();
    let margin = (sigma * DROP_SHADOW_MARGIN + drop).ceil();
    (sigma, drop, margin)
}

/// Error function (Abramowitz & Stegun 7.1.26, error below 1.5e-7).
fn erf(x: f32) -> f32 {
    let s = x.signum();
    let x = x.abs();
    let t = 1.0 / (1.0 + 0.327_591_1 * x);
    let y = 1.0 - (((((1.061_405_4 * t - 1.453_152_1) * t) + 1.421_413_7) * t - 0.284_496_74) * t + 0.254_829_6) * t * (-x * x).exp();
    s * y
}

/// The picture on a transparent margin with a soft drop shadow under it.
pub fn drop_shadow(img: &RgbaImage) -> RgbaImage {
    let (w, h) = img.dimensions();
    let (sigma, drop, margin) = drop_shadow_geometry(w as f32, h as f32);
    let m = margin as u32;
    let (ow, oh) = (w + 2 * m, h + 2 * m);
    // a blurred rectangle is the product of two blurred 1-D steps
    let k = 1.0 / (sigma * std::f32::consts::SQRT_2);
    let profile = |n: u32, a: f32, b: f32| -> Vec<f32> {
        (0..n).map(|i| {
            let p = i as f32 + 0.5;
            0.5 * (erf((p - a) * k) - erf((p - b) * k))
        }).collect()
    };
    let gx = profile(ow, margin, margin + w as f32);
    let gy = profile(oh, margin + drop, margin + drop + h as f32);
    let mut out = RgbaImage::from_fn(ow, oh, |x, y| {
        let a = (DROP_SHADOW_ALPHA * gx[x as usize] * gy[y as usize] * 255.0).round().clamp(0.0, 255.0) as u8;
        image::Rgba([0, 0, 0, a])
    });
    for (x, y, p) in img.enumerate_pixels() {
        let dst = out.get_pixel_mut(x + m, y + m);
        let sa = p[3] as f32 / 255.0;
        let da = dst[3] as f32 / 255.0;
        let oa = sa + da * (1.0 - sa);
        if oa > 0.0 {
            for c in 0..3 {
                dst[c] = ((p[c] as f32 * sa + dst[c] as f32 * da * (1.0 - sa)) / oa).round() as u8;
            }
        }
        dst[3] = (oa * 255.0).round() as u8;
    }
    out
}

/// How strongly the Blur tool hides things: the picture is shrunk by a factor
/// that grows with its size (text a few dozen pixels tall ends up one or two
/// pixels tall, so it can't be read back) and then softened.
pub const REDACT_DIVISOR: f32 = 90.0;
pub const REDACT_MIN: f32 = 6.0;
pub const REDACT_MAX: f32 = 24.0;

/// A small, blurred copy of the whole picture. Stretched back to full size it
/// gives the look of a heavy blur; the editor draws it as a texture and
/// `bake` scales it up.
pub fn redact_small(img: &RgbaImage) -> RgbaImage {
    let (w, h) = img.dimensions();
    let f = (w.min(h) as f32 / REDACT_DIVISOR).clamp(REDACT_MIN, REDACT_MAX);
    let sw = ((w as f32 / f).ceil() as u32).max(2);
    let sh = ((h as f32 / f).ceil() as u32).max(2);
    let mut small = image::imageops::resize(img, sw, sh, image::imageops::FilterType::Triangle);
    blur_region(&mut small, Rect::from_min_size(Pos2::ZERO, vec2(sw as f32, sh as f32)), 1);
    small
}

pub fn blur_region(img: &mut RgbaImage, r: Rect, radius: i32) {
    let (w, h) = (img.width() as i32, img.height() as i32);
    let x0 = (r.min.x.floor() as i32).clamp(0, w);
    let y0 = (r.min.y.floor() as i32).clamp(0, h);
    let x1 = (r.max.x.ceil() as i32).clamp(0, w);
    let y1 = (r.max.y.ceil() as i32).clamp(0, h);
    let (rw, rh) = (x1 - x0, y1 - y0);
    if rw < 2 || rh < 2 {
        return;
    }
    let mut buf: Vec<[f32; 4]> = Vec::with_capacity((rw * rh) as usize);
    for y in y0..y1 {
        for x in x0..x1 {
            let p = img.get_pixel(x as u32, y as u32);
            buf.push([p[0] as f32, p[1] as f32, p[2] as f32, p[3] as f32]);
        }
    }
    let rad = radius.max(1);
    for _ in 0..3 {
        // horizontal
        let mut tmp = buf.clone();
        for y in 0..rh {
            for x in 0..rw {
                let mut acc = [0.0f32; 4];
                let mut n = 0.0;
                for k in (x - rad).max(0)..=(x + rad).min(rw - 1) {
                    let p = buf[(y * rw + k) as usize];
                    for i in 0..4 {
                        acc[i] += p[i];
                    }
                    n += 1.0;
                }
                tmp[(y * rw + x) as usize] = [acc[0] / n, acc[1] / n, acc[2] / n, acc[3] / n];
            }
        }
        // vertical
        for y in 0..rh {
            for x in 0..rw {
                let mut acc = [0.0f32; 4];
                let mut n = 0.0;
                for k in (y - rad).max(0)..=(y + rad).min(rh - 1) {
                    let p = tmp[(k * rw + x) as usize];
                    for i in 0..4 {
                        acc[i] += p[i];
                    }
                    n += 1.0;
                }
                buf[(y * rw + x) as usize] = [acc[0] / n, acc[1] / n, acc[2] / n, acc[3] / n];
            }
        }
    }
    for y in 0..rh {
        for x in 0..rw {
            let p = buf[(y * rw + x) as usize];
            img.put_pixel(
                (x0 + x) as u32,
                (y0 + y) as u32,
                image::Rgba([p[0] as u8, p[1] as u8, p[2] as u8, p[3] as u8]),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blur_hides_fine_detail_in_the_saved_image() {
        // 1-px black and white stripes: after the blur they must be a flat grey
        let img = RgbaImage::from_fn(400, 300, |x, _| if x % 2 == 0 { image::Rgba([0, 0, 0, 255]) } else { image::Rgba([255, 255, 255, 255]) });
        let r = Rect::from_min_max(pos2(100.0, 100.0), pos2(300.0, 200.0));
        let blur = Shape { kind: Kind::Blur { r }, color: Color32::RED, width: 4.0 };
        let out = bake(&img, &[blur], &AnnotFont::load(None), true);
        for x in 120..280 {
            let v = out.get_pixel(x, 150)[0] as i32;
            assert!((v - 128).abs() < 30, "pixel {x} is {v}");
        }
        // outside the area nothing changes
        assert_eq!(out.get_pixel(10, 10), img.get_pixel(10, 10));
    }

    #[test]
    fn drop_shadow_adds_a_soft_transparent_margin() {
        let img = RgbaImage::from_pixel(400, 300, image::Rgba([255, 255, 255, 255]));
        let out = drop_shadow(&img);
        let (_, drop, margin) = drop_shadow_geometry(400.0, 300.0);
        let m = margin as u32;
        assert_eq!(out.dimensions(), (400 + 2 * m, 300 + 2 * m));
        // the picture itself is untouched, the corners are see-through and
        // there is a visible shadow just below the picture
        assert_eq!(*out.get_pixel(m + 200, m + 150), image::Rgba([255, 255, 255, 255]));
        assert_eq!(out.get_pixel(0, 0)[3], 0);
        let below = out.get_pixel(m + 200, m + 300 + drop as u32);
        assert!(below[3] > 60, "shadow under the picture: {}", below[3]);
    }

    #[test]
    fn baking_draws_shapes_and_text() {
        let img = RgbaImage::from_pixel(200, 120, image::Rgba([255, 255, 255, 255]));
        let font = AnnotFont::load(None);
        let red = Color32::from_rgb(255, 0, 0);
        let shapes = vec![
            Shape { kind: Kind::Arrow { a: pos2(10.0, 100.0), b: pos2(150.0, 20.0) }, color: red, width: 6.0 },
            Shape { kind: Kind::Text { pos: pos2(20.0, 10.0), text: "Hi".into(), size: 24.0, style: TextStyle::Plain }, color: Color32::BLACK, width: 6.0 },
        ];
        let out = bake(&img, &shapes, &font, true);
        let reds = out.pixels().filter(|p| p[0] > 200 && p[1] < 60 && p[2] < 60).count();
        let darks = out.pixels().filter(|p| p[0] < 60 && p[1] < 60 && p[2] < 60).count();
        assert!(reds > 100, "arrow drawn: {reds}");
        assert!(darks > 20, "text drawn: {darks}");
        let mut b = out.clone();
        blur_region(&mut b, Rect::from_min_max(pos2(0.0, 0.0), pos2(200.0, 120.0)), 8);
        assert_ne!(b.as_raw(), out.as_raw());
    }

    #[test]
    fn template_fills_tokens() {
        let s = crate::settings::apply_template("Snap %W %Y-%m-%d %T", "", "png");
        assert!(s.starts_with("Snap 20"), "{s}");
        assert!(s.ends_with("png"));
        assert!(!s.contains("  "));
    }

    #[test]
    fn text_styles_select_and_move() {
        let img = RgbaImage::from_pixel(240, 120, image::Rgba([255, 255, 255, 255]));
        let font = AnnotFont::load(None);
        let blue = Color32::from_rgb(0, 0, 255);
        let mut boxed = Shape {
            kind: Kind::Text { pos: pos2(20.0, 20.0), text: "Box".into(), size: 30.0, style: TextStyle::Box },
            color: blue,
            width: 4.0,
        };
        let out = bake(&img, &[boxed.clone()], &font, false);
        let blues = out.pixels().filter(|p| p[2] > 200 && p[0] < 60).count();
        assert!(blues > 400, "box drawn: {blues}");
        // hit inside the text, miss far away, then move it
        assert!(boxed.hit(pos2(30.0, 35.0), 4.0, &font));
        assert!(!boxed.hit(pos2(200.0, 100.0), 4.0, &font));
        boxed.translate(vec2(100.0, 10.0));
        assert!(boxed.hit(pos2(130.0, 45.0), 4.0, &font));
        let outline = Shape {
            kind: Kind::Text { pos: pos2(20.0, 60.0), text: "Hi".into(), size: 30.0, style: TextStyle::Outline },
            color: Color32::from_rgb(255, 0, 0),
            width: 4.0,
        };
        assert!(outline.prims(&font, true).len() >= 13);
    }
}
