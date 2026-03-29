#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod cli;
mod theme;
mod types;
mod ui;

use app::ImageDifferApp;
use eframe::egui;

fn main() -> eframe::Result<()> {
    let cfg = cli::parse_args();
    eframe::run_native(
        "IDiffer",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([1200.0, 800.0])
                .with_min_inner_size([700.0, 500.0]),
            ..Default::default()
        },
        Box::new(|cc| {
            theme::setup_fonts(&cc.egui_ctx);
            theme::setup_visuals(&cc.egui_ctx);
            Ok(Box::new(ImageDifferApp::new(cc, cfg)))
        }),
    )
}
