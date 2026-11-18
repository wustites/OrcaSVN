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

fn main() {
    tracing_subscriber::fmt::init();

    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(|cx: &mut App| {
            // Registers the GPUI Kit theme, root, inputs and dialogs.
            gpui_kit::init(cx);
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
