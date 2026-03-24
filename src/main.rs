#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use eframe::epaint::{ColorImage, TextureHandle};
use egui::{FontData, FontDefinitions, FontFamily, FontId, RichText};
use egui_shadcn::{button, separator, ControlSize, ControlVariant, Label, SeparatorProps, Theme};
use lucide_icons::{Icon, LUCIDE_FONT_BYTES};
use std::path::PathBuf;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 800.0])
            .with_min_inner_size([600.0, 400.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Image Differ",
        native_options,
        Box::new(|cc| {
            ensure_lucide_icon_font(&cc.egui_ctx);
            Ok(Box::new(ImageDifferApp::new(cc)))
        }),
    )
}

fn ensure_lucide_icon_font(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "lucide".into(),
        FontData::from_static(LUCIDE_FONT_BYTES).into(),
    );
    fonts
        .families
        .entry(FontFamily::Name("lucide".into()))
        .or_default()
        .push("lucide".into());
    ctx.set_fonts(fonts);
}

fn icon_text(icon: Icon, size: f32) -> RichText {
    RichText::new(icon.unicode().to_string())
        .font(FontId::new(size, FontFamily::Name("lucide".into())))
}

struct ImageDifferApp {
    texture_left: Option<TextureHandle>,
    texture_right: Option<TextureHandle>,
    path_left: Option<PathBuf>,
    path_right: Option<PathBuf>,
    slider_pos: f32,
    error_msg: Option<String>,
    theme: Theme,
}

impl ImageDifferApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            texture_left: None,
            texture_right: None,
            path_left: None,
            path_right: None,
            slider_pos: 0.5,
            error_msg: None,
            theme: Theme::default(),
        }
    }

    fn load_image_to_texture(&mut self, ctx: &egui::Context, path: PathBuf, is_left: bool) {
        match image::open(&path) {
            Ok(dynamic_image) => {
                let size = [
                    dynamic_image.width() as usize,
                    dynamic_image.height() as usize,
                ];
                let image_buffer = dynamic_image.to_rgba8();
                let pixels = image_buffer.as_flat_samples();
                let color_image = ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());

                let name = path.file_name().unwrap().to_string_lossy();
                let handle = ctx.load_texture(name, color_image, Default::default());

                if is_left {
                    self.texture_left = Some(handle);
                    self.path_left = Some(path);
                } else {
                    self.texture_right = Some(handle);
                    self.path_right = Some(path);
                }
                self.error_msg = None;
            }
            Err(e) => {
                self.error_msg = Some(format!("Failed to load {}: {}", path.display(), e));
            }
        }
    }
}

impl eframe::App for ImageDifferApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_header").show(ctx, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.add_space(8.0);
                Label::new("Image Differ")
                    .size(ControlSize::Lg)
                    .show(ui, &self.theme);

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(8.0);

                    // --- Right Image Slot ---
                    if let Some(tex) = self.texture_right.clone() {
                        let trash_icon = icon_text(Icon::Trash2, 14.0);
                        if button(
                            ui,
                            &self.theme,
                            trash_icon,
                            ControlVariant::Destructive,
                            ControlSize::IconSm,
                            true,
                        )
                        .on_hover_text("Clear selection")
                        .clicked()
                        {
                            self.texture_right = None;
                            self.path_right = None;
                        }

                        ui.add_space(4.0);
                        let name = self
                            .path_right
                            .as_ref()
                            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
                            .unwrap_or_default();
                        ui.label(name);

                        ui.add_space(4.0);
                        ui.add(egui::Image::from_texture(&tex).max_size(egui::vec2(32.0, 32.0)));
                    } else {
                        let right_icon = icon_text(Icon::Image, 14.0);
                        if button(
                            ui,
                            &self.theme,
                            right_icon,
                            ControlVariant::Outline,
                            ControlSize::Sm,
                            true,
                        )
                        .on_hover_text("Load Right Image")
                        .clicked()
                        {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("Images", &["png", "jpg", "jpeg", "webp"])
                                .pick_file()
                            {
                                self.load_image_to_texture(ctx, path, false);
                            }
                        }
                    }

                    ui.add_space(16.0);
                    ui.separator();
                    ui.add_space(16.0);

                    // --- Left Image Slot ---
                    if let Some(tex) = self.texture_left.clone() {
                        let trash_icon = icon_text(Icon::Trash2, 14.0);
                        if button(
                            ui,
                            &self.theme,
                            trash_icon,
                            ControlVariant::Destructive,
                            ControlSize::IconSm,
                            true,
                        )
                        .on_hover_text("Clear selection")
                        .clicked()
                        {
                            self.texture_left = None;
                            self.path_left = None;
                        }

                        ui.add_space(4.0);
                        let name = self
                            .path_left
                            .as_ref()
                            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
                            .unwrap_or_default();
                        ui.label(name);

                        ui.add_space(4.0);
                        ui.add(egui::Image::from_texture(&tex).max_size(egui::vec2(32.0, 32.0)));
                    } else {
                        let left_icon = icon_text(Icon::Image, 14.0);
                        if button(
                            ui,
                            &self.theme,
                            left_icon,
                            ControlVariant::Outline,
                            ControlSize::Sm,
                            true,
                        )
                        .on_hover_text("Load Left Image")
                        .clicked()
                        {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("Images", &["png", "jpg", "jpeg", "webp"])
                                .pick_file()
                            {
                                self.load_image_to_texture(ctx, path, true);
                            }
                        }
                    }
                });
            });
            ui.add_space(8.0);
            separator(ui, &self.theme, SeparatorProps::default());
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(msg) = &self.error_msg {
                ui.colored_label(egui::Color32::RED, msg);
            }

            if let (Some(tex_left), Some(tex_right)) = (&self.texture_left, &self.texture_right) {
                let available_size = ui.available_size();
                let (rect, response) = ui.allocate_exact_size(available_size, egui::Sense::drag());

                // Draw Left Image
                ui.painter().image(
                    tex_left.id(),
                    rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );

                // Draw Right Image Clipped
                let swipe_x = rect.left() + rect.width() * self.slider_pos;
                let clipped_rect = egui::Rect::from_min_max(
                    egui::pos2(swipe_x, rect.top()),
                    egui::pos2(rect.right(), rect.bottom()),
                );
                let painter = ui.painter_at(clipped_rect);
                painter.image(
                    tex_right.id(),
                    rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );

                // Draw Slider Handle
                ui.painter().line_segment(
                    [
                        egui::pos2(swipe_x, rect.top()),
                        egui::pos2(swipe_x, rect.bottom()),
                    ],
                    egui::Stroke::new(2.0, egui::Color32::WHITE),
                );

                if response.dragged() {
                    if let Some(pointer_pos) = response.interact_pointer_pos() {
                        let rel_x = pointer_pos.x - rect.left();
                        self.slider_pos = (rel_x / rect.width()).clamp(0.0, 1.0);
                    }
                }

                if response.hovered() && (response.hover_pos().unwrap().x - swipe_x).abs() < 20.0 {
                    ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::ResizeHorizontal);
                }
            } else {
                ui.centered_and_justified(|ui| {
                    ui.vertical_centered(|ui| {
                        Label::new("Ready to compare")
                            .size(ControlSize::Lg)
                            .show(ui, &self.theme);
                        ui.add_space(10.0);
                        Label::new("Load two images using the buttons above to begin.")
                            .size(ControlSize::Sm)
                            .show(ui, &self.theme);

                        ui.add_space(20.0);

                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 20.0;
                            let left_loaded = self.texture_left.is_some();
                            let right_loaded = self.texture_right.is_some();

                            if button(
                                ui,
                                &self.theme,
                                if left_loaded {
                                    "Image 1 Loaded ✓"
                                } else {
                                    "Select Image 1"
                                },
                                if left_loaded {
                                    ControlVariant::Secondary
                                } else {
                                    ControlVariant::Outline
                                },
                                ControlSize::Md,
                                true,
                            )
                            .clicked()
                            {
                                if let Some(path) = rfd::FileDialog::new().pick_file() {
                                    self.load_image_to_texture(ctx, path, true);
                                }
                            }

                            if button(
                                ui,
                                &self.theme,
                                if right_loaded {
                                    "Image 2 Loaded ✓"
                                } else {
                                    "Select Image 2"
                                },
                                if right_loaded {
                                    ControlVariant::Secondary
                                } else {
                                    ControlVariant::Outline
                                },
                                ControlSize::Md,
                                true,
                            )
                            .clicked()
                            {
                                if let Some(path) = rfd::FileDialog::new().pick_file() {
                                    self.load_image_to_texture(ctx, path, false);
                                }
                            }
                        });
                    });
                });
            }
        });
    }
}
