//! QuickSnap: a screenshot tool with a menu-bar icon, global hotkeys, an
//! annotation editor and a Preferences window with five tabs.
//!
//! `--screenshot <file.png>` starts normally (showing the editor with a sample
//! picture), draws about 120 frames, saves the window to that file and exits.

mod app;
mod appicon;
mod capture;
mod editor;
mod hotkeys;
mod icons;
mod overlay;
mod settings;
mod settings_ui;
mod shapes;
mod theme;

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let screenshot = args
        .iter()
        .position(|a| a == "--screenshot")
        .and_then(|i| args.get(i + 1))
        .map(std::path::PathBuf::from);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(settings::APP_NAME)
            .with_inner_size([1240.0, 800.0])
            .with_min_inner_size([1000.0, 620.0])
            .with_icon(std::sync::Arc::new(appicon::icon_data(256))),
        ..Default::default()
    };
    eframe::run_native(
        settings::APP_NAME,
        options,
        Box::new(move |cc| Ok(Box::new(app::App::new(cc, screenshot)))),
    )
}
