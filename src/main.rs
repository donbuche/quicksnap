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
mod i18n;
mod icons;
mod overlay;
mod settings;
mod settings_ui;
mod shapes;
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

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(settings::APP_NAME)
            .with_inner_size([1240.0, 800.0])
            .with_min_inner_size([1000.0, 620.0])
            .with_icon(std::sync::Arc::new(appicon::icon_data(256))),
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
