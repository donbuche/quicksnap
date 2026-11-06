//! Light and dark colour sets plus the "glass" look: rounded, semi-opaque
//! panels laid over a blurred copy of what is behind them.

use egui::epaint::{RectShape, Shadow};
use egui::{pos2, vec2, Color32, Context, CornerRadius, Painter, Rect, Stroke, StrokeKind, TextureId, Ui};
use image::RgbaImage;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum ThemeChoice {
    System,
    Light,
    Dark,
}

impl ThemeChoice {
    pub fn preference(self) -> egui::ThemePreference {
        match self {
            ThemeChoice::System => egui::ThemePreference::System,
            ThemeChoice::Light => egui::ThemePreference::Light,
            ThemeChoice::Dark => egui::ThemePreference::Dark,
        }
    }
}

/// Colours for the current mode.
#[derive(Clone, Copy)]
pub struct Pal {
    pub dark: bool,
    pub bg: Color32,
    pub glass: Color32,
    pub glass_strong: Color32,
    pub card: Color32,
    pub border: Color32,
    pub text: Color32,
    pub hint: Color32,
    pub accent: Color32,
    pub accent_soft: Color32,
    pub control: Color32,
    pub hover: Color32,
    pub canvas_tint: Color32,
    pub shadow: Color32,
}

const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Color32 {
    Color32::from_rgba_premultiplied(
        (r as u32 * a as u32 / 255) as u8,
        (g as u32 * a as u32 / 255) as u8,
        (b as u32 * a as u32 / 255) as u8,
        a,
    )
}

pub const LIGHT: Pal = Pal {
    dark: false,
    bg: Color32::from_rgb(0xf2, 0xf2, 0xf7),
    glass: rgba(246, 246, 248, 190),
    glass_strong: rgba(248, 248, 250, 235),
    card: rgba(255, 255, 255, 215),
    border: rgba(0, 0, 0, 22),
    text: Color32::from_rgb(0x1c, 0x1c, 0x1e),
    hint: Color32::from_rgb(0x6e, 0x6e, 0x73),
    accent: Color32::from_rgb(0x00, 0x7a, 0xff),
    accent_soft: rgba(0, 122, 255, 40),
    control: rgba(0, 0, 0, 14),
    hover: rgba(0, 0, 0, 24),
    canvas_tint: rgba(236, 236, 240, 170),
    shadow: rgba(0, 0, 0, 40),
};

pub const DARK: Pal = Pal {
    dark: true,
    bg: Color32::from_rgb(0x1c, 0x1c, 0x1e),
    glass: rgba(38, 38, 42, 190),
    glass_strong: rgba(44, 44, 48, 235),
    card: rgba(52, 52, 56, 200),
    border: Color32::from_rgb(64, 64, 70),
    text: Color32::from_rgb(0xf2, 0xf2, 0xf7),
    hint: Color32::from_rgb(0x98, 0x98, 0x9d),
    accent: Color32::from_rgb(0x0a, 0x84, 0xff),
    accent_soft: rgba(10, 132, 255, 50),
    control: rgba(255, 255, 255, 18),
    hover: rgba(255, 255, 255, 30),
    canvas_tint: rgba(18, 18, 20, 170),
    shadow: rgba(0, 0, 0, 110),
};

pub fn pal(ctx: &Context) -> Pal {
    if ctx.style().visuals.dark_mode { DARK } else { LIGHT }
}

/// Rounded corners, softer shadows and accent colour for both modes.
pub fn setup_styles(ctx: &Context) {
    for (theme, p) in [(egui::Theme::Light, LIGHT), (egui::Theme::Dark, DARK)] {
        ctx.style_mut_of(theme, |s| {
            let v = &mut s.visuals;
            v.panel_fill = p.bg;
            v.window_fill = p.glass_strong;
            v.extreme_bg_color = if p.dark { Color32::from_rgb(0x2a, 0x2a, 0x2d) } else { Color32::WHITE };
            v.window_corner_radius = CornerRadius::same(14);
            v.menu_corner_radius = CornerRadius::same(10);
            v.window_stroke = Stroke::new(1.0_f32, p.border);
            v.window_shadow = Shadow { offset: [0, 10], blur: 30, spread: 0, color: p.shadow };
            v.popup_shadow = Shadow { offset: [0, 6], blur: 18, spread: 0, color: p.shadow };
            v.selection.bg_fill = p.accent;
            v.selection.stroke = Stroke::new(1.0_f32, Color32::WHITE);
            v.hyperlink_color = p.accent;
            v.override_text_color = Some(p.text);
            let r = CornerRadius::same(8);
            for w in [&mut v.widgets.noninteractive, &mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.open] {
                w.corner_radius = r;
            }
            v.widgets.inactive.weak_bg_fill = p.control;
            v.widgets.inactive.bg_fill = p.control;
            v.widgets.inactive.bg_stroke = Stroke::NONE;
            v.widgets.hovered.weak_bg_fill = p.hover;
            v.widgets.hovered.bg_fill = p.hover;
            v.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, p.border);
            v.widgets.active.weak_bg_fill = p.accent_soft;
            v.widgets.active.bg_fill = p.accent;
            v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, p.border);
            s.spacing.button_padding = vec2(10.0, 5.0);
            s.spacing.interact_size.y = 24.0;
            s.spacing.slider_rail_height = 5.0;
        });
    }
}

/// Soft drop shadow behind a rounded rectangle.
pub fn shadow(p: &Painter, r: Rect, radius: u8, pal: &Pal) {
    let sh = Shadow { offset: [0, 8], blur: 26, spread: 0, color: pal.shadow };
    p.add(sh.as_shape(r, CornerRadius::same(radius)));
}

/// Shapes of a frosted-glass panel: a soft shadow, the blurred backdrop (if
/// any) and a semi-opaque tint with a hairline border, all rounded.
pub fn glass_shapes(r: Rect, radius: u8, pal: &Pal, backdrop: Option<Backdrop>) -> Vec<egui::Shape> {
    let mut v: Vec<egui::Shape> = Vec::new();
    let sh = Shadow { offset: [0, 8], blur: 26, spread: 0, color: pal.shadow };
    v.push(sh.as_shape(r, CornerRadius::same(radius)).into());
    if let Some(b) = backdrop {
        v.push(textured_rounded(r, radius as f32, &b));
    }
    v.push(RectShape::new(r, CornerRadius::same(radius), pal.glass, Stroke::new(1.0_f32, pal.border), StrokeKind::Inside).into());
    v
}

pub fn glass(p: &Painter, r: Rect, radius: u8, pal: &Pal, backdrop: Option<Backdrop>) {
    for s in glass_shapes(r, radius, pal, backdrop) {
        p.add(s);
    }
}

/// A blurred picture stretched over `area` (showing the `uv` part of it).
#[derive(Clone, Copy)]
pub struct Backdrop {
    pub tex: TextureId,
    pub area: Rect,
    pub uv: Rect,
}

impl Backdrop {
    #[allow(dead_code)]
    pub fn uv_for(&self, r: Rect) -> Rect {
        let f = |x: f32, y: f32| {
            pos2(
                self.uv.min.x + (x - self.area.min.x) / self.area.width() * self.uv.width(),
                self.uv.min.y + (y - self.area.min.y) / self.area.height() * self.uv.height(),
            )
        };
        Rect::from_min_max(f(r.min.x, r.min.y), f(r.max.x, r.max.y))
    }

    /// Cover `area` with a picture of `size`, cropping the overflow.
    pub fn cover(tex: TextureId, area: Rect, size: egui::Vec2) -> Backdrop {
        let ia = size.x / size.y.max(1.0);
        let aa = area.width() / area.height().max(1.0);
        let uv = if aa > ia {
            let h = ia / aa;
            Rect::from_min_max(pos2(0.0, 0.5 - h / 2.0), pos2(1.0, 0.5 + h / 2.0))
        } else {
            let w = aa / ia;
            Rect::from_min_max(pos2(0.5 - w / 2.0, 0.0), pos2(0.5 + w / 2.0, 1.0))
        };
        Backdrop { tex, area, uv }
    }

    pub fn paint(&self, p: &Painter) {
        let mut mesh = egui::Mesh::with_texture(self.tex);
        mesh.add_rect_with_uv(self.area, self.uv, Color32::WHITE);
        p.add(egui::Shape::mesh(mesh));
    }
}

/// The backdrop seen through a rounded rectangle, as a plain mesh (no edge
/// feathering, so no light halo appears around dark panels).
pub fn textured_rounded(r: Rect, radius: f32, b: &Backdrop) -> egui::Shape {
    // A mesh has no anti-aliased edge, so it sits 1 point inside the panel's
    // rounded outline (under its hairline border); otherwise bits of the
    // backdrop peek out along the corner arcs.
    let r = r.shrink(1.0);
    let rad = (radius - 1.0).max(0.0).min(r.width() / 2.0).min(r.height() / 2.0);
    let mut mesh = egui::Mesh::with_texture(b.tex);
    let uv = |p: egui::Pos2| {
        pos2(
            b.uv.min.x + (p.x - b.area.min.x) / b.area.width() * b.uv.width(),
            b.uv.min.y + (p.y - b.area.min.y) / b.area.height() * b.uv.height(),
        )
    };
    let c = r.center();
    mesh.vertices.push(egui::epaint::Vertex { pos: c, uv: uv(c), color: Color32::WHITE });
    let corners = [
        (pos2(r.max.x - rad, r.min.y + rad), -90.0f32),
        (pos2(r.max.x - rad, r.max.y - rad), 0.0),
        (pos2(r.min.x + rad, r.max.y - rad), 90.0),
        (pos2(r.min.x + rad, r.min.y + rad), 180.0),
    ];
    for (cc, start) in corners {
        for i in 0..=16 {
            let a = (start + i as f32 * 5.625).to_radians();
            let p = pos2(cc.x + rad * a.cos(), cc.y + rad * a.sin());
            mesh.vertices.push(egui::epaint::Vertex { pos: p, uv: uv(p), color: Color32::WHITE });
        }
    }
    let n = mesh.vertices.len() as u32 - 1;
    for i in 1..=n {
        let j = if i == n { 1 } else { i + 1 };
        mesh.add_triangle(0, i, j);
    }
    egui::Shape::mesh(mesh)
}

/// A floating glass panel laid out horizontally at `pos` (aligned by `pivot`).
pub fn floating<R>(
    ctx: &Context,
    id: &str,
    pivot: egui::Align2,
    pos: egui::Pos2,
    pal: &Pal,
    backdrop: Option<Backdrop>,
    add: impl FnOnce(&mut Ui) -> R,
) -> egui::InnerResponse<R> {
    egui::Area::new(egui::Id::new(id)).pivot(pivot).fixed_pos(pos).order(egui::Order::Middle).show(ctx, |ui| {
        let slots: Vec<_> = (0..3).map(|_| ui.painter().add(egui::Shape::Noop)).collect();
        let inner = egui::Frame::new().inner_margin(egui::Margin::symmetric(8, 6)).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                add(ui)
            })
            .inner
        });
        let rect = inner.response.rect;
        for (slot, shape) in slots.into_iter().zip(glass_shapes(rect, 14, pal, backdrop)) {
            ui.painter().set(slot, shape);
        }
        inner.inner
    })
}

/// A thin vertical divider inside a floating panel.
pub fn divider(ui: &mut Ui, pal: &Pal) {
    let (r, _) = ui.allocate_exact_size(vec2(9.0, 22.0), egui::Sense::hover());
    ui.painter().line_segment([r.center_top(), r.center_bottom()], Stroke::new(1.0_f32, pal.border));
}

/// A small, very blurred copy of a picture, used as a backdrop.
pub fn blurred_backdrop(img: &RgbaImage) -> egui::ColorImage {
    let (w, h) = img.dimensions();
    let k = (w.max(h) as f32 / 72.0).max(1.0);
    let tw = ((w as f32 / k).round() as u32).max(1);
    let th = ((h as f32 / k).round() as u32).max(1);
    let mut small = image::imageops::resize(img, tw, th, image::imageops::FilterType::Triangle);
    crate::shapes::blur_region(&mut small, Rect::from_min_size(pos2(0.0, 0.0), vec2(tw as f32, th as f32)), 3);
    egui::ColorImage::from_rgba_unmultiplied([tw as usize, th as usize], small.as_raw())
}

/// Soft colour blobs (the aperture's colours) for the home screen backdrop.
pub fn blob_backdrop(dark: bool) -> egui::ColorImage {
    let (w, h) = (160usize, 100usize);
    let base = if dark { [28.0f32, 28.0, 32.0] } else { [242.0f32, 242.0, 247.0] };
    let blobs: [(f32, f32, f32, [f32; 3]); 5] = [
        (0.18, 0.25, 0.32, [255.0, 59.0, 48.0]),
        (0.80, 0.20, 0.30, [10.0, 132.0, 255.0]),
        (0.55, 0.85, 0.34, [52.0, 199.0, 89.0]),
        (0.10, 0.90, 0.26, [255.0, 204.0, 0.0]),
        (0.92, 0.80, 0.28, [175.0, 82.0, 222.0]),
    ];
    let strength = if dark { 0.45 } else { 0.35 };
    let mut px = Vec::with_capacity(w * h);
    for y in 0..h {
        for x in 0..w {
            let fx = x as f32 / w as f32;
            let fy = y as f32 / h as f32;
            let mut c = base;
            for (bx, by, r, col) in blobs {
                let d = (((fx - bx) * 1.6).powi(2) + (fy - by).powi(2)).sqrt();
                let a = (1.0 - d / r).clamp(0.0, 1.0).powf(1.6) * strength;
                for i in 0..3 {
                    c[i] = c[i] * (1.0 - a) + col[i] * a;
                }
            }
            px.push(Color32::from_rgb(c[0] as u8, c[1] as u8, c[2] as u8));
        }
    }
    egui::ColorImage { size: [w, h], source_size: vec2(w as f32, h as f32), pixels: px }
}

/// Rounded sections ("cards") for the settings forms. Shapes are reserved
/// first and filled in once the section's height is known.
pub struct Section {
    idx: egui::layers::ShapeIdx,
    top: f32,
}

pub fn section_begin(ui: &mut Ui) -> Section {
    let idx = ui.painter().add(egui::Shape::Noop);
    ui.add_space(10.0);
    Section { idx, top: ui.cursor().top() - 10.0 }
}

pub fn section_end(ui: &mut Ui, s: Section) {
    ui.add_space(6.0);
    let bottom = ui.cursor().top();
    let full = ui.max_rect();
    let r = Rect::from_min_max(pos2(full.left(), s.top), pos2(full.right(), bottom));
    let p = pal(ui.ctx());
    ui.painter().set(s.idx, RectShape::new(r, CornerRadius::same(12), p.card, Stroke::new(1.0_f32, p.border), StrokeKind::Inside));
    ui.add_space(12.0);
}

pub fn section_next(ui: &mut Ui, s: Section) -> Section {
    section_end(ui, s);
    section_begin(ui)
}

/// How far the pointer's hover effect has faded in (0 → 1), animated so the
/// effect comes and goes smoothly.
pub const HOVER_FADE_SECONDS: f32 = 0.18;
pub fn hover_amount(ctx: &Context, id: egui::Id, hovered: bool) -> f32 {
    ctx.animate_bool_with_time(id.with("hover_fade"), hovered, HOVER_FADE_SECONDS)
}

/// A see-through white veil laid over a surface under the pointer: it makes
/// the surface lighter while letting what is behind show through.
pub fn hover_veil(pal: &Pal, amount: f32) -> Color32 {
    let max = if pal.dark { 30.0 } else { 150.0 };
    Color32::from_white_alpha((max * amount.clamp(0.0, 1.0)).round() as u8)
}

/// Mixes two colours (amount 0 = `a`, 1 = `b`).
pub fn mix(a: Color32, b: Color32, amount: f32) -> Color32 {
    let k = amount.clamp(0.0, 1.0);
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * k).round() as u8;
    Color32::from_rgba_premultiplied(f(a.r(), b.r()), f(a.g(), b.g()), f(a.b(), b.b()), f(a.a(), b.a()))
}

/// Draws a shortcut as a row of keycaps centred on `center`.
pub fn keycaps(p: &Painter, center: egui::Pos2, keys: &[String], pal: &Pal) {
    let font = egui::FontId::proportional(13.0);
    let h = 22.0;
    let gap = 4.0;
    let is_drawn = |k: &str| matches!(k, "⌥" | "⇧" | "⌃");
    let widths: Vec<f32> = keys
        .iter()
        .map(|k| if is_drawn(k) { h } else { (p.layout_no_wrap(k.clone(), font.clone(), pal.text).size().x + 12.0).max(h) })
        .collect();
    let total = widths.iter().sum::<f32>() + gap * (widths.len().saturating_sub(1)) as f32;
    let (fill, border) = if pal.dark {
        (Color32::from_white_alpha(22), Color32::from_white_alpha(38))
    } else {
        (Color32::from_white_alpha(230), Color32::from_black_alpha(30))
    };
    let mut x = center.x - total / 2.0;
    for (k, w) in keys.iter().zip(widths) {
        let r = Rect::from_min_size(pos2(x, center.y - h / 2.0), vec2(w, h));
        p.rect_filled(r, 6.0, fill);
        p.rect_stroke(r, 6.0, Stroke::new(1.0_f32, border), StrokeKind::Inside);
        if is_drawn(k) {
            modifier_glyph(p, Rect::from_center_size(r.center(), vec2(10.0, 10.0)), k, pal.text);
        } else {
            p.text(r.center(), egui::Align2::CENTER_CENTER, k, font.clone(), pal.text);
        }
        x += w + gap;
    }
}

/// ⌥ ⇧ ⌃ drawn as lines: the fonts at hand draw them too small next to ⌘.
fn modifier_glyph(p: &Painter, r: Rect, key: &str, c: Color32) {
    let at = |x: f32, y: f32| pos2(r.min.x + x * r.width(), r.min.y + y * r.height());
    let s = Stroke::new(1.4_f32, c);
    match key {
        "⌥" => {
            p.add(egui::Shape::line(vec![at(0.0, 0.15), at(0.32, 0.15), at(0.68, 0.85), at(1.0, 0.85)], s));
            p.line_segment([at(0.6, 0.15), at(1.0, 0.15)], s);
        }
        "⇧" => {
            let pts = vec![at(0.5, 0.02), at(0.98, 0.5), at(0.72, 0.5), at(0.72, 0.95), at(0.28, 0.95), at(0.28, 0.5), at(0.02, 0.5)];
            p.add(egui::Shape::closed_line(pts, s));
        }
        _ => {
            p.add(egui::Shape::line(vec![at(0.12, 0.72), at(0.5, 0.28), at(0.88, 0.72)], s));
        }
    }
}
