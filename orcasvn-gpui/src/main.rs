#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![recursion_limit = "512"]

mod app;
mod diff;
mod i18n;
mod pages;
mod settings;
mod svn;
mod system;
mod task;
mod types;
mod ui;

pub use types::*;

use gpui::{
    px, size, App, AppContext as _, Application, Bounds, TitlebarOptions, WindowBounds,
    WindowOptions,
};

fn main() {
    tracing_subscriber::fmt::init();

    Application::new().run(|cx: &mut App| {
        ui::init_fonts(cx);
        let bounds = Bounds::centered(None, size(px(1200.0), px(780.0)), cx);
        cx.open_window(
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
            |_, cx| cx.new(app::OrcaApp::new),
        )
        .expect("failed to open the OrcaSVN window");
        cx.activate(true);
    });
}
