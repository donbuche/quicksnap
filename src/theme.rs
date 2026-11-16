//! Light and dark colour sets plus the "glass" look: rounded, see-through,
//! nearly flat panels (only a faint shadow) with a soft sheen, floating over an "aurora" backdrop (a colour
//! gradient with glowing blobs) through which, on macOS, the desktop shows
//! blurred.

use std::sync::atomic::{AtomicBool, Ordering};

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
    /// The accent is a gradient from `accent` (violet) to `accent2`, the blue
    /// of the aurora's lower-left glow.
    pub accent2: Color32,
    /// Under the pointer things light up in a turquoise gradient (the colour
    /// of the aurora's lower glow), from `hover_a` to `hover_b`.
    pub hover_a: Color32,
    pub hover_b: Color32,
    pub accent_soft: Color32,
    pub control: Color32,
    pub hover: Color32,
    pub shadow: Color32,
    /// Light along the top edge of a glass panel (alpha 0-255).
    pub sheen: u8,
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
    bg: Color32::from_rgb(0xf3, 0xf1, 0xfb),
    glass: rgba(255, 255, 255, 150),
    glass_strong: rgba(252, 251, 255, 220),
    card: rgba(255, 255, 255, 165),
    border: rgba(70, 50, 140, 30),
    text: Color32::from_rgb(0x1d, 0x1b, 0x2b),
    hint: Color32::from_rgb(0x6b, 0x67, 0x80),
    accent: Color32::from_rgb(0x74, 0x52, 0xf5),
    accent2: Color32::from_rgb(0x4b, 0x5c, 0xe8),
    hover_a: Color32::from_rgb(0x0e, 0xa5, 0xb9),
    hover_b: Color32::from_rgb(0x22, 0xc8, 0xe0),
    accent_soft: rgba(116, 82, 245, 38),
    control: rgba(60, 40, 130, 16),
    hover: rgba(60, 40, 130, 28),
    shadow: rgba(60, 30, 120, 16),
    sheen: 150,
};

pub const DARK: Pal = Pal {
    dark: true,
    bg: Color32::from_rgb(0x15, 0x12, 0x26),
    glass: rgba(34, 28, 60, 140),
    glass_strong: rgba(32, 27, 54, 225),
    card: rgba(255, 255, 255, 16),
    border: rgba(255, 255, 255, 30),
    text: Color32::from_rgb(0xf4, 0xf2, 0xff),
    hint: Color32::from_rgb(0xa8, 0xa3, 0xc4),
    accent: Color32::from_rgb(0x8b, 0x6c, 0xff),
    accent2: Color32::from_rgb(0x4f, 0x5b, 0xf5),
    hover_a: Color32::from_rgb(0x12, 0xb5, 0xc8),
    hover_b: Color32::from_rgb(0x2a, 0xd8, 0xee),
    accent_soft: rgba(139, 108, 255, 55),
    control: rgba(255, 255, 255, 18),
    hover: rgba(255, 255, 255, 30),
    shadow: rgba(0, 0, 0, 70),
    sheen: 34,
};

/// Height of the macOS title bar. The main window has no visible title bar:
/// the app is drawn under it and only the red-yellow-green buttons float on
/// top, so nothing clickable may sit in this band.
pub const TITLEBAR: f32 = if cfg!(target_os = "macos") { 28.0 } else { 0.0 };

/// Whether the main window is see-through, with the desktop showing blurred
/// behind it (macOS vibrancy). Without it the aurora is painted opaque.
static VIBRANT: AtomicBool = AtomicBool::new(false);

pub fn set_vibrant(on: bool) {
    VIBRANT.store(on, Ordering::Relaxed);
}

pub fn vibrant() -> bool {
    VIBRANT.load(Ordering::Relaxed)
}

/// Makes the main window frosted glass: the desktop shows blurred behind it.
/// Returns whether it worked.
///
/// The blur view goes *behind* the window's content view, as a sibling in
/// the window's frame view. egui draws straight into the content view, so a
/// blur view placed inside it (as `window-vibrancy` does, which suits web
/// views) would cover the whole interface.
#[cfg(target_os = "macos")]
pub fn apply_vibrancy(cc: &eframe::CreationContext<'_>) -> bool {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{
        NSAutoresizingMaskOptions, NSView, NSVisualEffectBlendingMode, NSVisualEffectMaterial, NSVisualEffectState, NSVisualEffectView,
        NSWindowOrderingMode,
    };
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    let Ok(handle) = cc.window_handle() else { return false };
    let RawWindowHandle::AppKit(h) = handle.as_raw() else { return false };
    let Some(mtm) = MainThreadMarker::new() else { return false };
    // SAFETY: the handle points at the live content view of this window, and
    // we are on the main thread.
    let content: &NSView = unsafe { h.ns_view.cast::<NSView>().as_ref() };
    let Some(frame_view) = (unsafe { content.superview() }) else { return false };
    let blur = NSVisualEffectView::initWithFrame(mtm.alloc(), frame_view.bounds());
    blur.setMaterial(NSVisualEffectMaterial::UnderWindowBackground);
    blur.setBlendingMode(NSVisualEffectBlendingMode::BehindWindow);
    blur.setState(NSVisualEffectState::Active);
    blur.setAutoresizingMask(NSAutoresizingMaskOptions::ViewWidthSizable | NSAutoresizingMaskOptions::ViewHeightSizable);
    frame_view.addSubview_positioned_relativeTo(&blur, NSWindowOrderingMode::Below, Some(content));
    true
}

pub fn pal(ctx: &Context) -> Pal {
    if ctx.style().visuals.dark_mode { DARK } else { LIGHT }
}

/// Rounded corners, softer shadows and accent colour for both modes.
pub fn setup_styles(ctx: &Context) {
    for (theme, p) in [(egui::Theme::Light, LIGHT), (egui::Theme::Dark, DARK)] {
        ctx.style_mut_of(theme, |s| {
            let v = &mut s.visuals;
            v.panel_fill = p.bg;
            v.faint_bg_color = p.control;
            v.window_fill = p.glass_strong;
            v.extreme_bg_color = if p.dark { Color32::from_rgb(0x24, 0x1f, 0x3a) } else { Color32::WHITE };
            v.window_corner_radius = CornerRadius::same(16);
            v.menu_corner_radius = CornerRadius::same(10);
            v.window_stroke = Stroke::new(1.0_f32, p.border);
            v.window_shadow = FLAT_SHADOW.with_color(p.shadow);
            v.popup_shadow = FLAT_SHADOW.with_color(p.shadow);
            v.selection.bg_fill = p.accent;
            v.selection.stroke = Stroke::new(1.0_f32, Color32::WHITE);
            v.hyperlink_color = p.accent;
            // Text is the palette's text colour everywhere, except on selected
            // controls (filled with the accent), where it is white: dark text
            // on the violet would not reach a 4.5:1 contrast. (Not done with
            // `override_text_color`, which would win over the selection's.)
            v.override_text_color = None;
            let r = CornerRadius::same(9);
            for w in [&mut v.widgets.noninteractive, &mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.open] {
                w.corner_radius = r;
                w.fg_stroke.color = p.text;
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
            s.spacing.slider_rail_height = 4.0;
        });
    }
}

/// The one shadow of the (nearly flat) interface: barely there, just enough
/// to lift a panel off what is behind it.
const FLAT_SHADOW: Shadow = Shadow { offset: [0, 1], blur: 4, spread: 0, color: Color32::TRANSPARENT };

trait WithColor {
    fn with_color(self, c: Color32) -> Self;
}

impl WithColor for Shadow {
    fn with_color(self, color: Color32) -> Self {
        Shadow { color, ..self }
    }
}

/// Faint drop shadow behind a rounded rectangle.
pub fn shadow(p: &Painter, r: Rect, radius: u8, pal: &Pal) {
    p.add(FLAT_SHADOW.with_color(pal.shadow).as_shape(r, CornerRadius::same(radius)));
}

/// A coloured glow (the accent) around a rounded rectangle.
pub fn glow(p: &Painter, r: Rect, radius: u8, pal: &Pal, amount: f32) {
    if amount <= 0.01 {
        return;
    }
    let c = mix(pal.accent, pal.accent2, 0.5).gamma_multiply(if pal.dark { 0.25 } else { 0.15 } * amount);
    let sh = Shadow { offset: [0, 1], blur: 6, spread: 0, color: c };
    p.add(sh.as_shape(r, CornerRadius::same(radius)));
}

/// A turquoise glow (the hover colour) around a rounded rectangle.
pub fn hover_glow(p: &Painter, r: Rect, radius: u8, pal: &Pal, amount: f32) {
    if amount <= 0.01 {
        return;
    }
    let c = mix(pal.hover_a, pal.hover_b, 0.5).gamma_multiply(if pal.dark { 0.25 } else { 0.18 } * amount);
    let sh = Shadow { offset: [0, 1], blur: 6, spread: 0, color: c };
    p.add(sh.as_shape(r, CornerRadius::same(radius)));
}

/// A rounded rectangle filled with the turquoise hover gradient.
pub fn hover_fill(p: &Painter, r: Rect, radius: f32, pal: &Pal, alpha: f32) {
    if alpha <= 0.0 {
        return;
    }
    let (a, b) = (pal.hover_a, pal.hover_b);
    let tex = cached_tex(p.ctx(), &format!("hover-{}", pal.dark), || gradient_image(a, b, true));
    let tint = Color32::WHITE.gamma_multiply(alpha.clamp(0.0, 1.0));
    p.add(RectShape::filled(r, radius, tint).with_texture(tex, UV_FULL));
}

/// A small texture made once per context and kept in its memory.
fn cached_tex(ctx: &Context, name: &str, make: impl FnOnce() -> egui::ColorImage) -> TextureId {
    let id = egui::Id::new(("quicksnap-tex", name));
    if let Some(t) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(id)) {
        return t.id();
    }
    let t = ctx.load_texture(name, make(), egui::TextureOptions::LINEAR);
    let tid = t.id();
    ctx.data_mut(|d| d.insert_temp(id, t));
    tid
}

const UV_FULL: Rect = Rect { min: pos2(0.0, 0.0), max: pos2(1.0, 1.0) };

/// A linear gradient texture between two colours: diagonal (top-left to
/// bottom-right) or vertical.
fn gradient_image(a: Color32, b: Color32, diagonal: bool) -> egui::ColorImage {
    let n = 64usize;
    let mut px = Vec::with_capacity(n * n);
    for y in 0..n {
        for x in 0..n {
            let k = if diagonal { (x + y) as f32 / (2 * n - 2) as f32 } else { y as f32 / (n - 1) as f32 };
            px.push(mix(a, b, k));
        }
    }
    egui::ColorImage { size: [n, n], source_size: vec2(n as f32, n as f32), pixels: px }
}

fn accent_tex(ctx: &Context, pal: &Pal) -> TextureId {
    let (a, b) = (pal.accent, pal.accent2);
    cached_tex(ctx, &format!("accent-{}", pal.dark), || gradient_image(a, b, true))
}

/// A rounded rectangle filled with the accent gradient.
pub fn accent_fill(p: &Painter, r: Rect, radius: f32, pal: &Pal, alpha: f32) {
    let tex = accent_tex(p.ctx(), pal);
    let tint = Color32::WHITE.gamma_multiply(alpha.clamp(0.0, 1.0));
    p.add(RectShape::filled(r, radius, tint).with_texture(tex, UV_FULL));
}

/// Light falling on the top part of a glass surface, fading out downwards.
fn sheen_shape(ctx: &Context, r: Rect, radius: u8, pal: &Pal) -> egui::Shape {
    let tex = cached_tex(ctx, "sheen", || gradient_image(Color32::WHITE, Color32::TRANSPARENT, false));
    let tint = Color32::WHITE.gamma_multiply(pal.sheen as f32 / 255.0);
    // only the upper part of the panel catches the light
    let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, (r.height() / 38.0).max(1.0)));
    RectShape::filled(r.shrink(1.0), (radius as f32 - 1.0).max(0.0), tint).with_texture(tex, uv).into()
}

/// How many shapes [`glass_shapes`] can return (for reserving slots).
const GLASS_SHAPES_MAX: usize = 8;

/// Shapes of a frosted-glass panel: a soft shadow, what is behind it (if
/// given), a see-through tint, a sheen along the top and a hairline border,
/// all rounded.
pub fn glass_shapes(ctx: &Context, r: Rect, radius: u8, pal: &Pal, backdrops: &[Backdrop]) -> Vec<egui::Shape> {
    let mut v: Vec<egui::Shape> = Vec::new();
    v.push(FLAT_SHADOW.with_color(pal.shadow).as_shape(r, CornerRadius::same(radius)).into());
    for b in backdrops.iter().take(GLASS_SHAPES_MAX - 4) {
        v.push(textured_rounded(r, radius as f32, b));
    }
    v.push(RectShape::filled(r, CornerRadius::same(radius), pal.glass).into());
    v.push(sheen_shape(ctx, r, radius, pal));
    v.push(RectShape::stroke(r, CornerRadius::same(radius), Stroke::new(1.0_f32, pal.border), StrokeKind::Inside).into());
    v
}

pub fn glass(p: &Painter, r: Rect, radius: u8, pal: &Pal, backdrops: &[Backdrop]) {
    for s in glass_shapes(p.ctx(), r, radius, pal, backdrops) {
        p.add(s);
    }
}

/// A button filled with the accent gradient and white text.
pub fn gradient_button(ui: &mut Ui, text: &str, min: egui::Vec2) -> egui::Response {
    let pal = pal(ui.ctx());
    let slot = ui.painter().add(egui::Shape::Noop);
    let resp = ui.add(
        egui::Button::new(egui::RichText::new(text).color(Color32::WHITE).strong())
            .fill(Color32::TRANSPARENT)
            .stroke(Stroke::NONE)
            .min_size(min)
            .corner_radius(9.0),
    );
    let hov = hover_amount(ui.ctx(), resp.id, resp.hovered());
    let r = resp.rect;
    let mut shapes: Vec<egui::Shape> = Vec::new();
    let rest = mix(pal.accent, pal.accent2, 0.5);
    let c = mix(rest, mix(pal.hover_a, pal.hover_b, 0.5), hov).gamma_multiply(if pal.dark { 0.25 } else { 0.15 } * (0.6 + 0.4 * hov));
    shapes.push(FLAT_SHADOW.with_color(c).as_shape(r, CornerRadius::same(9)).into());
    let tex = accent_tex(ui.ctx(), &pal);
    shapes.push(RectShape::filled(r, 9.0, Color32::WHITE).with_texture(tex, UV_FULL).into());
    if hov > 0.0 {
        let (ha, hb) = (pal.hover_a, pal.hover_b);
        let htex = cached_tex(ui.ctx(), &format!("hover-{}", pal.dark), || gradient_image(ha, hb, true));
        shapes.push(RectShape::filled(r, 9.0, Color32::WHITE.gamma_multiply(hov)).with_texture(htex, UV_FULL).into());
    }
    if resp.is_pointer_button_down_on() {
        shapes.push(RectShape::filled(r, 9.0, Color32::from_black_alpha(30)).into());
    }
    ui.painter().set(slot, egui::Shape::Vec(shapes));
    resp
}

/// A blurred picture stretched over `area` (showing the `uv` part of it).
#[derive(Clone, Copy)]
pub struct Backdrop {
    pub tex: TextureId,
    pub area: Rect,
    pub uv: Rect,
    /// Multiplies the picture (white = as it is).
    pub tint: Color32,
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
        Backdrop { tex, area, uv, tint: Color32::WHITE }
    }

    /// The same backdrop, faded to `alpha` (0-1).
    pub fn faded(self, alpha: f32) -> Backdrop {
        Backdrop { tint: Color32::WHITE.gamma_multiply(alpha), ..self }
    }

    pub fn paint(&self, p: &Painter) {
        let mut mesh = egui::Mesh::with_texture(self.tex);
        mesh.add_rect_with_uv(self.area, self.uv, self.tint);
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
    mesh.vertices.push(egui::epaint::Vertex { pos: c, uv: uv(c), color: b.tint });
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
            mesh.vertices.push(egui::epaint::Vertex { pos: p, uv: uv(p), color: b.tint });
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
    backdrops: &[Backdrop],
    add: impl FnOnce(&mut Ui) -> R,
) -> egui::InnerResponse<R> {
    egui::Area::new(egui::Id::new(id)).pivot(pivot).fixed_pos(pos).order(egui::Order::Middle).show(ctx, |ui| {
        let slots: Vec<_> = (0..GLASS_SHAPES_MAX).map(|_| ui.painter().add(egui::Shape::Noop)).collect();
        let inner = egui::Frame::new().inner_margin(egui::Margin::symmetric(8, 6)).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                add(ui)
            })
            .inner
        });
        let rect = inner.response.rect;
        for (slot, shape) in slots.into_iter().zip(glass_shapes(ctx, rect, 16, pal, backdrops)) {
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

/// The aurora: a diagonal gradient with soft glowing colour blobs. With
/// `see_through` it is partly transparent so the (blurred) desktop shows.
pub fn aurora_image(dark: bool, see_through: bool) -> egui::ColorImage {
    let (w, h) = (192usize, 120usize);
    let (base_a, base_b) = if dark {
        ([20.0f32, 15.0, 42.0], [10.0f32, 22.0, 48.0])
    } else {
        ([246.0f32, 242.0, 255.0], [236.0f32, 245.0, 255.0])
    };
    // x, y, radius, colour
    let blobs: [(f32, f32, f32, [f32; 3]); 5] = if dark {
        [
            (0.12, 0.08, 0.55, [124.0, 92.0, 255.0]),
            (0.92, 0.12, 0.50, [240.0, 72.0, 200.0]),
            (0.82, 0.98, 0.55, [34.0, 200.0, 238.0]),
            (0.05, 0.95, 0.45, [79.0, 70.0, 229.0]),
            (0.50, 0.55, 0.40, [150.0, 60.0, 220.0]),
        ]
    } else {
        [
            (0.12, 0.08, 0.55, [167.0, 139.0, 250.0]),
            (0.92, 0.12, 0.50, [244.0, 143.0, 214.0]),
            (0.82, 0.98, 0.55, [125.0, 211.0, 252.0]),
            (0.05, 0.95, 0.45, [165.0, 180.0, 252.0]),
            (0.50, 0.55, 0.40, [216.0, 180.0, 254.0]),
        ]
    };
    let strength = if dark { 0.42 } else { 0.55 };
    let base_alpha = match (see_through, dark) {
        (false, _) => 1.0,
        (true, true) => 0.70,
        (true, false) => 0.72,
    };
    let mut px = Vec::with_capacity(w * h);
    for y in 0..h {
        for x in 0..w {
            let fx = x as f32 / (w - 1) as f32;
            let fy = y as f32 / (h - 1) as f32;
            let k = (fx + fy) / 2.0;
            let mut c = [0.0f32; 3];
            for i in 0..3 {
                c[i] = base_a[i] + (base_b[i] - base_a[i]) * k;
            }
            let mut alpha = base_alpha;
            for (bx, by, r, col) in blobs {
                let d = (((fx - bx) * 1.6).powi(2) + (fy - by).powi(2)).sqrt();
                let a = (1.0 - d / r).clamp(0.0, 1.0).powf(1.8) * strength;
                for i in 0..3 {
                    c[i] = c[i] * (1.0 - a) + col[i] * a;
                }
                alpha = alpha + (1.0 - alpha) * a * 0.6;
            }
            px.push(Color32::from_rgba_unmultiplied(c[0] as u8, c[1] as u8, c[2] as u8, (alpha * 255.0) as u8));
        }
    }
    egui::ColorImage { size: [w, h], source_size: vec2(w as f32, h as f32), pixels: px }
}

/// Paints the aurora over `area` and returns it as a backdrop (for glass
/// panels that need to hide what is under them). `see_through` only takes
/// effect when the window really is see-through.
pub fn background(p: &Painter, area: Rect, pal: &Pal, see_through: bool) -> Backdrop {
    let st = see_through && vibrant();
    let key = format!("aurora-{}-{}", pal.dark, st);
    let tex = cached_tex(p.ctx(), &key, || aurora_image(pal.dark, st));
    let bd = Backdrop::cover(tex, area, vec2(192.0, 120.0));
    bd.paint(p);
    bd
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
