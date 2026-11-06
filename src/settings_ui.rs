//! The Preferences window: a centred toolbar with five tabs (General,
//! Advanced, Appearance, Hotkeys, Fonts — the original's last two tabs,
//! Account and Feedback, are left out on request) and a form for each.

use crate::i18n::{self, t, tf};
use egui::{vec2, Align2, Color32, Context, CornerRadius, FontId, Frame, Key, Margin, Rect, RichText, Sense, Stroke, Ui};

use crate::hotkeys::{Action, Hotkey};
use crate::icons::{self, Icon};
use crate::settings::{
    self, AfterShot, AfterUpload, ImageFormat, Settings, VideoFormat, APP_NAME, COPY_IMAGE, COPY_PATH, COPY_URL,
    GIF_FPS_MAX, GIF_FPS_MIN, GIF_QUALITY_MAX, GIF_QUALITY_MIN, JPEG_QUALITY_MAX, JPEG_QUALITY_MIN,
};

/// Width of the original settings window (`-[MSSettingsWindow init]`).
pub const WINDOW_WIDTH: f32 = 500.0;
/// Height of the original's tab content area.
pub const CONTENT_HEIGHT: f32 = 372.0;
pub const TOOLBAR_HEIGHT: f32 = 62.0;
const LABEL_W: f32 = 180.0;
pub const HINT_COLOR: Color32 = Color32::from_rgb(0x60, 0x60, 0x60);
#[allow(dead_code)]
pub const WINDOW_BG: Color32 = Color32::from_rgb(0xec, 0xec, 0xec);
const NO_ACCOUNT: &str = "Needs a connected account, which this version doesn't include";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tab {
    General,
    Advanced,
    Appearance,
    Hotkeys,
    Fonts,
}

impl Tab {
    pub const ALL: [Tab; 5] = [Tab::General, Tab::Advanced, Tab::Appearance, Tab::Hotkeys, Tab::Fonts];
    pub fn label(self) -> &'static str {
        match self {
            Tab::General => t("General"),
            Tab::Advanced => t("Advanced"),
            Tab::Appearance => t("Appearance"),
            Tab::Hotkeys => t("Hotkeys"),
            Tab::Fonts => t("Fonts"),
        }
    }
    fn icon(self) -> Icon {
        match self {
            Tab::General => Icon::General,
            Tab::Advanced => Icon::Advanced,
            Tab::Appearance => Icon::Appearance,
            Tab::Hotkeys => Icon::Hotkeys,
            Tab::Fonts => Icon::Fonts,
        }
    }
}

#[derive(Default)]
pub struct Changes {
    pub hotkeys: bool,
    pub fonts: bool,
    pub start_at_launch: bool,
    pub reset: bool,
    pub about: bool,
}

pub struct SettingsUi {
    pub open: bool,
    pub tab: Tab,
    pub recording: Option<Action>,
    confirm_reset: bool,
    selected_font: Option<usize>,
    last_height: f32,
}

impl SettingsUi {
    pub fn new() -> SettingsUi {
        SettingsUi { open: false, tab: Tab::General, recording: None, confirm_reset: false, selected_font: None, last_height: 0.0 }
    }

    /// Draws the whole window into `ctx` (its own viewport).
    pub fn show(&mut self, ctx: &Context, s: &mut Settings) -> Changes {
        let mut ch = Changes::default();
        let pal = crate::theme::pal(ctx);
        egui::TopBottomPanel::top("prefs_toolbar")
            .exact_height(TOOLBAR_HEIGHT)
            .show_separator_line(false)
            .frame(Frame::new().fill(pal.glass_strong).stroke(Stroke::new(1.0_f32, pal.border)))
            .show(ctx, |ui| self.toolbar(ui));

        let mut content_h = 0.0;
        egui::CentralPanel::default()
            .frame(Frame::new().fill(pal.bg).inner_margin(Margin { left: 16, right: 16, top: 14, bottom: 14 }))
            .show(ctx, |ui| {
                ui.style_mut().spacing.item_spacing = vec2(8.0, 7.0);
                let top = ui.cursor().top();
                match self.tab {
                    Tab::General => self.general(ui, s, &mut ch),
                    Tab::Advanced => self.advanced(ui, s, &mut ch),
                    Tab::Appearance => self.appearance(ui, s),
                    Tab::Hotkeys => self.hotkeys(ui, s, &mut ch),
                    Tab::Fonts => self.fonts(ui, s, &mut ch),
                }
                content_h = ui.cursor().top() - top;
            });

        // The original resizes the window to fit each tab's content.
        let want = (TOOLBAR_HEIGHT + content_h + 36.0).max(TOOLBAR_HEIGHT + 200.0).round();
        if (want - self.last_height).abs() > 1.0 {
            self.last_height = want;
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(vec2(WINDOW_WIDTH, want)));
        }

        if self.confirm_reset {
            egui::Modal::new(egui::Id::new("confirm_reset")).show(ctx, |ui| {
                ui.set_width(280.0);
                ui.heading(t("Reset all settings"));
                ui.label(t("ALL settings in the application will be reset"));
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button(t("Cancel")).clicked() {
                        self.confirm_reset = false;
                    }
                    if ui.button(t("Reset")).clicked() {
                        *s = Settings::default();
                        ch.reset = true;
                        ch.hotkeys = true;
                        ch.fonts = true;
                        self.confirm_reset = false;
                    }
                });
            });
        }
        if self.recording.is_some() {
            self.capture_hotkey(ctx, s, &mut ch);
        }
        ch
    }

    fn toolbar(&mut self, ui: &mut Ui) {
        let item_w = 76.0;
        let total = item_w * Tab::ALL.len() as f32;
        let full = ui.max_rect();
        let start = full.center().x - total / 2.0;
        for (i, t) in Tab::ALL.iter().enumerate() {
            let r = Rect::from_min_size(egui::pos2(start + i as f32 * item_w, full.top() + 4.0), vec2(item_w - 4.0, full.height() - 8.0));
            let resp = ui.interact(r, ui.id().with(("tab", i)), Sense::click());
            let sel = self.tab == *t;
            let pal = crate::theme::pal(ui.ctx());
            let p = ui.painter();
            if sel {
                p.rect_filled(r, CornerRadius::same(10), pal.accent_soft);
            } else if resp.hovered() {
                p.rect_filled(r, CornerRadius::same(10), pal.hover);
            }
            let col = if sel { pal.accent } else { pal.hint };
            icons::paint(p, Rect::from_center_size(egui::pos2(r.center().x, r.top() + 19.0), vec2(26.0, 26.0)), t.icon(), col);
            p.text(
                egui::pos2(r.center().x, r.bottom() - 9.0),
                Align2::CENTER_CENTER,
                t.label(),
                FontId::proportional(11.5),
                if sel { pal.text } else { pal.hint },
            );
            if resp.clicked() {
                self.tab = *t;
                self.recording = None;
            }
        }
    }

    // ---------------------------------------------------------------- tabs

    fn general(&mut self, ui: &mut Ui, s: &mut Settings, ch: &mut Changes) {
        let mut sec = crate::theme::section_begin(ui);
        row(ui, t("Language:"), |ui| {
            i18n::lang_picker(ui, &mut s.language, false);
        });
        sec = crate::theme::section_next(ui, sec);
        row(ui, t("After screenshot:"), |ui| {
            egui::ComboBox::from_id_salt("after_shot")
                .width(230.0)
                .selected_text(s.after_shot.label())
                .show_ui(ui, |ui| {
                    for a in AfterShot::ALL {
                        let enabled = a != AfterShot::Upload;
                        let r = ui.add_enabled(enabled, egui::Button::selectable(s.after_shot == a, a.label()));
                        if r.clicked() {
                            s.after_shot = a;
                        }
                        if !enabled {
                            r.on_disabled_hover_text(t(NO_ACCOUNT));
                        }
                    }
                });
        });
        row(ui, "", |ui| {
            ui.checkbox(&mut s.sound_effects, t("Sound effects"));
        });
        sec = crate::theme::section_next(ui, sec);
        ui.add_enabled_ui(false, |ui| {
            row(ui, t("After upload:"), |ui| {
                let label = match s.after_upload {
                    AfterUpload::DoNotCopy => t("Do not copy"),
                    AfterUpload::CopySiteLink => t("Copy site link"),
                    AfterUpload::CopyDirectLink => t("Copy direct image link"),
                };
                egui::ComboBox::from_id_salt("after_upload").width(230.0).selected_text(label).show_ui(ui, |_| {});
            });
            row(ui, "", |ui| {
                ui.checkbox(&mut s.open_in_browser, t("Open in browser"));
            });
            row(ui, "", |ui| {
                ui.checkbox(&mut s.short_links, t("Short links"));
            });
            row(ui, "", |ui| {
                ui.checkbox(&mut s.apply_ocr, t("Apply text recognition (in development)"));
            });
        })
        .response
        .on_disabled_hover_text(t(NO_ACCOUNT));
        sec = crate::theme::section_next(ui, sec);
        row(ui, t("External Image Editor:"), |ui| {
            let name = std::path::Path::new(&s.external_editor)
                .file_stem()
                .and_then(|n| n.to_str())
                .unwrap_or("Preview")
                .to_string();
            egui::ComboBox::from_id_salt("ext_editor").width(230.0).selected_text(name.clone()).show_ui(ui, |ui| {
                let _ = ui.selectable_label(true, name);
                ui.separator();
                if ui.selectable_label(false, t("Choose application...")).clicked() {
                    if let Some(p) = rfd::FileDialog::new()
                        .set_title(t("Choose application"))
                        .set_directory("/Applications")
                        .add_filter(t("Application"), &["app"])
                        .pick_file()
                    {
                        s.external_editor = p.display().to_string();
                    }
                }
            });
        });
        sec = crate::theme::section_next(ui, sec);
        row(ui, t("Status bar:"), |ui| {
            if ui.checkbox(&mut s.start_at_launch, t("Launch at login")).changed() {
                ch.start_at_launch = true;
            }
        });
        ui.add_space(10.0);
        ui.vertical_centered(|ui| {
            if ui.button(tf("About {}", &[&APP_NAME])).clicked() {
                ch.about = true;
            }
        });
        crate::theme::section_end(ui, sec);
    }

    fn advanced(&mut self, ui: &mut Ui, s: &mut Settings, ch: &mut Changes) {
        let mut sec = crate::theme::section_begin(ui);
        let _ = ch;
        row(ui, t("Image File Format:"), |ui| {
            let label = match s.image_format {
                ImageFormat::Jpeg => "JPEG",
                ImageFormat::Png => "PNG",
                ImageFormat::Auto => t("Auto (smaller size)"),
            };
            egui::ComboBox::from_id_salt("img_fmt").width(180.0).selected_text(label).show_ui(ui, |ui| {
                ui.selectable_value(&mut s.image_format, ImageFormat::Jpeg, "JPEG");
                ui.selectable_value(&mut s.image_format, ImageFormat::Png, "PNG");
                ui.selectable_value(&mut s.image_format, ImageFormat::Auto, t("Auto (smaller size)"));
            });
        });
        row(ui, t("JPEG Quality:"), |ui| {
            ui.add_enabled(
                s.image_format != ImageFormat::Png,
                egui::Slider::new(&mut s.jpeg_quality, JPEG_QUALITY_MIN..=JPEG_QUALITY_MAX).suffix("%"),
            );
        });
        row(ui, t("Save folder:"), |ui| {
            ui.add(egui::Label::new(RichText::new(&s.save_folder).color(HINT_COLOR)).truncate())
                .on_hover_text(&s.save_folder);
        });
        row(ui, "", |ui| {
            if ui.button(t("Choose...")).clicked() {
                if let Some(p) = rfd::FileDialog::new().set_title(t("Choose save folder")).set_directory(s.save_dir()).pick_folder() {
                    let home = dirs::home_dir().map(|h| h.display().to_string()).unwrap_or_default();
                    let p = p.display().to_string();
                    s.save_folder = if !home.is_empty() && p.starts_with(&home) { format!("~{}", &p[home.len()..]) } else { p };
                }
            }
        });
        row(ui, "", |ui| {
            ui.checkbox(&mut s.always_save, t("Autosave screenshots"));
        });
        row(ui, "", |ui| {
            ui.checkbox(&mut s.shrink_retina, t("Shrink Retina snaps"));
        });
        sec = crate::theme::section_next(ui, sec);
        row(ui, t("Video File Format:"), |ui| {
            let label = match s.video_format {
                VideoFormat::Mpeg => "MPEG",
                VideoFormat::Gif => "GIF",
            };
            egui::ComboBox::from_id_salt("vid_fmt").width(180.0).selected_text(label).show_ui(ui, |ui| {
                ui.selectable_value(&mut s.video_format, VideoFormat::Mpeg, "MPEG");
                ui.selectable_value(&mut s.video_format, VideoFormat::Gif, "GIF");
            });
        });
        let gif = s.video_format == VideoFormat::Gif;
        row(ui, "GIF FPS:", |ui| {
            ui.add_enabled(gif, egui::Slider::new(&mut s.gif_fps, GIF_FPS_MIN..=GIF_FPS_MAX));
        });
        row(ui, t("GIF Quality:"), |ui| {
            ui.add_enabled(gif, egui::Slider::new(&mut s.gif_quality, GIF_QUALITY_MIN..=GIF_QUALITY_MAX).suffix("%"));
        });
        row(ui, "", |ui| {
            ui.checkbox(&mut s.always_save_movie, t("Autosave to \"Movies\" folder"));
        });
        sec = crate::theme::section_next(ui, sec);
        row(ui, t("Drag and drop copy:"), |ui| flags(ui, &mut s.drag_drop_flags));
        row(ui, t("Copy to clipboard:"), |ui| flags(ui, &mut s.clipboard_flags));
        sec = crate::theme::section_next(ui, sec);
        row(ui, t("Filename template:"), |ui| {
            ui.add(egui::TextEdit::singleline(&mut s.filename_template).desired_width(260.0));
        });
        ui.horizontal(|ui| {
            ui.add_space(70.0);
            ui.label(
                RichText::new(
                    t("%Y — 4-digit year,  %y — 2-digit year, %m — month, %d — day,\n%H — hour, %M — minutes, %S — seconds, %W — window title,\n%R — 5 random symbols, %T - file type"),
                )
                .size(10.0)
                .color(HINT_COLOR),
            );
        });
        row(ui, "", |ui| {
            ui.checkbox(&mut s.apply_template_to_opened, t("Apply to reopened files"));
        });
        sec = crate::theme::section_next(ui, sec);
        row(ui, "", |ui| {
            if ui.button(t("Reset all settings")).clicked() {
                self.confirm_reset = true;
            }
        });
        row(ui, "", |ui| {
            ui.label(RichText::new(t("ALL settings in the application will be reset")).size(10.0).color(HINT_COLOR));
        });
        crate::theme::section_end(ui, sec);
    }

    fn appearance(&mut self, ui: &mut Ui, s: &mut Settings) {
        let mut sec = crate::theme::section_begin(ui);
        row(ui, t("Theme:"), |ui| {
            use crate::theme::ThemeChoice;
            for (v, name) in [(ThemeChoice::System, t("Automatic")), (ThemeChoice::Light, t("Light")), (ThemeChoice::Dark, t("Dark"))] {
                ui.selectable_value(&mut s.theme, v, name);
            }
        });
        sec = crate::theme::section_next(ui, sec);
        row(ui, t("Capture area:"), |ui| {
            ui.checkbox(&mut s.show_magnifier, t("Show magnifier"));
        });
        row(ui, "", |ui| {
            ui.checkbox(&mut s.highlight_windows, t("Highlight window under cursor"));
        });
        row(ui, "", |ui| {
            ui.checkbox(&mut s.window_shadow, t("Add shadow to window"));
        });
        row(ui, "", |ui| {
            ui.checkbox(&mut s.shot_on_click, t("Click to snap a window"));
        });
        row(ui, "", |ui| {
            ui.checkbox(&mut s.show_tip, t("Show hints"));
        });
        sec = crate::theme::section_next(ui, sec);
        row(ui, t("Status bar:"), |ui| {
            ui.checkbox(&mut s.show_startup_tooltip, t("Show hints"));
        });
        row(ui, "", |ui| {
            ui.checkbox(&mut s.hide_animation, t("Hide animation"));
        });
        row(ui, "", |ui| {
            ui.checkbox(&mut s.dont_show_notifications, t("Hide all notifications"));
        });
        sec = crate::theme::section_next(ui, sec);
        row(ui, t("Editor:"), |ui| {
            ui.checkbox(&mut s.show_tooltips_editor, t("Show tooltips"));
        });
        ui.add_enabled_ui(false, |ui| {
            row(ui, "", |ui| {
                ui.checkbox(&mut s.close_after_upload, t("Close the editor after uploading"));
            });
            row(ui, "", |ui| {
                ui.checkbox(&mut s.allow_default_browser, t("Use default browser to sign in"));
            });
        })
        .response
        .on_disabled_hover_text(t(NO_ACCOUNT));
        crate::theme::section_end(ui, sec);
    }

    fn hotkeys(&mut self, ui: &mut Ui, s: &mut Settings, ch: &mut Changes) {
        let mut sec = crate::theme::section_begin(ui);
        for a in Action::CAPTURE_ROWS {
            self.hotkey_row(ui, s, a, ch);
        }
        sec = crate::theme::section_next(ui, sec);
        row(ui, t("Video record hotkeys:"), |_| {});
        for a in Action::VIDEO_ROWS {
            self.hotkey_row(ui, s, a, ch);
        }
        ui.add_space(6.0);
        row(ui, "", |ui| {
            if ui.button(t("Default")).on_hover_text(t("Restore the default hotkeys")).clicked() {
                s.hotkeys = crate::hotkeys::default_hotkeys();
                ch.hotkeys = true;
                self.recording = None;
            }
        });
        crate::theme::section_end(ui, sec);
    }

    fn hotkey_row(&mut self, ui: &mut Ui, s: &mut Settings, a: Action, ch: &mut Changes) {
        let current = s.hotkey(a);
        let rec = self.recording == Some(a);
        row(ui, a.label(), |ui| {
            let text = if rec {
                RichText::new(t("Press hotkey with ⌘/⌃")).color(crate::theme::pal(ui.ctx()).accent)
            } else {
                match &current {
                    Some(k) => RichText::new(k.display()),
                    None => RichText::new(t("Not set")).color(HINT_COLOR),
                }
            };
            let b = egui::Button::new(text).min_size(vec2(170.0, 22.0)).selected(rec);
            if ui.add(b).clicked() {
                self.recording = if rec { None } else { Some(a) };
            }
            let del = ui.add_enabled(current.is_some(), egui::Button::new("✕").min_size(vec2(22.0, 22.0)));
            if del.on_hover_text(t("Delete hotkey")).clicked() {
                s.set_hotkey(a, None);
                ch.hotkeys = true;
            }
        });
    }

    fn capture_hotkey(&mut self, ctx: &Context, s: &mut Settings, ch: &mut Changes) {
        let Some(a) = self.recording else { return };
        let events = ctx.input(|i| i.events.clone());
        for e in events {
            if let egui::Event::Key { key, pressed: true, modifiers, .. } = e {
                if key == Key::Escape {
                    self.recording = None;
                    return;
                }
                if key == Key::Backspace || key == Key::Delete {
                    s.set_hotkey(a, None);
                    ch.hotkeys = true;
                    self.recording = None;
                    return;
                }
                if !(modifiers.mac_cmd || modifiers.command || modifiers.ctrl) {
                    continue;
                }
                let k = Hotkey {
                    ctrl: modifiers.ctrl,
                    alt: modifiers.alt,
                    shift: modifiers.shift,
                    cmd: modifiers.mac_cmd || modifiers.command,
                    key: key.name().to_string(),
                };
                // one combination per action: take it away from any other row
                let others: Vec<Action> =
                    s.hotkeys.iter().filter(|(b, kk)| *b != a && kk.as_ref() == Some(&k)).map(|(b, _)| *b).collect();
                for o in others {
                    s.set_hotkey(o, None);
                }
                s.set_hotkey(a, Some(k));
                ch.hotkeys = true;
                self.recording = None;
                return;
            }
        }
    }

    fn fonts(&mut self, ui: &mut Ui, s: &mut Settings, ch: &mut Changes) {
        let sec = crate::theme::section_begin(ui);
        let name_of = |p: &str| {
            std::path::Path::new(p).file_stem().and_then(|n| n.to_str()).unwrap_or(p).to_string()
        };
        row(ui, t("Main font:"), |ui| {
            let cur = s.main_font.as_deref().map(name_of).unwrap_or_else(|| t("Default").into());
            egui::ComboBox::from_id_salt("main_font").width(220.0).selected_text(cur).show_ui(ui, |ui| {
                if ui.selectable_label(s.main_font.is_none(), t("Default")).clicked() {
                    s.main_font = None;
                    ch.fonts = true;
                }
                for f in s.additional_fonts.clone() {
                    if ui.selectable_label(s.main_font.as_deref() == Some(f.as_str()), name_of(&f)).clicked() {
                        s.main_font = Some(f.clone());
                        ch.fonts = true;
                    }
                }
            });
        });
        row(ui, t("Additional fonts:"), |ui| {
            ui.vertical(|ui| {
                Frame::new()
                    .fill(ui.visuals().extreme_bg_color)
                    .stroke(Stroke::new(1.0_f32, crate::theme::pal(ui.ctx()).border))
                    .corner_radius(8.0)
                    .inner_margin(Margin::same(6))
                    .show(ui, |ui| {
                        ui.set_min_size(vec2(220.0, 120.0));
                        ui.set_max_width(220.0);
                        egui::ScrollArea::vertical().max_height(120.0).show(ui, |ui| {
                            if s.additional_fonts.is_empty() {
                                ui.label(RichText::new(t("No additional fonts")).color(HINT_COLOR));
                            }
                            for (i, f) in s.additional_fonts.iter().enumerate() {
                                let sel = self.selected_font == Some(i);
                                if ui.selectable_label(sel, name_of(f)).on_hover_text(f).clicked() {
                                    self.selected_font = Some(i);
                                }
                            }
                        });
                    });
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    if ui.add(egui::Button::new("+").min_size(vec2(24.0, 20.0))).on_hover_text(t("Add font")).clicked() {
                        if let Some(files) = rfd::FileDialog::new()
                            .set_title(t("Add font"))
                            .set_directory("/Library/Fonts")
                            .add_filter(t("Font"), &["ttf", "otf", "ttc"])
                            .pick_files()
                        {
                            for f in files {
                                let p = f.display().to_string();
                                if !s.additional_fonts.contains(&p) {
                                    s.additional_fonts.push(p);
                                }
                            }
                            ch.fonts = true;
                        }
                    }
                    let can_remove = self.selected_font.map(|i| i < s.additional_fonts.len()).unwrap_or(false);
                    if ui
                        .add_enabled(can_remove, egui::Button::new("−").min_size(vec2(24.0, 20.0)))
                        .on_hover_text(t("Remove font"))
                        .clicked()
                    {
                        if let Some(i) = self.selected_font.take() {
                            let removed = s.additional_fonts.remove(i);
                            if s.main_font.as_deref() == Some(removed.as_str()) {
                                s.main_font = None;
                            }
                            ch.fonts = true;
                        }
                    }
                });
            });
        });
        row(ui, "", |ui| {
            ui.label(
                RichText::new(t("Attention: some fonts could not correctly displayed in the editor.")).size(10.0).color(HINT_COLOR),
            );
        });
        ui.add_space(6.0);
        row(ui, "", |ui| {
            if ui.button(t("Reset fonts")).clicked() {
                s.main_font = None;
                s.additional_fonts.clear();
                self.selected_font = None;
                ch.fonts = true;
            }
        });
        crate::theme::section_end(ui, sec);
    }
}

fn row(ui: &mut Ui, label: &str, add: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(vec2(LABEL_W, 20.0), Sense::hover());
        if !label.is_empty() {
            let pal = crate::theme::pal(ui.ctx());
            let col = if ui.is_enabled() { pal.text } else { pal.hint.gamma_multiply(0.7) };
            // long translations shrink a little to stay inside the label column
            let w = ui.fonts(|f| f.layout_no_wrap(label.to_string(), FontId::proportional(13.0), col).size().x);
            let size = if w > LABEL_W - 4.0 { (13.0 * (LABEL_W - 4.0) / w).max(9.5) } else { 13.0 };
            ui.painter().text(r.right_center() - vec2(2.0, 0.0), Align2::RIGHT_CENTER, label, FontId::proportional(size), col);
        }
        add(ui);
    });
}

#[allow(dead_code)]
fn separator(ui: &mut Ui) {
    ui.add_space(2.0);
    let w = ui.available_width();
    let (r, _) = ui.allocate_exact_size(vec2(w, 1.0), Sense::hover());
    ui.painter().line_segment([r.left_center(), r.right_center()], Stroke::new(1.0_f32, Color32::from_gray(0xd0)));
    ui.add_space(2.0);
}

fn flags(ui: &mut Ui, v: &mut u8) {
    for (bit, name) in [(COPY_IMAGE, t("Image")), (COPY_URL, "URL"), (COPY_PATH, t("Path"))] {
        let mut on = *v & bit != 0;
        let enabled = bit != COPY_URL;
        let r = ui.add_enabled(enabled, egui::Checkbox::new(&mut on, name));
        if r.changed() {
            if on {
                *v |= bit;
            } else {
                *v &= !bit;
            }
        }
        if !enabled {
            r.on_disabled_hover_text(t(NO_ACCOUNT));
        }
    }
}

/// Writes or removes the login item that starts the app when you log in.
pub fn apply_launch_at_login(on: bool) -> Result<(), String> {
    let dir = dirs::home_dir().ok_or("no home folder")?.join("Library/LaunchAgents");
    let file = dir.join(format!("com.{}.app.plist", APP_NAME.to_lowercase()));
    if on {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let plist = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\"><dict>\n<key>Label</key><string>com.{}.app</string>\n<key>ProgramArguments</key><array><string>{}</string></array>\n<key>RunAtLoad</key><true/>\n</dict></plist>\n",
            APP_NAME.to_lowercase(),
            exe.display()
        );
        std::fs::write(&file, plist).map_err(|e| e.to_string())
    } else {
        if file.exists() {
            std::fs::remove_file(&file).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}

#[allow(dead_code)]
pub fn template_preview(s: &Settings) -> String {
    settings::apply_template(&s.filename_template, "Window", "png")
}
