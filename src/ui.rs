use eframe::egui;
use egui::{Color32, FontFamily, FontId, RichText, Stroke};
use lucide_icons::Icon;

use crate::app::ImageDifferApp;
use crate::theme::{
    ACCENT, BG_CARD, BG_DEEP, BG_ELEVATED, BG_SURFACE, BORDER, DANGER, SUCCESS, TEXT, TEXT_DIM,
    TEXT_MUTED, calc_image_rect, icon_str, icon_text,
};
use crate::types::DiffMode;

impl ImageDifferApp {
    fn render_slot_header(
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

        let has_both = self.texture_left.is_some() && self.texture_right.is_some();
        let has_one = self.texture_left.is_some() || self.texture_right.is_some();

        // ── Drag & Drop ──────────────────────────────────────────────────────
        ctx.input(|i| {
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

        // ── Footer ───────────────────────────────────────────────────────────
        egui::Panel::bottom("footer")
            .frame(
                egui::Frame::NONE
                    .fill(BG_DEEP)
                    .stroke(Stroke::new(1.0, BORDER)),
            )
            .show_inside(ui, |ui| {
                ui.set_height(24.0);
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.add_space(12.0);
                    if let (Some(l), Some(r)) = (&self.image_left, &self.image_right) {
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
                        let ztxt = if self.zoom_level <= 0.0 {
                            "Fit".to_string()
                        } else {
                            format!("{:.0}%", self.zoom_level * 100.0)
                        };
                        if ui
                            .add(
                                egui::Button::new(RichText::new("1:1").size(10.0).color(TEXT))
                                    .fill(Color32::TRANSPARENT)
                                    .stroke(Stroke::NONE),
                            )
                            .clicked()
                        {
                            self.zoom_level = 1.0;
                            self.pan_offset = egui::Vec2::ZERO;
                        }
                        if ui
                            .add(
                                egui::Button::new(RichText::new("Fit").size(10.0).color(TEXT))
                                    .fill(Color32::TRANSPARENT)
                                    .stroke(Stroke::NONE),
                            )
                            .clicked()
                        {
                            self.zoom_level = 0.0;
                            self.pan_offset = egui::Vec2::ZERO;
                        }
                        ui.add(
                            egui::Slider::new(&mut self.zoom_level, 0.0..=5.0).show_value(false),
                        );
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
                        if !has_one && self.git_context.is_none() {
                            ui.add_space(16.0);
                            let hint = "idiffer --install  to register as git difftool";
                            ui.label(RichText::new(hint).size(9.0).color(TEXT_DIM).monospace());
                        }
                    });
                });
            });

        // ── Header ───────────────────────────────────────────────────────────
        egui::Panel::top("header")
            .exact_size(52.0)
            .frame(
                egui::Frame::NONE
                    .fill(BG_SURFACE)
                    .stroke(Stroke::new(1.0, BORDER)),
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
                    let (dvr, _) =
                        ui.allocate_exact_size(egui::vec2(1.0, 28.0), egui::Sense::hover());
                    ui.painter().rect_filled(dvr, 0.0, BORDER);
                    ui.add_space(16.0);

                    // Git context chip (shown when launched from git)
                    if let Some(ref gc) = self.git_context.clone() {
                        let (dvr2, _) =
                            ui.allocate_exact_size(egui::vec2(1.0, 28.0), egui::Sense::hover());
                        ui.painter().rect_filled(dvr2, 0.0, BORDER);
                        ui.add_space(12.0);

                        egui::Frame::NONE
                            .fill(SUCCESS.gamma_multiply(0.08))
                            .corner_radius(egui::CornerRadius::same(6u8))
                            .stroke(Stroke::new(1.0, SUCCESS.gamma_multiply(0.3)))
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
                                            .font(FontId::new(
                                                12.0,
                                                FontFamily::Name("lucide".into()),
                                            )),
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
                    self.render_slot_header(ui, true, "Original", &ctx);
                    if has_both {
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new(icon_str(Icon::ArrowLeftRight))
                                .size(14.0)
                                .color(TEXT_DIM)
                                .font(FontId::new(14.0, FontFamily::Name("lucide".into()))),
                        );
                        ui.add_space(8.0);
                    }
                    self.render_slot_header(ui, false, "New", &ctx);

                    // Right side actions
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(16.0);

                        // Clear
                        if has_one {
                            ui.add_space(8.0);
                            let clr_btn = egui::Button::new(
                                RichText::new("Clear").size(12.0).color(TEXT_MUTED),
                            )
                            .fill(Color32::TRANSPARENT)
                            .corner_radius(egui::CornerRadius::same(6u8))
                            .stroke(Stroke::new(1.0, BORDER));
                            if ui.add(clr_btn).clicked() {
                                self.clear_all();
                            }
                        }

                        // Error
                        if let Some(ref msg) = self.error_msg.clone() {
                            ui.add_space(8.0);
                            ui.label(
                                RichText::new(format!("{} {}", icon_str(Icon::CircleAlert), msg))
                                    .size(11.0)
                                    .color(DANGER),
                            );
                        }
                    });
                });
            });

        // ── Mode Toolbar (only when both images loaded) ───────────────────────
        if has_both {
            egui::Panel::top("mode_toolbar")
                .exact_size(40.0)
                .frame(
                    egui::Frame::NONE
                        .fill(BG_DEEP)
                        .stroke(Stroke::new(1.0, BORDER)),
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
                            let active = self.diff_mode == mode;
                            let (fill, color, stroke) = if active {
                                (
                                    ACCENT.gamma_multiply(0.12),
                                    ACCENT,
                                    Stroke::new(1.0, ACCENT.gamma_multiply(0.35)),
                                )
                            } else {
                                (Color32::TRANSPARENT, TEXT_MUTED, Stroke::NONE)
                            };
                            let tab_label = format!("{} {}", icon_str(icon), label);
                            let resp = ui.add(
                                egui::Button::new(RichText::new(tab_label).size(12.0).color(color))
                                    .fill(fill)
                                    .stroke(stroke)
                                    .corner_radius(egui::CornerRadius::same(6u8)),
                            );
                            if resp.clicked() {
                                self.diff_mode = mode;
                            }
                            ui.add_space(2.0);
                        }

                        // Inline Fade slider
                        if self.diff_mode == DiffMode::Fade {
                            ui.add_space(16.0);
                            let (sep, _) =
                                ui.allocate_exact_size(egui::vec2(1.0, 20.0), egui::Sense::hover());
                            ui.painter().rect_filled(sep, 0.0, BORDER);
                            ui.add_space(12.0);
                            ui.label(RichText::new("Opacity").size(11.0).color(TEXT_MUTED));
                            ui.add_space(6.0);
                            ui.add(
                                egui::Slider::new(&mut self.fade_opacity, 0.0..=1.0)
                                    .show_value(false),
                            );
                        }

                        // Diff % badge in toolbar when Difference mode
                        if self.diff_mode == DiffMode::Difference {
                            if let Some(pct) = self.diff_pixel_pct {
                                ui.add_space(16.0);
                                let (sep, _) = ui.allocate_exact_size(
                                    egui::vec2(1.0, 20.0),
                                    egui::Sense::hover(),
                                );
                                ui.painter().rect_filled(sep, 0.0, BORDER);
                                ui.add_space(12.0);
                                let color = if pct < 1.0 { SUCCESS } else { DANGER };
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

        // ── Central Panel ─────────────────────────────────────────────────────
        if has_both || has_one {
            ctx.input(|i| {
                if i.pointer.button_down(egui::PointerButton::Secondary)
                    || i.pointer.button_down(egui::PointerButton::Middle)
                {
                    self.pan_offset += i.pointer.delta();
                }
            });
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(BG_DEEP))
            .show_inside(ui, |ui| {
                if has_both {
                    // Clone handles up front to avoid borrow conflicts
                    let (Some(tex_left), Some(tex_right)) =
                        (self.texture_left.as_ref(), self.texture_right.as_ref())
                    else {
                        return;
                    };

                    match self.diff_mode {
                        DiffMode::Slider => {
                            let avail = ui.available_size();
                            let (rect, resp) = ui.allocate_exact_size(avail, egui::Sense::drag());
                            let p = ui.painter();

                            let img_rect = calc_image_rect(
                                tex_left.size_vec2(),
                                rect,
                                self.zoom_level,
                                self.pan_offset,
                            );
                            p.image(
                                tex_left.id(),
                                img_rect,
                                egui::Rect::from_min_max(
                                    egui::pos2(0.0, 0.0),
                                    egui::pos2(1.0, 1.0),
                                ),
                                Color32::WHITE,
                            );
                            let sx = rect.left() + rect.width() * self.slider_pos;
                            let clip = img_rect.intersect(egui::Rect::from_min_max(
                                egui::pos2(sx, f32::MIN),
                                egui::pos2(f32::MAX, f32::MAX),
                            ));
                            if clip.is_positive() {
                                ui.painter_at(clip).image(
                                    tex_right.id(),
                                    img_rect,
                                    egui::Rect::from_min_max(
                                        egui::pos2(0.0, 0.0),
                                        egui::pos2(1.0, 1.0),
                                    ),
                                    Color32::WHITE,
                                );
                            }
                            let p = ui.painter();

                            // Corner labels
                            let lbg = Color32::from_black_alpha(160);
                            let _pad = egui::vec2(8.0, 4.0);
                            let lbl_y = rect.top() + 12.0;
                            // ORIGINAL
                            let orig_r = egui::Rect::from_min_size(
                                egui::pos2(rect.left() + 12.0, lbl_y),
                                egui::vec2(68.0, 18.0),
                            );
                            p.rect_filled(orig_r, 4.0, lbg);
                            p.text(
                                orig_r.center(),
                                egui::Align2::CENTER_CENTER,
                                "ORIGINAL",
                                egui::FontId::monospace(10.0),
                                TEXT_MUTED,
                            );
                            // NEW
                            let new_r = egui::Rect::from_min_size(
                                egui::pos2(rect.right() - 44.0, lbl_y),
                                egui::vec2(36.0, 18.0),
                            );
                            p.rect_filled(new_r, 4.0, lbg);
                            p.text(
                                new_r.center(),
                                egui::Align2::CENTER_CENTER,
                                "NEW",
                                egui::FontId::monospace(10.0),
                                ACCENT,
                            );

                            // Glowing divider line
                            for glow in [6u8, 4, 2] {
                                let w = glow as f32 * 0.8;
                                p.line_segment(
                                    [egui::pos2(sx, rect.top()), egui::pos2(sx, rect.bottom())],
                                    Stroke::new(
                                        w,
                                        Color32::from_rgba_unmultiplied(
                                            0x06,
                                            0xb6,
                                            0xd4,
                                            glow * 18,
                                        ),
                                    ),
                                );
                            }
                            p.line_segment(
                                [egui::pos2(sx, rect.top()), egui::pos2(sx, rect.bottom())],
                                Stroke::new(1.5, ACCENT),
                            );

                            // Handle
                            let hc = egui::pos2(sx, rect.center().y);
                            p.circle_stroke(hc, 26.0, Stroke::new(1.0, ACCENT.gamma_multiply(0.2)));
                            p.circle_filled(hc, 20.0, BG_SURFACE);
                            p.circle_stroke(hc, 20.0, Stroke::new(1.5, ACCENT));
                            p.text(
                                hc,
                                egui::Align2::CENTER_CENTER,
                                icon_str(Icon::ChevronsLeftRight),
                                FontId::new(16.0, FontFamily::Name("lucide".into())),
                                ACCENT,
                            );

                            // Drag
                            if resp.dragged() {
                                if let Some(ptr) = ctx.input(|i| i.pointer.latest_pos()) {
                                    self.slider_pos =
                                        ((ptr.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
                                }
                            }
                            if ui.rect_contains_pointer(egui::Rect::from_center_size(
                                hc,
                                egui::vec2(48.0, 48.0),
                            )) {
                                ui.output_mut(|o| {
                                    o.cursor_icon = egui::CursorIcon::ResizeHorizontal
                                });
                            }
                        }

                        DiffMode::SideBySide => {
                            let avail = ui.available_size();
                            let half = (avail.x * 0.5).floor();

                            let left_rect = egui::Rect::from_min_size(
                                ui.cursor().min,
                                egui::vec2(half, avail.y),
                            );
                            let right_rect = egui::Rect::from_min_size(
                                egui::pos2(left_rect.right() + 1.0, left_rect.top()),
                                egui::vec2(avail.x - half - 1.0, avail.y),
                            );

                            let (_, _) = ui.allocate_exact_size(avail, egui::Sense::hover());
                            let p = ui.painter();

                            let disp_l = calc_image_rect(
                                tex_left.size_vec2(),
                                left_rect,
                                self.zoom_level,
                                self.pan_offset,
                            );
                            let disp_r = calc_image_rect(
                                tex_right.size_vec2(),
                                right_rect,
                                self.zoom_level,
                                self.pan_offset,
                            );

                            // Checkerboard bg (simple solid for now)
                            p.rect_filled(left_rect, 0.0, BG_DEEP);
                            p.rect_filled(right_rect, 0.0, BG_DEEP);

                            p.image(
                                tex_left.id(),
                                disp_l,
                                egui::Rect::from_min_max(
                                    egui::pos2(0.0, 0.0),
                                    egui::pos2(1.0, 1.0),
                                ),
                                Color32::WHITE,
                            );
                            p.image(
                                tex_right.id(),
                                disp_r,
                                egui::Rect::from_min_max(
                                    egui::pos2(0.0, 0.0),
                                    egui::pos2(1.0, 1.0),
                                ),
                                Color32::WHITE,
                            );

                            // Center divider
                            p.rect_filled(
                                egui::Rect::from_min_size(
                                    egui::pos2(left_rect.right(), left_rect.top()),
                                    egui::vec2(1.0, avail.y),
                                ),
                                0.0,
                                BORDER,
                            );

                            // Corner labels
                            let lbg = Color32::from_black_alpha(180);
                            let ly = left_rect.top() + 10.0;
                            let orig_r = egui::Rect::from_min_size(
                                egui::pos2(left_rect.left() + 10.0, ly),
                                egui::vec2(68.0, 18.0),
                            );
                            p.rect_filled(orig_r, 4.0, lbg);
                            p.text(
                                orig_r.center(),
                                egui::Align2::CENTER_CENTER,
                                "ORIGINAL",
                                egui::FontId::monospace(10.0),
                                TEXT_MUTED,
                            );

                            let new_r = egui::Rect::from_min_size(
                                egui::pos2(right_rect.left() + 10.0, ly),
                                egui::vec2(36.0, 18.0),
                            );
                            p.rect_filled(new_r, 4.0, lbg);
                            p.text(
                                new_r.center(),
                                egui::Align2::CENTER_CENTER,
                                "NEW",
                                egui::FontId::monospace(10.0),
                                ACCENT,
                            );
                        }

                        DiffMode::Difference => {
                            self.generate_diff_texture(&ctx);
                            let avail = ui.available_size();
                            let pos = ui.cursor().min;
                            let (_, _) = ui.allocate_exact_size(avail, egui::Sense::hover());
                            let p = ui.painter();

                            if let Some(ref tex_diff) = self.texture_diff {
                                let container = egui::Rect::from_min_size(pos, avail);
                                let disp = calc_image_rect(
                                    tex_diff.size_vec2(),
                                    container,
                                    self.zoom_level,
                                    self.pan_offset,
                                );
                                p.image(
                                    tex_diff.id(),
                                    disp,
                                    egui::Rect::from_min_max(
                                        egui::pos2(0.0, 0.0),
                                        egui::pos2(1.0, 1.0),
                                    ),
                                    Color32::WHITE,
                                );
                            } else {
                                p.text(
                                    egui::Rect::from_min_size(pos, avail).center(),
                                    egui::Align2::CENTER_CENTER,
                                    "Computing diff…",
                                    egui::FontId::monospace(12.0),
                                    TEXT_MUTED,
                                );
                            }
                        }

                        DiffMode::Fade => {
                            let avail = ui.available_size();
                            let pos = ui.cursor().min;
                            let container = egui::Rect::from_min_size(pos, avail);
                            let (_, _) = ui.allocate_exact_size(avail, egui::Sense::hover());
                            let p = ui.painter();
                            let disp = calc_image_rect(
                                tex_left.size_vec2(),
                                container,
                                self.zoom_level,
                                self.pan_offset,
                            );
                            p.image(
                                tex_left.id(),
                                disp,
                                egui::Rect::from_min_max(
                                    egui::pos2(0.0, 0.0),
                                    egui::pos2(1.0, 1.0),
                                ),
                                Color32::WHITE,
                            );
                            p.image(
                                tex_right.id(),
                                disp,
                                egui::Rect::from_min_max(
                                    egui::pos2(0.0, 0.0),
                                    egui::pos2(1.0, 1.0),
                                ),
                                Color32::WHITE.gamma_multiply(self.fade_opacity),
                            );
                        }
                    }
                } else if let Some(path) = self.path_left.clone().or(self.path_right.clone()) {
                    // One image loaded
                    if self.is_in_git_repo(&path) {
                        // Git history browser
                        ui.centered_and_justified(|ui| {
                            ui.vertical_centered(|ui| {
                                ui.add_space(40.0);
                                ui.label(icon_text(Icon::GitBranch, 40.0).color(ACCENT));
                                ui.add_space(16.0);
                                ui.label(
                                    RichText::new("Compare with Git History")
                                        .size(18.0)
                                        .color(TEXT)
                                        .strong(),
                                );
                                ui.add_space(6.0);
                                ui.label(
                                    RichText::new(
                                        "Select a revision to load alongside your image.",
                                    )
                                    .size(13.0)
                                    .color(TEXT_MUTED),
                                );
                                ui.add_space(24.0);

                                let commits = if self.path_left.is_some() {
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
                                        RichText::new("No commits found for this file.")
                                            .size(12.0)
                                            .color(TEXT_MUTED),
                                    );
                                } else {
                                    egui::Frame::NONE
                                        .fill(BG_CARD)
                                        .corner_radius(egui::CornerRadius::same(10))
                                        .stroke(Stroke::new(1.0, BORDER))
                                        .inner_margin(egui::Margin::same(8))
                                        .show(ui, |ui| {
                                            ui.set_width(560.0);
                                            egui::ScrollArea::vertical().max_height(380.0).show(
                                                ui,
                                                |ui| {
                                                    for (i, commit) in commits.iter().enumerate() {
                                                        if i > 0 {
                                                            ui.add(
                                                                egui::Separator::default()
                                                                    .spacing(0.0),
                                                            );
                                                        }
                                                        let row_resp = ui.horizontal(|ui| {
                                                            ui.set_min_height(40.0);
                                                            ui.add_space(8.0);
                                                            ui.label(
                                                                RichText::new(&commit.hash)
                                                                    .monospace()
                                                                    .size(12.0)
                                                                    .color(ACCENT),
                                                            );
                                                            ui.add_space(10.0);
                                                            let msg = if commit.message.len() > 48 {
                                                                format!(
                                                                    "{}…",
                                                                    &commit.message[..46]
                                                                )
                                                            } else {
                                                                commit.message.clone()
                                                            };
                                                            ui.label(
                                                                RichText::new(msg)
                                                                    .size(13.0)
                                                                    .color(TEXT),
                                                            );
                                                            ui.with_layout(
                                                                egui::Layout::right_to_left(
                                                                    egui::Align::Center,
                                                                ),
                                                                |ui| {
                                                                    ui.add_space(8.0);
                                                                    ui.label(
                                                                        RichText::new(&commit.date)
                                                                            .size(11.0)
                                                                            .color(TEXT_MUTED)
                                                                            .monospace(),
                                                                    );
                                                                },
                                                            );
                                                        });

                                                        let r = row_resp.response.rect;
                                                        let hovered = ui.rect_contains_pointer(r);
                                                        if hovered {
                                                            ui.painter().rect_filled(
                                                                r,
                                                                6.0,
                                                                ACCENT.gamma_multiply(0.08),
                                                            );
                                                            ui.output_mut(|o| {
                                                                o.cursor_icon =
                                                                    egui::CursorIcon::PointingHand
                                                            });
                                                        }
                                                        if ui
                                                            .interact(
                                                                r,
                                                                ui.id().with(i),
                                                                egui::Sense::click(),
                                                            )
                                                            .clicked()
                                                        {
                                                            let is_left_empty =
                                                                self.texture_left.is_none();
                                                            self.load_from_git(
                                                                &ctx,
                                                                &path,
                                                                &commit.hash,
                                                                is_left_empty,
                                                            );
                                                        }
                                                    }
                                                },
                                            );
                                        });
                                }

                                ui.add_space(20.0);
                                let local_btn = egui::Button::new(
                                    RichText::new(format!(
                                        "{} Browse local file instead",
                                        icon_str(Icon::FolderOpen)
                                    ))
                                    .size(12.0)
                                    .color(TEXT_MUTED),
                                )
                                .fill(Color32::TRANSPARENT)
                                .stroke(Stroke::new(1.0, BORDER))
                                .corner_radius(egui::CornerRadius::same(6u8));
                                if ui.add(local_btn).clicked() {
                                    if let Some(p) = crate::utils::pick_image_file() {
                                        let is_left_empty = self.texture_left.is_none();
                                        self.load_image_to_texture(&ctx, p, is_left_empty);
                                    }
                                }
                            });
                        });
                    } else {
                        // Non-git: show "load second" state
                        ui.centered_and_justified(|ui| {
                            ui.vertical_centered(|ui| {
                                ui.add_space(40.0);
                                ui.label(icon_text(Icon::ImagePlus, 40.0).color(ACCENT));
                                ui.add_space(16.0);
                                ui.label(
                                    RichText::new("Load the Second Image")
                                        .size(18.0)
                                        .color(TEXT)
                                        .strong(),
                                );
                                ui.add_space(6.0);
                                ui.label(
                                    RichText::new(
                                        "One image is loaded. Add another to start comparing.",
                                    )
                                    .size(13.0)
                                    .color(TEXT_MUTED),
                                );
                                ui.add_space(28.0);
                                let l = self.texture_left.is_some();
                                let _r = self.texture_right.is_some();
                                let btn = egui::Button::new(
                                    RichText::new(format!(
                                        "{} Browse File",
                                        icon_str(Icon::ImagePlus)
                                    ))
                                    .size(13.0)
                                    .color(ACCENT),
                                )
                                .fill(ACCENT.gamma_multiply(0.1))
                                .stroke(Stroke::new(1.0, ACCENT.gamma_multiply(0.4)))
                                .corner_radius(egui::CornerRadius::same(8u8))
                                .min_size(egui::vec2(180.0, 44.0));
                                if ui.add(btn).clicked() {
                                    if let Some(p) = crate::utils::pick_image_file() {
                                        let is_left_empty = !l;
                                        self.load_image_to_texture(&ctx, p, is_left_empty);
                                    }
                                }
                            });
                        });
                    }
                } else {
                    // ── Empty State ─────────────────────────────────────────
                    let _avail = ui.available_size();
                    ui.centered_and_justified(|ui| {
                        ui.vertical_centered(|ui| {
                            ui.add_space(32.0);
                            ui.label(icon_text(Icon::ImagePlay, 48.0));
                            ui.add_space(20.0);
                            ui.label(
                                RichText::new("Drop images to compare")
                                    .size(22.0)
                                    .color(TEXT)
                                    .strong(),
                            );
                            ui.add_space(8.0);
                            ui.label(
                                RichText::new(
                                    "Or choose files below — supports PNG, JPG, WebP, BMP",
                                )
                                .size(13.0)
                                .color(TEXT_MUTED),
                            );
                            ui.add_space(40.0);

                            let card_size = egui::vec2(280.0, 200.0);
                            egui::Frame::NONE
                                .fill(BG_CARD)
                                .corner_radius(egui::CornerRadius::same(12))
                                .stroke(Stroke::new(1.0, BORDER))
                                .inner_margin(egui::Margin::same(24))
                                .show(ui, |ui| {
                                    ui.set_min_size(card_size);
                                    ui.vertical_centered(|ui| {
                                        ui.add_space(24.0);
                                        ui.label(icon_text(Icon::FolderOpen, 36.0).color(TEXT_DIM));
                                        ui.add_space(16.0);
                                        ui.label(
                                            RichText::new("Open Images")
                                                .size(14.0)
                                                .color(TEXT)
                                                .strong(),
                                        );
                                        ui.add_space(4.0);
                                        ui.label(
                                            RichText::new("Select one or two files")
                                                .size(12.0)
                                                .color(TEXT_MUTED),
                                        );
                                        ui.add_space(20.0);
                                        let browse_btn = egui::Button::new(
                                            RichText::new("Browse Files").size(12.0).color(TEXT),
                                        )
                                        .fill(BG_ELEVATED)
                                        .stroke(Stroke::new(1.0, BORDER))
                                        .corner_radius(egui::CornerRadius::same(6u8))
                                        .min_size(egui::vec2(140.0, 32.0));

                                        if ui.add(browse_btn).clicked() {
                                            match crate::utils::pick_image_files() {
                                                Some(paths) if paths.len() == 2 => {
                                                    let a = &paths[0];
                                                    let b = &paths[1];
                                                    self.load_image_to_texture(
                                                        &ctx,
                                                        a.to_path_buf(),
                                                        true,
                                                    );
                                                    self.load_image_to_texture(
                                                        &ctx,
                                                        b.to_path_buf(),
                                                        false,
                                                    );
                                                }
                                                _ => {
                                                    self.error_msg = Some(
                                                        "Please select exactly two images."
                                                            .to_string(),
                                                    );
                                                }
                                            };
                                        }
                                    });
                                });
                        });
                    });
                }
            });
    }
}
