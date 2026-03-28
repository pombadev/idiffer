use eframe::egui;
use egui::{Color32, RichText, Stroke};
use lucide_icons::Icon;

use crate::app::ImageDifferApp;
use crate::theme::{ACCENT, BG_CARD, BORDER, TEXT, TEXT_MUTED, icon_str};

pub mod central_panel;
pub mod footer;
pub mod header;
pub mod mode_toolbar;

impl ImageDifferApp {
    pub fn render_slot_header(
        &mut self,
        ui: &mut egui::Ui,
        is_left: bool,
        _label: &str,
        ctx: &egui::Context,
    ) {
        let has_tex = if is_left {
            self.texture_left.is_some()
        } else {
            self.texture_right.is_some()
        };

        if has_tex {
            // Pill container
            let _frame_resp = egui::Frame::NONE
                .fill(BG_CARD)
                .corner_radius(egui::CornerRadius::same(8))
                .stroke(Stroke::new(1.0, BORDER))
                .inner_margin(egui::Margin {
                    left: 6,
                    right: 6,
                    top: 4,
                    bottom: 4,
                })
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        // thumbnail
                        if let Some(tex) = if is_left {
                            &self.texture_left
                        } else {
                            &self.texture_right
                        } {
                            ui.add(egui::Image::from_texture(tex).max_size(egui::vec2(28.0, 28.0)));
                        }
                        // filename
                        let name = if is_left {
                            &self.path_left
                        } else {
                            &self.path_right
                        };
                        let display = name
                            .as_ref()
                            .and_then(|p| p.file_name())
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default();
                        let truncated = if display.len() > 22 {
                            format!("{}…", &display[..20])
                        } else {
                            display
                        };
                        ui.label(RichText::new(truncated).size(12.0).color(TEXT));

                        // git history button
                        let path_clone = if is_left {
                            self.path_left.clone()
                        } else {
                            self.path_right.clone()
                        };
                        if let Some(path) = path_clone {
                            ui.menu_button(RichText::new(icon_str(Icon::GitBranch)), |ui| {
                                let commits = if is_left {
                                    if self.commits_left.is_empty() {
                                        self.commits_left = self.get_git_history(&path);
                                    }
                                    self.commits_left.clone()
                                } else {
                                    if self.commits_right.is_empty() {
                                        self.commits_right = self.get_git_history(&path);
                                    }
                                    self.commits_right.clone()
                                };
                                if commits.is_empty() {
                                    ui.label(
                                        RichText::new("No git history")
                                            .size(12.0)
                                            .color(TEXT_MUTED),
                                    );
                                } else {
                                    egui::ScrollArea::vertical()
                                        .max_height(250.0)
                                        .show(ui, |ui| {
                                            ui.set_min_width(360.0);
                                            for commit in &commits {
                                                ui.horizontal(|ui| {
                                                    ui.label(
                                                        RichText::new(&commit.hash)
                                                            .monospace()
                                                            .size(11.0)
                                                            .color(ACCENT),
                                                    );
                                                    ui.add_space(4.0);
                                                    // Calculate a truncated message
                                                    let msg = if commit.message.len() > 36 {
                                                        format!("{}…", &commit.message[..34])
                                                    } else {
                                                        commit.message.clone()
                                                    };

                                                    // Use horizontal layout but ensure date is on the right
                                                    ui.horizontal(|ui| {
                                                        ui.label(
                                                            RichText::new(&commit.hash)
                                                                .monospace()
                                                                .size(11.0)
                                                                .color(ACCENT),
                                                        );
                                                        ui.add_space(4.0);

                                                        // Give the text label flexible horizontal space by putting the date aligned right!
                                                        ui.with_layout(
                                                            egui::Layout::right_to_left(
                                                                egui::Align::Center,
                                                            ),
                                                            |ui| {
                                                                // Print the date on the far right
                                                                ui.label(
                                                                    RichText::new(&commit.date)
                                                                        .size(10.0)
                                                                        .color(TEXT_MUTED),
                                                                );

                                                                // The commit message takes up the rest of the available middle space
                                                                ui.with_layout(
                                                                    egui::Layout::left_to_right(
                                                                        egui::Align::Center,
                                                                    ),
                                                                    |ui| {
                                                                        if ui
                                                                            .selectable_label(
                                                                                false,
                                                                                RichText::new(msg)
                                                                                    .size(12.0)
                                                                                    .color(TEXT),
                                                                            )
                                                                            .clicked()
                                                                        {
                                                                            self.load_from_git(
                                                                                ctx,
                                                                                &path,
                                                                                &commit.hash,
                                                                                is_left,
                                                                            );
                                                                            ui.close();
                                                                        }
                                                                    },
                                                                );
                                                            },
                                                        );
                                                    });
                                                });
                                            }
                                        });
                                }
                            });
                        }

                        // delete button
                        let del_btn = egui::Button::new(
                            RichText::new(icon_str(Icon::X))
                                .size(11.0)
                                .color(TEXT_MUTED),
                        )
                        .fill(Color32::TRANSPARENT)
                        .stroke(Stroke::NONE);

                        if ui.add(del_btn).on_hover_text("Remove").clicked() {
                            if is_left {
                                self.texture_left = None;
                                self.image_left = None;
                                self.path_left = None;
                                self.commits_left.clear();
                            } else {
                                self.texture_right = None;
                                self.image_right = None;
                                self.path_right = None;
                                self.commits_right.clear();
                            }
                            self.texture_diff = None;
                            self.diff_pixel_pct = None;
                        }
                    });
                });
        }
    }
}

impl eframe::App for ImageDifferApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        // ── Load pending images (deferred from startup args) ──────────────────
        if self.pending_left.is_some() || self.pending_right.is_some() {
            if let Some(p) = self.pending_left.take() {
                self.load_image_to_texture(&ctx, p, true);
            }
            if let Some(p) = self.pending_right.take() {
                self.load_image_to_texture(&ctx, p, false);
            }
        }

        // ── Drag & Drop ──────────────────────────────────────────────────────
        ui.ctx().input(|i| {
            if !i.raw.dropped_files.is_empty() {
                let mut iter = i.raw.dropped_files.clone().into_iter();
                if let Some(f) = iter.next() {
                    if let Some(p) = f.path {
                        self.load_image_to_texture(&ctx, p, true);
                    }
                }
                if let Some(f) = iter.next() {
                    if let Some(p) = f.path {
                        self.load_image_to_texture(&ctx, p, false);
                    }
                }
            }
        });

        footer::render_footer(self, ui, &ctx);
        header::render_header(self, ui, &ctx);
        mode_toolbar::render_mode_toolbar(self, ui, &ctx);
        central_panel::render_central_panel(self, ui, &ctx);
    }
}
