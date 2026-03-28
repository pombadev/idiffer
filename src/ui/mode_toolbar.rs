use eframe::egui;
use egui::{Color32, RichText};
use lucide_icons::Icon;

use crate::app::ImageDifferApp;
use crate::theme::{ACCENT, BG_DEEP, BORDER, TEXT_MUTED, icon_str};
use crate::types::DiffMode;

pub fn render_mode_toolbar(app: &mut ImageDifferApp, ui: &mut egui::Ui, _ctx: &egui::Context) {
    if !(app.texture_left.is_some() && app.texture_right.is_some()) {
        return;
    }

    egui::Panel::top("mode_toolbar")
        .exact_size(40.0)
        .frame(
            egui::Frame::NONE
                .fill(BG_DEEP)
                .stroke(egui::Stroke::new(1.0, BORDER)),
        )
        .show_inside(ui, |ui| {
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.add_space(16.0);

                let modes = [
                    (DiffMode::Slider, Icon::Split, "Slider"),
                    (DiffMode::SideBySide, Icon::Columns2, "Side by Side"),
                    (DiffMode::Difference, Icon::Diff, "Difference"),
                    (DiffMode::Fade, Icon::Blend, "Fade"),
                ];

                for (mode, icon, label) in modes {
                    let active = app.diff_mode == mode;
                    let (fill, color, stroke) = if active {
                        (
                            ACCENT.gamma_multiply(0.12),
                            ACCENT,
                            egui::Stroke::new(1.0, ACCENT.gamma_multiply(0.35)),
                        )
                    } else {
                        (Color32::TRANSPARENT, TEXT_MUTED, egui::Stroke::NONE)
                    };
                    let tab_label = format!("{} {}", icon_str(icon), label);
                    let resp = ui.add(
                        egui::Button::new(RichText::new(tab_label).size(12.0).color(color))
                            .fill(fill)
                            .stroke(stroke)
                            .corner_radius(egui::CornerRadius::same(6u8)),
                    );
                    if resp.clicked() {
                        app.diff_mode = mode;
                    }
                    ui.add_space(2.0);
                }

                // Inline Fade slider
                if app.diff_mode == DiffMode::Fade {
                    ui.add_space(16.0);
                    let (sep, _) =
                        ui.allocate_exact_size(egui::vec2(1.0, 20.0), egui::Sense::hover());
                    ui.painter().rect_filled(sep, 0.0, BORDER);
                    ui.add_space(12.0);
                    ui.label(RichText::new("Opacity").size(11.0).color(TEXT_MUTED));
                    ui.add_space(6.0);
                    ui.add(egui::Slider::new(&mut app.fade_opacity, 0.0..=1.0).show_value(false));
                }

                // Diff % badge in toolbar when Difference mode
                if app.diff_mode == DiffMode::Difference {
                    if let Some(pct) = app.diff_pixel_pct {
                        ui.add_space(16.0);
                        let (sep, _) =
                            ui.allocate_exact_size(egui::vec2(1.0, 20.0), egui::Sense::hover());
                        ui.painter().rect_filled(sep, 0.0, BORDER);
                        ui.add_space(12.0);
                        let color = if pct < 1.0 {
                            crate::theme::SUCCESS
                        } else {
                            crate::theme::DANGER
                        };
                        ui.label(
                            RichText::new(format!("{:.2}% changed", pct))
                                .size(11.0)
                                .color(color)
                                .monospace(),
                        );
                    }
                }
            });
        });
}
