//! All preferences, with the starting values read from the original program
//! (see MECHANICS.md for the addresses each one came from).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::hotkeys::{Action, Hotkey};

pub const APP_NAME: &str = "QuickSnap";

// ---- Advanced tab ----
pub const JPEG_QUALITY_DEFAULT: u32 = 100;
pub const JPEG_QUALITY_MIN: u32 = 50;
pub const JPEG_QUALITY_MAX: u32 = 100;
pub const GIF_FPS_DEFAULT: u32 = 10;
pub const GIF_FPS_MIN: u32 = 1;
pub const GIF_FPS_MAX: u32 = 30;
pub const GIF_QUALITY_DEFAULT: u32 = 100;
pub const GIF_QUALITY_MIN: u32 = 10;
pub const GIF_QUALITY_MAX: u32 = 100;
pub const COPY_IMAGE: u8 = 1;
pub const COPY_URL: u8 = 2;
pub const COPY_PATH: u8 = 4;
pub const CLIPBOARD_FLAGS_DEFAULT: u8 = 1;
pub const DRAG_DROP_FLAGS_DEFAULT: u8 = 7;
pub const DELAYED_CAPTURE_SECONDS: u32 = 10;

// ---- Editor ----
pub const LINE_WIDTH_DEFAULT: f32 = 6.0;
pub const LINE_WIDTH_MIN: f32 = 1.0;
pub const LINE_WIDTH_MAX: f32 = 12.0;

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum ImageFormat {
    Jpeg,
    Png,
    Auto,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum VideoFormat {
    Mpeg,
    Gif,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum AfterShot {
    OpenEditor,
    Save,
    Upload,
    ExternalEditor,
    Copy,
}

impl AfterShot {
    pub const ALL: [AfterShot; 5] = [
        AfterShot::OpenEditor,
        AfterShot::Save,
        AfterShot::Upload,
        AfterShot::ExternalEditor,
        AfterShot::Copy,
    ];
    pub fn label(self) -> String {
        match self {
            AfterShot::OpenEditor => crate::i18n::tf("Open {} editor", &[&APP_NAME]),
            AfterShot::Save => crate::i18n::t("Save screenshot").into(),
            AfterShot::Upload => crate::i18n::t("Upload screenshot").into(),
            AfterShot::ExternalEditor => crate::i18n::t("Open external editor").into(),
            AfterShot::Copy => crate::i18n::t("Copy image to clipboard").into(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum AfterUpload {
    DoNotCopy,
    CopySiteLink,
    CopyDirectLink,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct Settings {
    // General
    pub after_shot: AfterShot,
    pub sound_effects: bool,
    pub after_upload: AfterUpload,
    pub close_after_upload: bool,
    pub open_in_browser: bool,
    pub short_links: bool,
    pub apply_ocr: bool,
    pub external_editor: String,
    pub start_at_launch: bool,

    // Advanced
    pub image_format: ImageFormat,
    pub jpeg_quality: u32,
    pub save_folder: String,
    pub always_save: bool,
    pub shrink_retina: bool,
    pub video_format: VideoFormat,
    pub gif_fps: u32,
    pub gif_quality: u32,
    pub always_save_movie: bool,
    pub drag_drop_flags: u8,
    pub clipboard_flags: u8,
    pub filename_template: String,
    pub apply_template_to_opened: bool,

    // Appearance
    pub show_magnifier: bool,
    pub highlight_windows: bool,
    pub window_shadow: bool,
    pub shot_on_click: bool,
    pub show_tip: bool,
    pub show_startup_tooltip: bool,
    pub hide_animation: bool,
    pub dont_show_notifications: bool,
    pub show_tooltips_editor: bool,
    pub allow_default_browser: bool,
    pub theme: crate::theme::ThemeChoice,
    /// Interface language (English unless the person picks another).
    pub language: crate::i18n::Lang,

    // Hotkeys
    pub hotkeys: Vec<(Action, Option<Hotkey>)>,

    // Fonts
    pub main_font: Option<String>,
    pub additional_fonts: Vec<String>,

    // Editor (remembered between sessions like the original)
    pub editor_color: [u8; 4],
    pub editor_width: f32,
    pub editor_shadow: bool,
    pub editor_text_size: f32,
    pub editor_text_style: crate::shapes::TextStyle,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            after_shot: AfterShot::OpenEditor,
            sound_effects: false,
            after_upload: AfterUpload::CopySiteLink,
            close_after_upload: true,
            open_in_browser: true,
            short_links: false,
            apply_ocr: false,
            external_editor: "/System/Applications/Preview.app".into(),
            start_at_launch: false,

            image_format: ImageFormat::Png,
            jpeg_quality: JPEG_QUALITY_DEFAULT,
            save_folder: format!("~/Pictures/{APP_NAME}"),
            always_save: false,
            shrink_retina: false,
            video_format: VideoFormat::Mpeg,
            gif_fps: GIF_FPS_DEFAULT,
            gif_quality: GIF_QUALITY_DEFAULT,
            always_save_movie: false,
            drag_drop_flags: DRAG_DROP_FLAGS_DEFAULT,
            clipboard_flags: CLIPBOARD_FLAGS_DEFAULT,
            filename_template: format!("{APP_NAME} %W %Y-%m-%d %H-%M-%S"),
            apply_template_to_opened: false,

            show_magnifier: true,
            highlight_windows: true,
            window_shadow: true,
            shot_on_click: true,
            show_tip: true,
            show_startup_tooltip: true,
            hide_animation: false,
            dont_show_notifications: false,
            show_tooltips_editor: true,
            allow_default_browser: false,
            theme: crate::theme::ThemeChoice::System,
            language: crate::i18n::Lang::En,

            hotkeys: crate::hotkeys::default_hotkeys(),

            main_font: None,
            additional_fonts: Vec::new(),

            editor_color: [255, 0, 0, 255],
            editor_width: LINE_WIDTH_DEFAULT,
            editor_shadow: true,
            editor_text_size: crate::shapes::TEXT_SIZE_DEFAULT,
            editor_text_style: crate::shapes::TextStyle::Outline,
        }
    }
}

impl Settings {
    fn file() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join(APP_NAME).join("settings.json"))
    }

    pub fn load() -> Settings {
        let mut s: Settings = Self::file()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
        // Make sure every action has a row even if the file is older.
        for (a, k) in crate::hotkeys::default_hotkeys() {
            if !s.hotkeys.iter().any(|(b, _)| *b == a) {
                s.hotkeys.push((a, k));
            }
        }
        s
    }

    pub fn save(&self) {
        if let Some(p) = Self::file() {
            if let Some(dir) = p.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            if let Ok(t) = serde_json::to_string_pretty(self) {
                let _ = std::fs::write(p, t);
            }
        }
    }

    pub fn save_dir(&self) -> PathBuf {
        expand_tilde(&self.save_folder)
    }

    pub fn hotkey(&self, a: Action) -> Option<Hotkey> {
        self.hotkeys.iter().find(|(b, _)| *b == a).and_then(|(_, k)| k.clone())
    }

    pub fn set_hotkey(&mut self, a: Action, k: Option<Hotkey>) {
        if let Some(slot) = self.hotkeys.iter_mut().find(|(b, _)| *b == a) {
            slot.1 = k;
        } else {
            self.hotkeys.push((a, k));
        }
    }
}

pub fn expand_tilde(p: &str) -> PathBuf {
    if let Some(rest) = p.strip_prefix("~/") {
        if let Some(h) = dirs::home_dir() {
            return h.join(rest);
        }
    }
    PathBuf::from(p)
}

/// Fill in a filename template. Tokens from the original's tip text:
/// %Y year, %y 2-digit year, %m month, %d day, %H hour, %M minutes, %S seconds,
/// %W window title, %R 5 random symbols, %T file type.
pub fn apply_template(template: &str, window_title: &str, file_type: &str) -> String {
    use chrono::{Datelike, Local, Timelike};
    let now = Local::now();
    let mut out = String::new();
    let mut chars = template.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('Y') => out.push_str(&format!("{:04}", now.year())),
            Some('y') => out.push_str(&format!("{:02}", now.year() % 100)),
            Some('m') => out.push_str(&format!("{:02}", now.month())),
            Some('d') => out.push_str(&format!("{:02}", now.day())),
            Some('H') => out.push_str(&format!("{:02}", now.hour())),
            Some('M') => out.push_str(&format!("{:02}", now.minute())),
            Some('S') => out.push_str(&format!("{:02}", now.second())),
            Some('W') => out.push_str(window_title),
            Some('T') => out.push_str(file_type),
            Some('R') => out.push_str(&random_symbols(5)),
            Some(o) => {
                out.push('%');
                out.push(o);
            }
            None => out.push('%'),
        }
    }
    // Collapse doubled spaces left by an empty %W and strip characters
    // that are not allowed in file names.
    let mut cleaned = out.replace(['/', ':'], "-");
    while cleaned.contains("  ") {
        cleaned = cleaned.replace("  ", " ");
    }
    let cleaned = cleaned.trim().to_string();
    if cleaned.is_empty() { APP_NAME.to_string() } else { cleaned }
}

fn random_symbols(n: usize) -> String {
    const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(42);
    (0..n)
        .map(|_| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            ALPHABET[(seed % ALPHABET.len() as u64) as usize] as char
        })
        .collect()
}
