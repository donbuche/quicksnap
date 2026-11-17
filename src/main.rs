//! QuickSnap: a screenshot tool with a menu-bar icon, global hotkeys, an
//! annotation editor and a Preferences window with five tabs.
//!
//! `--screenshot <file.png>` starts normally (showing the editor with a sample
//! picture), draws about 120 frames, saves the window to that file and exits.

mod app;
mod appicon;
mod capture;
mod colour_picker;
mod editor;
mod hotkeys;
mod i18n;
mod icons;
mod overlay;
mod screens;
mod selector;
mod settings;
mod settings_ui;
mod shapes;
mod sound;
mod theme;

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    // `--write-icon <file.png> <size>`: render the app icon to a PNG and exit
    // (used by scripts/package-macos.sh to build the .icns).
    if let Some(i) = args.iter().position(|a| a == "--write-icon") {
        let path = args.get(i + 1).expect("--write-icon <file.png> <size>");
        let size: u32 = args.get(i + 2).and_then(|s| s.parse().ok()).unwrap_or(1024);
        appicon::render(size).save(path).expect("could not write icon");
        return Ok(());
    }
    // `--write-tray-icon <file.png> <size>`: the one-ink menu-bar glyph.
    if let Some(i) = args.iter().position(|a| a == "--write-tray-icon") {
        let path = args.get(i + 1).expect("--write-tray-icon <file.png> <size>");
        let size: u32 = args.get(i + 2).and_then(|s| s.parse().ok()).unwrap_or(44);
        appicon::render_template(size).save(path).expect("could not write icon");
        return Ok(());
    }
    let screenshot = args
        .iter()
        .position(|a| a == "--screenshot")
        .and_then(|i| args.get(i + 1))
        .map(std::path::PathBuf::from);

    // QUICKSNAP_WINDOW=<w>x<h> starts with another window size (for checking
    // layouts with --screenshot).
    let size = std::env::var("QUICKSNAP_WINDOW")
        .ok()
        .and_then(|v| v.split_once('x').and_then(|(w, h)| Some([w.parse::<f32>().ok()?, h.parse::<f32>().ok()?])))
        .unwrap_or([1240.0, 800.0]);
    // QUICKSNAP_ZOOM=2 draws everything twice as large in a window twice as
    // large: Retina-sharp screenshots of the same layout on a 1x display.
    let zoom = screenshot_zoom();
    let size = [size[0] * zoom, size[1] * zoom];
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(settings::APP_NAME)
            .with_inner_size(size)
            // wide enough for the editor's three top bars in one row, in
            // every language (screenshots set their own size: the
            // Preferences ones are narrower)
            .with_min_inner_size(if screenshot.is_some() { [0.0, 0.0] } else { [1180.0, 640.0] })
            .with_icon(std::sync::Arc::new(appicon::icon_data(256)))
            // see-through, for the frosted-glass look (see theme::apply_vibrancy)
            .with_transparent(cfg!(target_os = "macos") && screenshot.is_none())
            // no title bar: the app's background runs up to the top edge and
            // only the red-yellow-green buttons float over it
            .with_fullsize_content_view(true)
            .with_titlebar_shown(false)
            .with_title_shown(false),
        event_loop_builder: Some(Box::new(|_builder| {
            // Live in the menu bar only: no Dock icon, no app menu. Closing the
            // window just hides it; Quit is in the menu-bar icon's menu.
            #[cfg(target_os = "macos")]
            {
                use winit::platform::macos::{ActivationPolicy, EventLoopBuilderExtMacOS};
                _builder.with_activation_policy(ActivationPolicy::Accessory);
            }
        })),
        ..Default::default()
    };
    eframe::run_native(
        settings::APP_NAME,
        options,
        Box::new(move |cc| Ok(Box::new(app::App::new(cc, screenshot)))),
    )
}

/// The QUICKSNAP_ZOOM factor for `--screenshot` (1 when unset).
pub fn screenshot_zoom() -> f32 {
    std::env::var("QUICKSNAP_ZOOM").ok().and_then(|z| z.parse::<f32>().ok()).filter(|z| *z >= 1.0).unwrap_or(1.0)
}
