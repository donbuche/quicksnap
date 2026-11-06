//! "Capture frozen area": the screen is captured first, shown full size in a
//! borderless window on top, and the person drags out the part to keep. Shows
//! the magnifier and size hints when those Appearance options are on.

use egui::{pos2, vec2, Align2, Color32, Context, FontId, Pos2, Rect, Sense, Stroke, TextureHandle, TextureOptions};
use image::RgbaImage;

/// Magnifier: 15×15 screen pixels shown 8× (estimated).
const MAG_PIXELS: f32 = 15.0;
const MAG_SIZE: f32 = 120.0;

pub struct Overlay {
    pub image: RgbaImage,
    texture: Option<TextureHandle>,
    start: Option<Pos2>,
    sel: Option<Rect>,
    /// None while open; Some(None) when cancelled; Some(Some(rect)) in image pixels.
    pub result: Option<Option<Rect>>,
}

impl Overlay {
    pub fn new(image: RgbaImage) -> Overlay {
        Overlay { image, texture: None, start: None, sel: None, result: None }
    }

    pub fn show(&mut self, ctx: &Context, show_magnifier: bool, show_hints: bool) {
        let tex = self
            .texture
            .get_or_insert_with(|| {
                let (w, h) = self.image.dimensions();
                let ci = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], self.image.as_raw());
                ctx.load_texture("frozen", ci, TextureOptions::NEAREST)
            })
            .clone();
        let (iw, ih) = (self.image.width() as f32, self.image.height() as f32);

        egui::CentralPanel::default().frame(egui::Frame::new()).show(ctx, |ui| {
            let full = ui.max_rect();
            let resp = ui.allocate_rect(full, Sense::click_and_drag());
            let p = ui.painter();
            let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
            p.image(tex.id(), full, uv, Color32::WHITE);
            let to_img = |q: Pos2| pos2((q.x - full.min.x) / full.width() * iw, (q.y - full.min.y) / full.height() * ih);
            ctx.set_cursor_icon(egui::CursorIcon::Crosshair);

            if resp.drag_started() {
                self.start = ctx.input(|i| i.pointer.press_origin()).or(resp.interact_pointer_pos());
            }
            if resp.dragged() {
                if let (Some(a), Some(b)) = (self.start, resp.interact_pointer_pos()) {
                    let b = if ctx.input(|i| i.modifiers.shift) { crate::shapes::square(a, b) } else { b };
                    self.sel = Some(Rect::from_two_pos(a, b).intersect(full));
                }
            }
            let dim = Color32::from_rgba_unmultiplied(0, 0, 0, 90);
            match self.sel {
                Some(s) => {
                    for part in crate::shapes::outside_parts(full, s) {
                        p.rect_filled(part, 0.0, dim);
                    }
                    p.rect_stroke(s, 0.0, Stroke::new(1.0_f32, Color32::WHITE), egui::StrokeKind::Outside);
                    if show_hints {
                        let a = to_img(s.min);
                        let b = to_img(s.max);
                        let label = format!("{} × {}", (b.x - a.x).round(), (b.y - a.y).round());
                        let at = pos2(s.min.x, (s.min.y - 6.0).max(full.min.y + 16.0));
                        badge(p, at, Align2::LEFT_BOTTOM, &label);
                    }
                }
                None => {
                    p.rect_filled(full, 0.0, dim);
                }
            }
            if resp.drag_stopped() {
                if let Some(s) = self.sel {
                    if s.width() >= 3.0 && s.height() >= 3.0 {
                        let a = to_img(s.min);
                        let b = to_img(s.max);
                        self.result = Some(Some(Rect::from_min_max(a, b)));
                    }
                }
                self.start = None;
            }
            if resp.clicked() {
                // a click without dragging keeps the whole screen (estimated)
                self.result = Some(Some(Rect::from_min_size(Pos2::ZERO, vec2(iw, ih))));
            }

            if let Some(hp) = ctx.input(|i| i.pointer.hover_pos()) {
                if self.sel.is_none() || self.start.is_some() {
                    p.line_segment([pos2(full.min.x, hp.y), pos2(full.max.x, hp.y)], Stroke::new(1.0_f32, Color32::from_white_alpha(120)));
                    p.line_segment([pos2(hp.x, full.min.y), pos2(hp.x, full.max.y)], Stroke::new(1.0_f32, Color32::from_white_alpha(120)));
                }
                if show_magnifier {
                    let ip = to_img(hp);
                    let half = MAG_PIXELS / 2.0;
                    let src = Rect::from_min_max(
                        pos2((ip.x - half) / iw, (ip.y - half) / ih),
                        pos2((ip.x + half) / iw, (ip.y + half) / ih),
                    );
                    let mut box_min = hp + vec2(20.0, 20.0);
                    if box_min.x + MAG_SIZE > full.max.x {
                        box_min.x = hp.x - 20.0 - MAG_SIZE;
                    }
                    if box_min.y + MAG_SIZE + 22.0 > full.max.y {
                        box_min.y = hp.y - 20.0 - MAG_SIZE - 22.0;
                    }
                    let mag = Rect::from_min_size(box_min, vec2(MAG_SIZE, MAG_SIZE));
                    p.rect_filled(mag.expand(2.0), 4.0, Color32::from_gray(30));
                    p.image(tex.id(), mag, src, Color32::WHITE);
                    let c = mag.center();
                    let cell = MAG_SIZE / MAG_PIXELS;
                    p.rect_stroke(Rect::from_center_size(c, vec2(cell, cell)), 0.0, Stroke::new(1.0_f32, Color32::RED), egui::StrokeKind::Middle);
                    p.rect_stroke(mag, 4.0, Stroke::new(1.0_f32, Color32::WHITE), egui::StrokeKind::Outside);
                    let px = ip.x.clamp(0.0, iw - 1.0) as u32;
                    let py = ip.y.clamp(0.0, ih - 1.0) as u32;
                    let col = self.image.get_pixel(px, py);
                    let info = format!("{}, {}   #{:02X}{:02X}{:02X}", px, py, col[0], col[1], col[2]);
                    badge(p, pos2(mag.min.x, mag.max.y + 4.0), Align2::LEFT_TOP, &info);
                }
            }
            if show_hints && self.sel.is_none() {
                badge(
                    p,
                    pos2(full.center().x, full.min.y + 40.0),
                    Align2::CENTER_TOP,
                    "Drag to select an area · Click to keep the whole screen · Hold ⇧ for a square · Esc to cancel",
                );
            }
        });
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.result = Some(None);
        }
        ctx.request_repaint();
    }
}

fn badge(p: &egui::Painter, at: Pos2, align: Align2, text: &str) {
    let galley = p.layout_no_wrap(text.to_string(), FontId::proportional(12.0), Color32::WHITE);
    let r = align.anchor_size(at, galley.size() + vec2(12.0, 6.0));
    p.rect_filled(r, 4.0, Color32::from_black_alpha(190));
    p.galley(r.min + vec2(6.0, 3.0), galley, Color32::WHITE);
}
