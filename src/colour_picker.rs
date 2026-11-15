//! The custom-colour popup of the editor: a hex field, a saturation /
//! brightness map and a hue bar, all at one fixed width so the popup never
//! changes size while picking.

use egui::epaint::Vertex;
use egui::{pos2, vec2, Color32, FontId, Mesh, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, Ui};

use crate::theme::Pal;

/// Width of everything in the popup.
const WIDTH: f32 = 240.0;
const MAP_HEIGHT: f32 = 150.0;
const HUE_HEIGHT: f32 = 12.0;
const MARKER_RADIUS: f32 = 5.5;
/// The map is shown a little dimmer in dark mode so its white corner does
/// not glare; the colour picked is still the exact one.
const MAP_DIM_DARK: f32 = 0.86;

/// Hue (0-1), saturation (0-1) and value (0-1) of an sRGB colour.
fn to_hsv(c: Color32) -> [f32; 3] {
    let (r, g, b) = (c.r() as f32 / 255.0, c.g() as f32 / 255.0, c.b() as f32 / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let h = if d == 0.0 {
        0.0
    } else if max == r {
        ((g - b) / d).rem_euclid(6.0) / 6.0
    } else if max == g {
        ((b - r) / d + 2.0) / 6.0
    } else {
        ((r - g) / d + 4.0) / 6.0
    };
    let s = if max == 0.0 { 0.0 } else { d / max };
    [h, s, max]
}

fn from_hsv([h, s, v]: [f32; 3]) -> Color32 {
    let h6 = (h.rem_euclid(1.0)) * 6.0;
    let c = v * s;
    let x = c * (1.0 - (h6 % 2.0 - 1.0).abs());
    let (r, g, b) = match h6 as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    let f = |u: f32| ((u + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    Color32::from_rgb(f(r), f(g), f(b))
}

pub fn to_hex(c: Color32) -> String {
    format!("#{:02X}{:02X}{:02X}", c.r(), c.g(), c.b())
}

/// "#RRGGBB", "RRGGBB", "#RGB" or "RGB" (any case).
pub fn parse_hex(s: &str) -> Option<Color32> {
    let h = s.trim().trim_start_matches('#');
    if !h.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let v = |i: usize, n: usize| u8::from_str_radix(&h[i..i + n], 16).ok();
    match h.len() {
        6 => Some(Color32::from_rgb(v(0, 2)?, v(2, 2)?, v(4, 2)?)),
        3 => {
            let d = |i: usize| v(i, 1).map(|x| x * 17);
            Some(Color32::from_rgb(d(0)?, d(1)?, d(2)?))
        }
        _ => None,
    }
}

fn dim(c: Color32, k: f32) -> Color32 {
    let f = |x: u8| (x as f32 * k).round() as u8;
    Color32::from_rgb(f(c.r()), f(c.g()), f(c.b()))
}

/// A small marker: the colour inside a soft, slightly see-through ring.
fn marker(p: &egui::Painter, at: Pos2, fill: Color32) {
    p.circle_filled(at, MARKER_RADIUS + 1.5, Color32::from_black_alpha(50));
    p.circle_filled(at, MARKER_RADIUS, fill);
    p.circle_stroke(at, MARKER_RADIUS, Stroke::new(1.5_f32, Color32::from_rgba_unmultiplied(235, 235, 240, 200)));
}

/// Shows the picker; returns true when `color` changed.
pub fn show(ui: &mut Ui, color: &mut Color32, pal: &Pal) -> bool {
    let id = ui.id().with("quicksnap_colour_picker");
    // Hue and saturation are lost for greys and black, so they are kept
    // between frames as long as the colour is the one last picked here.
    let mut hsv = ui
        .ctx()
        .data(|d| d.get_temp::<(Color32, [f32; 3])>(id))
        .filter(|(c, _)| c == color)
        .map(|(_, hsv)| hsv)
        .unwrap_or_else(|| to_hsv(*color));
    let start = *color;
    let k = if pal.dark { MAP_DIM_DARK } else { 1.0 };

    ui.set_width(WIDTH);
    ui.spacing_mut().item_spacing = vec2(8.0, 8.0);

    // --- preview and hex code
    ui.horizontal(|ui| {
        let (sw, _) = ui.allocate_exact_size(vec2(28.0, 28.0), Sense::hover());
        ui.painter().rect_filled(sw, 7.0, *color);
        ui.painter().rect_stroke(sw, 7.0, Stroke::new(1.0_f32, pal.border), StrokeKind::Inside);
        let tid = id.with("hex");
        let mut text = ui.ctx().data(|d| d.get_temp::<String>(tid)).unwrap_or_else(|| to_hex(*color));
        let focused = ui.memory(|m| m.has_focus(tid));
        if !focused {
            text = to_hex(*color);
        }
        let resp = ui.add(
            egui::TextEdit::singleline(&mut text)
                .id(tid)
                .font(FontId::monospace(14.0))
                .char_limit(7)
                .desired_width(WIDTH - 28.0 - 8.0 - 12.0)
                .vertical_align(egui::Align::Center)
                .min_size(vec2(0.0, 28.0))
                .hint_text("#RRGGBB"),
        );
        if resp.changed() {
            if let Some(c) = parse_hex(&text).filter(|_| text.trim().trim_start_matches('#').len() == 6) {
                *color = c;
                hsv = to_hsv(c);
            }
        }
        if resp.lost_focus() {
            if let Some(c) = parse_hex(&text) {
                *color = c;
                hsv = to_hsv(c);
            }
            text = to_hex(*color);
        }
        ui.ctx().data_mut(|d| d.insert_temp(tid, text));
    });

    // --- saturation (left to right) / brightness (top to bottom) map
    let (map, resp) = ui.allocate_exact_size(vec2(WIDTH, MAP_HEIGHT), Sense::click_and_drag());
    if let Some(p) = resp.interact_pointer_pos().filter(|_| resp.is_pointer_button_down_on() || resp.clicked()) {
        hsv[1] = ((p.x - map.left()) / map.width()).clamp(0.0, 1.0);
        hsv[2] = 1.0 - ((p.y - map.top()) / map.height()).clamp(0.0, 1.0);
        *color = from_hsv(hsv);
    }
    {
        let (cols, rows) = (16usize, 10usize);
        let mut mesh = Mesh::default();
        for j in 0..=rows {
            for i in 0..=cols {
                let (fx, fy) = (i as f32 / cols as f32, j as f32 / rows as f32);
                let c = dim(from_hsv([hsv[0], fx, 1.0 - fy]), k);
                mesh.vertices.push(Vertex { pos: pos2(map.left() + fx * map.width(), map.top() + fy * map.height()), uv: egui::epaint::WHITE_UV, color: c });
            }
        }
        let w = (cols + 1) as u32;
        for j in 0..rows as u32 {
            for i in 0..cols as u32 {
                let a = j * w + i;
                mesh.add_triangle(a, a + 1, a + w);
                mesh.add_triangle(a + 1, a + w + 1, a + w);
            }
        }
        let p = ui.painter();
        p.add(Shape::mesh(mesh));
        p.rect_stroke(map, 0.0, Stroke::new(1.0_f32, pal.border), StrokeKind::Inside);
        let at = pos2(map.left() + hsv[1] * map.width(), map.top() + (1.0 - hsv[2]) * map.height());
        marker(p, at, *color);
    }

    // --- hue bar
    let (bar, resp) = ui.allocate_exact_size(vec2(WIDTH, HUE_HEIGHT + 6.0), Sense::click_and_drag());
    let bar_r = Rect::from_center_size(bar.center(), vec2(WIDTH, HUE_HEIGHT));
    if let Some(p) = resp.interact_pointer_pos().filter(|_| resp.is_pointer_button_down_on() || resp.clicked()) {
        hsv[0] = ((p.x - bar_r.left()) / bar_r.width()).clamp(0.0, 0.9999);
        *color = from_hsv(hsv);
    }
    {
        let n = 36usize;
        let mut mesh = Mesh::default();
        for i in 0..=n {
            let f = i as f32 / n as f32;
            let c = dim(from_hsv([f.min(0.9999), 1.0, 1.0]), k);
            let x = bar_r.left() + f * bar_r.width();
            mesh.vertices.push(Vertex { pos: pos2(x, bar_r.top()), uv: egui::epaint::WHITE_UV, color: c });
            mesh.vertices.push(Vertex { pos: pos2(x, bar_r.bottom()), uv: egui::epaint::WHITE_UV, color: c });
        }
        for i in 0..n as u32 {
            let a = i * 2;
            mesh.add_triangle(a, a + 1, a + 2);
            mesh.add_triangle(a + 1, a + 3, a + 2);
        }
        let p = ui.painter();
        p.add(Shape::mesh(mesh));
        p.rect_stroke(bar_r, 0.0, Stroke::new(1.0_f32, pal.border), StrokeKind::Inside);
        marker(p, pos2(bar_r.left() + hsv[0] * bar_r.width(), bar_r.center().y), from_hsv([hsv[0], 1.0, 1.0]));
    }

    ui.ctx().data_mut(|d| d.insert_temp(id, (*color, hsv)));
    *color != start
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_round_trip() {
        let c = Color32::from_rgb(0x12, 0xab, 0xef);
        assert_eq!(to_hex(c), "#12ABEF");
        assert_eq!(parse_hex("#12abef"), Some(c));
        assert_eq!(parse_hex("12ABEF"), Some(c));
        assert_eq!(parse_hex("#fff"), Some(Color32::WHITE));
        assert_eq!(parse_hex("#12ab"), None);
        assert_eq!(parse_hex("zzzzzz"), None);
    }

    #[test]
    fn hsv_round_trip() {
        for c in [Color32::from_rgb(255, 114, 0), Color32::from_rgb(31, 99, 190), Color32::WHITE, Color32::BLACK, Color32::from_rgb(137, 137, 137)] {
            assert_eq!(from_hsv(to_hsv(c)), c);
        }
    }
}
