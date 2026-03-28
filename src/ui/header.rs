use eframe::egui;
use egui::{FontFamily, FontId, RichText};
use lucide_icons::Icon;

use crate::app::ImageDifferApp;
use crate::theme::{BG_SURFACE, BORDER, SUCCESS, TEXT, icon_str};

pub fn render_header(app: &mut ImageDifferApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    let has_both = app.texture_left.is_some() && app.texture_right.is_some();
    let has_one = app.texture_left.is_some() || app.texture_right.is_some();

    egui::Panel::top("header")
        .exact_size(52.0)
        .frame(
            egui::Frame::NONE
                .fill(BG_SURFACE)
                .stroke(egui::Stroke::new(1.0, BORDER)),
        )
        .show_inside(ui, |ui| {
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.add_space(16.0);

                // Logo
                ui.label(
                    RichText::new(icon_str(Icon::SquareSplitHorizontal))
                        .font(FontId::new(50.0, FontFamily::Name("lucide".into()))),
                );
                ui.add_space(8.0);
                ui.label(RichText::new("IDIFFER").size(13.0).color(TEXT).strong());
                ui.add_space(16.0);

                // Divider
                let (dvr, _) = ui.allocate_exact_size(egui::vec2(1.0, 28.0), egui::Sense::hover());
                ui.painter().rect_filled(dvr, 0.0, BORDER);
                ui.add_space(16.0);

                // Git context chip (shown when launched from git)
                if let Some(ref gc) = app.git_context.clone() {
                    let (dvr2, _) =
                        ui.allocate_exact_size(egui::vec2(1.0, 28.0), egui::Sense::hover());
                    ui.painter().rect_filled(dvr2, 0.0, BORDER);
                    ui.add_space(12.0);

                    egui::Frame::NONE
                        .fill(SUCCESS.gamma_multiply(0.08))
                        .corner_radius(egui::CornerRadius::same(6u8))
                        .stroke(egui::Stroke::new(1.0, SUCCESS.gamma_multiply(0.3)))
                        .inner_margin(egui::Margin {
                            left: 8,
                            right: 8,
                            top: 3,
                            bottom: 3,
                        })
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing.x = 6.0;
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(icon_str(Icon::GitBranch))
                                        .size(12.0)
                                        .color(SUCCESS)
                                        .font(FontId::new(12.0, FontFamily::Name("lucide".into()))),
                                );
                                ui.label(
                                    RichText::new(&gc.filename)
                                        .size(12.0)
                                        .color(SUCCESS)
                                        .strong(),
                                );
                                ui.label(
                                    RichText::new(&gc.old_rev)
                                        .size(11.0)
                                        .color(SUCCESS.gamma_multiply(0.7))
                                        .monospace(),
                                );
                                ui.label(
                                    RichText::new("→")
                                        .size(11.0)
                                        .color(SUCCESS.gamma_multiply(0.5)),
                                );
                                ui.label(
                                    RichText::new(&gc.new_rev)
                                        .size(11.0)
                                        .color(SUCCESS.gamma_multiply(0.7))
                                        .monospace(),
                                );
                            });
                        });
                    ui.add_space(4.0);
                }

                // Slots
                app.render_slot_header(ui, true, "Original", ctx);
                if has_both {
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(icon_str(Icon::ArrowLeftRight))
                            .size(14.0)
                            .color(crate::theme::TEXT_DIM)
                            .font(FontId::new(14.0, FontFamily::Name("lucide".into()))),
                    );
                    ui.add_space(8.0);
                }
                app.render_slot_header(ui, false, "New", ctx);

                // Right side actions
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(16.0);

                    // Clear
                    if has_one {
                        ui.add_space(8.0);
                        let clr_btn = egui::Button::new(
                            RichText::new("Clear")
                                .size(12.0)
                                .color(crate::theme::TEXT_MUTED),
                        )
                        .fill(egui::Color32::TRANSPARENT)
                        .corner_radius(egui::CornerRadius::same(6u8))
                        .stroke(egui::Stroke::new(1.0, BORDER));
                        if ui.add(clr_btn).clicked() {
                            app.clear_all();
                        }
                    }

                    // Error
                    if let Some(ref msg) = app.error_msg.clone() {
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new(format!("{} {}", icon_str(Icon::CircleAlert), msg))
                                .size(11.0)
                                .color(crate::theme::DANGER),
                        );
                    }
                });
            });
        });
}
