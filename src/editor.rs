//! The image editor. The picture sits on a blurred copy of itself; floating
//! glass panels hold the tools (top left), the actions (top right), the
//! style bar (bottom centre: colour, size, text size and style, shadow) and
//! the zoom controls (bottom right).
//!
//! The style bar always changes the selected item, and also becomes the
//! default for new items. Text is typed in place; the Select tool picks
//! up, moves, re-edits (double-click) or deletes anything already added.

use egui::{
    pos2, vec2, Align2, Color32, Context, CornerRadius, Event, FontFamily, FontId, Frame, Id, Key, Margin, Modifiers,
    Painter, Pos2, Rect, RichText, Sense, Shape as EShape, Stroke, TextureHandle, TextureOptions, Ui,
};
use image::RgbaImage;
use std::path::PathBuf;

use crate::icons::{self, Icon};
use crate::settings::{Settings, LINE_WIDTH_MAX, LINE_WIDTH_MIN};
use crate::shapes::{self, AnnotFont, Kind, Prim, Shape, TextStyle, TEXT_SIZE_MAX, TEXT_SIZE_MIN};
use crate::theme::{self, Backdrop, Pal};

/// Palette from the original's tool-preferences panel, plus pure red which
/// is the starting colour.
pub const PALETTE: [u32; 10] =
    [0xff0000, 0xe504c3, 0xe50404, 0x53e504, 0x1f63be, 0xffea00, 0xff7200, 0x898989, 0x000000, 0xffffff];

pub const BLUR_RADIUS: i32 = 8;
/// Crop highlighter: black at 30 % outside the crop (0x100775c28, colour
/// constant 0x100904c20 = 0, 0, 0, 0.3).
pub const CROP_DIM_ALPHA: u8 = 77;
pub const UNDO_LIMIT: usize = 40;
#[allow(dead_code)]
pub const ACCENT: Color32 = Color32::from_rgb(0x00, 0x7a, 0xff);

pub fn hex(c: u32) -> Color32 {
    Color32::from_rgb((c >> 16) as u8, (c >> 8) as u8, c as u8)
}

fn color_of(c: [u8; 4]) -> Color32 {
    Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3])
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tool {
    Select,
    Arrow,
    Rect,
    Line,
    Ellipse,
    Pen,
    Text,
    Bubble,
    Blur,
    Crop,
}

impl Tool {
    pub const DRAW: [Tool; 9] =
        [Tool::Select, Tool::Arrow, Tool::Rect, Tool::Line, Tool::Ellipse, Tool::Pen, Tool::Text, Tool::Bubble, Tool::Blur];

    fn icon(self) -> Icon {
        match self {
            Tool::Select => Icon::Select,
            Tool::Arrow => Icon::Arrow,
            Tool::Rect => Icon::Rect,
            Tool::Line => Icon::Line,
            Tool::Ellipse => Icon::Ellipse,
            Tool::Pen => Icon::Pen,
            Tool::Text => Icon::Text,
            Tool::Bubble => Icon::Bubble,
            Tool::Blur => Icon::Blur,
            Tool::Crop => Icon::Crop,
        }
    }

    fn name(self) -> &'static str {
        self.tip().0
    }

    /// Title, shortcut and hint, from the original's tooltip and hotkey texts.
    fn tip(self) -> (&'static str, &'static str, &'static str) {
        match self {
            Tool::Select => (
                "Select & move",
                "V",
                "Click an item to select it, drag to move it\nDouble-click a text to edit it · Delete removes it",
            ),
            Tool::Arrow => ("Arrow", "⌘1", "Hold ⇧ to draw at 45°\nHold ⌘ to draw from arrow’s head to tail"),
            Tool::Rect => (
                "Rectangle",
                "⌘2",
                "Hold ⇧ to draw a square\nHold ⌥ to dark all except selected area\nHold ⌘ to draw filled rectangle",
            ),
            Tool::Line => ("Line", "⌘3", "Hold ⇧ to draw at 45°"),
            Tool::Ellipse => ("Ellipse", "⌘4", "Hold ⇧ to draw a circle\nHold ⌘ to draw filled ellipse"),
            Tool::Pen => ("Pen", "⌘5", "Hold ⌘ to convert into highlighter"),
            Tool::Text => ("Text", "⌘6", "Click to type · click an existing text to edit it"),
            Tool::Bubble => ("Text with arrow", "⌘7", "Drag from the point to where the note goes, then type"),
            Tool::Blur => ("Blur", "⌘8", "Hold ⇧ to blur a square area"),
            Tool::Crop => ("Crop", "", "Hold ⇧ for a square\nEnter to apply, Esc to cancel"),
        }
    }
}

#[derive(Clone)]
struct Doc {
    image: RgbaImage,
    shapes: Vec<Shape>,
}

/// Things the editor asks the app to do.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cmd {
    Save,
    Copy,
    External,
    PasteImage,
    Home,
}

pub struct Editor {
    doc: Doc,
    undo: Vec<Doc>,
    redo: Vec<Doc>,
    texture: Option<TextureHandle>,
    backdrop_tex: Option<TextureHandle>,
    backdrop: Option<Backdrop>,
    tex_dirty: bool,
    pub tool: Tool,
    drag_start: Option<Pos2>,
    pending: Option<Shape>,
    pending_blur: Option<Rect>,
    selected: Option<usize>,
    moving: bool,
    editing: Option<usize>,
    focus_text: bool,
    style_undo_armed: bool,
    crop: Option<Rect>,
    crop_dragging: bool,
    zoom: Option<f32>,
    last_scale: f32,
    resize: Option<(u32, u32, bool)>,
    img_rect: Option<Rect>,
    pub title: String,
    pub source: Option<PathBuf>,
}

impl Editor {
    pub fn new(image: RgbaImage, title: &str) -> Editor {
        Editor {
            doc: Doc { image, shapes: Vec::new() },
            undo: Vec::new(),
            redo: Vec::new(),
            texture: None,
            backdrop_tex: None,
            backdrop: None,
            tex_dirty: true,
            tool: Tool::Arrow,
            drag_start: None,
            pending: None,
            pending_blur: None,
            selected: None,
            moving: false,
            editing: None,
            focus_text: false,
            style_undo_armed: true,
            crop: None,
            crop_dragging: false,
            zoom: None,
            last_scale: 1.0,
            resize: None,
            img_rect: None,
            title: title.to_string(),
            source: None,
        }
    }

    /// Adds shapes without an undo step (used for the sample picture).
    pub fn add_shapes(&mut self, shapes: Vec<Shape>) {
        self.doc.shapes.extend(shapes);
    }

    /// Selects a shape (used for the sample picture).
    pub fn select(&mut self, i: usize) {
        if i < self.doc.shapes.len() {
            self.selected = Some(i);
            self.tool = Tool::Select;
        }
    }

    pub fn size(&self) -> (u32, u32) {
        self.doc.image.dimensions()
    }

    /// The picture with all annotations drawn into it.
    pub fn flattened(&self, font: &AnnotFont) -> RgbaImage {
        shapes::bake(&self.doc.image, &self.doc.shapes, font)
    }

    fn push_undo(&mut self) {
        self.undo.push(self.doc.clone());
        if self.undo.len() > UNDO_LIMIT {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    pub fn undo(&mut self) {
        self.finish_text();
        if let Some(d) = self.undo.pop() {
            self.redo.push(std::mem::replace(&mut self.doc, d));
            self.tex_dirty = true;
            self.selected = None;
        }
    }

    pub fn redo(&mut self) {
        if let Some(d) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.doc, d));
            self.tex_dirty = true;
            self.selected = None;
        }
    }

    fn bake_in_place(&mut self, font: &AnnotFont) {
        if !self.doc.shapes.is_empty() {
            self.doc.image = shapes::bake(&self.doc.image, &self.doc.shapes, font);
            self.doc.shapes.clear();
        }
        self.selected = None;
    }

    pub fn rotate(&mut self, clockwise: bool, font: &AnnotFont) {
        self.finish_text();
        self.push_undo();
        self.bake_in_place(font);
        self.doc.image = if clockwise {
            image::imageops::rotate90(&self.doc.image)
        } else {
            image::imageops::rotate270(&self.doc.image)
        };
        self.tex_dirty = true;
    }

    fn apply_crop(&mut self, font: &AnnotFont) {
        let Some(r) = self.crop.take() else { return };
        let (w, h) = self.size();
        let r = r.intersect(Rect::from_min_size(Pos2::ZERO, vec2(w as f32, h as f32)));
        if r.width() < 2.0 || r.height() < 2.0 {
            return;
        }
        self.push_undo();
        self.bake_in_place(font);
        self.doc.image = image::imageops::crop_imm(
            &self.doc.image,
            r.min.x.round() as u32,
            r.min.y.round() as u32,
            r.width().round() as u32,
            r.height().round() as u32,
        )
        .to_image();
        self.tex_dirty = true;
        self.tool = Tool::Select;
    }

    fn apply_resize(&mut self, nw: u32, nh: u32, font: &AnnotFont) {
        if nw == 0 || nh == 0 {
            return;
        }
        self.push_undo();
        self.bake_in_place(font);
        self.doc.image = image::imageops::resize(&self.doc.image, nw, nh, image::imageops::FilterType::Lanczos3);
        self.tex_dirty = true;
    }

    /// Ends typing; an empty text is removed again.
    fn finish_text(&mut self) {
        if let Some(i) = self.editing.take() {
            let empty = self.doc.shapes.get_mut(i).and_then(|s| s.text_mut()).map(|t| t.trim().is_empty()).unwrap_or(false);
            if empty {
                self.doc.shapes.remove(i);
                self.selected = None;
            }
        }
    }

    fn start_editing(&mut self, i: usize) {
        self.editing = Some(i);
        self.selected = Some(i);
        self.focus_text = true;
    }

    fn textures(&mut self, ctx: &Context) -> TextureHandle {
        if self.tex_dirty || self.texture.is_none() {
            let (w, h) = self.size();
            let ci = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], self.doc.image.as_raw());
            match &mut self.texture {
                Some(t) => t.set(ci, TextureOptions::LINEAR),
                None => self.texture = Some(ctx.load_texture("editor-image", ci, TextureOptions::LINEAR)),
            }
            let bd = theme::blurred_backdrop(&self.doc.image);
            match &mut self.backdrop_tex {
                Some(t) => t.set(bd, TextureOptions::LINEAR),
                None => self.backdrop_tex = Some(ctx.load_texture("editor-backdrop", bd, TextureOptions::LINEAR)),
            }
            self.tex_dirty = false;
        }
        self.texture.clone().unwrap()
    }

    // ------------------------------------------------------------------ UI

    pub fn ui(&mut self, ctx: &Context, s: &mut Settings, font: &AnnotFont) -> Vec<Cmd> {
        let pal = theme::pal(ctx);
        let mut cmds = Vec::new();
        self.shortcuts(ctx, s, font, &mut cmds);
        egui::CentralPanel::default().frame(Frame::new().fill(pal.bg)).show(ctx, |ui| {
            self.canvas(ui, s, font, &pal);
        });
        let avail = ctx.available_rect();
        self.tools_panel(ctx, s, font, &pal, avail, &mut cmds);
        let actions = self.actions_panel(ctx, s, &pal, avail, &mut cmds);
        self.style_panel(ctx, s, &pal, avail);
        self.zoom_panel(ctx, &pal, actions.left_top() + vec2(-10.0, 0.0));
        self.text_editor(ctx, font, &pal);
        self.resize_dialog(ctx, font);
        cmds
    }

    fn shortcuts(&mut self, ctx: &Context, s: &mut Settings, font: &AnnotFont, cmds: &mut Vec<Cmd>) {
        if self.editing.is_some() || ctx.wants_keyboard_input() {
            return;
        }
        let tools = [
            (Key::Num1, Tool::Arrow),
            (Key::Num2, Tool::Rect),
            (Key::Num3, Tool::Line),
            (Key::Num4, Tool::Ellipse),
            (Key::Num5, Tool::Pen),
            (Key::Num6, Tool::Text),
            (Key::Num7, Tool::Bubble),
            (Key::Num8, Tool::Blur),
        ];
        let mut nudge = Vec2Opt::default();
        let mut delete = false;
        ctx.input_mut(|i| {
            for (k, t) in tools {
                if i.consume_key(Modifiers::COMMAND, k) {
                    self.tool = t;
                    self.crop = None;
                }
            }
            if i.consume_key(Modifiers::NONE, Key::V) {
                self.tool = Tool::Select;
                self.crop = None;
            }
            if i.consume_key(Modifiers::COMMAND, Key::OpenBracket) {
                s.editor_width = (s.editor_width - 1.0).max(LINE_WIDTH_MIN);
            }
            if i.consume_key(Modifiers::COMMAND, Key::CloseBracket) {
                s.editor_width = (s.editor_width + 1.0).min(LINE_WIDTH_MAX);
            }
            if i.consume_key(Modifiers::COMMAND | Modifiers::SHIFT, Key::Z) {
                self.redo();
            }
            if i.consume_key(Modifiers::COMMAND, Key::Z) {
                self.undo();
            }
            if i.consume_key(Modifiers::COMMAND, Key::S) {
                cmds.push(Cmd::Save);
            }
            if i.consume_key(Modifiers::COMMAND, Key::E) {
                cmds.push(Cmd::External);
            }
            if i.consume_key(Modifiers::COMMAND, Key::Equals) || i.consume_key(Modifiers::COMMAND, Key::Plus) {
                self.zoom = Some(self.last_scale * 1.25);
            }
            if i.consume_key(Modifiers::COMMAND, Key::Minus) {
                self.zoom = Some((self.last_scale / 1.25).max(0.05));
            }
            if i.consume_key(Modifiers::COMMAND, Key::Num0) {
                self.zoom = None;
            }
            if self.selected.is_some() {
                delete = i.consume_key(Modifiers::NONE, Key::Backspace) || i.consume_key(Modifiers::NONE, Key::Delete);
                let step = if i.modifiers.shift { 10.0 } else { 1.0 };
                for (k, d) in [
                    (Key::ArrowLeft, vec2(-step, 0.0)),
                    (Key::ArrowRight, vec2(step, 0.0)),
                    (Key::ArrowUp, vec2(0.0, -step)),
                    (Key::ArrowDown, vec2(0.0, step)),
                ] {
                    if i.consume_key(Modifiers::NONE, k) || i.consume_key(Modifiers::SHIFT, k) {
                        nudge.0 = Some(nudge.0.unwrap_or(egui::Vec2::ZERO) + d);
                    }
                }
            }
            for e in &i.events {
                match e {
                    Event::Copy => cmds.push(Cmd::Copy),
                    Event::Paste(_) => cmds.push(Cmd::PasteImage),
                    _ => {}
                }
            }
        });
        if delete {
            if let Some(idx) = self.selected.take() {
                if idx < self.doc.shapes.len() {
                    self.push_undo();
                    self.doc.shapes.remove(idx);
                }
            }
        }
        if let (Some(d), Some(idx)) = (nudge.0, self.selected) {
            if idx < self.doc.shapes.len() {
                self.push_undo();
                self.doc.shapes[idx].translate(d);
            }
        }
        let (rot_cw, rot_ccw, esc, enter) = ctx.input_mut(|i| {
            (
                i.consume_key(Modifiers::COMMAND, Key::ArrowRight),
                i.consume_key(Modifiers::COMMAND, Key::ArrowLeft),
                i.key_pressed(Key::Escape),
                i.key_pressed(Key::Enter),
            )
        });
        if rot_cw {
            self.rotate(true, font);
        }
        if rot_ccw {
            self.rotate(false, font);
        }
        if esc {
            self.crop = None;
            self.pending = None;
            self.pending_blur = None;
            self.selected = None;
        }
        if enter {
            if self.crop.is_some() {
                self.apply_crop(font);
            } else if let Some(i) = self.selected {
                if self.doc.shapes.get(i).map(|s| s.is_text()).unwrap_or(false) {
                    self.start_editing(i);
                }
            }
        }
    }

    fn tool_button(ui: &mut Ui, pal: &Pal, s: &Settings, icon: Icon, selected: bool, tip: (&str, &str, &str)) -> egui::Response {
        let (rect, resp) = ui.allocate_exact_size(vec2(32.0, 32.0), Sense::click());
        let p = ui.painter();
        if selected {
            p.rect_filled(rect, 9.0, pal.accent);
        } else if resp.hovered() {
            p.rect_filled(rect, 9.0, pal.hover);
        }
        let col = if selected { Color32::WHITE } else { pal.text };
        icons::paint(p, Rect::from_center_size(rect.center(), vec2(19.0, 19.0)), icon, col);
        if s.show_tooltips_editor {
            let (title, key, hint) = tip;
            let mut t = title.to_string();
            if !key.is_empty() {
                t.push_str(&format!("  {key}"));
            }
            if !hint.is_empty() {
                t.push('\n');
                t.push_str(hint);
            }
            return resp.on_hover_text(t);
        }
        resp
    }

    fn tools_panel(&mut self, ctx: &Context, s: &mut Settings, font: &AnnotFont, pal: &Pal, avail: Rect, cmds: &mut Vec<Cmd>) {
        theme::floating(ctx, "tools_panel", Align2::LEFT_TOP, avail.left_top() + vec2(12.0, 12.0), pal, self.backdrop, |ui| {
            if Self::tool_button(ui, pal, s, Icon::Back, false, ("Home", "", "Back to the capture screen")).clicked() {
                cmds.push(Cmd::Home);
            }
            theme::divider(ui, pal);
            for t in Tool::DRAW {
                let sel = self.tool == t && self.crop.is_none();
                if Self::tool_button(ui, pal, s, t.icon(), sel, t.tip()).clicked() {
                    self.finish_text();
                    self.tool = t;
                    self.crop = None;
                    if t != Tool::Select {
                        self.selected = None;
                    }
                }
            }
            theme::divider(ui, pal);
            let sel = self.tool == Tool::Crop;
            if Self::tool_button(ui, pal, s, Icon::Crop, sel, Tool::Crop.tip()).clicked() {
                self.finish_text();
                self.tool = Tool::Crop;
                self.selected = None;
            }
            if Self::tool_button(ui, pal, s, Icon::Rotate, false, ("Rotate clockwise", "⌘→", "Hold ⌘ to rotate counterclockwise"))
                .clicked()
            {
                let ccw = ui.input(|i| i.modifiers.command);
                self.rotate(!ccw, font);
            }
            if Self::tool_button(ui, pal, s, Icon::Resize, false, ("Resize", "", "")).clicked() {
                let (w, h) = self.size();
                self.resize = Some((w, h, true));
            }
            theme::divider(ui, pal);
            if Self::tool_button(ui, pal, s, Icon::Undo, false, ("Undo", "⌘Z", "")).clicked() {
                self.undo();
            }
            if Self::tool_button(ui, pal, s, Icon::Redo, false, ("Redo", "⇧⌘Z", "")).clicked() {
                self.redo();
            }
        });
    }

    fn actions_panel(&mut self, ctx: &Context, s: &Settings, pal: &Pal, avail: Rect, cmds: &mut Vec<Cmd>) -> Rect {
        theme::floating(ctx, "actions_panel", Align2::RIGHT_TOP, avail.right_top() + vec2(-12.0, 12.0), pal, self.backdrop, |ui| {
            let b = |t: &str| egui::Button::new(t).min_size(vec2(0.0, 32.0)).corner_radius(9.0);
            let c = ui.add(b("Copy"));
            if c.clicked() {
                cmds.push(Cmd::Copy);
            }
            if s.show_tooltips_editor {
                c.on_hover_text("Copy image  ⌘C");
            }
            let e = ui.add(b("Open in…"));
            if e.clicked() {
                cmds.push(Cmd::External);
            }
            if s.show_tooltips_editor {
                e.on_hover_text("External editor  ⌘E");
            }
            ui.add_enabled(false, b("Upload"))
                .on_disabled_hover_text("Uploading needs a connected account, which this version doesn't include");
            let save = egui::Button::new(RichText::new("Save").color(Color32::WHITE).strong())
                .fill(pal.accent)
                .min_size(vec2(64.0, 32.0))
                .corner_radius(9.0);
            let r = ui.add(save);
            if r.clicked() {
                cmds.push(Cmd::Save);
            }
            if s.show_tooltips_editor {
                r.on_hover_text("Save  ⌘S");
            }
        })
        .response
        .rect
    }

    /// The always-visible style bar. It edits the selected item if there is
    /// one, and sets the defaults for new items either way.
    fn style_panel(&mut self, ctx: &Context, s: &mut Settings, pal: &Pal, avail: Rect) {
        let target = self.selected.filter(|i| *i < self.doc.shapes.len());
        let (color, width, shadow, tsize, tstyle, is_text, is_bubble, label) = match target {
            Some(i) => {
                let sh = &self.doc.shapes[i];
                let (ts, st) = match &sh.kind {
                    Kind::Text { size, style, .. } => (*size, Some(*style)),
                    Kind::Bubble { size, .. } => (*size, None),
                    _ => (s.editor_text_size, None),
                };
                let what = match &sh.kind {
                    Kind::Arrow { .. } => "arrow",
                    Kind::Line { .. } => "line",
                    Kind::Rect { .. } => "rectangle",
                    Kind::Ellipse { .. } => "ellipse",
                    Kind::Pen { .. } => "drawing",
                    Kind::Text { .. } => "text",
                    Kind::Bubble { .. } => "note",
                };
                let label = if self.editing.is_some() { "Typing — Esc to finish".to_string() } else { format!("Selected {what}") };
                (sh.color, sh.width, sh.shadow, ts, st, sh.is_text(), matches!(sh.kind, Kind::Bubble { .. }), label)
            }
            None => {
                let text_tool = matches!(self.tool, Tool::Text | Tool::Bubble);
                let label = match self.tool {
                    Tool::Text => "Click on the picture to type".to_string(),
                    Tool::Select => "Click an item to change it".to_string(),
                    t => format!("New {}", t.name().to_lowercase()),
                };
                (
                    color_of(s.editor_color),
                    s.editor_width,
                    s.editor_shadow,
                    s.editor_text_size,
                    if self.tool == Tool::Text { Some(s.editor_text_style) } else { None },
                    text_tool,
                    self.tool == Tool::Bubble,
                    label,
                )
            }
        };
        let _ = is_bubble;
        let mut new_color = color;
        let mut new_width = width;
        let mut new_shadow = shadow;
        let mut new_tsize = tsize;
        let mut new_style = tstyle;
        let mut slider_active = false;
        let mut slider_started = false;

        theme::floating(ctx, "style_panel", Align2::CENTER_BOTTOM, pos2(avail.center().x, avail.bottom() - 14.0), pal, self.backdrop, |ui| {
            ui.add_sized(vec2(118.0, 32.0), egui::Label::new(RichText::new(label).size(12.0).color(pal.hint)).truncate());
            theme::divider(ui, pal);
            // colours
            ui.spacing_mut().item_spacing.x = 3.0;
            for c in PALETTE {
                let col = hex(c);
                let (r, resp) = ui.allocate_exact_size(vec2(22.0, 32.0), Sense::click());
                let cur = new_color.r() == col.r() && new_color.g() == col.g() && new_color.b() == col.b();
                let rad = if resp.hovered() { 9.5 } else { 8.5 };
                let pnt = ui.painter();
                if cur {
                    pnt.circle_stroke(r.center(), 11.0, Stroke::new(2.0_f32, pal.accent));
                }
                pnt.circle_filled(r.center(), rad, col);
                pnt.circle_stroke(r.center(), rad, Stroke::new(1.0_f32, pal.border));
                if resp.clicked() {
                    new_color = col;
                }
            }
            let mut rgb = [new_color.r(), new_color.g(), new_color.b()];
            ui.spacing_mut().interact_size = vec2(20.0, 20.0);
            if egui::color_picker::color_edit_button_srgb(ui, &mut rgb).on_hover_text("Any colour").changed() {
                new_color = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
            }
            ui.spacing_mut().item_spacing.x = 4.0;
            theme::divider(ui, pal);
            // line size with a live preview
            let (pr, _) = ui.allocate_exact_size(vec2(26.0, 32.0), Sense::hover());
            ui.painter().circle_filled(pr.center(), (new_width * 0.9 + 1.0).min(12.0), new_color);
            ui.spacing_mut().slider_width = 80.0;
            let sr = ui
                .add(egui::Slider::new(&mut new_width, LINE_WIDTH_MIN..=LINE_WIDTH_MAX).step_by(1.0).show_value(false))
                .on_hover_text("Size of lines, arrows and shapes  ⌘[ ⌘]");
            slider_active |= sr.dragged() || sr.changed();
            slider_started |= sr.drag_started();
            if is_text {
                theme::divider(ui, pal);
                ui.label(RichText::new("Aa").size(15.0).strong().color(pal.text));
                ui.spacing_mut().slider_width = 80.0;
                ui.spacing_mut().interact_size = vec2(34.0, 22.0);
                let tr = ui
                    .add(egui::Slider::new(&mut new_tsize, TEXT_SIZE_MIN..=TEXT_SIZE_MAX).step_by(1.0).suffix("").logarithmic(true))
                    .on_hover_text("Text size");
                slider_active |= tr.dragged() || tr.changed();
                slider_started |= tr.drag_started();
                if let Some(st) = new_style.as_mut() {
                    for (variant, tip) in [
                        (TextStyle::Plain, "Plain text"),
                        (TextStyle::Outline, "Text with an outline"),
                        (TextStyle::Box, "Text on a coloured box"),
                    ] {
                        let (r, resp) = ui.allocate_exact_size(vec2(30.0, 30.0), Sense::click());
                        let on = *st == variant;
                        let p = ui.painter();
                        if on {
                            p.rect_filled(r, 8.0, pal.accent_soft);
                            p.rect_stroke(r, 8.0, Stroke::new(1.0_f32, pal.accent), egui::StrokeKind::Inside);
                        } else if resp.hovered() {
                            p.rect_filled(r, 8.0, pal.hover);
                        }
                        let fid = FontId::new(17.0, FontFamily::Name("annot".into()));
                        let c = r.center();
                        match variant {
                            TextStyle::Plain => {
                                p.text(c, Align2::CENTER_CENTER, "A", fid, new_color);
                            }
                            TextStyle::Outline => {
                                let oc = shapes::contrast(new_color);
                                for k in 0..8 {
                                    let a = k as f32 / 8.0 * std::f32::consts::TAU;
                                    p.text(c + vec2(a.cos(), a.sin()) * 1.4, Align2::CENTER_CENTER, "A", fid.clone(), oc);
                                }
                                p.text(c, Align2::CENTER_CENTER, "A", fid, new_color);
                            }
                            TextStyle::Box => {
                                p.rect_filled(Rect::from_center_size(c, vec2(22.0, 20.0)), 5.0, new_color);
                                p.text(c, Align2::CENTER_CENTER, "A", fid, shapes::contrast(new_color));
                            }
                        }
                        if resp.on_hover_text(tip).clicked() {
                            *st = variant;
                        }
                    }
                }
            }
            theme::divider(ui, pal);
            ui.toggle_value(&mut new_shadow, "Shadow").on_hover_text("Shadow & outline under new items");
        });

        let changed = new_color != color
            || new_width != width
            || new_shadow != shadow
            || new_tsize != tsize
            || new_style != tstyle;
        if !changed {
            if !slider_active {
                self.style_undo_armed = true;
            }
            return;
        }
        // in the original the font size follows the line width
        if is_text && new_width != width && new_tsize == tsize {
            new_tsize = shapes::text_size_for_width(new_width);
        }
        // defaults for new items
        s.editor_color = [new_color.r(), new_color.g(), new_color.b(), 255];
        s.editor_width = new_width;
        s.editor_shadow = new_shadow;
        if is_text {
            s.editor_text_size = new_tsize;
        }
        if let Some(st) = new_style {
            s.editor_text_style = st;
        }
        // and the selected item (one undo step per change or slider drag)
        if let Some(i) = target {
            if self.style_undo_armed || slider_started {
                self.push_undo();
                self.style_undo_armed = !slider_active;
            }
            let sh = &mut self.doc.shapes[i];
            sh.color = new_color;
            sh.width = new_width;
            sh.shadow = new_shadow;
            match &mut sh.kind {
                Kind::Text { size, style, .. } => {
                    *size = new_tsize;
                    if let Some(st) = new_style {
                        *style = st;
                    }
                }
                Kind::Bubble { size, .. } => *size = new_tsize,
                _ => {}
            }
        }
    }

    fn zoom_panel(&mut self, ctx: &Context, pal: &Pal, at: Pos2) {
        let ppp = ctx.pixels_per_point();
        theme::floating(ctx, "zoom_panel", Align2::RIGHT_TOP, at, pal, self.backdrop, |ui| {
            ui.set_min_height(32.0);
            let (w, h) = self.size();
            ui.label(RichText::new(format!("{w} × {h}")).size(12.0).color(pal.hint));
            theme::divider(ui, pal);
            if ui.small_button("−").on_hover_text("Zoom out  ⌘−").clicked() {
                self.zoom = Some((self.last_scale / 1.25).max(0.05));
            }
            let pct = (self.last_scale * ppp * 100.0).round();
            ui.label(RichText::new(format!("{pct}%")).size(12.0));
            if ui.small_button("+").on_hover_text("Zoom in  ⌘+").clicked() {
                self.zoom = Some(self.last_scale * 1.25);
            }
            if ui.small_button("Fit").on_hover_text("Fit to window  ⌘0").clicked() {
                self.zoom = None;
            }
            if ui.small_button("1:1").on_hover_text("Original size").clicked() {
                self.zoom = Some(1.0 / ppp);
            }
        });
    }

    /// In-place typing: a borderless text field drawn exactly over the text.
    fn text_editor(&mut self, ctx: &Context, font: &AnnotFont, pal: &Pal) {
        let Some(i) = self.editing else { return };
        let Some(img_rect) = self.img_rect else { return };
        let scale = self.last_scale;
        let Some(sh) = self.doc.shapes.get_mut(i) else {
            self.editing = None;
            return;
        };
        let (Some(origin), Some(size)) = (sh.text_origin(), sh.text_size()) else {
            self.editing = None;
            return;
        };
        let color = sh.text_color();
        let screen = img_rect.min + origin.to_vec2() * scale;
        let fid = FontId::new(size * scale / font.em_ratio(), FontFamily::Name("annot".into()));
        let text_now = sh.text_mut().cloned().unwrap_or_default();
        let width = (font.measure(if text_now.is_empty() { "Type text…" } else { &text_now }, size).x * scale + size * scale * 2.0).max(60.0);
        let id = Id::new("annotation_text_edit");
        let mut lost = false;
        let mut focus = self.focus_text;
        egui::Area::new(Id::new("annotation_text_area")).fixed_pos(screen).order(egui::Order::Foreground).show(ctx, |ui| {
            let Some(text) = sh.text_mut() else { return };
            let te = egui::TextEdit::multiline(text)
                .id(id)
                .frame(false)
                .margin(Margin::ZERO)
                .font(fid.clone())
                .text_color(color)
                .desired_width(width)
                .desired_rows(1)
                .hint_text(RichText::new("Type text…").font(fid).color(pal.hint));
            let r = ui.add(te);
            if focus {
                r.request_focus();
                if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), id) {
                    let end = egui::text::CCursor::new(text.chars().count());
                    state.cursor.set_char_range(Some(egui::text::CCursorRange::one(end)));
                    state.store(ui.ctx(), id);
                }
                focus = false;
            } else if r.lost_focus() {
                lost = true;
            }
        });
        self.focus_text = focus;
        if lost {
            self.finish_text();
        }
    }

    fn canvas(&mut self, ui: &mut Ui, s: &mut Settings, font: &AnnotFont, pal: &Pal) {
        let ctx = ui.ctx().clone();
        let tex = self.textures(&ctx);
        let (iw, ih) = self.size();
        let (iw, ih) = (iw as f32, ih as f32);
        let full_area = ui.max_rect();
        // blurred copy of the picture behind everything
        if let Some(bt) = &self.backdrop_tex {
            let bd = Backdrop::cover(bt.id(), full_area, vec2(iw, ih));
            bd.paint(ui.painter());
            ui.painter().rect_filled(full_area, 0.0, pal.canvas_tint);
            self.backdrop = Some(bd);
        }
        let avail = ui.available_size();
        let ppp = ctx.pixels_per_point();
        // leave room for the floating panels above and below
        let fit = ((avail.x - 60.0) / iw).min((avail.y - 160.0) / ih).min(1.0 / ppp).max(0.02);
        let scale = self.zoom.unwrap_or(fit);
        self.last_scale = scale;
        let size = vec2(iw * scale, ih * scale);

        egui::ScrollArea::both()
            .scroll_source(egui::scroll_area::ScrollSource::MOUSE_WHEEL | egui::scroll_area::ScrollSource::SCROLL_BAR)
            .auto_shrink(false)
            .show(ui, |ui| {
                let total = vec2((size.x + 60.0).max(ui.available_width()), (size.y + 150.0).max(ui.available_height()));
                let (rect, resp) = ui.allocate_exact_size(total, Sense::click_and_drag());
                let img_rect = Rect::from_center_size(rect.center() + vec2(0.0, -4.0), size);
                self.img_rect = Some(img_rect);
                let painter = ui.painter_at(rect);
                theme::shadow(&painter, img_rect, 6, pal);
                painter.add(
                    egui::epaint::RectShape::filled(img_rect, CornerRadius::same(6), Color32::WHITE)
                        .with_texture(tex.id(), Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0))),
                );

                let to_screen = |p: Pos2| img_rect.min + p.to_vec2() * scale;
                let to_img = |p: Pos2| ((p - img_rect.min) / scale).to_pos2();
                let clip = ui.painter_at(img_rect);
                let full = Rect::from_min_size(Pos2::ZERO, vec2(iw, ih));

                for (i, sh) in self.doc.shapes.iter().enumerate() {
                    let mut prims = sh.prims(font);
                    if self.editing == Some(i) {
                        // the text field draws the letters while typing
                        let n_text = prims.iter().filter(|p| matches!(p, Prim::Text { .. })).count();
                        if n_text > 0 {
                            prims.retain(|p| !matches!(p, Prim::Text { .. }));
                        }
                    }
                    paint_prims(&clip, &prims, &to_screen, scale, full, font.em_ratio());
                }
                if let Some(p) = &self.pending {
                    paint_prims(&clip, &p.prims(font), &to_screen, scale, full, font.em_ratio());
                }
                if let Some(b) = self.pending_blur {
                    let r = Rect::from_min_max(to_screen(b.min), to_screen(b.max));
                    clip.rect_filled(r, 4.0, Color32::from_white_alpha(50));
                    dashed_rect(&clip, r, Color32::WHITE);
                }
                // selection frame
                if let Some(i) = self.selected {
                    if let Some(sh) = self.doc.shapes.get(i) {
                        let b = sh.bounds(font);
                        let r = Rect::from_min_max(to_screen(b.min), to_screen(b.max)).expand(4.0);
                        painter.rect_stroke(r, 6.0, Stroke::new(1.5_f32, pal.accent), egui::StrokeKind::Middle);
                        for c in [r.left_top(), r.right_top(), r.left_bottom(), r.right_bottom()] {
                            painter.circle_filled(c, 4.5, Color32::WHITE);
                            painter.circle_stroke(c, 4.5, Stroke::new(1.5_f32, pal.accent));
                        }
                    }
                }
                // crop overlay
                if let Some(c) = self.crop {
                    let r = Rect::from_min_max(to_screen(c.min), to_screen(c.max));
                    for part in shapes::outside_parts(img_rect, r) {
                        clip.rect_filled(part, 0.0, Color32::from_black_alpha(CROP_DIM_ALPHA));
                    }
                    dashed_rect(&clip, r, Color32::WHITE);
                    let label = format!("{} × {}", c.width().round(), c.height().round());
                    badge(&painter, r.left_top() - vec2(0.0, 6.0), Align2::LEFT_BOTTOM, &label);
                    if !self.crop_dragging {
                        let at = pos2(r.right(), r.bottom() + 8.0).min(rect.max - vec2(0.0, 40.0));
                        let mut apply = false;
                        let mut cancel = false;
                        egui::Area::new(Id::new("crop_buttons"))
                            .order(egui::Order::Foreground)
                            .pivot(Align2::RIGHT_TOP)
                            .fixed_pos(at)
                            .show(&ctx, |ui| {
                                Frame::popup(ui.style()).corner_radius(10.0).show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        if ui.button("Cancel").clicked() {
                                            cancel = true;
                                        }
                                        if ui.add(egui::Button::new(RichText::new("Crop").color(Color32::WHITE)).fill(pal.accent)).clicked() {
                                            apply = true;
                                        }
                                    });
                                });
                            });
                        if apply {
                            self.apply_crop(font);
                        }
                        if cancel {
                            self.crop = None;
                        }
                    }
                }

                self.handle_pointer(&ctx, &resp, s, font, &to_img, full, scale);
            });
    }

    fn hit_test(&self, p: Pos2, scale: f32, font: &AnnotFont) -> Option<usize> {
        let tol = 6.0 / scale.max(0.01);
        (0..self.doc.shapes.len()).rev().find(|&i| self.doc.shapes[i].hit(p, tol, font))
    }

    #[allow(clippy::too_many_arguments)]
    fn handle_pointer(
        &mut self,
        ctx: &Context,
        resp: &egui::Response,
        s: &mut Settings,
        font: &AnnotFont,
        to_img: &dyn Fn(Pos2) -> Pos2,
        full: Rect,
        scale: f32,
    ) {
        let mods = ctx.input(|i| i.modifiers);
        let color = color_of(s.editor_color);
        let width = s.editor_width;
        let shadow = s.editor_shadow;
        let tsize = s.editor_text_size;
        let tstyle = s.editor_text_style;

        if let Some(hp) = resp.hover_pos() {
            let over = self.hit_test(to_img(hp), scale, font);
            let icon = match self.tool {
                Tool::Select if over.is_some() => egui::CursorIcon::Grab,
                Tool::Select => egui::CursorIcon::Default,
                Tool::Text if over.map(|i| self.doc.shapes[i].is_text()).unwrap_or(false) => egui::CursorIcon::Text,
                Tool::Text => egui::CursorIcon::Text,
                _ => egui::CursorIcon::Crosshair,
            };
            ctx.set_cursor_icon(if self.moving { egui::CursorIcon::Grabbing } else { icon });
        }

        if resp.double_clicked() {
            if let Some(pp) = resp.interact_pointer_pos() {
                if let Some(i) = self.hit_test(to_img(pp), scale, font) {
                    if self.doc.shapes[i].is_text() {
                        self.finish_text();
                        self.start_editing(i);
                        return;
                    }
                }
            }
        }

        if resp.drag_started() {
            self.finish_text();
            let origin = ctx.input(|i| i.pointer.press_origin()).or(resp.interact_pointer_pos());
            if let Some(o) = origin {
                let p = to_img(o);
                self.drag_start = Some(p);
                let mk = |kind| Shape { kind, color, width, shadow };
                match self.tool {
                    Tool::Select => {
                        self.selected = self.hit_test(p, scale, font);
                        if self.selected.is_some() {
                            self.push_undo();
                            self.moving = true;
                        }
                    }
                    Tool::Arrow => self.pending = Some(mk(Kind::Arrow { a: p, b: p })),
                    Tool::Line => self.pending = Some(mk(Kind::Line { a: p, b: p })),
                    Tool::Rect => self.pending = Some(mk(Kind::Rect { r: Rect::from_two_pos(p, p), filled: false, darken: false })),
                    Tool::Ellipse => self.pending = Some(mk(Kind::Ellipse { r: Rect::from_two_pos(p, p), filled: false })),
                    Tool::Pen => self.pending = Some(mk(Kind::Pen { pts: vec![p], highlighter: mods.command })),
                    Tool::Bubble => self.pending = Some(mk(Kind::Bubble { tail: p, pos: p, text: String::new(), size: tsize })),
                    Tool::Blur => self.pending_blur = Some(Rect::from_two_pos(p, p)),
                    Tool::Crop => {
                        self.crop = Some(Rect::from_two_pos(p, p));
                        self.crop_dragging = true;
                    }
                    Tool::Text => {
                        // dragging a text with the Text tool moves it
                        if let Some(i) = self.hit_test(p, scale, font).filter(|i| self.doc.shapes[*i].is_text()) {
                            self.selected = Some(i);
                            self.push_undo();
                            self.moving = true;
                        }
                    }
                }
            }
        }

        if resp.dragged() {
            if self.moving {
                if let Some(i) = self.selected {
                    let d = resp.drag_delta() / scale;
                    if let Some(sh) = self.doc.shapes.get_mut(i) {
                        sh.translate(d);
                    }
                }
            } else if let (Some(a), Some(cur)) = (self.drag_start, resp.interact_pointer_pos()) {
                let b = to_img(cur);
                match self.tool {
                    Tool::Blur => {
                        let b2 = if mods.shift { shapes::square(a, b) } else { b };
                        self.pending_blur = Some(Rect::from_two_pos(a, b2).intersect(full));
                    }
                    Tool::Crop => {
                        let b2 = if mods.shift { shapes::square(a, b) } else { b };
                        self.crop = Some(Rect::from_two_pos(a, b2).intersect(full));
                    }
                    _ => {}
                }
                if let Some(p) = &mut self.pending {
                    match &mut p.kind {
                        Kind::Arrow { a: pa, b: pb } => {
                            let end = if mods.shift { shapes::snap45(a, b) } else { b };
                            if mods.command {
                                *pa = end;
                                *pb = a;
                            } else {
                                *pa = a;
                                *pb = end;
                            }
                        }
                        Kind::Line { b: pb, .. } => *pb = if mods.shift { shapes::snap45(a, b) } else { b },
                        Kind::Rect { r, filled, darken } => {
                            let b2 = if mods.shift { shapes::square(a, b) } else { b };
                            *r = Rect::from_two_pos(a, b2);
                            *filled = mods.command;
                            *darken = mods.alt;
                        }
                        Kind::Ellipse { r, filled } => {
                            let b2 = if mods.shift { shapes::square(a, b) } else { b };
                            *r = Rect::from_two_pos(a, b2);
                            *filled = mods.command;
                        }
                        Kind::Pen { pts, .. } => {
                            if pts.last().map(|l| l.distance(b) > 0.8).unwrap_or(true) {
                                pts.push(b);
                            }
                        }
                        Kind::Bubble { pos, .. } => *pos = if mods.shift { shapes::snap45(a, b) } else { b },
                        Kind::Text { .. } => {}
                    }
                }
            }
        }

        if resp.drag_stopped() {
            self.drag_start = None;
            if self.moving {
                self.moving = false;
            }
            if let Some(b) = self.pending_blur.take() {
                if b.width() >= 2.0 && b.height() >= 2.0 {
                    self.push_undo();
                    shapes::blur_region(&mut self.doc.image, b, BLUR_RADIUS);
                    self.tex_dirty = true;
                }
            }
            if self.tool == Tool::Crop {
                self.crop_dragging = false;
                if self.crop.map(|c| c.width() < 3.0 || c.height() < 3.0).unwrap_or(false) {
                    self.crop = None;
                }
            }
            if let Some(p) = self.pending.take() {
                let keep = match &p.kind {
                    Kind::Arrow { a, b } | Kind::Line { a, b } => a.distance(*b) > 3.0,
                    Kind::Rect { r, .. } | Kind::Ellipse { r, .. } => r.width() > 2.0 && r.height() > 2.0,
                    Kind::Pen { .. } | Kind::Bubble { .. } => true,
                    Kind::Text { .. } => false,
                };
                if keep {
                    self.push_undo();
                    let is_bubble = matches!(p.kind, Kind::Bubble { .. });
                    self.doc.shapes.push(p);
                    let idx = self.doc.shapes.len() - 1;
                    // the new item stays selected so its colour and size can be tweaked
                    self.selected = Some(idx);
                    if is_bubble {
                        self.start_editing(idx);
                    }
                }
            }
        }

        if resp.clicked() {
            let was_editing = self.editing.is_some();
            self.finish_text();
            if let Some(pp) = resp.interact_pointer_pos() {
                let p = to_img(pp);
                let hit = self.hit_test(p, scale, font);
                let mk = |kind| Shape { kind, color, width, shadow };
                match self.tool {
                    Tool::Text => {
                        if let Some(i) = hit.filter(|i| self.doc.shapes[*i].is_text()) {
                            self.start_editing(i);
                        } else if !was_editing {
                            self.push_undo();
                            let pos = p - vec2(0.0, tsize * 0.6);
                            self.doc.shapes.push(mk(Kind::Text { pos, text: String::new(), size: tsize, style: tstyle }));
                            self.start_editing(self.doc.shapes.len() - 1);
                        } else {
                            self.selected = None;
                        }
                    }
                    Tool::Bubble if hit.is_none() && !was_editing => {
                        self.push_undo();
                        let off = vec2(40.0, -70.0) * (tsize / 24.0);
                        self.doc.shapes.push(mk(Kind::Bubble { tail: p, pos: p + off, text: String::new(), size: tsize }));
                        self.start_editing(self.doc.shapes.len() - 1);
                    }
                    Tool::Crop | Tool::Blur => {}
                    _ => {
                        // a click (no drag) picks an existing item with any tool
                        self.selected = hit;
                    }
                }
            }
        }
    }

    fn resize_dialog(&mut self, ctx: &Context, font: &AnnotFont) {
        let Some((mut w, mut h, mut keep)) = self.resize else { return };
        let (ow, oh) = self.size();
        let mut open = true;
        let mut apply = false;
        let mut close = false;
        egui::Window::new("Resize")
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
            .show(ctx, |ui| {
                egui::Grid::new("resize_grid").num_columns(2).show(ui, |ui| {
                    ui.label("Width:");
                    if ui.add(egui::DragValue::new(&mut w).range(1..=20000).suffix(" px")).changed() && keep {
                        h = ((w as f32) * oh as f32 / ow as f32).round().max(1.0) as u32;
                    }
                    ui.end_row();
                    ui.label("Height:");
                    if ui.add(egui::DragValue::new(&mut h).range(1..=20000).suffix(" px")).changed() && keep {
                        w = ((h as f32) * ow as f32 / oh as f32).round().max(1.0) as u32;
                    }
                    ui.end_row();
                });
                ui.checkbox(&mut keep, "Save proportions");
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        close = true;
                    }
                    if ui.button("OK").clicked() {
                        apply = true;
                    }
                });
            });
        if apply {
            self.resize = None;
            self.apply_resize(w, h, font);
        } else if close || !open {
            self.resize = None;
        } else {
            self.resize = Some((w, h, keep));
        }
    }
}

#[derive(Default)]
struct Vec2Opt(Option<egui::Vec2>);

fn badge(p: &Painter, at: Pos2, align: Align2, text: &str) {
    let galley = p.layout_no_wrap(text.to_string(), FontId::proportional(12.0), Color32::WHITE);
    let r = align.anchor_size(at, galley.size() + vec2(12.0, 6.0));
    p.rect_filled(r, 6.0, Color32::from_black_alpha(190));
    p.galley(r.min + vec2(6.0, 3.0), galley, Color32::WHITE);
}

fn dashed_rect(p: &Painter, r: Rect, c: Color32) {
    let pts = [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()];
    p.add(EShape::dashed_line(&pts, Stroke::new(1.0_f32, Color32::from_black_alpha(160)), 5.0, 0.0));
    p.add(EShape::dashed_line(&pts, Stroke::new(1.0_f32, c), 5.0, 5.0));
}

fn paint_prims(p: &Painter, prims: &[Prim], to_screen: &dyn Fn(Pos2) -> Pos2, scale: f32, full: Rect, em_ratio: f32) {
    for prim in prims {
        match prim {
            Prim::Stroke { pts, width, color, closed } => {
                let w = (width * scale).max(0.6);
                let pts: Vec<Pos2> = pts.iter().map(|q| to_screen(*q)).collect();
                if pts.len() == 1 {
                    p.circle_filled(pts[0], w / 2.0, *color);
                } else if *closed {
                    p.add(EShape::closed_line(pts, Stroke::new(w, *color)));
                } else {
                    p.circle_filled(pts[0], w / 2.0, *color);
                    p.circle_filled(*pts.last().unwrap(), w / 2.0, *color);
                    p.add(EShape::line(pts, Stroke::new(w, *color)));
                }
            }
            Prim::Fill { pts, color } => {
                let pts: Vec<Pos2> = pts.iter().map(|q| to_screen(*q)).collect();
                p.add(EShape::convex_polygon(pts, *color, Stroke::NONE));
            }
            Prim::Poly { pts, color } => {
                // concave but star-shaped from the first point: a triangle fan,
                // with a thin outline of the same colour for smooth edges
                let pts: Vec<Pos2> = pts.iter().map(|q| to_screen(*q)).collect();
                let mut mesh = egui::Mesh::default();
                for q in &pts {
                    mesh.colored_vertex(*q, *color);
                }
                for i in 1..pts.len().saturating_sub(1) {
                    mesh.add_triangle(0, i as u32, i as u32 + 1);
                }
                p.add(EShape::mesh(mesh));
                p.add(EShape::closed_line(pts, Stroke::new(0.8_f32, *color)));
            }
            Prim::Text { pos, text, size, color } => {
                p.text(
                    to_screen(*pos),
                    Align2::LEFT_TOP,
                    text,
                    FontId::new(size * scale / em_ratio, FontFamily::Name("annot".into())),
                    *color,
                );
            }
            Prim::DarkenOutside { r } => {
                for part in shapes::outside_parts(full, *r) {
                    p.rect_filled(
                        Rect::from_min_max(to_screen(part.min), to_screen(part.max)),
                        0.0,
                        Color32::from_rgba_unmultiplied(0, 0, 0, shapes::DARKEN_ALPHA),
                    );
                }
            }
        }
    }
}
