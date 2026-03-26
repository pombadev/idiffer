#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use eframe::epaint::{ColorImage, TextureHandle};
use egui::{FontData, FontDefinitions, FontFamily, FontId, RichText};
use egui_shadcn::{ControlSize, ControlVariant, Label, SeparatorProps, Theme, button, separator};
use lucide_icons::{Icon, LUCIDE_FONT_BYTES};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone, Debug)]
struct GitCommit {
    hash: String,
    message: String,
    date: String,
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DiffMode {
    Slider,
    SideBySide,
    Difference,
    Fade,
}

struct ImageDifferApp {
    texture_left: Option<TextureHandle>,
    texture_right: Option<TextureHandle>,
    texture_diff: Option<TextureHandle>,

    // We store the raw image data for diffing
    image_left: Option<image::RgbaImage>,
    image_right: Option<image::RgbaImage>,

    path_left: Option<PathBuf>,
    path_right: Option<PathBuf>,
    slider_pos: f32,
    fade_opacity: f32,
    diff_mode: DiffMode,
    error_msg: Option<String>,
    theme: Theme,

    // Git integration
    commits_left: Vec<GitCommit>,
    commits_right: Vec<GitCommit>,
}

impl ImageDifferApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            texture_left: None,
            texture_right: None,
            texture_diff: None,
            image_left: None,
            image_right: None,
            path_left: None,
            path_right: None,
            slider_pos: 0.5,
            fade_opacity: 0.5,
            diff_mode: DiffMode::Slider,
            error_msg: None,
            theme: Theme::default(),
            commits_left: Vec::new(),
            commits_right: Vec::new(),
        }
    }

    fn get_git_history(&self, path: &Path) -> Vec<GitCommit> {
        let output = Command::new("git")
            .arg("log")
            .arg("--pretty=format:%h|%s|%cd")
            .arg("--date=short")
            // .arg("-n")
            // .arg("10")
            .arg("--")
            .arg(path)
            .output();

        match output {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                stdout
                    .lines()
                    .filter_map(|line| {
                        let parts: Vec<&str> = line.split('|').collect();
                        if parts.len() >= 3 {
                            Some(GitCommit {
                                hash: parts[0].to_string(),
                                message: parts[1].to_string(),
                                date: parts[2].to_string(),
                            })
                        } else {
                            None
                        }
                    })
                    .collect()
            }
            _ => Vec::new(),
        }
    }

    fn load_from_git(&mut self, ctx: &egui::Context, path: &Path, rev: &str, is_left: bool) {
        // We need the relative path from the git root
        let root_output = Command::new("git")
            .arg("rev-parse")
            .arg("--show-toplevel")
            .output();

        let rel_path = if let Ok(out) = root_output {
            let root = String::from_utf8_lossy(&out.stdout).trim().to_string();
            path.strip_prefix(root).unwrap_or(path).to_path_buf()
        } else {
            path.to_path_buf()
        };

        let output = Command::new("git")
            .arg("show")
            .arg(format!("{}:{}", rev, rel_path.display()))
            .output();

        match output {
            Ok(output) if output.status.success() => {
                match image::load_from_memory(&output.stdout) {
                    Ok(dynamic_image) => {
                        let size = [
                            dynamic_image.width() as usize,
                            dynamic_image.height() as usize,
                        ];
                        let image_buffer = dynamic_image.to_rgba8();
                        let pixels = image_buffer.as_flat_samples();
                        let color_image =
                            ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());

                        let name =
                            format!("{} ({})", path.file_name().unwrap().to_string_lossy(), rev);
                        let handle = ctx.load_texture(name, color_image, Default::default());

                        if is_left {
                            self.texture_left = Some(handle);
                            self.image_left = Some(image_buffer);
                            self.path_left = Some(path.to_path_buf());
                            self.commits_left.clear();
                        } else {
                            self.texture_right = Some(handle);
                            self.image_right = Some(image_buffer);
                            self.path_right = Some(path.to_path_buf());
                            self.commits_right.clear();
                        }
                        self.texture_diff = None; // Force clear diff
                        self.error_msg = None;
                    }
                    Err(e) => {
                        self.error_msg =
                            Some(format!("Failed to parse image from git ({}): {}", rev, e));
                    }
                }
            }
            Ok(out) => {
                self.error_msg = Some(format!(
                    "Git error: {}",
                    String::from_utf8_lossy(&out.stderr)
                ));
            }
            Err(e) => {
                self.error_msg = Some(format!("Failed to run git: {}", e));
            }
        }
    }

    fn is_in_git_repo(&self, path: &Path) -> bool {
        let output = Command::new("git")
            .arg("rev-parse")
            .arg("--is-inside-work-tree")
            .current_dir(path.parent().unwrap_or(Path::new(".")))
            .output();

        match output {
            Ok(output) => {
                output.status.success() && String::from_utf8_lossy(&output.stdout).trim() == "true"
            }
            _ => false,
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
                    self.image_left = Some(image_buffer);
                    self.path_left = Some(path);
                    self.commits_left.clear();
                } else {
                    self.texture_right = Some(handle);
                    self.image_right = Some(image_buffer);
                    self.path_right = Some(path);
                    self.commits_right.clear();
                }
                self.texture_diff = None; // Force clear diff
                self.error_msg = None;
            }
            Err(e) => {
                self.error_msg = Some(format!("Failed to load {}: {}", path.display(), e));
            }
        }
    }

    fn generate_diff_texture(&mut self, ctx: &egui::Context) {
        if self.texture_diff.is_some() {
            return;
        }

        if let (Some(img_left), Some(img_right)) = (&self.image_left, &self.image_right) {
            let width = img_left.width().min(img_right.width());
            let height = img_left.height().min(img_right.height());

            let mut diff_img = image::RgbaImage::new(width, height);

            for y in 0..height {
                for x in 0..width {
                    let p1 = img_left.get_pixel(x, y);
                    let p2 = img_right.get_pixel(x, y);

                    if p1 != p2 {
                        // Magenta for diff (Diffchecker style)
                        diff_img.put_pixel(x, y, image::Rgba([255, 0, 255, 255]));
                    } else {
                        // Dimm grayscale for base
                        let gray = (p1[0] as u32 + p1[1] as u32 + p1[2] as u32) / 8; // Very dim
                        diff_img.put_pixel(
                            x,
                            y,
                            image::Rgba([gray as u8, gray as u8, gray as u8, 255]),
                        );
                    }
                }
            }

            let size = [width as usize, height as usize];
            let pixels = diff_img.as_flat_samples();
            let color_image = ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
            self.texture_diff =
                Some(ctx.load_texture("pixel_diff", color_image, Default::default()));
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

                    // --- Multi Load ---
                    let multi_icon = icon_text(Icon::Files, 14.0);
                    if button(
                        ui,
                        &self.theme,
                        multi_icon,
                        ControlVariant::Outline,
                        ControlSize::Sm,
                        true,
                    )
                    .on_hover_text("Select Image Pair")
                    .clicked()
                    {
                        if let Some(paths) = rfd::FileDialog::new()
                            .add_filter("Images", &["png", "jpg", "jpeg", "webp"])
                            .pick_files()
                        {
                            let mut iter = paths.into_iter();
                            if let Some(p1) = iter.next() {
                                self.load_image_to_texture(ctx, p1, true);
                            }
                            if let Some(p2) = iter.next() {
                                self.load_image_to_texture(ctx, p2, false);
                            }
                        }
                    }

                    ui.add_space(16.0);
                    ui.separator();
                    ui.add_space(16.0);

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

                        // --- Git History (Right) ---
                        if let Some(path) = self.path_right.clone() {
                            let history_icon = icon_text(Icon::History, 14.0);
                            ui.menu_button(history_icon, |ui| {
                                if self.commits_right.is_empty() {
                                    self.commits_right = self.get_git_history(&path);
                                }

                                let commits = self.commits_right.clone(); // Clone to avoid borrow conflict
                                if commits.is_empty() {
                                    ui.label("No git history found");
                                } else {
                                    for commit in commits {
                                        if ui
                                            .button(format!(
                                                "[{}] {} ({})",
                                                commit.hash, commit.message, commit.date
                                            ))
                                            .clicked()
                                        {
                                            self.load_from_git(ctx, &path, &commit.hash, false);
                                            ui.close();
                                        }
                                    }
                                }
                            });
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

                        // --- Git History (Left) ---
                        if let Some(path) = self.path_left.clone() {
                            let history_icon = icon_text(Icon::History, 14.0);
                            ui.menu_button(history_icon, |ui| {
                                if self.commits_left.is_empty() {
                                    self.commits_left = self.get_git_history(&path);
                                }

                                let commits = self.commits_left.clone(); // Clone to avoid borrow conflict
                                if commits.is_empty() {
                                    ui.label("No git history found");
                                } else {
                                    for commit in commits {
                                        if ui
                                            .button(format!(
                                                "[{}] {} ({})",
                                                commit.hash, commit.message, commit.date
                                            ))
                                            .clicked()
                                        {
                                            self.load_from_git(ctx, &path, &commit.hash, true);
                                            ui.close();
                                        }
                                    }
                                }
                            });
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

        // --- Global Drag and Drop ---
        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                let mut iter = i.raw.dropped_files.clone().into_iter();
                if let Some(f1) = iter.next() {
                    if let Some(path) = f1.path {
                        self.load_image_to_texture(ctx, path, true);
                    }
                }
                if let Some(f2) = iter.next() {
                    if let Some(path) = f2.path {
                        self.load_image_to_texture(ctx, path, false);
                    }
                }
            }
        });

        egui::TopBottomPanel::top("mode_toolbar").show(ctx, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.add_space(8.0);

                // Mode Selectors (Diffchecker style)
                let modes = [
                    (DiffMode::Slider, Icon::Split, "Slider"),
                    (DiffMode::SideBySide, Icon::Columns2, "Side by Side"),
                    (DiffMode::Difference, Icon::Target, "Difference"),
                    (DiffMode::Fade, Icon::Sun, "Fade"),
                ];

                for (mode, icon, label) in modes {
                    let is_selected = self.diff_mode == mode;
                    if button(
                        ui,
                        &self.theme,
                        format!(" {} {}", icon_text(icon, 14.0).text(), label),
                        if is_selected {
                            ControlVariant::Primary
                        } else {
                            ControlVariant::Ghost
                        },
                        ControlSize::Sm,
                        true,
                    )
                    .clicked()
                    {
                        self.diff_mode = mode;
                    }
                    ui.add_space(4.0);
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(8.0);
                    if button(
                        ui,
                        &self.theme,
                        "Clear",
                        ControlVariant::Outline,
                        ControlSize::Sm,
                        true,
                    )
                    .clicked()
                    {
                        self.texture_left = None;
                        self.texture_right = None;
                        self.texture_diff = None;
                        self.path_left = None;
                        self.path_right = None;
                    }
                });
            });
            ui.add_space(8.0);
            separator(ui, &self.theme, SeparatorProps::default());
        });

        // --- Global Drag and Drop ---
        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                let mut iter = i.raw.dropped_files.clone().into_iter();
                if let Some(f1) = iter.next() {
                    if let Some(path) = f1.path {
                        self.load_image_to_texture(ctx, path, true);
                    }
                }
                if let Some(f2) = iter.next() {
                    if let Some(path) = f2.path {
                        self.load_image_to_texture(ctx, path, false);
                    }
                }
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(msg) = &self.error_msg {
                ui.colored_label(egui::Color32::RED, msg);
            }

            if let (Some(tex_left), Some(tex_right)) = (&self.texture_left, &self.texture_right) {
                match self.diff_mode {
                    DiffMode::Slider => {
                        let available_size = ui.available_size();
                        let (rect, response) =
                            ui.allocate_exact_size(available_size, egui::Sense::drag());

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

                        // Slider Grip logic... (omitted for brevity, keeping existing slider logic)
                        let line_stroke = egui::Stroke::new(2.0, egui::Color32::WHITE);
                        ui.painter().line_segment(
                            [
                                egui::pos2(swipe_x, rect.top()),
                                egui::pos2(swipe_x, rect.bottom()),
                            ],
                            line_stroke,
                        );

                        let handle_pos = egui::pos2(swipe_x, rect.center().y);
                        ui.painter().circle(
                            handle_pos,
                            18.0,
                            egui::Color32::from_black_alpha(180),
                            egui::Stroke::new(2.0, egui::Color32::WHITE),
                        );
                        ui.painter().text(
                            handle_pos,
                            egui::Align2::CENTER_CENTER,
                            icon_text(Icon::ChevronsLeftRight, 16.0).text(),
                            egui::FontId::new(16.0, FontFamily::Name("lucide".into())),
                            egui::Color32::WHITE,
                        );

                        if response.dragged() {
                            if let Some(pointer_pos) = ctx.input(|i| i.pointer.latest_pos()) {
                                let rel_x = pointer_pos.x - rect.left();
                                self.slider_pos = (rel_x / rect.width()).clamp(0.0, 1.0);
                            }
                        }
                    }
                    DiffMode::SideBySide => {
                        ui.columns(2, |columns| {
                            let size = columns[0].available_size();
                            columns[0].vertical_centered(|ui| {
                                ui.label(RichText::new("Original").strong());
                                ui.add(egui::Image::from_texture(tex_left).fit_to_exact_size(size));
                            });
                            columns[1].vertical_centered(|ui| {
                                ui.label(RichText::new("New").strong());
                                ui.add(
                                    egui::Image::from_texture(tex_right).fit_to_exact_size(size),
                                );
                            });
                        });
                    }
                    DiffMode::Difference => {
                        self.generate_diff_texture(ctx);
                        if let Some(tex_diff) = &self.texture_diff {
                            let available_size = ui.available_size();
                            ui.centered_and_justified(|ui| {
                                ui.add(
                                    egui::Image::from_texture(tex_diff)
                                        .fit_to_exact_size(available_size),
                                );
                            });
                        } else {
                            ui.centered_and_justified(|ui| {
                                ui.label("Calculating difference...");
                            });
                        }
                    }
                    DiffMode::Fade => {
                        let available_size = ui.available_size();
                        let (rect, _response) =
                            ui.allocate_exact_size(available_size, egui::Sense::hover());

                        ui.painter().image(
                            tex_left.id(),
                            rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                        ui.painter().image(
                            tex_right.id(),
                            rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE.gamma_multiply(self.fade_opacity),
                        );

                        ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                            ui.add_space(10.0);
                            ui.add(
                                egui::Slider::new(&mut self.fade_opacity, 0.0..=1.0)
                                    .text("Fade Opacity"),
                            );
                        });
                    }
                }
            } else if let Some(path) = self.path_left.clone().or(self.path_right.clone()) {
                // One image is loaded. Is it in Git?
                if self.is_in_git_repo(&path) {
                    ui.centered_and_justified(|ui| {
                        ui.vertical_centered(|ui| {
                            let history_icon = icon_text(Icon::History, 32.0)
                                .color(ui.visuals().selection.bg_fill);
                            ui.label(history_icon);
                            ui.add_space(8.0);
                            Label::new("Compare with History")
                                .size(ControlSize::Lg)
                                .show(ui, &self.theme);
                            ui.add_space(4.0);
                            Label::new("Found this image in a Git repository.")
                                .size(ControlSize::Sm)
                                .show(ui, &self.theme);

                            ui.add_space(24.0);

                            // Load commits if not already loaded
                            let commits = if self.path_left.is_some() {
                                if self.commits_left.is_empty() {
                                    self.commits_left = self.get_git_history(&path);
                                }
                                &self.commits_left
                            } else {
                                if self.commits_right.is_empty() {
                                    self.commits_right = self.get_git_history(&path);
                                }
                                &self.commits_right
                            }
                            .clone();

                            if commits.is_empty() {
                                ui.label("No git history found for this file.");
                            } else {
                                egui::Frame::NONE
                                    .fill(ui.visuals().window_fill)
                                    .corner_radius(12.0)
                                    .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
                                    .inner_margin(egui::Margin::same(1)) // For precise border
                                    .show(ui, |ui| {
                                        ui.set_width(500.0);
                                        egui::ScrollArea::vertical().max_height(400.0).show(
                                            ui,
                                            |ui| {
                                                ui.add_space(8.0);
                                                for (i, commit) in commits.into_iter().enumerate() {
                                                    if i > 0 {
                                                        ui.add_space(2.0);
                                                        ui.separator();
                                                        ui.add_space(2.0);
                                                    }

                                                    let response = ui
                                                        .horizontal(|ui| {
                                                            ui.set_min_height(40.0);
                                                            ui.add_space(8.0);

                                                            // Hash column
                                                            ui.colored_label(
                                                                ui.visuals().selection.bg_fill,
                                                                RichText::new(commit.hash.clone())
                                                                    .monospace()
                                                                    .strong(),
                                                            );
                                                            ui.add_space(8.0);

                                                            // Message (expanding)
                                                            let message =
                                                                if commit.message.len() > 40 {
                                                                    format!(
                                                                        "{}...",
                                                                        &commit.message[..37]
                                                                    )
                                                                } else {
                                                                    commit.message.clone()
                                                                };
                                                            ui.label(
                                                                RichText::new(message).size(14.0),
                                                            );

                                                            ui.with_layout(
                                                                egui::Layout::right_to_left(
                                                                    egui::Align::Center,
                                                                ),
                                                                |ui| {
                                                                    ui.add_space(8.0);
                                                                    ui.colored_label(
                                                                        ui.visuals()
                                                                            .weak_text_color(),
                                                                        RichText::new(commit.date)
                                                                            .size(12.0),
                                                                    );
                                                                },
                                                            );
                                                        })
                                                        .response;

                                                    let rect = response.rect;
                                                    let is_hovered = ui.rect_contains_pointer(rect);

                                                    if is_hovered {
                                                        ui.painter().rect_filled(
                                                            rect,
                                                            4.0,
                                                            ui.visuals()
                                                                .widgets
                                                                .hovered
                                                                .bg_fill
                                                                .gamma_multiply(0.3),
                                                        );
                                                        ui.output_mut(|o| {
                                                            o.cursor_icon =
                                                                egui::CursorIcon::PointingHand
                                                        });
                                                    }

                                                    // Make entire row clickable
                                                    let click_area = ui.interact(
                                                        rect,
                                                        ui.id().with(i),
                                                        egui::Sense::click(),
                                                    );
                                                    if click_area.clicked() {
                                                        let is_left_empty =
                                                            self.texture_left.is_none();
                                                        self.load_from_git(
                                                            ctx,
                                                            &path,
                                                            &commit.hash,
                                                            is_left_empty,
                                                        );
                                                    }
                                                }
                                                ui.add_space(8.0);
                                            },
                                        );
                                    });
                            }

                            ui.add_space(24.0);

                            if button(
                                ui,
                                &self.theme,
                                "Select local image instead",
                                ControlVariant::Ghost,
                                ControlSize::Sm,
                                true,
                            )
                            .clicked()
                            {
                                if let Some(p) = rfd::FileDialog::new().pick_file() {
                                    let is_left_empty = self.texture_left.is_none();
                                    self.load_image_to_texture(ctx, p, is_left_empty);
                                }
                            }
                        });
                    });
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.vertical_centered(|ui| {
                            let image_icon = icon_text(Icon::ImagePlus, 32.0)
                                .color(ui.visuals().selection.bg_fill);
                            ui.label(image_icon);
                            ui.add_space(8.0);
                            Label::new("Ready to Compare")
                                .size(ControlSize::Lg)
                                .show(ui, &self.theme);
                            ui.add_space(4.0);
                            Label::new("Load the second image to start.").show(ui, &self.theme);

                            ui.add_space(32.0);

                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 24.0;
                                let left_loaded = self.texture_left.is_some();
                                let right_loaded = self.texture_right.is_some();

                                if button(
                                    ui,
                                    &self.theme,
                                    if left_loaded {
                                        "Slot 1: Loaded ✓"
                                    } else {
                                        "Select Image 1"
                                    },
                                    if left_loaded {
                                        ControlVariant::Secondary
                                    } else {
                                        ControlVariant::Primary
                                    },
                                    ControlSize::Lg,
                                    true,
                                )
                                .clicked()
                                {
                                    if let Some(p) = rfd::FileDialog::new().pick_file() {
                                        self.load_image_to_texture(ctx, p, true);
                                    }
                                }

                                if button(
                                    ui,
                                    &self.theme,
                                    if right_loaded {
                                        "Slot 2: Loaded ✓"
                                    } else {
                                        "Select Image 2"
                                    },
                                    if right_loaded {
                                        ControlVariant::Secondary
                                    } else {
                                        ControlVariant::Primary
                                    },
                                    ControlSize::Lg,
                                    true,
                                )
                                .clicked()
                                {
                                    if let Some(p) = rfd::FileDialog::new().pick_file() {
                                        self.load_image_to_texture(ctx, p, false);
                                    }
                                }
                            });
                        });
                    });
                }
            } else {
                // Mode Switcher Empty State (Diffchecker style)
                ui.centered_and_justified(|ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 24.0;

                        let card_width = 400.0;
                        let card_height = 300.0;

                        // Original Card
                        egui::Frame::NONE
                            .fill(ui.visuals().faint_bg_color)
                            .corner_radius(12.0)
                            .stroke(egui::Stroke::new(
                                2.0,
                                ui.visuals().widgets.noninteractive.bg_stroke.color,
                            ))
                            .show(ui, |ui| {
                                ui.set_min_size(egui::vec2(card_width, card_height));
                                ui.centered_and_justified(|ui| {
                                    ui.vertical_centered(|ui| {
                                        ui.add_space(80.0);
                                        ui.label(icon_text(Icon::Image, 48.0));
                                        ui.add_space(20.0);
                                        Label::new("Original Image")
                                            .size(ControlSize::Lg)
                                            .show(ui, &self.theme);
                                        ui.add_space(10.0);
                                        if button(
                                            ui,
                                            &self.theme,
                                            "Browse File",
                                            ControlVariant::Outline,
                                            ControlSize::Sm,
                                            true,
                                        )
                                        .clicked()
                                        {
                                            if let Some(p) = rfd::FileDialog::new().pick_file() {
                                                self.load_image_to_texture(ctx, p, true);
                                            }
                                        }
                                    });
                                });
                            });

                        // New Card
                        egui::Frame::NONE
                            .fill(ui.visuals().faint_bg_color)
                            .corner_radius(12.0)
                            .stroke(egui::Stroke::new(
                                2.0,
                                ui.visuals().widgets.noninteractive.bg_stroke.color,
                            ))
                            .show(ui, |ui| {
                                ui.set_min_size(egui::vec2(card_width, card_height));
                                ui.centered_and_justified(|ui| {
                                    ui.vertical_centered(|ui| {
                                        ui.add_space(80.0);
                                        ui.label(icon_text(Icon::ImagePlus, 48.0));
                                        ui.add_space(20.0);
                                        Label::new("New Image")
                                            .size(ControlSize::Lg)
                                            .show(ui, &self.theme);
                                        ui.add_space(10.0);
                                        if button(
                                            ui,
                                            &self.theme,
                                            "Browse File",
                                            ControlVariant::Outline,
                                            ControlSize::Sm,
                                            true,
                                        )
                                        .clicked()
                                        {
                                            if let Some(p) = rfd::FileDialog::new().pick_file() {
                                                self.load_image_to_texture(ctx, p, false);
                                            }
                                        }
                                    });
                                });
                            });
                    });
                });
            }
        });

        egui::TopBottomPanel::bottom("footer").show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.add_space(8.0);
                ui.label(RichText::new("IMAGE DIFFER").size(10.0).weak());

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(8.0);
                    // Dummy zoom - could be implemented with a state scale f32
                    ui.add_space(4.0);
                    ui.label("100%");
                    ui.add_space(4.0);
                    ui.label(icon_text(Icon::Search, 12.0));
                });
            });
            ui.add_space(4.0);
        });
    }
}
