#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod diff;
mod i18n;
mod pages;
mod settings;
mod svn;
mod system;
mod task;
mod types;

pub use types::*;

fn load_icon() -> Option<egui::IconData> {
    let image = image::load_from_memory(include_bytes!("../icon.png"))
        .ok()?
        .into_rgba8();
    let (width, height) = image.dimensions();
    Some(egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    })
}

fn main() -> eframe::Result {
    tracing_subscriber::fmt::init();

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("OrcaSVN")
        .with_inner_size([1200.0, 780.0])
        .with_min_inner_size([720.0, 480.0])
        .with_app_id("orcasvn");
    if let Some(icon) = load_icon() {
        viewport = viewport.with_icon(icon);
    }

    eframe::run_native(
        "OrcaSVN",
        eframe::NativeOptions {
            viewport,
            ..Default::default()
        },
        Box::new(|cc| Ok(Box::new(app::OrcaApp::new(cc)))),
    )
}
