//! The short click played when a button or other control is pressed (the
//! same one as in MacControl). Sound: "audio-volume-change" from the
//! freedesktop.org sound theme (CC-BY-SA, Lucas McCallister), see
//! assets/sounds/CREDITS.txt. Played with rodio on its own thread.

use std::io::Cursor;
use std::sync::mpsc::{channel, Sender};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use egui::Context;

const CLICK: &[u8] = include_bytes!("../assets/sounds/click.wav");

static TX: Mutex<Option<Sender<()>>> = Mutex::new(None);

/// Opens the sound output once, on its own thread (only when first needed,
/// so nothing is opened while the click sound is off).
fn sender() -> Option<Sender<()>> {
    let mut guard = TX.lock().ok()?;
    if guard.is_none() {
        let (tx, rx) = channel::<()>();
        *guard = Some(tx);
        std::thread::Builder::new()
            .name("sound".into())
            .spawn(move || {
                let sink = match open_sink() {
                    Ok(mut s) => {
                        s.log_on_drop(false);
                        s
                    }
                    Err(e) => {
                        eprintln!("click sound: no audio output: {e}");
                        while rx.recv().is_ok() {}
                        return;
                    }
                };
                let mut last = Instant::now() - Duration::from_secs(1);
                while rx.recv().is_ok() {
                    // a burst of presses plays one click
                    if last.elapsed() < Duration::from_millis(45) {
                        continue;
                    }
                    last = Instant::now();
                    if let Ok(src) = rodio::Decoder::new(Cursor::new(CLICK)) {
                        use rodio::Source;
                        sink.mixer().add(src.amplify(0.55));
                    }
                }
            })
            .ok();
    }
    guard.clone()
}

/// Opens the default audio output. rodio asks for a buffer of about 50 ms
/// (2048 samples at 48 kHz), which some outputs refuse (many only allow up
/// to 1024); then smaller fixed sizes are tried, and finally the device's own.
fn open_sink() -> Result<rodio::stream::MixerDeviceSink, rodio::stream::DeviceSinkError> {
    use rodio::cpal::BufferSize;
    use rodio::DeviceSinkBuilder;
    let first = match DeviceSinkBuilder::open_default_sink() {
        Ok(s) => return Ok(s),
        Err(e) => e,
    };
    for size in [BufferSize::Fixed(1024), BufferSize::Fixed(512), BufferSize::Fixed(256), BufferSize::Default] {
        if let Ok(s) = DeviceSinkBuilder::from_default_device().and_then(|b| b.with_buffer_size(size).open_stream()) {
            return Ok(s);
        }
    }
    Err(first)
}

pub fn click() {
    if let Some(tx) = sender() {
        let _ = tx.send(());
    }
}

/// Call once per frame of a window (after its widgets are laid out): plays
/// the click when the primary button goes down on something clickable —
/// a button, tab, tile, checkbox, menu item… Large surfaces (the editor's
/// canvas, the capture overlay) are left out: drawing on them stays silent.
pub fn click_feedback(ctx: &Context, on: bool) {
    if !on || !ctx.input(|i| i.pointer.primary_pressed()) {
        return;
    }
    let screen = ctx.screen_rect().area().max(1.0);
    let hit = ctx.interaction_snapshot(|s| s.hovered.iter().copied().collect::<Vec<_>>()).into_iter().any(|id| {
        ctx.read_response(id)
            .is_some_and(|r| r.enabled() && r.sense.senses_click() && r.rect.area() < screen * 0.25)
    });
    if hit {
        click();
    }
}
