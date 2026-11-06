//! The app icon, drawn in code: a camera lens whose aperture diaphragm has
//! seven coloured blades around a dark opening, with a metal rim and a glint.

use image::RgbaImage;
use tiny_skia::{
    Color, FillRule, GradientStop, LinearGradient, Mask, Paint, PathBuilder, Pixmap, Point, RadialGradient, Shader,
    SpreadMode, Stroke, Transform,
};

pub const BLADE_COLORS: [(u8, u8, u8); 7] = [
    (255, 59, 48),   // red
    (255, 149, 0),   // orange
    (255, 204, 0),   // yellow
    (52, 199, 89),   // green
    (0, 199, 190),   // teal
    (10, 132, 255),  // blue
    (175, 82, 222),  // purple
];

fn circle(cx: f32, cy: f32, r: f32) -> tiny_skia::Path {
    PathBuilder::from_circle(cx, cy, r).expect("circle")
}

fn poly(pts: &[(f32, f32)]) -> Option<tiny_skia::Path> {
    let mut pb = PathBuilder::new();
    pb.move_to(pts[0].0, pts[0].1);
    for p in &pts[1..] {
        pb.line_to(p.0, p.1);
    }
    pb.close();
    pb.finish()
}

fn shade(c: (u8, u8, u8), k: f32) -> Color {
    let f = |v: u8| ((v as f32 * k).clamp(0.0, 255.0)) as u8;
    Color::from_rgba8(f(c.0), f(c.1), f(c.2), 255)
}

/// Renders the icon at `n`×`n` pixels.
pub fn render(n: u32) -> RgbaImage {
    let mut px = Pixmap::new(n, n).expect("pixmap");
    let s = n as f32;
    let (cx, cy) = (s / 2.0, s / 2.0);
    let id = Transform::identity();

    // Barrel: dark metal with a vertical gradient.
    let r_outer = s * 0.47;
    let mut paint = Paint::default();
    paint.anti_alias = true;
    paint.shader = LinearGradient::new(
        Point::from_xy(cx, cy - r_outer),
        Point::from_xy(cx, cy + r_outer),
        vec![
            GradientStop::new(0.0, Color::from_rgba8(70, 76, 88, 255)),
            GradientStop::new(1.0, Color::from_rgba8(16, 18, 24, 255)),
        ],
        SpreadMode::Pad,
        id,
    )
    .unwrap_or(Shader::SolidColor(Color::from_rgba8(40, 44, 52, 255)));
    px.fill_path(&circle(cx, cy, r_outer), &paint, FillRule::Winding, id, None);

    // Grip ring.
    let mut ring = Paint::default();
    ring.anti_alias = true;
    ring.set_color(Color::from_rgba8(110, 118, 132, 255));
    let st = Stroke { width: s * 0.012, ..Default::default() };
    px.stroke_path(&circle(cx, cy, s * 0.435), &ring, &st, id, None);

    // Aperture blades, clipped to the lens opening.
    let ra = s * 0.395;
    let mut mask = Mask::new(n, n).expect("mask");
    mask.fill_path(&circle(cx, cy, ra), FillRule::Winding, true, id);
    let mut base = Paint::default();
    base.anti_alias = true;
    base.set_color(Color::from_rgba8(10, 12, 18, 255));
    px.fill_path(&circle(cx, cy, ra), &base, FillRule::Winding, id, None);

    let nb = BLADE_COLORS.len();
    let r0 = s * 0.14;
    let rot = -std::f32::consts::FRAC_PI_2 + 0.25;
    let v: Vec<(f32, f32)> = (0..nb)
        .map(|i| {
            let a = rot + i as f32 * std::f32::consts::TAU / nb as f32;
            (cx + r0 * a.cos(), cy + r0 * a.sin())
        })
        .collect();
    let dir = |i: usize| {
        let a = v[i % nb];
        let b = v[(i + 1) % nb];
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let l = (dx * dx + dy * dy).sqrt();
        (dx / l, dy / l)
    };
    let far = s * 0.9;
    for i in 0..nb {
        let v1 = v[(i + 1) % nb];
        let v2 = v[(i + 2) % nb];
        let d0 = dir(i);
        let d1 = dir(i + 1);
        let f1 = (v1.0 + d0.0 * far, v1.1 + d0.1 * far);
        let f2 = (v2.0 + d1.0 * far, v2.1 + d1.1 * far);
        let col = BLADE_COLORS[i];
        if let Some(path) = poly(&[v1, f1, f2, v2]) {
            let mut p = Paint::default();
            p.anti_alias = true;
            let mid = ((v1.0 + v2.0) / 2.0, (v1.1 + v2.1) / 2.0);
            let out = (mid.0 + (mid.0 - cx) * 2.4, mid.1 + (mid.1 - cy) * 2.4);
            p.shader = LinearGradient::new(
                Point::from_xy(mid.0, mid.1),
                Point::from_xy(out.0, out.1),
                vec![GradientStop::new(0.0, shade(col, 0.72)), GradientStop::new(1.0, shade(col, 1.12))],
                SpreadMode::Pad,
                id,
            )
            .unwrap_or(Shader::SolidColor(shade(col, 1.0)));
            px.fill_path(&path, &p, FillRule::Winding, id, Some(&mask));
        }
        // blade edge
        let mut pb = PathBuilder::new();
        pb.move_to(v1.0, v1.1);
        pb.line_to(f1.0, f1.1);
        if let Some(edge) = pb.finish() {
            let mut ep = Paint::default();
            ep.anti_alias = true;
            ep.set_color(Color::from_rgba8(0, 0, 0, 90));
            let st = Stroke { width: (s * 0.008).max(0.8), ..Default::default() };
            px.stroke_path(&edge, &ep, &st, id, Some(&mask));
        }
    }

    // The dark opening in the middle.
    if let Some(open) = poly(&v) {
        let mut p = Paint::default();
        p.anti_alias = true;
        p.shader = RadialGradient::new(
            Point::from_xy(cx - r0 * 0.3, cy - r0 * 0.3),
            Point::from_xy(cx, cy),
            r0 * 1.3,
            vec![
                GradientStop::new(0.0, Color::from_rgba8(40, 60, 110, 255)),
                GradientStop::new(1.0, Color::from_rgba8(4, 6, 14, 255)),
            ],
            SpreadMode::Pad,
            id,
        )
        .unwrap_or(Shader::SolidColor(Color::from_rgba8(8, 10, 20, 255)));
        px.fill_path(&open, &p, FillRule::Winding, id, None);
    }

    // Glass reflections.
    let mut glint = Paint::default();
    glint.anti_alias = true;
    glint.shader = RadialGradient::new(
        Point::from_xy(cx - ra * 0.35, cy - ra * 0.45),
        Point::from_xy(cx - ra * 0.35, cy - ra * 0.45),
        ra * 0.75,
        vec![
            GradientStop::new(0.0, Color::from_rgba8(255, 255, 255, 120)),
            GradientStop::new(1.0, Color::from_rgba8(255, 255, 255, 0)),
        ],
        SpreadMode::Pad,
        id,
    )
    .unwrap_or(Shader::SolidColor(Color::from_rgba8(255, 255, 255, 30)));
    px.fill_path(&circle(cx, cy, ra), &glint, FillRule::Winding, id, None);
    let mut dot = Paint::default();
    dot.anti_alias = true;
    dot.set_color(Color::from_rgba8(255, 255, 255, 210));
    px.fill_path(&circle(cx - r0 * 0.35, cy - r0 * 0.4, (s * 0.022).max(1.0)), &dot, FillRule::Winding, id, None);

    // Bright metal rim around the opening.
    let mut rim = Paint::default();
    rim.anti_alias = true;
    rim.shader = LinearGradient::new(
        Point::from_xy(cx - ra, cy - ra),
        Point::from_xy(cx + ra, cy + ra),
        vec![
            GradientStop::new(0.0, Color::from_rgba8(235, 238, 245, 255)),
            GradientStop::new(1.0, Color::from_rgba8(120, 126, 140, 255)),
        ],
        SpreadMode::Pad,
        id,
    )
    .unwrap_or(Shader::SolidColor(Color::from_rgba8(200, 200, 210, 255)));
    let st = Stroke { width: (s * 0.018).max(1.0), ..Default::default() };
    px.stroke_path(&circle(cx, cy, ra), &rim, &st, id, None);

    let mut out = RgbaImage::new(n, n);
    for (i, p) in px.pixels().iter().enumerate() {
        let c = p.demultiply();
        out.as_mut()[i * 4..i * 4 + 4].copy_from_slice(&[c.red(), c.green(), c.blue(), c.alpha()]);
    }
    out
}

/// The menu-bar glyph: the same lens and aperture drawn in a single ink
/// (black on transparent), meant to be used as a macOS template image.
pub fn render_template(n: u32) -> RgbaImage {
    let mut px = Pixmap::new(n, n).expect("pixmap");
    let s = n as f32;
    let (cx, cy) = (s / 2.0, s / 2.0);
    let id = Transform::identity();
    let mut ink = Paint::default();
    ink.anti_alias = true;
    ink.set_color(Color::BLACK);

    // Outer barrel ring.
    let ring = Stroke { width: s * 0.085, ..Default::default() };
    px.stroke_path(&circle(cx, cy, s * 0.44), &ink, &ring, id, None);

    // Aperture: a seven-sided opening with one blade edge running from each
    // corner out to the ring.
    let nb = BLADE_COLORS.len();
    let r0 = s * 0.15;
    let rot = -std::f32::consts::FRAC_PI_2 + 0.25;
    let v: Vec<(f32, f32)> = (0..nb)
        .map(|i| {
            let a = rot + i as f32 * std::f32::consts::TAU / nb as f32;
            (cx + r0 * a.cos(), cy + r0 * a.sin())
        })
        .collect();
    let mut mask = Mask::new(n, n).expect("mask");
    mask.fill_path(&circle(cx, cy, s * 0.42), FillRule::Winding, true, id);
    let blade = Stroke { width: s * 0.06, ..Default::default() };
    for i in 0..nb {
        let a = v[i];
        let b = v[(i + 1) % nb];
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let l = (dx * dx + dy * dy).sqrt();
        let far = (b.0 + dx / l * s, b.1 + dy / l * s);
        let mut pb = PathBuilder::new();
        pb.move_to(a.0, a.1);
        pb.line_to(far.0, far.1);
        if let Some(edge) = pb.finish() {
            px.stroke_path(&edge, &ink, &blade, id, Some(&mask));
        }
    }

    let mut out = RgbaImage::new(n, n);
    for (i, p) in px.pixels().iter().enumerate() {
        let c = p.demultiply();
        out.as_mut()[i * 4..i * 4 + 4].copy_from_slice(&[c.red(), c.green(), c.blue(), c.alpha()]);
    }
    out
}

pub fn icon_data(n: u32) -> egui::IconData {
    let img = render(n);
    egui::IconData { rgba: img.into_raw(), width: n, height: n }
}
