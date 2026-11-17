//! The application: menu-bar icon, global hotkeys, the main window (home
//! screen or editor), the Preferences window and the capture flow.

use egui::{vec2, Align2, Color32, Context, FontFamily, Frame, Margin, Rect, RichText, Sense, Stroke, ViewportBuilder, ViewportId};
use image::RgbaImage;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;

use crate::capture::{self, CaptureKind, CaptureOptions, CaptureResult, Recorder};
use crate::editor::{self, Cmd, Editor};
use crate::hotkeys::{Action, GlobalKeys};
use crate::i18n::{self, t, tf, Lang};
use crate::icons::{self, Icon};
use crate::overlay::Overlay;
use crate::selector::Selector;
use crate::settings::{AfterShot, Settings, APP_NAME};
use crate::settings_ui::{self, SettingsUi};
use crate::shapes::{AnnotFont, Kind, Shape, TextStyle};
use crate::theme;

enum Msg {
    Captured(CaptureResult),
    Frozen(CaptureResult),
    Previous(CaptureResult),
    Hotkey(u32),
    Menu(String),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Command {
    Capture(CaptureKind),
    PreviousArea,
    RecordVideo,
    StopVideo,
    Open,
    OpenClipboard,
    OpenLatest,
    Preferences,
    ShowWindow,
    Quit,
}

/// How long a capture waits after hiding QuickSnap's windows, so they are
/// really off screen when the picture is taken.
const HIDE_BEFORE_CAPTURE: std::time::Duration = std::time::Duration::from_millis(250);

/// Room above a home tile's icon and below its shortcut keycaps.
const TILE_PAD: f32 = 20.0;

/// How long a capture of a rectangle waits for the area selector's windows
/// to leave the screen.
const SELECTOR_GONE: std::time::Duration = std::time::Duration::from_millis(120);

struct Toast {
    text: String,
    at: f64,
}

pub struct ScreenshotJob {
    pub path: PathBuf,
    pub frame: u32,
    pub requested: bool,
}

pub struct App {
    settings: Settings,
    saved_json: String,
    persist: bool,
    editor: Option<Editor>,
    font: AnnotFont,
    prefs: SettingsUi,
    keys: Option<GlobalKeys>,
    keys_paused: bool,
    tray: Option<tray_icon::TrayIcon>,
    tray_items: HashMap<String, Command>,
    record_item: Option<tray_icon::menu::MenuItem>,
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
    busy: Option<String>,
    recorder: Option<Recorder>,
    overlay: Option<Overlay>,
    /// "Capture Area" selection layer, while it is up.
    selector: Option<Selector>,
    previous_area: Option<Rect>,
    latest: Option<RgbaImage>,
    toasts: Vec<Toast>,
    screenshot: Option<ScreenshotJob>,
    started: bool,
    /// Set by Quit; otherwise closing the main window only hides it.
    quitting: bool,
    /// The main window is hidden (closed to the menu bar, or out of the way
    /// of a capture).
    hidden: bool,
    /// The main window was hidden only for the capture in progress and comes
    /// back when it ends.
    restore_after_capture: bool,
    /// For checking layouts: draw one view in the main window
    /// (QUICKSNAP_VIEW=home or prefs-general / prefs-advanced / ...).
    view: Option<String>,
    applied_theme: Option<theme::ThemeChoice>,
    /// The language the menu-bar menu was built in.
    tray_lang: Option<Lang>,
    logo: Option<egui::TextureHandle>,
    /// The language whose Chinese/Japanese/Korean font is loaded, if any.
    cjk_font_for: Option<Lang>,
}

fn data_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join(APP_NAME))
}

/// System fonts for a language whose script the UI font lacks (Chinese,
/// Japanese, Korean), first one first: path and face index in the collection.
/// Only the current language's are loaded (they are large); Arial Unicode
/// covers the rest. Japanese and Korean also get the Chinese font, for the
/// simplified characters in "简体中文" in the language menu, which their own
/// fonts lack (one character from another font looked out of place).
fn cjk_fonts(lang: Lang) -> &'static [(&'static str, u32)] {
    const JA: (&str, u32) = ("/System/Library/Fonts/ヒラギノ角ゴシック W3.ttc", 0);
    const ZH: (&str, u32) = ("/System/Library/Fonts/Hiragino Sans GB.ttc", 0);
    const KO: (&str, u32) = ("/System/Library/Fonts/AppleSDGothicNeo.ttc", 0);
    match lang {
        Lang::Ja => &[JA, ZH],
        Lang::Zh => &[ZH],
        Lang::Ko => &[KO, ZH],
        _ => &[],
    }
}

fn install_fonts(ctx: &Context, font: &AnnotFont, lang: Lang) {
    let mut defs = egui::FontDefinitions::default();
    for path in ["/System/Library/Fonts/Helvetica.ttc", "/System/Library/Fonts/HelveticaNeue.ttc"] {
        if let Ok(bytes) = std::fs::read(path) {
            defs.font_data.insert(
                "ui".into(),
                Arc::new(egui::FontData { font: std::borrow::Cow::Owned(bytes), index: 0, tweak: Default::default() }),
            );
            if let Some(f) = defs.families.get_mut(&FontFamily::Proportional) {
                f.insert(0, "ui".into());
            }
            break;
        }
    }
    // Chinese, Japanese or Korean UI text in that language's own font, ahead
    // of the generic fallbacks below.
    for (i, (path, index)) in cjk_fonts(lang).iter().enumerate() {
        if let Ok(bytes) = std::fs::read(path) {
            let name = format!("cjk{i}");
            defs.font_data
                .insert(name.clone(), Arc::new(egui::FontData { font: std::borrow::Cow::Owned(bytes), index: *index, tweak: Default::default() }));
            for fam in [FontFamily::Proportional, FontFamily::Monospace] {
                if let Some(f) = defs.families.get_mut(&fam) {
                    f.push(name.clone());
                }
            }
        }
    }
    // Key symbols (⌥ ⌃ ⇧ ✕) are missing from Helvetica, so add symbol fonts as fallbacks.
    for (name, path) in [
        ("symbols", "/System/Library/Fonts/Apple Symbols.ttf"),
        ("unicode", "/System/Library/Fonts/Supplemental/Arial Unicode.ttf"),
    ] {
        if let Ok(bytes) = std::fs::read(path) {
            defs.font_data.insert(name.into(), Arc::new(egui::FontData::from_owned(bytes)));
            for fam in [FontFamily::Proportional, FontFamily::Monospace] {
                if let Some(f) = defs.families.get_mut(&fam) {
                    f.push(name.into());
                }
            }
        }
    }
    defs.font_data.insert("annot".into(), Arc::new(egui::FontData::from_owned(font.bytes.to_vec())));
    let mut fam = vec!["annot".to_string()];
    if let Some(p) = defs.families.get(&FontFamily::Proportional) {
        fam.extend(p.iter().filter(|n| n.as_str() != "annot").cloned());
    }
    defs.families.insert(FontFamily::Name("annot".into()), fam);
    ctx.set_fonts(defs);
}

/// A neutral sample picture (gradient and simple shapes) for the screenshot
/// check, so the editor can be shown without capturing the real screen.
fn sample_picture() -> RgbaImage {
    let (w, h) = (1280u32, 800u32);
    let mut img = RgbaImage::new(w, h);
    for (x, y, p) in img.enumerate_pixels_mut() {
        let t = x as f32 / w as f32;
        let u = y as f32 / h as f32;
        let r = (236.0 - 30.0 * u) as u8;
        let g = (240.0 - 20.0 * t) as u8;
        let b = (248.0 - 10.0 * u) as u8;
        *p = image::Rgba([r, g, b, 255]);
        // a fake window
        let in_win = (160..1120).contains(&x) && (110..700).contains(&y);
        if in_win {
            *p = image::Rgba([255, 255, 255, 255]);
            if y < 150 {
                *p = image::Rgba([232, 232, 236, 255]);
            }
            if (200..520).contains(&x) && (190..360).contains(&y) {
                *p = image::Rgba([(90.0 + 80.0 * t) as u8, (150.0 + 40.0 * u) as u8, 220, 255]);
            }
            for (k, row) in [400u32, 440, 480, 520, 560].iter().enumerate() {
                let len = 760 - k as u32 * 90;
                if (200..200 + len).contains(&x) && (*row..row + 14).contains(&y) {
                    *p = image::Rgba([210, 212, 218, 255]);
                }
            }
            if (580..1060).contains(&x) && (190..360).contains(&y) {
                *p = image::Rgba([246, 247, 250, 255]);
            }
        }
        for (cx, cy) in [(186u32, 130u32), (210, 130), (234, 130)] {
            let dx = x as i32 - cx as i32;
            let dy = y as i32 - cy as i32;
            if dx * dx + dy * dy < 64 {
                *p = image::Rgba([200, 90, 80, 255]);
            }
        }
    }
    img
}

fn sample_shapes() -> Vec<Shape> {
    let red = Color32::from_rgb(255, 0, 0);
    vec![
        Shape { kind: Kind::Arrow { a: egui::pos2(900.0, 560.0), b: egui::pos2(560.0, 360.0) }, color: red, width: 6.0 },
        Shape {
            kind: Kind::Rect { r: Rect::from_min_max(egui::pos2(190.0, 180.0), egui::pos2(530.0, 370.0)), filled: false, darken: false },
            color: red,
            width: 6.0,
        },
        Shape {
            kind: Kind::Bubble { tail: egui::pos2(760.0, 460.0), pos: egui::pos2(820.0, 600.0), text: "Sample note".into(), size: 24.0 },
            color: editor::hex(0x1f63be),
            width: 6.0,
        },
        Shape {
            kind: Kind::Text { pos: egui::pos2(600.0, 200.0), text: "New layout".into(), size: 40.0, style: TextStyle::Outline },
            color: editor::hex(0xff7200),
            width: 6.0,
        },
    ]
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, screenshot: Option<PathBuf>) -> App {
        let persist = screenshot.is_none();
        let settings = if persist { Settings::load() } else { Settings::default() };
        let font = AnnotFont::load(settings.main_font.as_deref());
        install_fonts(&cc.egui_ctx, &font, settings.language);
        let settings_lang = Some(settings.language);
        theme::setup_styles(&cc.egui_ctx);
        // see-through window with the desktop blurred behind it (the window
        // was created transparent for this in main.rs); screenshots for the
        // README stay opaque
        #[cfg(target_os = "macos")]
        if screenshot.is_none() {
            theme::set_vibrant(theme::apply_vibrancy(cc));
        }
        let (tx, rx) = channel();
        let saved_json = serde_json::to_string(&settings).unwrap_or_default();
        let latest = if persist {
            data_dir().and_then(|d| image::open(d.join("latest.png")).ok()).map(|i| i.to_rgba8())
        } else {
            None
        };
        let mut app = App {
            settings,
            saved_json,
            persist,
            editor: None,
            font,
            prefs: SettingsUi::new(),
            keys: None,
            keys_paused: false,
            tray: None,
            tray_items: HashMap::new(),
            record_item: None,
            tx,
            rx,
            busy: None,
            recorder: None,
            overlay: None,
            selector: None,
            previous_area: None,
            latest,
            toasts: Vec::new(),
            screenshot: screenshot.map(|p| ScreenshotJob { path: p, frame: 0, requested: false }),
            started: false,
            quitting: false,
            hidden: false,
            restore_after_capture: false,
            view: std::env::var("QUICKSNAP_VIEW").ok(),
            applied_theme: None,
            tray_lang: None,
            logo: None,
            cjk_font_for: settings_lang.filter(|l| l.is_cjk()),
        };
        if let Ok(l) = std::env::var("QUICKSNAP_LANG") {
            app.settings.language = match l.as_str() {
                "es" => Lang::Es,
                "ca" => Lang::Ca,
                "fr" => Lang::Fr,
                "de" => Lang::De,
                "pt" => Lang::Pt,
                "it" => Lang::It,
                "gl" => Lang::Gl,
                "eu" => Lang::Eu,
                "pt-br" | "ptbr" | "br" => Lang::PtBr,
                "nl" => Lang::Nl,
                "pl" => Lang::Pl,
                "ja" => Lang::Ja,
                "zh" => Lang::Zh,
                "ko" => Lang::Ko,
                _ => Lang::En,
            };
        }
        if let Ok(t) = std::env::var("QUICKSNAP_THEME") {
            app.settings.theme = if t == "dark" { theme::ThemeChoice::Dark } else { theme::ThemeChoice::Light };
        }
        if app.screenshot.is_some() {
            cc.egui_ctx.set_zoom_factor(crate::screenshot_zoom());
        }
        if app.screenshot.is_some() && app.view.is_none() {
            let mut ed = Editor::new(sample_picture(), "Sample");
            ed.add_shapes(sample_shapes());
            ed.select(3);
            app.editor = Some(ed);
        }
        app
    }

    /// Menu-bar icon and global hotkeys need the event loop running, so they
    /// are set up on the first frame.
    fn start_services(&mut self, ctx: &Context) {
        if self.started {
            return;
        }
        self.started = true;
        if self.screenshot.is_some() {
            return;
        }
        let mut keys = GlobalKeys::new();
        keys.apply(&self.settings.hotkeys);
        self.keys = Some(keys);
        {
            let tx = self.tx.clone();
            let c = ctx.clone();
            global_hotkey::GlobalHotKeyEvent::set_event_handler(Some(move |e: global_hotkey::GlobalHotKeyEvent| {
                if e.state == global_hotkey::HotKeyState::Pressed {
                    let _ = tx.send(Msg::Hotkey(e.id));
                    c.request_repaint();
                }
            }));
        }
        {
            let tx = self.tx.clone();
            let c = ctx.clone();
            tray_icon::menu::MenuEvent::set_event_handler(Some(move |e: tray_icon::menu::MenuEvent| {
                let _ = tx.send(Msg::Menu(e.id.0.clone()));
                c.request_repaint();
            }));
        }
        self.build_tray();
    }

    fn build_tray(&mut self) {
        use tray_icon::menu::{Menu, MenuItem, PredefinedMenuItem};
        let menu = Menu::new();
        let mut add = |text: &str, cmd: Option<Command>, enabled: bool| -> MenuItem {
            let item = MenuItem::new(text, enabled, None);
            if let Some(c) = cmd {
                self.tray_items.insert(item.id().0.clone(), c);
            }
            let _ = menu.append(&item);
            item
        };
        add(&tf("Show {}", &[&APP_NAME]), Some(Command::ShowWindow), true);
        let _ = menu.append(&PredefinedMenuItem::separator());
        add(t("Capture Area"), Some(Command::Capture(CaptureKind::Area)), true);
        add(t("Capture Fullscreen"), Some(Command::Capture(CaptureKind::Fullscreen)), true);
        let rec_label = if self.recorder.is_some() { t("Stop Recording") } else { t("Record Video") };
        let rec = add(rec_label, Some(Command::RecordVideo), true);
        let _ = menu.append(&PredefinedMenuItem::separator());
        add(t("Open…"), Some(Command::Open), true);
        add(t("Open from Clipboard"), Some(Command::OpenClipboard), true);
        add(t("Open Latest Image"), Some(Command::OpenLatest), true);
        let _ = menu.append(&PredefinedMenuItem::separator());
        add(t("Preferences…"), Some(Command::Preferences), true);
        let _ = menu.append(&PredefinedMenuItem::separator());
        add(t("Quit"), Some(Command::Quit), true);
        self.record_item = Some(rec);

        // One-ink lens-and-aperture glyph; as a template image macOS tints it
        // to match the menu bar (dark or light).
        let n = 44u32;
        let img = crate::appicon::render_template(n);
        if let Ok(icon) = tray_icon::Icon::from_rgba(img.into_raw(), n, n) {
            self.tray = tray_icon::TrayIconBuilder::new()
                .with_menu(Box::new(menu))
                .with_icon(icon)
                .with_icon_as_template(true)
                .with_tooltip(APP_NAME)
                .build()
                .ok();
        }
    }

    fn toast(&mut self, ctx: &Context, text: impl Into<String>) {
        let at = ctx.input(|i| i.time);
        self.toasts.push(Toast { text: text.into(), at });
        if self.toasts.len() > 4 {
            self.toasts.remove(0);
        }
    }

    /// Brings the (possibly hidden) main window back to the front.
    fn show_main(&mut self, ctx: &Context) {
        self.hidden = false;
        self.restore_after_capture = false;
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    }

    fn run(&mut self, ctx: &Context, cmd: Command) {
        match cmd {
            Command::Capture(kind) => self.start_capture(ctx, kind),
            Command::PreviousArea => self.start_previous(ctx),
            Command::RecordVideo => {
                if self.recorder.is_some() {
                    self.run(ctx, Command::StopVideo);
                } else {
                    match Recorder::start(self.settings.sound_effects) {
                        Ok(r) => {
                            self.recorder = Some(r);
                            if let Some(i) = &self.record_item {
                                i.set_text(t("Stop Recording"));
                            }
                            self.toast(ctx, t("Recording the screen…"));
                        }
                        Err(e) => self.toast(ctx, e),
                    }
                }
            }
            Command::StopVideo => {
                if let Some(r) = self.recorder.take() {
                    if let Some(i) = &self.record_item {
                        i.set_text(t("Record Video"));
                    }
                    match r.stop(&self.settings) {
                        Ok(p) => {
                            self.toast(ctx, tf("Video saved to {}", &[&p.display()]));
                            capture::reveal_in_finder(&p);
                        }
                        Err(e) => self.toast(ctx, e),
                    }
                }
            }
            Command::Open => {
                if let Some(p) = rfd::FileDialog::new()
                    .set_title(t("Open"))
                    .add_filter(t("Images"), &["png", "jpg", "jpeg", "gif", "bmp", "tiff", "tif", "webp"])
                    .pick_file()
                {
                    match image::open(&p) {
                        Ok(i) => {
                            let title = p.file_stem().and_then(|s| s.to_str()).unwrap_or("Image").to_string();
                            let mut ed = Editor::new(i.to_rgba8(), &title);
                            ed.source = Some(p);
                            self.editor = Some(ed);
                            self.show_main(ctx);
                        }
                        Err(e) => self.toast(ctx, tf("Can't open this file: {}", &[&e])),
                    }
                }
            }
            Command::OpenClipboard => match capture::image_from_clipboard() {
                Ok(img) => {
                    self.editor = Some(Editor::new(img, "Clipboard"));
                    self.show_main(ctx);
                }
                Err(e) => self.toast(ctx, e),
            },
            Command::OpenLatest => match &self.latest {
                Some(img) => {
                    self.editor = Some(Editor::new(img.clone(), "Screenshot"));
                    self.show_main(ctx);
                }
                None => self.toast(ctx, t("There is no screenshot yet")),
            },
            Command::Preferences => {
                self.prefs.open = true;
                ctx.send_viewport_cmd_to(ViewportId::from_hash_of("preferences"), egui::ViewportCommand::Focus);
            }
            Command::ShowWindow => self.show_main(ctx),
            Command::Quit => {
                self.quitting = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }

    fn capture_options(&self, ctx: &Context) -> CaptureOptions {
        let scale = ctx.input(|i| i.viewport().native_pixels_per_point).unwrap_or(ctx.pixels_per_point());
        CaptureOptions::from(&self.settings, scale)
    }

    fn start_capture(&mut self, ctx: &Context, kind: CaptureKind) {
        if self.busy.is_some() || self.overlay.is_some() || self.selector.is_some() {
            return;
        }
        if kind == CaptureKind::Area {
            // QuickSnap's own live selection layer; it is drawn above
            // everything, so the windows can hide while it is up
            self.hide_for_capture(ctx);
            crate::screens::activate_app();
            self.selector = Some(Selector::new());
            return;
        }
        let opt = self.capture_options(ctx);
        let wait = self.hide_for_capture(ctx);
        let tx = self.tx.clone();
        let c = ctx.clone();
        let frozen = kind == CaptureKind::FrozenArea;
        self.busy = Some(match kind {
            CaptureKind::FullscreenDelayed => format!("Capturing in {} seconds…", crate::settings::DELAYED_CAPTURE_SECONDS),
            _ => "Capturing…".to_string(),
        });
        std::thread::spawn(move || {
            if wait {
                std::thread::sleep(HIDE_BEFORE_CAPTURE);
            }
            let msg = if frozen {
                // freeze the screen first; selection happens in the overlay
                let mut o = opt.clone();
                o.shrink_retina = false;
                Msg::Frozen(capture::capture(CaptureKind::Fullscreen, &o))
            } else {
                Msg::Captured(capture::capture(kind, &opt))
            };
            let _ = tx.send(msg);
            c.request_repaint();
        });
    }

    fn start_previous(&mut self, ctx: &Context) {
        if self.busy.is_some() || self.overlay.is_some() || self.selector.is_some() {
            return;
        }
        let mut opt = self.capture_options(ctx);
        opt.shrink_retina = false;
        let wait = self.hide_for_capture(ctx);
        let tx = self.tx.clone();
        let c = ctx.clone();
        self.busy = Some("Capturing…".into());
        std::thread::spawn(move || {
            if wait {
                std::thread::sleep(HIDE_BEFORE_CAPTURE);
            }
            let _ = tx.send(Msg::Previous(capture::capture(CaptureKind::Fullscreen, &opt)));
            c.request_repaint();
        });
    }

    /// Gets every QuickSnap window out of the way before a capture. Returns
    /// whether something was on screen (the capture then waits a moment so
    /// the windows are really gone).
    fn hide_for_capture(&mut self, ctx: &Context) -> bool {
        let was_shown = !self.hidden || self.prefs.open;
        if !self.hidden {
            self.hidden = true;
            self.restore_after_capture = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }
        self.prefs.open = false;
        self.prefs.recording = None;
        was_shown
    }

    /// After a capture: the main window comes back if it was hidden only for
    /// it and nothing else (like the editor) brought it back already.
    fn end_capture(&mut self, ctx: &Context) {
        let open = self.overlay.is_some() || self.selector.is_some();
        if self.restore_after_capture && self.hidden && !open {
            self.hidden = false;
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        }
        if !open {
            self.restore_after_capture = false;
        }
    }

    fn crop_full(&self, ctx: &Context, img: &RgbaImage, r: Rect) -> RgbaImage {
        let r = r.intersect(Rect::from_min_size(egui::Pos2::ZERO, vec2(img.width() as f32, img.height() as f32)));
        let mut out = image::imageops::crop_imm(
            img,
            r.min.x.round() as u32,
            r.min.y.round() as u32,
            r.width().round().max(1.0) as u32,
            r.height().round().max(1.0) as u32,
        )
        .to_image();
        let opt = self.capture_options(ctx);
        if opt.shrink_retina && opt.retina_scale > 1.0 {
            let w = (out.width() as f32 / opt.retina_scale).round().max(1.0) as u32;
            let h = (out.height() as f32 / opt.retina_scale).round().max(1.0) as u32;
            out = image::imageops::resize(&out, w, h, image::imageops::FilterType::Lanczos3);
        }
        out
    }

    /// What happens to a fresh screenshot ("After screenshot" setting).
    fn handle_new_image(&mut self, ctx: &Context, img: RgbaImage) {
        self.latest = Some(img.clone());
        if let Some(d) = data_dir() {
            let _ = std::fs::create_dir_all(&d);
            let _ = img.save(d.join("latest.png"));
        }
        let mut saved: Option<PathBuf> = None;
        if self.settings.always_save {
            match capture::save_to_folder(&img, &self.settings, "") {
                Ok(p) => saved = Some(p),
                Err(e) => self.toast(ctx, e),
            }
        }
        match self.settings.after_shot {
            AfterShot::OpenEditor => {
                self.editor = Some(Editor::new(img, "Screenshot"));
                self.show_main(ctx);
            }
            AfterShot::Save => {
                let p = match saved.clone() {
                    Some(p) => Ok(p),
                    None => capture::save_to_folder(&img, &self.settings, ""),
                };
                match p {
                    Ok(p) => self.toast(ctx, tf("Saved to {}", &[&p.display()])),
                    Err(e) => self.toast(ctx, e),
                }
            }
            AfterShot::ExternalEditor => {
                let file = match saved.clone() {
                    Some(p) => p,
                    None => {
                        let p = std::env::temp_dir().join(format!("{} {}.png", APP_NAME, chrono::Local::now().format("%H-%M-%S")));
                        let _ = img.save(&p);
                        p
                    }
                };
                if let Err(e) = capture::open_in_app(&self.settings.external_editor, &file) {
                    self.toast(ctx, e);
                }
            }
            AfterShot::Copy => match capture::copy_to_clipboard(&img, &self.settings, saved.as_deref()) {
                Ok(m) => self.toast(ctx, m),
                Err(e) => self.toast(ctx, e),
            },
        }
    }

    fn handle_messages(&mut self, ctx: &Context) {
        while let Ok(m) = self.rx.try_recv() {
            match m {
                Msg::Captured(res) => {
                    self.busy = None;
                    match res {
                        CaptureResult::Image(img) => self.handle_new_image(ctx, img),
                        CaptureResult::Cancelled => {}
                        CaptureResult::Failed(e) => self.toast(ctx, e),
                    }
                    self.end_capture(ctx);
                }
                Msg::Frozen(res) => {
                    self.busy = None;
                    match res {
                        CaptureResult::Image(img) => self.overlay = Some(Overlay::new(img)),
                        CaptureResult::Cancelled => {}
                        CaptureResult::Failed(e) => self.toast(ctx, e),
                    }
                    self.end_capture(ctx);
                }
                Msg::Previous(res) => {
                    self.busy = None;
                    match res {
                        CaptureResult::Image(img) => match self.previous_area {
                            Some(r) => {
                                let cut = self.crop_full(ctx, &img, r);
                                self.handle_new_image(ctx, cut);
                            }
                            None => self.overlay = Some(Overlay::new(img)),
                        },
                        CaptureResult::Cancelled => {}
                        CaptureResult::Failed(e) => self.toast(ctx, e),
                    }
                    self.end_capture(ctx);
                }
                Msg::Hotkey(id) => {
                    let action = self.keys.as_ref().and_then(|k| k.action_for(id));
                    let cmd = match action {
                        Some(Action::CaptureArea) => Some(Command::Capture(CaptureKind::Area)),
                        Some(Action::CaptureFrozenArea) => Some(Command::Capture(CaptureKind::FrozenArea)),
                        Some(Action::CaptureFullscreen) => Some(Command::Capture(CaptureKind::Fullscreen)),
                        Some(Action::CaptureFullscreenDelayed) => Some(Command::Capture(CaptureKind::FullscreenDelayed)),
                        Some(Action::OpenEditor) => Some(Command::OpenLatest),
                        Some(Action::RecordVideo) => Some(Command::RecordVideo),
                        Some(Action::CapturePreviousArea) => Some(Command::PreviousArea),
                        Some(Action::OpenFromClipboard) => Some(Command::OpenClipboard),
                        Some(Action::VideoRecordPause) | Some(Action::VideoStop) => {
                            if self.recorder.is_some() { Some(Command::StopVideo) } else { None }
                        }
                        _ => None,
                    };
                    if let Some(c) = cmd {
                        self.run(ctx, c);
                    }
                }
                Msg::Menu(id) => {
                    if let Some(c) = self.tray_items.get(&id).copied() {
                        self.run(ctx, c);
                    }
                }
            }
        }
    }

    fn handle_editor_cmd(&mut self, ctx: &Context, cmd: Cmd) {
        let Some(ed) = &self.editor else { return };
        match cmd {
            Cmd::Save => {
                let img = ed.flattened(&self.font, self.settings.editor_shadow);
                let ext = match self.settings.image_format {
                    crate::settings::ImageFormat::Jpeg => "jpg",
                    _ => "png",
                };
                let use_template = ed.source.is_none() || self.settings.apply_template_to_opened;
                let name = if use_template {
                    crate::settings::apply_template(&self.settings.filename_template, "", ext)
                } else {
                    ed.title.clone()
                };
                let dir = self.settings.save_dir();
                let _ = std::fs::create_dir_all(&dir);
                if let Some(p) = rfd::FileDialog::new()
                    .set_title(t("Save image"))
                    .set_directory(&dir)
                    .set_file_name(format!("{name}.{ext}"))
                    .add_filter("PNG", &["png"])
                    .add_filter("JPEG", &["jpg", "jpeg"])
                    .save_file()
                {
                    match capture::save_to_path(&img, &self.settings, &p) {
                        Ok(()) => self.toast(ctx, tf("Saved to {}", &[&p.display()])),
                        Err(e) => self.toast(ctx, e),
                    }
                }
            }
            Cmd::Copy => {
                let img = ed.flattened(&self.font, self.settings.editor_shadow);
                let mut s = self.settings.clone();
                s.clipboard_flags |= crate::settings::COPY_IMAGE;
                match capture::copy_to_clipboard(&img, &s, None) {
                    Ok(_) => self.toast(ctx, t("Image copied to clipboard")),
                    Err(e) => self.toast(ctx, e),
                }
            }
            Cmd::External => {
                let img = ed.flattened(&self.font, self.settings.editor_shadow);
                let p = std::env::temp_dir().join(format!("{} {}.png", APP_NAME, chrono::Local::now().format("%H-%M-%S")));
                match img.save(&p) {
                    Ok(()) => {
                        if let Err(e) = capture::open_in_app(&self.settings.external_editor, &p) {
                            self.toast(ctx, e);
                        }
                    }
                    Err(e) => self.toast(ctx, e.to_string()),
                }
            }
            Cmd::PasteImage => self.run(ctx, Command::OpenClipboard),
            Cmd::Home => self.editor = None,
        }
    }

    // ---------------------------------------------------------------- UI

    fn logo(&mut self, ctx: &Context) -> egui::TextureHandle {
        self.logo
            .get_or_insert_with(|| {
                let img = crate::appicon::render(256);
                let ci = egui::ColorImage::from_rgba_unmultiplied([256, 256], img.as_raw());
                ctx.load_texture("app-logo", ci, egui::TextureOptions::LINEAR)
            })
            .clone()
    }

    fn home(&mut self, ctx: &Context) {
        let pal = theme::pal(ctx);
        let logo = self.logo(ctx);
        let mut chosen: Option<Command> = None;
        egui::CentralPanel::default().frame(Frame::new()).show(ctx, |ui| {
            let full = ui.max_rect();
            // the colour blobs drift slowly while the window is on screen and
            // in front (still for --screenshot, so pictures are repeatable)
            let focused = ctx.input(|i| i.viewport().focused).unwrap_or(true);
            if self.screenshot.is_some() {
                theme::background(ui.painter(), full, &pal, true);
            } else {
                theme::background_drifting(ui.painter(), full, &pal, true, focused && !self.hidden);
            }
            // 9 % of the height above the logo, less in a short window so the
            // bottom buttons still fit (the rest of the screen is ~610 pt)
            ui.add_space(((full.height() - 610.0) * 0.5).clamp(10.0, (full.height() * 0.09).max(16.0)));
            ui.vertical_centered(|ui| {
                let (r, _) = ui.allocate_exact_size(vec2(96.0, 96.0), Sense::hover());
                theme::shadow(ui.painter(), r.shrink(6.0), 44, &pal);
                ui.painter().image(logo.id(), r, Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE);
                ui.add_space(10.0);
                ui.label(RichText::new(APP_NAME).size(30.0).strong().color(pal.text));
                ui.label(RichText::new(t("Take a screenshot, mark it up, save or copy it.")).size(14.0).color(pal.hint));
                ui.add_space(44.0);
            });
            let hk = |a: Action| self.settings.hotkey(a).map(|k| k.parts()).unwrap_or_default();
            let tiles = [
                (Icon::CaptureArea, t("Capture Area"), hk(Action::CaptureArea), Command::Capture(CaptureKind::Area)),
                (Icon::Fullscreen, t("Capture Fullscreen"), hk(Action::CaptureFullscreen), Command::Capture(CaptureKind::Fullscreen)),
                (Icon::Timer, t("Fullscreen after 10s"), hk(Action::CaptureFullscreenDelayed), Command::Capture(CaptureKind::FullscreenDelayed)),
                (Icon::Crop, t("Capture Frozen Area"), hk(Action::CaptureFrozenArea), Command::Capture(CaptureKind::FrozenArea)),
                (
                    Icon::Video,
                    if self.recorder.is_some() { t("Stop Recording") } else { t("Record Video") },
                    hk(Action::RecordVideo),
                    Command::RecordVideo,
                ),
                (Icon::Clipboard, t("Open from Clipboard"), hk(Action::OpenFromClipboard), Command::OpenClipboard),
            ];
            // wide enough for most labels in every language (longer ones shrink)
            let tile = vec2(224.0, 148.0);
            // room kept free between a tile's label and its left/right edges
            let pad_x = 22.0;
            let gap = 16.0;
            let cols = 3usize;
            let grid_w = tile.x * cols as f32 + gap * (cols as f32 - 1.0);
            let top = ui.cursor().top();
            let left = full.center().x - grid_w / 2.0;
            for (i, (icon, label, key, cmd)) in tiles.iter().enumerate() {
                let r = Rect::from_min_size(
                    egui::pos2(left + (i % cols) as f32 * (tile.x + gap), top + (i / cols) as f32 * (tile.y + gap)),
                    tile,
                );
                let resp = ui.interact(r, ui.id().with(("tile", i)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
                // hover, keyboard focus and press all light the tile up
                let lit = resp.hovered() || resp.has_focus() || resp.is_pointer_button_down_on();
                let hov = theme::hover_amount(ctx, resp.id, lit);
                let p = ui.painter();
                theme::flat_button(p, r, 20, &pal, hov, resp.is_pointer_button_down_on(), 0.18);
                // bare one-ink icons: accent at rest, lighter violet when the tile
                // is hovered, focused or pressed (red while recording)
                let recording = *cmd == Command::RecordVideo && self.recorder.is_some();
                // icon and name sit at the same height in every tile, so they
                // line up across a row; a tile without a shortcut keeps its
                // keycap row empty
                let dy = 0.0;
                let icon_r = Rect::from_center_size(egui::pos2(r.center().x, r.top() + TILE_PAD + 18.0 + dy), vec2(36.0, 36.0));
                let rest_ic = if pal.dark { Color32::from_rgb(0xc9, 0xbb, 0xff) } else { pal.accent };
                let lit_ic = if pal.dark { pal.hover_b } else { pal.hover_a };
                let ic = if recording { Color32::from_rgb(0xff, 0x3b, 0x30) } else { theme::mix(rest_ic, lit_ic, hov) };
                icons::paint(p, icon_r, *icon, ic);
                // long translations shrink a little to fit the tile
                let lw = ui.fonts(|f| f.layout_no_wrap(label.to_string(), egui::FontId::proportional(14.0), pal.text).size().x);
                let room = tile.x - 2.0 * pad_x;
                let size = if lw > room { (14.0 * room / lw).max(10.0) } else { 14.0 };
                p.text(egui::pos2(r.center().x, r.top() + 84.0 + dy), Align2::CENTER_CENTER, *label, egui::FontId::proportional(size), pal.text);
                // the shortcut as keycaps; nothing when there is none
                if !key.is_empty() {
                    theme::keycaps(p, egui::pos2(r.center().x, r.bottom() - TILE_PAD - 11.0), key, &pal);
                }
                if resp.clicked() {
                    chosen = Some(*cmd);
                }
            }
            let rows = tiles.len().div_ceil(cols) as f32;
            // the secondary buttons stand well apart from the capture tiles
            ui.allocate_exact_size(vec2(0.0, rows * (tile.y + gap) - gap + 64.0), Sense::hover());
            let actions = [
                (Icon::Folder, t("Open…"), Command::Open),
                (Icon::Image, t("Open Latest Image"), Command::OpenLatest),
                (Icon::Gear, t("Preferences…"), Command::Preferences),
            ];
            let font = egui::FontId::proportional(14.0);
            let (icon_w, icon_gap, pad) = (17.0, 8.0, 18.0);
            // all three as wide as the longest label needs
            let text_w = |s: &str| ctx.fonts(|f| f.layout_no_wrap(s.to_string(), font.clone(), pal.text).size().x);
            let bw = actions.iter().map(|a| text_w(a.1)).fold(0.0_f32, f32::max).max(110.0) + icon_w + icon_gap + 2.0 * pad;
            let bgap = 12.0;
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = bgap;
                let w = 3.0 * bw + 2.0 * bgap;
                ui.add_space(((ui.available_width() - w) / 2.0).max(0.0));
                for (icon, label, cmd) in actions {
                    let (r, resp) = ui.allocate_exact_size(vec2(bw, 40.0), Sense::click());
                    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                    let lit = resp.hovered() || resp.has_focus() || resp.is_pointer_button_down_on();
                    let hov = theme::hover_amount(ctx, resp.id, lit);
                    let p = ui.painter();
                    theme::flat_button(p, r, 11, &pal, hov, resp.is_pointer_button_down_on(), 0.18);
                    // icon and label centred together
                    let group = icon_w + icon_gap + text_w(label);
                    let x0 = r.center().x - group / 2.0;
                    let rest_ic = if pal.dark { Color32::from_rgb(0xc9, 0xbb, 0xff) } else { pal.accent };
                    let lit_ic = if pal.dark { pal.hover_b } else { pal.hover_a };
                    let ir = Rect::from_center_size(egui::pos2(x0 + icon_w / 2.0, r.center().y), vec2(icon_w, icon_w));
                    icons::paint(p, ir, icon, theme::mix(rest_ic, lit_ic, hov));
                    p.text(egui::pos2(x0 + icon_w + icon_gap, r.center().y), Align2::LEFT_CENTER, label, font.clone(), pal.text);
                    if resp.clicked() {
                        chosen = Some(cmd);
                    }
                }
            });
            // language picker in the top-right corner
            // as far from the right edge as from the top one
            let margin = theme::TITLEBAR + 8.0;
            let picker = Rect::from_min_size(egui::pos2(full.right() - margin - 160.0, full.top() + margin), vec2(160.0, i18n::PICKER_H));
            ui.scope_builder(egui::UiBuilder::new().max_rect(picker).layout(egui::Layout::right_to_left(egui::Align::Center)), |ui| {
                i18n::lang_picker(ui, &mut self.settings.language, false);
            });
        });
        if let Some(c) = chosen {
            self.run(ctx, c);
        }
    }

    fn top_strip(&mut self, ctx: &Context) {
        // Only the recording indicator: a "Capturing…" pill would get in the
        // way and end up in the screenshot itself.
        let recording = self.recorder.as_ref().map(|r| r.started.elapsed().as_secs());
        if recording.is_none() {
            return;
        }
        let pal = theme::pal(ctx);
        let mut stop = false;
        let avail = ctx.available_rect();
        theme::floating(ctx, "status_pill", Align2::CENTER_TOP, egui::pos2(avail.center().x, avail.top() + 64.0), &pal, &[], |ui| {
            if let Some(secs) = recording {
                let (r, _) = ui.allocate_exact_size(vec2(14.0, 20.0), Sense::hover());
                ui.painter().circle_filled(r.center(), 5.0, Color32::from_rgb(0xff, 0x3b, 0x30));
                ui.label(RichText::new(tf("Recording {}", &[&format!("{:02}:{:02}", secs / 60, secs % 60)])).color(pal.text));
                if ui.button(t("Stop")).clicked() {
                    stop = true;
                }
                ctx.request_repaint_after(std::time::Duration::from_millis(500));
            }
        });
        if stop {
            self.run(ctx, Command::StopVideo);
        }
    }

    fn toasts_ui(&mut self, ctx: &Context) {
        let now = ctx.input(|i| i.time);
        self.toasts.retain(|t| now - t.at < 4.0);
        if self.settings.dont_show_notifications || self.toasts.is_empty() {
            return;
        }
        let fade = !self.settings.hide_animation;
        egui::Area::new(egui::Id::new("toasts"))
            .anchor(Align2::RIGHT_BOTTOM, vec2(-16.0, -40.0))
            .order(egui::Order::Foreground)
            .interactable(false)
            .show(ctx, |ui| {
                for t in &self.toasts {
                    let age = (now - t.at) as f32;
                    let a = if fade { (1.0 - ((age - 3.4) / 0.6).clamp(0.0, 1.0)) * (age / 0.2).clamp(0.0, 1.0) } else { 1.0 };
                    Frame::new()
                        .fill(Color32::from_black_alpha((190.0 * a) as u8))
                        .corner_radius(14.0)
                        .stroke(Stroke::new(1.0_f32, Color32::from_white_alpha((30.0 * a) as u8)))
                        .inner_margin(Margin::symmetric(14, 10))
                        .show(ui, |ui| {
                            ui.set_max_width(360.0);
                            ui.label(RichText::new(&t.text).color(Color32::from_white_alpha((255.0 * a) as u8)));
                        });
                    ui.add_space(6.0);
                }
            });
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }

    fn prefs_window(&mut self, ctx: &Context) {
        if !self.prefs.open {
            return;
        }
        let title = self.prefs.tab.label().to_string();
        let prefs = &mut self.prefs;
        let settings = &mut self.settings;
        let ch = ctx.show_viewport_immediate(
            ViewportId::from_hash_of("preferences"),
            ViewportBuilder::default()
                .with_title(title)
                .with_inner_size([settings_ui::WINDOW_WIDTH, settings_ui::TOOLBAR_HEIGHT + settings_ui::CONTENT_HEIGHT])
                .with_resizable(false)
                .with_active(true)
                .with_maximize_button(false)
                .with_minimize_button(false),
            |vctx, _class| {
                let ch = prefs.show(vctx, settings);
                if vctx.input(|i| i.viewport().close_requested()) {
                    prefs.open = false;
                    prefs.recording = None;
                }
                ch
            },
        );
        if ch.hotkeys {
            if let Some(k) = &mut self.keys {
                k.apply(&self.settings.hotkeys);
            }
        }
        if ch.fonts {
            self.font = AnnotFont::load(self.settings.main_font.as_deref());
            install_fonts(ctx, &self.font, self.settings.language);
        }
        if ch.start_at_launch || ch.reset {
            if let Err(e) = settings_ui::apply_launch_at_login(self.settings.start_at_launch) {
                self.toast(ctx, e);
            }
        }
        if ch.reset {
            self.toast(ctx, t("All settings have been reset"));
        }
        // While a hotkey is being typed in, global hotkeys must not swallow it.
        let want_pause = self.prefs.recording.is_some();
        if want_pause != self.keys_paused {
            self.keys_paused = want_pause;
            if let Some(k) = &mut self.keys {
                if want_pause {
                    k.apply(&[]);
                } else {
                    k.apply(&self.settings.hotkeys);
                }
            }
        }
    }

    fn overlay_window(&mut self, ctx: &Context) {
        let Some(ov) = &mut self.overlay else { return };
        let monitor = ctx.input(|i| i.viewport().monitor_size).unwrap_or(vec2(1440.0, 900.0));
        let (mag, hints) = (self.settings.show_magnifier, self.settings.show_tip);
        ctx.show_viewport_immediate(
            ViewportId::from_hash_of("frozen_overlay"),
            ViewportBuilder::default()
                .with_title("Capture")
                .with_decorations(false)
                .with_position([0.0, 0.0])
                .with_inner_size(monitor)
                .with_window_level(egui::WindowLevel::AlwaysOnTop)
                .with_active(true),
            |vctx, _| {
                ov.show(vctx, mag, hints);
                if vctx.input(|i| i.viewport().close_requested()) {
                    ov.result = Some(None);
                }
            },
        );
        if let Some(res) = ov.result.take() {
            let ov = self.overlay.take().unwrap();
            if let Some(r) = res {
                self.previous_area = Some(r);
                let cut = self.crop_full(ctx, &ov.image, r);
                self.handle_new_image(ctx, cut);
            }
            self.end_capture(ctx);
        }
    }

    fn selector_window(&mut self, ctx: &Context) {
        let Some(sel) = &mut self.selector else { return };
        sel.show(ctx, self.settings.shot_on_click, self.settings.show_tip);
        let Some(res) = sel.result.take() else { return };
        self.selector = None;
        let Some(choice) = res else {
            self.end_capture(ctx);
            return;
        };
        if let Some(r) = choice.previous_area {
            self.previous_area = Some(r);
        }
        let mut opt = self.capture_options(ctx);
        opt.retina_scale = choice.scale;
        let tx = self.tx.clone();
        let c = ctx.clone();
        self.busy = Some("Capturing…".to_string());
        std::thread::spawn(move || {
            // a window is captured on its own; a rectangle waits until the
            // selection layer is off the screen
            if matches!(choice.target, capture::Target::Rect(_)) {
                std::thread::sleep(SELECTOR_GONE);
            }
            let _ = tx.send(Msg::Captured(capture::capture_target(choice.target, &opt)));
            c.request_repaint();
        });
    }

    fn screenshot_step(&mut self, ctx: &Context) {
        let Some(job) = &mut self.screenshot else { return };
        job.frame += 1;
        let shot = ctx.input(|i| {
            i.events.iter().find_map(|e| match e {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(img) = shot {
            let [w, h] = img.size;
            let mut buf = Vec::with_capacity(w * h * 4);
            for p in &img.pixels {
                buf.extend_from_slice(&p.to_srgba_unmultiplied());
            }
            if let Some(out) = RgbaImage::from_raw(w as u32, h as u32, buf) {
                if let Some(dir) = job.path.parent() {
                    let _ = std::fs::create_dir_all(dir);
                }
                if let Err(e) = out.save(&job.path) {
                    eprintln!("could not save screenshot: {e}");
                }
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        if job.frame >= 120 && !job.requested {
            job.requested = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }
        ctx.request_repaint();
    }
}

impl eframe::App for App {
    /// With --screenshot the window shows a fixed layout: the real pointer
    /// (which may pass over it or click it) is ignored, so no button lights
    /// up or gets pressed in the picture.
    fn raw_input_hook(&mut self, _ctx: &Context, raw: &mut egui::RawInput) {
        if self.screenshot.is_some() {
            raw.events.retain(|e| {
                !matches!(
                    e,
                    egui::Event::PointerMoved(_)
                        | egui::Event::MouseMoved(_)
                        | egui::Event::PointerButton { .. }
                        | egui::Event::PointerGone
                        | egui::Event::MouseWheel { .. }
                        | egui::Event::Zoom(_)
                        | egui::Event::Touch { .. }
                )
            });
        }
    }

    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        // the area selector's windows are see-through (the main window is
        // hidden meanwhile)
        if theme::vibrant() || self.selector.is_some() {
            [0.0; 4]
        } else {
            visuals.panel_fill.to_normalized_gamma_f32()
        }
    }

    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        if self.applied_theme != Some(self.settings.theme) {
            ctx.set_theme(self.settings.theme.preference());
            self.applied_theme = Some(self.settings.theme);
        }
        i18n::set(self.settings.language);
        // switching to or from Chinese, Japanese or Korean changes the font
        let cjk = Some(self.settings.language).filter(|l| l.is_cjk());
        if cjk != self.cjk_font_for {
            install_fonts(ctx, &self.font, self.settings.language);
            self.cjk_font_for = cjk;
        }
        self.start_services(ctx);
        // the menu-bar menu is native: rebuild it when the language changes
        if self.tray.is_some() && self.tray_lang != Some(self.settings.language) {
            self.tray = None;
            self.tray_items.clear();
            self.build_tray();
        }
        if self.tray.is_some() {
            self.tray_lang = Some(self.settings.language);
        }
        // With the menu-bar icon running, closing the window only hides it.
        if self.tray.is_some() && !self.quitting && ctx.input(|i| i.viewport().close_requested()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            self.hidden = true;
        }
        // ⌘, opens Preferences, as in every Mac app
        if self.screenshot.is_none() && ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Comma)) {
            self.run(ctx, Command::Preferences);
        }
        self.handle_messages(ctx);

        if let Some(v) = self.view.clone() {
            if let Some(tab) = v.strip_prefix("prefs-") {
                self.prefs.tab = match tab {
                    "advanced" => settings_ui::Tab::Advanced,
                    "appearance" => settings_ui::Tab::Appearance,
                    "hotkeys" => settings_ui::Tab::Hotkeys,
                    "fonts" => settings_ui::Tab::Fonts,
                    _ => settings_ui::Tab::General,
                };
                // prefs-about: the About box over the General tab
                self.prefs.about = tab == "about";
                let _ = self.prefs.show(ctx, &mut self.settings);
                self.screenshot_step(ctx);
                return;
            }
        }
        self.top_strip(ctx);
        if self.editor.is_some() {
            let cmds = {
                let ed = self.editor.as_mut().unwrap();
                ed.ui(ctx, &mut self.settings, &self.font)
            };
            for c in cmds {
                self.handle_editor_cmd(ctx, c);
            }
        } else {
            self.home(ctx);
        }
        self.prefs_window(ctx);
        self.overlay_window(ctx);
        self.selector_window(ctx);
        self.toasts_ui(ctx);
        // no click while a capture overlay is up
        if self.overlay.is_none() && self.selector.is_none() {
            crate::sound::click_feedback(ctx, self.settings.click_sound);
        }

        if self.persist {
            let json = serde_json::to_string(&self.settings).unwrap_or_default();
            if json != self.saved_json {
                self.settings.save();
                self.saved_json = json;
            }
        }
        if self.busy.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(200));
        }
        self.screenshot_step(ctx);
    }
}
