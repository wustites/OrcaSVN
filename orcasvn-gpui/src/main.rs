#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![recursion_limit = "512"]

mod app;
mod diff;
mod i18n;
mod pages;
mod settings;
mod system;
mod task;
mod ui;

pub use orcasvn_core::svn;
pub use orcasvn_core::*;

use gpui_kit::component::{Theme as KitTheme, ThemeMode};
use gpui_kit::{
    px, size, App, AppContext as _, Bounds, TitlebarOptions, WindowBounds, WindowOptions,
};
use gpui_kit_material::color::dynamic::SeedScheme;
use gpui_kit_material::theme;

/// OrcaSVN brand primary, matching `src/design-tokens.css` on the Tauri client.
const BRAND: u32 = 0x1261a0;

fn main() {
    tracing_subscriber::fmt::init();

    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(|cx: &mut App| {
            // Registers the GPUI Kit theme, root, inputs and dialogs.
            gpui_kit::init(cx);
            // Note: `gpui_kit_material::init` is deliberately not called. It
            // installs an icon resolver that points Kit's internal icons at
            // Material Symbols assets; without `embedded-symbols` (~51 MB of
            // fonts) those lookups fail and Kit's dropdown/empty-state icons
            // render blank. Phase 1+ needs that resolved before using it.
            ui::init_fonts(cx);

            // Select the theme before any input entity is created; inputs
            // capture their styling when they are constructed.
            let settings = crate::settings::Settings::load();
            let mode = match settings.theme {
                crate::settings::Theme::Light => ThemeMode::Light,
                crate::settings::Theme::Dark => ThemeMode::Dark,
                crate::settings::Theme::System => ThemeMode::from(cx.window_appearance()),
            };
            KitTheme::change(mode, None, cx);

            // Derive the whole Material scheme from the OrcaSVN brand color, so
            // surfaces and containers are tuned around the brand hue instead of
            // the default Baseline purple. Must run after the mode config is
            // loaded and before the first Root render.
            let scheme = SeedScheme::new(BRAND).dark(mode.is_dark()).scheme();
            theme::init_component_theme(scheme, settings.zoom, cx);

            let bounds = Bounds::centered(None, size(px(1200.0), px(780.0)), cx);
            gpui_kit::open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    window_min_size: Some(size(px(720.0), px(480.0))),
                    titlebar: Some(TitlebarOptions {
                        title: Some("OrcaSVN".into()),
                        ..Default::default()
                    }),
                    app_id: Some("orcasvn".to_string()),
                    ..Default::default()
                },
                cx,
                |window, cx| cx.new(|cx| app::OrcaApp::new(window, cx)),
            )
            .expect("failed to open the OrcaSVN window");
        });
}
