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
use crate::i18n::t;
use crate::settings::{Settings, LINE_WIDTH_MAX, LINE_WIDTH_MIN};
use crate::shapes::{self, AnnotFont, Handle, Kind, Prim, Shape, TextStyle, TEXT_SIZE_MAX, TEXT_SIZE_MIN};
use crate::theme::{self, Backdrop, Pal};

/// Palette from the original's tool-preferences panel. Pure red is the
/// starting colour; the original's darker red (0xe50404) is left out because
/// next to it the two looked like the same swatch twice.
pub const PALETTE: [u32; 9] =
    [0xff0000, 0xe504c3, 0x53e504, 0x1f63be, 0xffea00, 0xff7200, 0x898989, 0x000000, 0xffffff];

/// Crop highlighter: black at 30 % outside the crop (0x100775c28, colour
/// constant 0x100904c20 = 0, 0, 0, 0.3).
pub const CROP_DIM_ALPHA: u8 = 77;
pub const UNDO_LIMIT: usize = 40;
/// Top bars: distance from the window edge, gap between bars, and the height
/// one row of bars takes.
const BAR_MARGIN: f32 = 12.0;
const BAR_GAP: f32 = 10.0;
const BAR_ROW: f32 = 54.0;
/// Zoom limits (screen points per picture pixel).
const ZOOM_MIN: f32 = 0.05;
const ZOOM_MAX: f32 = 8.0;
const SHADOW_TIP: &str = "Drop shadow around the picture and under the annotations";
/// Radius of a selection handle and how far from it a press still grabs it
/// (screen points).
const HANDLE_RADIUS: f32 = 4.5;
const HANDLE_GRAB: f32 = 9.0;
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


    /// Title, shortcut and hint, from the original's tooltip and hotkey texts.
    fn tip(self) -> (&'static str, &'static str, &'static str) {
        match self {
            Tool::Select => (
                t("Select & move"),
                "V",
                t("Click an item to select it, drag to move it\nDouble-click a text to edit it · Delete removes it"),
            ),
            Tool::Arrow => (t("Arrow"), "⌘1", t("Hold ⇧ to draw at 45°\nHold ⌘ to draw from arrow’s head to tail")),
            Tool::Rect => (
                t("Rectangle"),
                "⌘2",
                t("Hold ⇧ to draw a square\nHold ⌥ to dark all except selected area\nHold ⌘ to draw filled rectangle"),
            ),
            Tool::Line => (t("Line"), "⌘3", t("Hold ⇧ to draw at 45°")),
            Tool::Ellipse => (t("Ellipse"), "⌘4", t("Hold ⇧ to draw a circle\nHold ⌘ to draw filled ellipse")),
            Tool::Pen => (t("Pen"), "⌘5", t("Hold ⌘ to convert into highlighter")),
            Tool::Text => (t("Text"), "⌘6", t("Click to type · click an existing text to edit it")),
            Tool::Bubble => (t("Text with arrow"), "⌘7", t("Drag from the point to where the note goes, then type")),
            Tool::Blur => (t("Blur"), "⌘8", t("Drag over what you want to hide\nHold ⇧ for a square")),
            Tool::Crop => (t("Crop"), "", t("Hold ⇧ for a square\nEnter to apply, Esc to cancel")),
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
    /// The picture strongly blurred, sampled by Blur items.
    blur_tex: Option<TextureHandle>,
    /// What the floating panels must reproduce behind them (the aurora and
    /// the faded, blurred picture) so the picture does not show through.
    backdrop: Vec<Backdrop>,
    tex_dirty: bool,
    pub tool: Tool,
    drag_start: Option<Pos2>,
    pending: Option<Shape>,
    selected: Option<usize>,
    moving: bool,
    /// A handle of the selected item being dragged: which one, the item as
    /// it was when the drag began, its selection frame then, and where it began.
    reshaping: Option<(Handle, Shape, Rect, Pos2)>,
    editing: Option<usize>,
    focus_text: bool,
    style_undo_armed: bool,
    crop: Option<Rect>,
    crop_dragging: bool,
    zoom: Option<f32>,
    last_scale: f32,
    resize: Option<(u32, u32, bool)>,
    /// The Shadow toggle as of this frame (used when annotations get baked
    /// into the picture by crop, rotate or resize).
    shadows: bool,
    /// Widths of the tools, zoom and actions bars as last drawn, and whether
    /// the zoom bar had to drop to a second row because they don't fit side
    /// by side.
    bar_widths: [f32; 2],
    /// The window is too narrow for both top bars in one row: the right one
    /// (zoom and actions) goes on a second row.
    actions_below: bool,
    /// Scroll position to apply to the canvas on the next frame (zooming
    /// keeps the point under the pointer in place).
    pending_scroll: Option<egui::Vec2>,
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
            blur_tex: None,
            backdrop: Vec::new(),
            tex_dirty: true,
            tool: Tool::Arrow,
            drag_start: None,
            pending: None,
            selected: None,
            moving: false,
            reshaping: None,
            editing: None,
            focus_text: false,
            style_undo_armed: true,
            crop: None,
            crop_dragging: false,
            zoom: None,
            last_scale: 1.0,
            resize: None,
            shadows: true,
            bar_widths: [0.0; 2],
            actions_below: false,
            pending_scroll: None,
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
    /// The picture with all annotations drawn into it and, with `shadow`,
    /// shadows under the annotations and a drop shadow around the picture.
    pub fn flattened(&self, font: &AnnotFont, shadow: bool) -> RgbaImage {
        let img = shapes::bake(&self.doc.image, &self.doc.shapes, font, shadow);
        if shadow {
            shapes::drop_shadow(&img)
        } else {
            img
        }
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
            self.doc.image = shapes::bake(&self.doc.image, &self.doc.shapes, font, self.shadows);
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
            let small = shapes::redact_small(&self.doc.image);
            let bi = egui::ColorImage::from_rgba_unmultiplied([small.width() as usize, small.height() as usize], small.as_raw());
            match &mut self.blur_tex {
                Some(t) => t.set(bi, TextureOptions::LINEAR),
                None => self.blur_tex = Some(ctx.load_texture("editor-blur", bi, TextureOptions::LINEAR)),
            }
            self.tex_dirty = false;
        }
        self.texture.clone().unwrap()
    }

    // ------------------------------------------------------------------ UI

    pub fn ui(&mut self, ctx: &Context, s: &mut Settings, font: &AnnotFont) -> Vec<Cmd> {
        let pal = theme::pal(ctx);
        let mut cmds = Vec::new();
        self.shadows = s.editor_shadow;
        self.shortcuts(ctx, s, font, &mut cmds);
        // the bars stay clear of the (invisible) title bar and its buttons
        let mut avail = ctx.available_rect();
        avail.min.y += theme::TITLEBAR;
        // two top bars: tools on the left, zoom and actions on the right; if
        // they do not fit side by side the right one goes on a second row
        let [tw, aw] = self.bar_widths;
        let actions_below = BAR_MARGIN * 2.0 + tw + aw + BAR_GAP > avail.width();
        if actions_below != self.actions_below {
            self.actions_below = actions_below;
            ctx.request_repaint();
        }
        egui::CentralPanel::default().frame(Frame::new()).show(ctx, |ui| {
            self.canvas(ui, s, font, &pal);
        });
        let tools = self.tools_panel(ctx, s, font, &pal, avail, &mut cmds);
        let actions_at = if self.actions_below { vec2(-BAR_MARGIN, BAR_MARGIN + BAR_ROW) } else { vec2(-BAR_MARGIN, BAR_MARGIN) };
        let actions = self.actions_panel(ctx, s, &pal, avail.right_top() + actions_at, &mut cmds);
        self.style_panel(ctx, s, &pal, avail);
        self.bar_widths = [tools.width(), actions.width()];
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
                self.zoom = Some((self.last_scale * 1.25).min(ZOOM_MAX));
            }
            if i.consume_key(Modifiers::COMMAND, Key::Minus) {
                self.zoom = Some((self.last_scale / 1.25).max(ZOOM_MIN));
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
        let hov = theme::hover_amount(ui.ctx(), resp.id, resp.hovered() && !selected);
        if selected {
            theme::glow(p, rect, 9, pal, 1.0);
            theme::accent_fill(p, rect, 9.0, pal, 1.0);
        } else if hov > 0.0 {
            theme::hover_fill(p, rect, 9.0, pal, if pal.dark { 0.35 } else { 0.28 } * hov);
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

    fn tools_panel(&mut self, ctx: &Context, s: &mut Settings, font: &AnnotFont, pal: &Pal, avail: Rect, cmds: &mut Vec<Cmd>) -> Rect {
        let bds = self.backdrop.clone();
        theme::floating(ctx, "tools_panel", Align2::LEFT_TOP, avail.left_top() + vec2(BAR_MARGIN, BAR_MARGIN), pal, &bds, |ui| {
            if Self::tool_button(ui, pal, s, Icon::Back, false, (t("Home"), "", t("Back to the capture screen"))).clicked() {
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
            if Self::tool_button(ui, pal, s, Icon::Rotate, false, (t("Rotate clockwise"), "⌘→", t("Hold ⌘ to rotate counterclockwise")))
                .clicked()
            {
                let ccw = ui.input(|i| i.modifiers.command);
                self.rotate(!ccw, font);
            }
            if Self::tool_button(ui, pal, s, Icon::Resize, false, (t("Resize"), "", "")).clicked() {
                let (w, h) = self.size();
                self.resize = Some((w, h, true));
            }
            theme::divider(ui, pal);
            if Self::tool_button(ui, pal, s, Icon::Undo, false, (t("Undo"), "⌘Z", "")).clicked() {
                self.undo();
            }
            if Self::tool_button(ui, pal, s, Icon::Redo, false, (t("Redo"), "⇧⌘Z", "")).clicked() {
                self.redo();
            }
        })
        .response
        .rect
    }

    /// The right-hand bar: the zoom menu, Copy and Open in… as icons, and
    /// Save.
    fn actions_panel(&mut self, ctx: &Context, s: &Settings, pal: &Pal, at: Pos2, cmds: &mut Vec<Cmd>) -> Rect {
        let bds = self.backdrop.clone();
        theme::floating(ctx, "actions_panel", Align2::RIGHT_TOP, at, pal, &bds, |ui| {
            self.zoom_menu(ui, pal);
            theme::divider(ui, pal);
            let c = Self::tool_button(ui, pal, s, Icon::Copy, false, (t("Copy image  ⌘C"), "", ""));
            if c.clicked() {
                cmds.push(Cmd::Copy);
            }
            let e = Self::tool_button(ui, pal, s, Icon::OpenExternal, false, (t("External editor  ⌘E"), "", ""));
            if e.clicked() {
                cmds.push(Cmd::External);
            }
            ui.add_space(2.0);
            let r = theme::gradient_button(ui, t("Save"), vec2(64.0, 32.0));
            if r.clicked() {
                cmds.push(Cmd::Save);
            }
            if s.show_tooltips_editor {
                r.on_hover_text(t("Save  ⌘S"));
            }
        })
        .response
        .rect
    }

    /// "100% ⌄": a click opens the zoom menu (with the picture size), a
    /// double-click switches between fitting the window and 100%.
    fn zoom_menu(&mut self, ui: &mut Ui, pal: &Pal) {
        let ppp = ui.ctx().pixels_per_point();
        let pct = (self.last_scale * ppp * 100.0).round();
        // fixed width, so the bar does not jump as the number changes
        let resp = ui.add(
            egui::Button::new(RichText::new(format!("{pct}%\u{2002}\u{2002}")).size(12.5).color(pal.text))
                .fill(Color32::TRANSPARENT)
                .min_size(vec2(72.0, 32.0))
                .corner_radius(9.0),
        );
        // the chevron, drawn (the fonts at hand have no good one)
        let c = pos2(resp.rect.right() - 12.0, resp.rect.center().y + 1.0);
        let stroke = Stroke::new(1.4_f32, pal.hint);
        ui.painter().add(egui::Shape::line(vec![c + vec2(-3.5, -2.0), c + vec2(0.0, 1.5), c + vec2(3.5, -2.0)], stroke));
        let resp = resp.on_hover_text(t("Zoom  ·  double-click: fit / 100%"));
        if resp.double_clicked() {
            self.zoom = if self.zoom.is_none() { Some(1.0 / ppp) } else { None };
        }
        egui::Popup::menu(&resp).close_behavior(egui::PopupCloseBehavior::CloseOnClick).show(|ui| {
            ui.set_min_width(200.0);
            if ui.add(egui::Button::new(t("Zoom in")).shortcut_text("⌘+")).clicked() {
                self.zoom = Some((self.last_scale * 1.25).min(ZOOM_MAX));
            }
            if ui.add(egui::Button::new(t("Zoom out")).shortcut_text("⌘−")).clicked() {
                self.zoom = Some((self.last_scale / 1.25).max(ZOOM_MIN));
            }
            ui.separator();
            if ui.add(egui::Button::new(t("Fit to window")).shortcut_text("⌘0")).clicked() {
                self.zoom = None;
            }
            if ui.add(egui::Button::new(t("Original size"))).clicked() {
                self.zoom = Some(1.0 / ppp);
            }
            ui.separator();
            let (w, h) = self.size();
            ui.label(RichText::new(format!("{w} × {h} px")).size(12.0).color(pal.hint));
        });
    }

    /// The always-visible style bar. It edits the selected item if there is
    /// one, and sets the defaults for new items either way.
    fn style_panel(&mut self, ctx: &Context, s: &mut Settings, pal: &Pal, avail: Rect) {
        let bds = self.backdrop.clone();
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
                    Kind::Arrow { .. } => t("Selected arrow"),
                    Kind::Line { .. } => t("Selected line"),
                    Kind::Rect { .. } => t("Selected rectangle"),
                    Kind::Ellipse { .. } => t("Selected ellipse"),
                    Kind::Pen { .. } => t("Selected drawing"),
                    Kind::Text { .. } => t("Selected text"),
                    Kind::Bubble { .. } => t("Selected note"),
                    Kind::Blur { .. } => t("Selected blur area"),
                };
                let label = if self.editing.is_some() { t("Typing — Esc to finish").to_string() } else { what.to_string() };
                (sh.color, sh.width, s.editor_shadow, ts, st, sh.is_text(), matches!(sh.kind, Kind::Bubble { .. }), label)
            }
            None => {
                let text_tool = matches!(self.tool, Tool::Text | Tool::Bubble);
                let label = match self.tool {
                    Tool::Text => t("Click on the picture to type").to_string(),
                    Tool::Select => t("Click an item to change it").to_string(),
                    Tool::Arrow => t("New arrow").to_string(),
                    Tool::Rect => t("New rectangle").to_string(),
                    Tool::Line => t("New line").to_string(),
                    Tool::Ellipse => t("New ellipse").to_string(),
                    Tool::Pen => t("New drawing").to_string(),
                    Tool::Bubble => t("New text with arrow").to_string(),
                    Tool::Blur => t("New blur area").to_string(),
                    Tool::Crop => t("Crop area").to_string(),
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
        // blur areas have no colour or size: the bar just explains them
        let is_blur = match target {
            Some(i) => matches!(self.doc.shapes[i].kind, Kind::Blur { .. }),
            None => self.tool == Tool::Blur,
        };
        if is_blur {
            theme::floating(ctx, "style_panel", Align2::CENTER_BOTTOM, pos2(avail.center().x, avail.bottom() - 14.0), pal, &bds, |ui| {
                ui.add_sized(vec2(118.0, 32.0), egui::Label::new(RichText::new(label).size(12.0).color(pal.hint)).truncate());
                theme::divider(ui, pal);
                ui.label(RichText::new(t("Drag over anything you want to hide · ⇧ for a square")).size(12.0).color(pal.text));
                theme::divider(ui, pal);
                shadow_toggle(ui, &mut s.editor_shadow).on_hover_text(t(SHADOW_TIP));
            });
            return;
        }
        let mut new_color = color;
        let mut new_width = width;
        let mut new_shadow = shadow;
        let mut new_tsize = tsize;
        let mut new_style = tstyle;
        let mut slider_active = false;
        let mut slider_started = false;

        theme::floating(ctx, "style_panel", Align2::CENTER_BOTTOM, pos2(avail.center().x, avail.bottom() - 14.0), pal, &bds, |ui| {
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
                    pnt.circle_filled(r.center(), 11.5, theme::mix(pal.accent, pal.accent2, 0.35));
                    pnt.circle_filled(r.center(), 9.8, if pal.dark { Color32::from_rgb(0x22, 0x1c, 0x3c) } else { Color32::WHITE });
                }
                pnt.circle_filled(r.center(), rad, col);
                // a little light on top of each swatch, like a glass bead
                pnt.circle_filled(r.center() - vec2(0.0, rad * 0.35), rad * 0.45, Color32::from_white_alpha(38));
                pnt.circle_stroke(r.center(), rad, Stroke::new(1.0_f32, Color32::from_black_alpha(if pal.dark { 60 } else { 28 })));
                if resp.clicked() {
                    new_color = col;
                }
            }
            // any other colour: a rainbow square that opens a colour picker
            let custom = !PALETTE.iter().map(|c| hex(*c)).any(|c| c.r() == new_color.r() && c.g() == new_color.g() && c.b() == new_color.b());
            let (r, resp) = ui.allocate_exact_size(vec2(26.0, 32.0), Sense::click());
            let sq = Rect::from_center_size(r.center(), if resp.hovered() { vec2(20.0, 20.0) } else { vec2(18.0, 18.0) });
            if custom {
                ui.painter().rect_stroke(sq.expand(3.5), 7.0, Stroke::new(2.0_f32, pal.accent), egui::StrokeKind::Middle);
            }
            paint_rainbow(ui.painter(), sq, 5.0);
            ui.painter().rect_stroke(sq, 5.0, Stroke::new(1.0_f32, pal.border), egui::StrokeKind::Inside);
            let resp = resp.on_hover_text(t("Any colour"));
            egui::Popup::from_toggle_button_response(&resp)
                .align(egui::RectAlign::TOP)
                .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                .show(|ui| {
                    let mut c = new_color;
                    if crate::colour_picker::show(ui, &mut c, pal) {
                        new_color = c;
                    }
                });
            ui.spacing_mut().item_spacing.x = 4.0;
            theme::divider(ui, pal);
            // line size with a live preview
            let (pr, _) = ui.allocate_exact_size(vec2(26.0, 32.0), Sense::hover());
            ui.painter().circle_filled(pr.center(), (new_width * 0.9 + 1.0).min(12.0), new_color);
            ui.spacing_mut().slider_width = 80.0;
            let sr = ui
                .add(egui::Slider::new(&mut new_width, LINE_WIDTH_MIN..=LINE_WIDTH_MAX).step_by(1.0).show_value(false))
                .on_hover_text(t("Size of lines, arrows and shapes  ⌘[ ⌘]"));
            slider_active |= sr.dragged() || sr.changed();
            slider_started |= sr.drag_started();
            if is_text {
                theme::divider(ui, pal);
                ui.label(RichText::new("Aa").size(15.0).strong().color(pal.text));
                ui.spacing_mut().slider_width = 80.0;
                ui.spacing_mut().interact_size = vec2(34.0, 22.0);
                let tr = ui
                    .add(egui::Slider::new(&mut new_tsize, TEXT_SIZE_MIN..=TEXT_SIZE_MAX).step_by(1.0).suffix("").logarithmic(true))
                    .on_hover_text(t("Text size"));
                slider_active |= tr.dragged() || tr.changed();
                slider_started |= tr.drag_started();
                if let Some(st) = new_style.as_mut() {
                    for (variant, tip) in [
                        (TextStyle::Plain, t("Plain text")),
                        (TextStyle::Outline, t("Text with an outline")),
                        (TextStyle::Box, t("Text on a coloured box")),
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
            shadow_toggle(ui, &mut new_shadow).on_hover_text(t(SHADOW_TIP));
        });

        // the shadow is for the whole picture, not for one item
        s.editor_shadow = new_shadow;
        let changed = new_color != color
            || new_width != width
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
        let width = (font.measure(if text_now.is_empty() { t("Type text…") } else { &text_now }, size).x * scale + size * scale * 2.0).max(60.0);
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
                .hint_text(RichText::new(t("Type text…")).font(fid).color(pal.hint));
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
        // the aurora, tinted by a faded, blurred copy of the picture
        self.backdrop.clear();
        self.backdrop.push(theme::background(ui.painter(), full_area, pal, true));
        if let Some(bt) = &self.backdrop_tex {
            let bd = Backdrop::cover(bt.id(), full_area, vec2(iw, ih)).faded(if pal.dark { 0.28 } else { 0.35 });
            bd.paint(ui.painter());
            self.backdrop.push(bd);
        }
        let avail = ui.available_size();
        let ppp = ctx.pixels_per_point();
        // leave room for the floating panels above and below
        // with the zoom bar on a second row the picture moves down under it
        let below = if self.actions_below { BAR_ROW } else { 0.0 } + theme::TITLEBAR;
        let fit = ((avail.x - 60.0) / iw).min((avail.y - 160.0 - below) / ih).min(1.0 / ppp).max(0.02);
        self.pinch_zoom(ui, full_area, avail, below, fit);
        let scale = self.zoom.unwrap_or(fit);
        self.last_scale = scale;
        let size = vec2(iw * scale, ih * scale);

        let mut area = egui::ScrollArea::both()
            .scroll_source(egui::scroll_area::ScrollSource::MOUSE_WHEEL | egui::scroll_area::ScrollSource::SCROLL_BAR)
            .auto_shrink(false);
        if let Some(off) = self.pending_scroll.take() {
            area = area.scroll_offset(off);
        }
        area.show(ui, |ui| {
                let total = vec2((size.x + 60.0).max(ui.available_width()), (size.y + 150.0).max(ui.available_height()));
                let (rect, resp) = ui.allocate_exact_size(total, Sense::click_and_drag());
                let img_rect = Rect::from_center_size(rect.center() + vec2(0.0, -4.0 + below / 2.0), size);
                self.img_rect = Some(img_rect);
                let painter = ui.painter_at(rect);
                // with Shadow on, the same drop shadow the exported picture gets
                if s.editor_shadow {
                    let (sigma, drop, _) = shapes::drop_shadow_geometry(iw, ih);
                    let sh = egui::epaint::Shadow {
                        offset: [0, (drop * scale).round() as i8],
                        blur: (sigma * 2.5 * scale).round().clamp(1.0, 255.0) as u8,
                        spread: 0,
                        color: Color32::from_black_alpha((shapes::DROP_SHADOW_ALPHA * 255.0) as u8),
                    };
                    painter.add(sh.as_shape(img_rect, CornerRadius::same(2)));
                } else {
                    painter.rect_stroke(img_rect.expand(0.5), 2.0, Stroke::new(1.0_f32, pal.border), egui::StrokeKind::Outside);
                }
                painter.add(
                    egui::epaint::RectShape::filled(img_rect, CornerRadius::same(2), Color32::WHITE)
                        .with_texture(tex.id(), Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0))),
                );

                let to_screen = |p: Pos2| img_rect.min + p.to_vec2() * scale;
                let to_img = |p: Pos2| ((p - img_rect.min) / scale).to_pos2();
                let clip = ui.painter_at(img_rect);
                let full = Rect::from_min_size(Pos2::ZERO, vec2(iw, ih));

                let blur_tex = self.blur_tex.as_ref().map(|t| t.id());
                for (i, sh) in self.doc.shapes.iter().enumerate() {
                    let mut prims = sh.prims(font, s.editor_shadow);
                    if self.editing == Some(i) {
                        // the text field draws the letters while typing
                        let n_text = prims.iter().filter(|p| matches!(p, Prim::Text { .. })).count();
                        if n_text > 0 {
                            prims.retain(|p| !matches!(p, Prim::Text { .. }));
                        }
                    }
                    paint_prims(&clip, &prims, &to_screen, scale, full, font.em_ratio(), blur_tex);
                }
                if let Some(p) = &self.pending {
                    paint_prims(&clip, &p.prims(font, s.editor_shadow), &to_screen, scale, full, font.em_ratio(), blur_tex);
                    if let Kind::Blur { r } = p.kind {
                        dashed_rect(&clip, Rect::from_min_max(to_screen(r.min), to_screen(r.max)), Color32::WHITE);
                    }
                }
                // selection frame
                if let Some(i) = self.selected {
                    if let Some(sh) = self.doc.shapes.get(i) {
                        let frame = Self::sel_frame(sh, font, scale);
                        if sh.shows_frame() {
                            let r = Rect::from_min_max(to_screen(frame.min), to_screen(frame.max));
                            painter.rect_stroke(r, 6.0, Stroke::new(1.5_f32, pal.accent), egui::StrokeKind::Middle);
                        }
                        for (_, c) in sh.handles(frame) {
                            let c = to_screen(c);
                            painter.circle_filled(c, HANDLE_RADIUS, Color32::WHITE);
                            painter.circle_stroke(c, HANDLE_RADIUS, Stroke::new(1.5_f32, pal.accent));
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
                                        if ui.button(t("Cancel")).clicked() {
                                            cancel = true;
                                        }
                                        if theme::gradient_button(ui, t("Crop"), vec2(0.0, 24.0)).clicked() {
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

    /// Pinching the trackpad, or ⌘ + scroll wheel, zooms around the pointer.
    fn pinch_zoom(&mut self, ui: &Ui, view: Rect, view_size: egui::Vec2, below: f32, fit: f32) {
        let ctx = ui.ctx();
        let (delta, pointer) = ui.input(|i| (i.zoom_delta(), i.pointer.hover_pos()));
        if (delta - 1.0).abs() < 1e-4 {
            return;
        }
        // only over the canvas itself, not over the floating bars
        let Some(p) = pointer.filter(|p| view.contains(*p)) else { return };
        if ctx.layer_id_at(p).is_some_and(|l| l.order != egui::Order::Background) {
            return;
        }
        let old = self.zoom.unwrap_or(fit);
        let new = (old * delta).clamp(ZOOM_MIN, ZOOM_MAX);
        if new == old {
            return;
        }
        self.zoom = Some(new);
        // keep the picture point under the pointer where it is
        let (iw, ih) = self.size();
        let Some(img) = self.img_rect else { return };
        let f = (p - img.min) / img.size();
        let size = vec2(iw as f32, ih as f32) * new;
        let total = vec2((size.x + 60.0).max(view_size.x), (size.y + 150.0).max(view_size.y));
        let img_min = total / 2.0 + vec2(0.0, -4.0 + below / 2.0) - size / 2.0;
        let off = img_min + f * size - (p - view.min);
        let max = (total - view_size).max(egui::Vec2::ZERO);
        self.pending_scroll = Some(off.clamp(egui::Vec2::ZERO, max));
        ctx.request_repaint();
    }

    /// The selection frame around an item, in picture pixels (4 screen
    /// points outside its bounds).
    fn sel_frame(sh: &Shape, font: &AnnotFont, scale: f32) -> Rect {
        sh.bounds(font).expand(4.0 / scale.max(0.01))
    }

    /// The handle of the selected item under `p` (picture pixels), if any,
    /// with the item's selection frame.
    fn handle_at(&self, p: Pos2, scale: f32, font: &AnnotFont) -> Option<(Handle, Rect)> {
        let sh = self.doc.shapes.get(self.selected?)?;
        let frame = Self::sel_frame(sh, font, scale);
        let grab = HANDLE_GRAB / scale.max(0.01);
        sh.handles(frame)
            .into_iter()
            .filter(|(_, c)| c.distance(p) <= grab)
            .min_by(|x, y| x.1.distance(p).total_cmp(&y.1.distance(p)))
            .map(|(h, _)| (h, frame))
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
        let tsize = s.editor_text_size;
        let tstyle = s.editor_text_style;

        if let Some(hp) = resp.hover_pos() {
            let over = self.hit_test(to_img(hp), scale, font);
            let handle = match &self.reshaping {
                Some(r) => Some(r.0),
                None if self.moving || self.tool == Tool::Crop => None,
                None => self.handle_at(to_img(hp), scale, font).map(|h| h.0),
            };
            let icon = match self.tool {
                _ if handle.is_some() => match handle {
                    Some(Handle::Corner(0 | 3)) => egui::CursorIcon::ResizeNwSe,
                    Some(Handle::Corner(_)) => egui::CursorIcon::ResizeNeSw,
                    _ => egui::CursorIcon::Crosshair,
                },
                Tool::Select if over.is_some() => egui::CursorIcon::Grab,
                Tool::Select => egui::CursorIcon::Default,
                Tool::Text if over.map(|i| self.doc.shapes[i].is_text()).unwrap_or(false) => egui::CursorIcon::Text,
                Tool::Text => egui::CursorIcon::Text,
                Tool::Crop => egui::CursorIcon::Crosshair,
                _ if over.is_some() && over == self.selected => egui::CursorIcon::Grab,
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
                let mk = |kind| Shape { kind, color, width };
                let free_tool = self.tool != Tool::Crop;
                let handle = if free_tool { self.handle_at(p, scale, font) } else { None };
                let on_selected = self.selected.is_some() && self.hit_test(p, scale, font) == self.selected;
                if let (Some((h, frame)), Some(i)) = (handle, self.selected) {
                    // a handle of the selected item reshapes it, whatever the tool
                    let orig = self.doc.shapes[i].clone();
                    self.push_undo();
                    self.reshaping = Some((h, orig, frame, p));
                } else if on_selected && free_tool && self.tool != Tool::Select {
                    // dragging the selected item itself moves it, whatever the tool
                    self.push_undo();
                    self.moving = true;
                } else {
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
                        Tool::Blur => self.pending = Some(mk(Kind::Blur { r: Rect::from_two_pos(p, p) })),
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
        }

        if resp.dragged() {
            if let Some((h, orig, frame, start)) = &self.reshaping {
                if let (Some(i), Some(cur)) = (self.selected, resp.interact_pointer_pos()) {
                    let delta = to_img(cur) - *start;
                    if let Some(sh) = self.doc.shapes.get_mut(i) {
                        sh.drag_handle(orig, *h, *frame, delta, mods.shift);
                    }
                }
            } else if self.moving {
                if let Some(i) = self.selected {
                    let d = resp.drag_delta() / scale;
                    if let Some(sh) = self.doc.shapes.get_mut(i) {
                        sh.translate(d);
                    }
                }
            } else if let (Some(a), Some(cur)) = (self.drag_start, resp.interact_pointer_pos()) {
                let b = to_img(cur);
                match self.tool {
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
                        Kind::Blur { r } => {
                            let b2 = if mods.shift { shapes::square(a, b) } else { b };
                            *r = Rect::from_two_pos(a, b2).intersect(full);
                        }
                        Kind::Text { .. } => {}
                    }
                }
            }
        }

        if resp.drag_stopped() {
            self.drag_start = None;
            self.reshaping = None;
            if self.moving {
                self.moving = false;
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
                    Kind::Rect { r, .. } | Kind::Ellipse { r, .. } | Kind::Blur { r } => r.width() > 2.0 && r.height() > 2.0,
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
                if self.tool != Tool::Crop && self.handle_at(p, scale, font).is_some() {
                    // a click on a handle keeps the item selected
                    return;
                }
                let hit = self.hit_test(p, scale, font);
                let mk = |kind| Shape { kind, color, width };
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
                    Tool::Crop => {}
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
        egui::Window::new(t("Resize"))
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
            .show(ctx, |ui| {
                egui::Grid::new("resize_grid").num_columns(2).show(ui, |ui| {
                    ui.label(t("Width:"));
                    if ui.add(egui::DragValue::new(&mut w).range(1..=20000).suffix(" px")).changed() && keep {
                        h = ((w as f32) * oh as f32 / ow as f32).round().max(1.0) as u32;
                    }
                    ui.end_row();
                    ui.label(t("Height:"));
                    if ui.add(egui::DragValue::new(&mut h).range(1..=20000).suffix(" px")).changed() && keep {
                        w = ((h as f32) * ow as f32 / oh as f32).round().max(1.0) as u32;
                    }
                    ui.end_row();
                });
                ui.checkbox(&mut keep, t("Save proportions"));
                ui.horizontal(|ui| {
                    if ui.button(t("Cancel")).clicked() {
                        close = true;
                    }
                    if ui.button(t("OK")).clicked() {
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

/// A rounded square filled with a colour wheel: every hue around the edge,
/// fading to white in the middle.
fn paint_rainbow(p: &Painter, r: Rect, radius: f32) {
    let rad = radius.min(r.width() / 2.0).min(r.height() / 2.0);
    let corners = [
        (pos2(r.max.x - rad, r.min.y + rad), -90.0f32),
        (pos2(r.max.x - rad, r.max.y - rad), 0.0),
        (pos2(r.min.x + rad, r.max.y - rad), 90.0),
        (pos2(r.min.x + rad, r.min.y + rad), 180.0),
    ];
    let c = r.center();
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(c, Color32::WHITE);
    for (k, start) in corners {
        for i in 0..=8 {
            let a = (start + i as f32 * 11.25).to_radians();
            let q = pos2(k.x + rad * a.cos(), k.y + rad * a.sin());
            let hue = ((q.y - c.y).atan2(q.x - c.x) / std::f32::consts::TAU).rem_euclid(1.0);
            mesh.colored_vertex(q, egui::ecolor::Hsva::new(hue, 0.95, 1.0, 1.0).into());
        }
    }
    let n = mesh.vertices.len() as u32;
    for i in 1..n {
        mesh.add_triangle(0, i, if i + 1 < n { i + 1 } else { 1 });
    }
    p.add(EShape::mesh(mesh));
}

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

fn paint_prims(
    p: &Painter,
    prims: &[Prim],
    to_screen: &dyn Fn(Pos2) -> Pos2,
    scale: f32,
    full: Rect,
    em_ratio: f32,
    blur_tex: Option<egui::TextureId>,
) {
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
            Prim::Blur { r } => {
                let r = r.intersect(full);
                if let Some(t) = blur_tex {
                    let uv = Rect::from_min_max(
                        pos2(r.min.x / full.width(), r.min.y / full.height()),
                        pos2(r.max.x / full.width(), r.max.y / full.height()),
                    );
                    p.image(t, Rect::from_min_max(to_screen(r.min), to_screen(r.max)), uv, Color32::WHITE);
                }
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

/// The Shadow toggle of the style bar: its icon and label.
fn shadow_toggle(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let id = ui.id().with("shadow_icon");
    let out = egui::Button::selectable(*on, (egui::Atom::custom(id, vec2(15.0, 15.0)), t("Shadow"))).atom_ui(ui);
    if let Some(r) = out.rect(id) {
        let c = ui.style().interact_selectable(&out.response, *on).text_color();
        icons::paint(ui.painter(), r, Icon::Shadow, c);
    }
    let mut resp = out.response;
    if resp.clicked() {
        *on = !*on;
        resp.mark_changed();
    }
    resp
}
