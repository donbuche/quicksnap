//! Taking screenshots and screen recordings with the system's own capture
//! tool, saving them with the filename template and copying to the clipboard.

use image::RgbaImage;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};

use crate::settings::{self, ImageFormat, Settings, VideoFormat, COPY_IMAGE, COPY_PATH};
use crate::i18n::{t, tf};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CaptureKind {
    Area,
    FrozenArea,
    Fullscreen,
    FullscreenDelayed,
}

/// What the capture thread needs from the settings.
#[derive(Clone)]
pub struct CaptureOptions {
    pub window_shadow: bool,
    pub shot_on_click: bool,
    pub sound: bool,
    pub shrink_retina: bool,
    pub retina_scale: f32,
}

impl CaptureOptions {
    pub fn from(s: &Settings, retina_scale: f32) -> CaptureOptions {
        CaptureOptions {
            window_shadow: s.window_shadow,
            shot_on_click: s.shot_on_click,
            sound: s.sound_effects,
            shrink_retina: s.shrink_retina,
            retina_scale,
        }
    }
}

pub enum CaptureResult {
    Image(RgbaImage),
    Cancelled,
    Failed(String),
}

fn temp_file(ext: &str) -> PathBuf {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("{}-{}.{}", settings::APP_NAME.to_lowercase(), stamp, ext))
}

/// Runs the system capture tool and blocks until it finishes; call it from a
/// worker thread.
pub fn capture(kind: CaptureKind, opt: &CaptureOptions) -> CaptureResult {
    let file = temp_file("png");
    let mut cmd = Command::new("/usr/sbin/screencapture");
    if !opt.sound {
        cmd.arg("-x");
    }
    match kind {
        CaptureKind::Area | CaptureKind::FrozenArea => {
            cmd.arg("-i");
            if !opt.shot_on_click {
                cmd.arg("-s"); // selection only, no click-to-snap-window
            }
            if !opt.window_shadow {
                cmd.arg("-o");
            }
        }
        CaptureKind::Fullscreen => {
            cmd.arg("-m");
        }
        CaptureKind::FullscreenDelayed => {
            cmd.arg("-m").arg(format!("-T{}", settings::DELAYED_CAPTURE_SECONDS));
        }
    }
    cmd.arg(&file);
    match cmd.status() {
        Ok(_) => {}
        Err(e) => return CaptureResult::Failed(tf("Could not start the capture tool: {}", &[&e])),
    }
    if !file.exists() {
        return CaptureResult::Cancelled;
    }
    let img = match image::open(&file) {
        Ok(i) => i.to_rgba8(),
        Err(e) => return CaptureResult::Failed(tf("Could not read the capture: {}", &[&e])),
    };
    let _ = std::fs::remove_file(&file);
    let img = if opt.shrink_retina && opt.retina_scale > 1.0 {
        let w = (img.width() as f32 / opt.retina_scale).round().max(1.0) as u32;
        let h = (img.height() as f32 / opt.retina_scale).round().max(1.0) as u32;
        image::imageops::resize(&img, w, h, image::imageops::FilterType::Lanczos3)
    } else {
        img
    };
    CaptureResult::Image(img)
}

fn encode_png(img: &RgbaImage) -> Result<Vec<u8>, String> {
    let mut buf = std::io::Cursor::new(Vec::new());
    img.write_to(&mut buf, image::ImageFormat::Png).map_err(|e| e.to_string())?;
    Ok(buf.into_inner())
}

fn encode_jpeg(img: &RgbaImage, quality: u32) -> Result<Vec<u8>, String> {
    // JPEG has no transparency: see-through parts (like a drop shadow's
    // margin) go on white instead of turning black
    let rgb = image::RgbImage::from_fn(img.width(), img.height(), |x, y| {
        let p = img.get_pixel(x, y);
        let a = p[3] as u32;
        let c = |v: u8| ((v as u32 * a + 255 * (255 - a)) / 255) as u8;
        image::Rgb([c(p[0]), c(p[1]), c(p[2])])
    });
    let mut buf = Vec::new();
    let enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, quality.clamp(1, 100) as u8);
    rgb.write_with_encoder(enc).map_err(|e| e.to_string())?;
    Ok(buf)
}

/// Encodes in the chosen format. "Auto (smaller size)" tries both and keeps
/// the smaller file.
pub fn encode(img: &RgbaImage, s: &Settings) -> Result<(Vec<u8>, &'static str), String> {
    match s.image_format {
        ImageFormat::Png => Ok((encode_png(img)?, "png")),
        ImageFormat::Jpeg => Ok((encode_jpeg(img, s.jpeg_quality)?, "jpg")),
        ImageFormat::Auto => {
            let p = encode_png(img)?;
            let j = encode_jpeg(img, s.jpeg_quality)?;
            if j.len() < p.len() { Ok((j, "jpg")) } else { Ok((p, "png")) }
        }
    }
}

fn unique_path(dir: &Path, stem: &str, ext: &str) -> PathBuf {
    let mut p = dir.join(format!("{stem}.{ext}"));
    let mut n = 2;
    while p.exists() {
        p = dir.join(format!("{stem} {n}.{ext}"));
        n += 1;
    }
    p
}

/// Saves into the save folder with the filename template.
pub fn save_to_folder(img: &RgbaImage, s: &Settings, title: &str) -> Result<PathBuf, String> {
    let (bytes, ext) = encode(img, s)?;
    let dir = s.save_dir();
    std::fs::create_dir_all(&dir).map_err(|e| tf("Can't create {}: {}", &[&dir.display(), &e]))?;
    let stem = settings::apply_template(&s.filename_template, title, ext);
    let path = unique_path(&dir, &stem, ext);
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    Ok(path)
}

/// Saves to a path the person picked; the extension decides the format.
pub fn save_to_path(img: &RgbaImage, s: &Settings, path: &Path) -> Result<(), String> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("png").to_lowercase();
    let bytes = if ext == "jpg" || ext == "jpeg" { encode_jpeg(img, s.jpeg_quality)? } else { encode_png(img)? };
    std::fs::write(path, bytes).map_err(|e| e.to_string())
}

/// Copies following the "Copy to clipboard" flags (Image / Path).
pub fn copy_to_clipboard(img: &RgbaImage, s: &Settings, saved: Option<&Path>) -> Result<&'static str, String> {
    let mut cb = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    if s.clipboard_flags & COPY_IMAGE != 0 || saved.is_none() {
        cb.set_image(arboard::ImageData {
            width: img.width() as usize,
            height: img.height() as usize,
            bytes: std::borrow::Cow::Borrowed(img.as_raw()),
        })
        .map_err(|e| e.to_string())?;
        return Ok(t("Screenshot copied to clipboard"));
    }
    if s.clipboard_flags & COPY_PATH != 0 {
        if let Some(p) = saved {
            cb.set_text(p.display().to_string()).map_err(|e| e.to_string())?;
            return Ok(t("Path copied to clipboard"));
        }
    }
    Ok(t("Nothing to copy"))
}

pub fn image_from_clipboard() -> Result<RgbaImage, String> {
    let mut cb = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    match cb.get_image() {
        Ok(data) => RgbaImage::from_raw(data.width as u32, data.height as u32, data.bytes.into_owned())
            .ok_or_else(|| t("Clipboard doesn't contain image").to_string()),
        Err(_) => Err(t("Clipboard doesn't contain image").to_string()),
    }
}

pub fn open_in_app(app: &str, file: &Path) -> Result<(), String> {
    Command::new("/usr/bin/open").arg("-a").arg(app).arg(file).spawn().map(|_| ()).map_err(|e| e.to_string())
}

pub fn reveal_in_finder(file: &Path) {
    let _ = Command::new("/usr/bin/open").arg("-R").arg(file).spawn();
}

// ---------------- screen recording ----------------

pub struct Recorder {
    child: Child,
    file: PathBuf,
    pub started: std::time::Instant,
}

impl Recorder {
    /// Records the main screen with clicks and cursor shown (the original's
    /// video defaults: Capture Mouse Clicks on, Capture Mouse Cursor on).
    pub fn start(sound: bool) -> Result<Recorder, String> {
        let file = temp_file("mov");
        let mut cmd = Command::new("/usr/sbin/screencapture");
        cmd.arg("-v").arg("-k").arg("-C");
        if !sound {
            cmd.arg("-x");
        }
        cmd.arg(&file);
        let child = cmd.spawn().map_err(|e| tf("Could not start recording: {}", &[&e]))?;
        Ok(Recorder { child, file, started: std::time::Instant::now() })
    }

    /// Stops the recording and puts the movie where the settings say.
    pub fn stop(mut self, s: &Settings) -> Result<PathBuf, String> {
        unsafe {
            libc::kill(self.child.id() as i32, libc::SIGINT);
        }
        let _ = self.child.wait();
        if !self.file.exists() {
            return Err(t("The recording was cancelled").into());
        }
        let mut out = self.file.clone();
        if s.video_format == VideoFormat::Gif {
            let gif = self.file.with_extension("gif");
            let scale = s.gif_quality as f32 / 100.0;
            let filter = format!("fps={},scale=trunc(iw*{scale:.2}/2)*2:-2:flags=lanczos", s.gif_fps);
            let ok = ["/opt/homebrew/bin/ffmpeg", "/usr/local/bin/ffmpeg", "ffmpeg"].iter().any(|bin| {
                Command::new(bin)
                    .args(["-y", "-loglevel", "error", "-i"])
                    .arg(&self.file)
                    .args(["-vf", &filter])
                    .arg(&gif)
                    .status()
                    .map(|st| st.success())
                    .unwrap_or(false)
            });
            if ok {
                let _ = std::fs::remove_file(&self.file);
                out = gif;
            }
        }
        let ext = out.extension().and_then(|e| e.to_str()).unwrap_or("mov").to_string();
        let dir = if s.always_save_movie {
            dirs::video_dir().unwrap_or_else(|| s.save_dir())
        } else {
            s.save_dir()
        };
        let _ = std::fs::create_dir_all(&dir);
        let stem = settings::apply_template(&s.filename_template, "Video", &ext);
        let dest = unique_path(&dir, &stem, &ext);
        if std::fs::rename(&out, &dest).is_err() {
            std::fs::copy(&out, &dest).map_err(|e| e.to_string())?;
            let _ = std::fs::remove_file(&out);
        }
        Ok(dest)
    }
}
