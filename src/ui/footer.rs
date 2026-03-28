use eframe::egui;
use egui::{Color32, RichText};
use lucide_icons::Icon;

use crate::app::ImageDifferApp;
use crate::theme::{ACCENT, BG_DEEP, BORDER, TEXT, TEXT_DIM, icon_text};

pub fn render_footer(app: &mut ImageDifferApp, ui: &mut egui::Ui, _ctx: &egui::Context) {
    let has_both = app.texture_left.is_some() && app.texture_right.is_some();
    let has_one = app.texture_left.is_some() || app.texture_right.is_some();

    egui::Panel::bottom("footer")
        .frame(
            egui::Frame::NONE
                .fill(BG_DEEP)
                .stroke(egui::Stroke::new(1.0, BORDER)),
        )
        .show_inside(ui, |ui| {
            ui.set_height(24.0);
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.add_space(12.0);
                if let (Some(l), Some(r)) = (&app.image_left, &app.image_right) {
                    ui.label(
                        RichText::new(format!("{}×{}", l.width(), l.height()))
                            .size(10.0)
                            .color(TEXT_DIM)
                            .monospace(),
                    );
                    ui.label(RichText::new(" → ").size(10.0).color(TEXT_DIM));
                    ui.label(
                        RichText::new(format!("{}×{}", r.width(), r.height()))
                            .size(10.0)
                            .color(TEXT_DIM)
                            .monospace(),
                    );
                }
                if has_both || has_one {
                    ui.add_space(16.0);
                    ui.label(icon_text(Icon::ZoomIn, 12.0).color(TEXT_DIM));
                    let ztxt = if app.zoom_level <= 0.0 {
                        "Fit".to_string()
                    } else {
                        format!("{:.0}%", app.zoom_level * 100.0)
                    };
                    if ui
                        .add(
                            egui::Button::new(RichText::new("1:1").size(10.0).color(TEXT))
                                .fill(Color32::TRANSPARENT)
                                .stroke(egui::Stroke::NONE),
                        )
                        .clicked()
                    {
                        app.zoom_level = 1.0;
                        app.pan_offset = egui::Vec2::ZERO;
                    }
                    if ui
                        .add(
                            egui::Button::new(RichText::new("Fit").size(10.0).color(TEXT))
                                .fill(Color32::TRANSPARENT)
                                .stroke(egui::Stroke::NONE),
                        )
                        .clicked()
                    {
                        app.zoom_level = 0.0;
                        app.pan_offset = egui::Vec2::ZERO;
                    }
                    ui.add(egui::Slider::new(&mut app.zoom_level, 0.0..=5.0).show_value(false));
                    ui.label(RichText::new(ztxt).size(10.0).color(TEXT_DIM).monospace());
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(12.0);
                    ui.label(RichText::new("IDIFFER").size(9.0).color(TEXT_DIM).strong());
                    ui.add_space(6.0);
                    let (dot_rect, _) =
                        ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
                    ui.painter().circle_filled(dot_rect.center(), 3.0, ACCENT);
                    // Setup hint (only when totally empty and no git context)
                    if !has_one && app.git_context.is_none() {
                        ui.add_space(16.0);
                        let hint = "idiffer --install  to register as git difftool";
                        ui.label(RichText::new(hint).size(9.0).color(TEXT_DIM).monospace());
                    }
                });
            });
        });
}
