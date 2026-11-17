//! "Capture Area": QuickSnap's own selection layer over every display. The
//! screen stays live (nothing is frozen): a see-through window covers each
//! display, the person drags out an area or clicks, and the system capture
//! tool then takes exactly that rectangle or window.
//!
//! A click without dragging takes the window under the pointer when "Click
//! to snap a window" is on (that window is highlighted beforehand), or the
//! whole display that was clicked.
//!
//! macOS sends mouse moves only to the key window, so the pointer position
//! is read from the window server every frame; presses and releases come
//! from whichever selector window got them.

use crate::capture::Target;
use crate::i18n::t;
use crate::screens::{self, Display, ScreenRect};
use egui::{pos2, vec2, Align2, Color32, Context, FontId, Pos2, Rect, Stroke, ViewportBuilder, ViewportId};

/// Below this many points of movement a press is a click, not a drag.
const CLICK_SLOP: f64 = 3.0;

/// What the person chose.
pub struct Choice {
    pub target: Target,
    /// Pixels per point of the display it is on (for "shrink Retina").
    pub scale: f32,
    /// The selection in pixels of the main display, for "Capture previous
    /// area" (only for a dragged area on the main display).
    pub previous_area: Option<Rect>,
}

pub struct Selector {
    displays: Vec<Display>,
    /// Whether each display's window was already raised above everything.
    raised: Vec<bool>,
    /// Pixels per point of each display, as its window reports it.
    scales: Vec<f32>,
    /// Display index and global start point of the press in progress.
    press: Option<(usize, (f64, f64))>,
    pointer: Option<(f64, f64)>,
    /// The window a click would snap, refreshed when the pointer moves.
    hover: Option<(u32, ScreenRect)>,
    hover_at: Option<(f64, f64)>,
    /// None while open; Some(None) when cancelled.
    pub result: Option<Option<Choice>>,
}

impl Selector {
    pub fn new() -> Selector {
        let displays = screens::displays();
        let n = displays.len();
        Selector {
            displays,
            raised: vec![false; n],
            scales: vec![1.0; n],
            press: None,
            pointer: screens::pointer(),
            hover: None,
            hover_at: None,
            result: None,
        }
    }

    fn display_at(&self, p: (f64, f64)) -> Option<usize> {
        self.displays.iter().position(|d| d.rect.contains(p.0, p.1))
    }

    pub fn show(&mut self, ctx: &Context, snap: bool, hints: bool) {
        if self.displays.is_empty() {
            self.result = Some(None);
            return;
        }
        self.pointer = screens::pointer().or(self.pointer);
        if snap && self.press.is_none() && self.pointer != self.hover_at {
            self.hover_at = self.pointer;
            self.hover = self.pointer.and_then(|p| {
                let screen = self.display_at(p).map(|i| self.displays[i].rect);
                screens::window_at(p.0, p.1, screen)
            });
        }
        if screens::escape_down() {
            self.result = Some(None);
        }
        let under = self.pointer.and_then(|p| self.display_at(p));
        for i in 0..self.displays.len() {
            let d = self.displays[i].rect;
            let title = format!("QuickSnap Selector {}", i + 1);
            ctx.show_viewport_immediate(
                ViewportId::from_hash_of(("area_selector", i)),
                ViewportBuilder::default()
                    .with_title(title.clone())
                    .with_decorations(false)
                    .with_transparent(true)
                    .with_has_shadow(false)
                    .with_resizable(false)
                    .with_clamp_size_to_monitor_size(false)
                    .with_position([d.x as f32, d.y as f32])
                    .with_inner_size([d.w as f32, d.h as f32])
                    .with_window_level(egui::WindowLevel::AlwaysOnTop)
                    .with_active(under == Some(i)),
                |vctx, _| {
                    self.viewport(vctx, i, snap, hints);
                    if vctx.input(|i| i.viewport().close_requested()) {
                        self.result = Some(None);
                    }
                },
            );
            if !self.raised[i] {
                self.raised[i] = screens::raise_selector_window(&title, d);
            }
        }
        ctx.request_repaint();
    }

    fn viewport(&mut self, vctx: &Context, i: usize, snap: bool, hints: bool) {
        let d = self.displays[i].rect;
        self.scales[i] = vctx.input(|inp| inp.viewport().native_pixels_per_point).unwrap_or(vctx.pixels_per_point());
        let to_global = |p: Pos2| (d.x + p.x as f64, d.y + p.y as f64);
        let to_local = |p: (f64, f64)| pos2((p.0 - d.x) as f32, (p.1 - d.y) as f32);

        let (pressed, released, origin, esc, shift) = vctx.input(|inp| {
            (
                inp.pointer.primary_pressed(),
                inp.pointer.primary_released(),
                inp.pointer.press_origin(),
                inp.key_pressed(egui::Key::Escape),
                inp.modifiers.shift,
            )
        });
        if esc {
            self.result = Some(None);
        }
        if pressed && self.press.is_none() {
            let start = origin.map(to_global).or(self.pointer);
            if let Some(s) = start {
                self.press = Some((i, s));
            }
        }
        if released {
            if let Some((pi, start)) = self.press.take() {
                let end = vctx.input(|inp| inp.pointer.latest_pos()).map(to_global).or(self.pointer).unwrap_or(start);
                self.finish(pi, start, end, shift, snap);
            }
        }

        // the selection while dragging, on the display where it started
        let sel = self.press.filter(|(pi, _)| *pi == i).and_then(|(_, start)| {
            let end = self.pointer?;
            let moved = (end.0 - start.0).abs().max((end.1 - start.1).abs());
            (moved >= CLICK_SLOP).then(|| selection(d, start, end, shift))
        });

        vctx.set_cursor_icon(egui::CursorIcon::Crosshair);
        egui::CentralPanel::default().frame(egui::Frame::NONE).show(vctx, |ui| {
            let full = Rect::from_min_size(Pos2::ZERO, vec2(d.w as f32, d.h as f32));
            let p = ui.painter();
            // never fully see-through, or clicks would fall through to the apps
            p.rect_filled(full, 0.0, Color32::from_black_alpha(1));
            let dim = Color32::from_black_alpha(90);
            match sel {
                Some(s) => {
                    let s = Rect::from_min_max(to_local((s.x, s.y)), to_local((s.x + s.w, s.y + s.h)));
                    for part in crate::shapes::outside_parts(full, s) {
                        p.rect_filled(part, 0.0, dim);
                    }
                    p.rect_stroke(s, 0.0, Stroke::new(1.0_f32, Color32::WHITE), egui::StrokeKind::Outside);
                    if hints {
                        let k = self.scales[i];
                        let label = format!("{} × {}", (s.width() * k).round(), (s.height() * k).round());
                        let at = pos2(s.min.x, (s.min.y - 6.0).max(full.min.y + 16.0));
                        badge(p, at, Align2::LEFT_BOTTOM, &label);
                    }
                }
                None if self.press.is_none() => {
                    if let Some((_, w)) = self.hover.filter(|_| snap) {
                        let r = Rect::from_min_max(to_local((w.x, w.y)), to_local((w.x + w.w, w.y + w.h))).intersect(full);
                        if r.is_positive() {
                            let blue = Color32::from_rgb(10, 132, 255);
                            p.rect_filled(r, 0.0, blue.gamma_multiply(0.18));
                            p.rect_stroke(r, 0.0, Stroke::new(2.0_f32, blue), egui::StrokeKind::Inside);
                        }
                    }
                    if let Some(ptr) = self.pointer.filter(|q| d.contains(q.0, q.1)) {
                        let hp = to_local(ptr);
                        let line = Stroke::new(1.0_f32, Color32::from_white_alpha(120));
                        p.line_segment([pos2(full.min.x, hp.y), pos2(full.max.x, hp.y)], line);
                        p.line_segment([pos2(hp.x, full.min.y), pos2(hp.x, full.max.y)], line);
                        if hints {
                            let text = if snap {
                                t("Drag to select an area · Click to capture a window · Hold ⇧ for a square · Esc to cancel")
                            } else {
                                t("Drag to select an area · Click to keep the whole screen · Hold ⇧ for a square · Esc to cancel")
                            };
                            badge(p, pos2(full.center().x, full.min.y + 40.0), Align2::CENTER_TOP, text);
                        }
                    }
                }
                None => {}
            }
        });
        vctx.request_repaint();
    }

    fn finish(&mut self, i: usize, start: (f64, f64), end: (f64, f64), shift: bool, snap: bool) {
        let d = self.displays[i];
        let scale = self.scales[i];
        let moved = (end.0 - start.0).abs().max((end.1 - start.1).abs());
        let choice = if moved < CLICK_SLOP {
            let window = if snap { screens::window_at(start.0, start.1, Some(d.rect)) } else { None };
            match window {
                Some((id, _)) => Choice { target: Target::Window(id), scale, previous_area: None },
                None => Choice { target: Target::Rect(d.rect), scale, previous_area: None },
            }
        } else {
            let s = selection(d.rect, start, end, shift);
            let previous_area = d.main.then(|| {
                Rect::from_min_size(
                    pos2(((s.x - d.rect.x) as f32) * scale, ((s.y - d.rect.y) as f32) * scale),
                    vec2(s.w as f32 * scale, s.h as f32 * scale),
                )
            });
            Choice { target: Target::Rect(s), scale, previous_area }
        };
        self.result = Some(Some(choice));
    }
}

/// The dragged rectangle, kept on display `d` and squared with ⇧.
fn selection(d: ScreenRect, start: (f64, f64), end: (f64, f64), square: bool) -> ScreenRect {
    let (mut dx, mut dy) = (end.0 - start.0, end.1 - start.1);
    if square {
        let side = dx.abs().max(dy.abs());
        dx = side.copysign(dx);
        dy = side.copysign(dy);
    }
    let clamp = |v: f64, lo: f64, hi: f64| v.max(lo).min(hi);
    let (x0, x1) = (start.0.min(start.0 + dx), start.0.max(start.0 + dx));
    let (y0, y1) = (start.1.min(start.1 + dy), start.1.max(start.1 + dy));
    let (x0, x1) = (clamp(x0, d.x, d.x + d.w), clamp(x1, d.x, d.x + d.w));
    let (y0, y1) = (clamp(y0, d.y, d.y + d.h), clamp(y1, d.y, d.y + d.h));
    ScreenRect { x: x0, y: y0, w: (x1 - x0).max(1.0), h: (y1 - y0).max(1.0) }
}

fn badge(p: &egui::Painter, at: Pos2, align: Align2, text: &str) {
    let galley = p.layout_no_wrap(text.to_string(), FontId::proportional(12.0), Color32::WHITE);
    let r = align.anchor_size(at, galley.size() + vec2(12.0, 6.0));
    p.rect_filled(r, 4.0, Color32::from_black_alpha(190));
    p.galley(r.min + vec2(6.0, 3.0), galley, Color32::WHITE);
}
