use eframe::egui;
use egui::{Color32, FontFamily, FontId, RichText, Stroke};
use lucide_icons::Icon;

use crate::app::ImageDifferApp;
use crate::theme::{
    ACCENT, BG_CARD, BG_DEEP, BG_ELEVATED, BORDER, TEXT, TEXT_DIM, TEXT_MUTED, calc_image_rect,
    icon_str, icon_text,
};
use crate::types::DiffMode;

pub fn render_central_panel(app: &mut ImageDifferApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    let has_both = app.texture_left.is_some() && app.texture_right.is_some();
    let has_one = app.texture_left.is_some() || app.texture_right.is_some();

    if has_both || has_one {
        if ui.ctx().input(|i| {
            i.pointer.button_down(egui::PointerButton::Secondary)
                || i.pointer.button_down(egui::PointerButton::Middle)
        }) {
            app.pan_offset += ui.ctx().input(|i| i.pointer.delta());
        }
    }

    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.fill(BG_DEEP))
        .show_inside(ui, |ui| {
            if has_both {
                // Clone handles up front to avoid borrow conflicts
                let (Some(tex_left), Some(tex_right)) =
                    (app.texture_left.as_ref(), app.texture_right.as_ref())
                else {
                    return;
                };

                match app.diff_mode {
                    DiffMode::Slider => {
                        let avail = ui.available_size();
                        let (rect, resp) = ui.allocate_exact_size(avail, egui::Sense::drag());
                        let p = ui.painter();

                        let img_rect = calc_image_rect(
                            tex_left.size_vec2(),
                            rect,
                            app.zoom_level,
                            app.pan_offset,
                        );
                        p.image(
                            tex_left.id(),
                            img_rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            Color32::WHITE,
                        );
                        let sx = rect.left() + rect.width() * app.slider_pos;
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
                                    Color32::from_rgba_unmultiplied(0x06, 0xb6, 0xd4, glow * 18),
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
                        p.circle_filled(hc, 20.0, crate::theme::BG_SURFACE);
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
                                app.slider_pos =
                                    ((ptr.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
                            }
                        }
                        if ui.rect_contains_pointer(egui::Rect::from_center_size(
                            hc,
                            egui::vec2(48.0, 48.0),
                        )) {
                            ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::ResizeHorizontal);
                        }
                    }

                    DiffMode::SideBySide => {
                        let avail = ui.available_size();
                        let half = (avail.x * 0.5).floor();

                        let left_rect =
                            egui::Rect::from_min_size(ui.cursor().min, egui::vec2(half, avail.y));
                        let right_rect = egui::Rect::from_min_size(
                            egui::pos2(left_rect.right() + 1.0, left_rect.top()),
                            egui::vec2(avail.x - half - 1.0, avail.y),
                        );

                        let (_, _) = ui.allocate_exact_size(avail, egui::Sense::hover());
                        let p = ui.painter();

                        let disp_l = calc_image_rect(
                            tex_left.size_vec2(),
                            left_rect,
                            app.zoom_level,
                            app.pan_offset,
                        );
                        let disp_r = calc_image_rect(
                            tex_right.size_vec2(),
                            right_rect,
                            app.zoom_level,
                            app.pan_offset,
                        );

                        // Checkerboard bg (simple solid for now)
                        p.rect_filled(left_rect, 0.0, BG_DEEP);
                        p.rect_filled(right_rect, 0.0, BG_DEEP);

                        p.image(
                            tex_left.id(),
                            disp_l,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            Color32::WHITE,
                        );
                        p.image(
                            tex_right.id(),
                            disp_r,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
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
                        app.generate_diff_texture(ctx);
                        let avail = ui.available_size();
                        let pos = ui.cursor().min;
                        let (_, _) = ui.allocate_exact_size(avail, egui::Sense::hover());
                        let p = ui.painter();

                        if let Some(ref tex_diff) = app.texture_diff {
                            let container = egui::Rect::from_min_size(pos, avail);
                            let disp = calc_image_rect(
                                tex_diff.size_vec2(),
                                container,
                                app.zoom_level,
                                app.pan_offset,
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
                            app.zoom_level,
                            app.pan_offset,
                        );
                        p.image(
                            tex_left.id(),
                            disp,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            Color32::WHITE,
                        );
                        p.image(
                            tex_right.id(),
                            disp,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            Color32::WHITE.gamma_multiply(app.fade_opacity),
                        );
                    }
                }
            } else if let Some(path) = app.path_left.clone().or(app.path_right.clone()) {
                // One image loaded
                if app.is_in_git_repo(&path) {
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
                                RichText::new("Select a revision to load alongside your image.")
                                    .size(13.0)
                                    .color(TEXT_MUTED),
                            );
                            ui.add_space(24.0);

                            let commits = if app.path_left.is_some() {
                                if app.commits_left.is_empty() {
                                    app.commits_left = app.get_git_history(&path);
                                }
                                app.commits_left.clone()
                            } else {
                                if app.commits_right.is_empty() {
                                    app.commits_right = app.get_git_history(&path);
                                }
                                app.commits_right.clone()
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
                                                            egui::Separator::default().spacing(0.0),
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
                                                            format!("{}…", &commit.message[..46])
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
                                                            app.texture_left.is_none();
                                                        app.load_from_git(
                                                            ctx,
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
                                if let Some(p) = rfd::FileDialog::new()
                                    .set_directory(".")
                                    .add_filter(
                                        "Images",
                                        &["png", "jpg", "jpeg", "webp", "bmp", "gif"],
                                    )
                                    .pick_file()
                                {
                                    let is_left_empty = app.texture_left.is_none();
                                    app.load_image_to_texture(ctx, p, is_left_empty);
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
                            let l = app.texture_left.is_some();
                            let btn = egui::Button::new(
                                RichText::new(format!("{} Browse File", icon_str(Icon::ImagePlus)))
                                    .size(13.0)
                                    .color(ACCENT),
                            )
                            .fill(ACCENT.gamma_multiply(0.1))
                            .stroke(Stroke::new(1.0, ACCENT.gamma_multiply(0.4)))
                            .corner_radius(egui::CornerRadius::same(8u8))
                            .min_size(egui::vec2(180.0, 44.0));
                            if ui.add(btn).clicked() {
                                if let Some(p) = rfd::FileDialog::new()
                                    .set_directory(".")
                                    .add_filter(
                                        "Images",
                                        &["png", "jpg", "jpeg", "webp", "bmp", "gif"],
                                    )
                                    .pick_file()
                                {
                                    let is_left_empty = !l;
                                    app.load_image_to_texture(ctx, p, is_left_empty);
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
                            RichText::new("Or choose files below — supports PNG, JPG, WebP, BMP")
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
                                        if let Some(paths) = rfd::FileDialog::new()
                                            .set_directory(std::env::current_dir().unwrap())
                                            .add_filter(
                                                "Images",
                                                &["png", "jpg", "jpeg", "webp", "bmp", "gif"],
                                            )
                                            .pick_files()
                                        {
                                            if let Some(path) = paths.iter().next() {
                                                app.load_image_to_texture(
                                                    ctx,
                                                    path.to_path_buf(),
                                                    true,
                                                );
                                            }

                                            if let Some(path) = paths.iter().next() {
                                                app.load_image_to_texture(
                                                    ctx,
                                                    path.to_path_buf(),
                                                    false,
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
