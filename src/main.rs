#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod cli;
mod theme;
mod types;
mod ui;
mod utils;

use app::ImageDifferApp;
use cli::parse_args;
use eframe::egui;
use theme::{setup_fonts, setup_visuals};

fn main() -> eframe::Result<()> {
    let cfg = parse_args();
    eframe::run_native(
        "IDiffer",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([1200.0, 800.0])
                .with_min_inner_size([700.0, 500.0]),
            ..Default::default()
        },
        Box::new(|cc| {
            setup_fonts(&cc.egui_ctx);
            setup_visuals(&cc.egui_ctx);
            Ok(Box::new(ImageDifferApp::new(cc, cfg)))
        }),
    )
}
