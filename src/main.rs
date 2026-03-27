#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use eframe::epaint::{ColorImage, TextureHandle};
use egui::{Color32, FontData, FontDefinitions, FontFamily, FontId, RichText, Stroke};
use egui_shadcn::Theme;
use lucide_icons::{Icon, LUCIDE_FONT_BYTES};
use std::default;
use std::io::{IsTerminal, Read};
use std::path::{Path, PathBuf};
use std::process::Command;

// ── Palette ────────────────────────────────────────────────────────────────
const ACCENT: Color32 = Color32::from_rgb(0x06, 0xb6, 0xd4);
const BG_DEEP: Color32 = Color32::from_rgb(0x09, 0x09, 0x0e);
const BG_SURFACE: Color32 = Color32::from_rgb(0x10, 0x10, 0x18);
const BG_CARD: Color32 = Color32::from_rgb(0x18, 0x18, 0x22);
const BG_ELEVATED: Color32 = Color32::from_rgb(0x22, 0x22, 0x2e);
const BORDER: Color32 = Color32::from_rgb(0x28, 0x28, 0x38);
const TEXT: Color32 = Color32::from_rgb(0xe0, 0xe6, 0xf0);
const TEXT_MUTED: Color32 = Color32::from_rgb(0x58, 0x68, 0x82);
const TEXT_DIM: Color32 = Color32::from_rgb(0x30, 0x38, 0x48);
const SUCCESS: Color32 = Color32::from_rgb(0x10, 0xb9, 0x81);
const DANGER: Color32 = Color32::from_rgb(0xf8, 0x71, 0x71);

// ── Types ───────────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
struct GitCommit {
    hash: String,
    message: String,
    date: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DiffMode {
    Slider,
    SideBySide,
    Difference,
    Fade,
}

/// Metadata about how the app was launched from git.
#[derive(Clone, Debug, Default)]
struct GitContext {
    /// e.g. "icon.png" — the file being diffed
    filename: String,
    /// The old blob hash (7-char)
    old_rev: String,
    /// The new blob hash (7-char)
    new_rev: String,
}

/// Startup configuration parsed from args / stdin before the UI is created.
#[derive(Default)]
struct StartupConfig {
    left_path: Option<PathBuf>,
    right_path: Option<PathBuf>,
    git_context: Option<GitContext>,
    initial_mode: DiffMode,
}

struct ImageDifferApp {
    texture_left: Option<TextureHandle>,
    texture_right: Option<TextureHandle>,
    texture_diff: Option<TextureHandle>,
    image_left: Option<image::RgbaImage>,
    image_right: Option<image::RgbaImage>,
    path_left: Option<PathBuf>,
    path_right: Option<PathBuf>,
    slider_pos: f32,
    fade_opacity: f32,
    diff_mode: DiffMode,
    diff_pixel_pct: Option<f64>,
    error_msg: Option<String>,
    theme: Theme,
    commits_left: Vec<GitCommit>,
    commits_right: Vec<GitCommit>,
    // Git-launch context
    git_context: Option<GitContext>,
    // Deferred loading (paths provided before egui context exists)
    pending_left: Option<PathBuf>,
    pending_right: Option<PathBuf>,
}

// ── Font / Visual Setup ─────────────────────────────────────────────────────

fn setup_fonts(ctx: &egui::Context) {
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
    // Lucide fallback in Proportional so icons render in mixed strings
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .push("lucide".into());
    ctx.set_fonts(fonts);
}

fn setup_visuals(ctx: &egui::Context) {
    let mut v = egui::Visuals::dark();
    v.panel_fill = BG_SURFACE;
    v.window_fill = BG_SURFACE;
    v.faint_bg_color = BG_CARD;
    v.extreme_bg_color = BG_DEEP;
    v.selection.bg_fill = ACCENT;
    v.selection.stroke = Stroke::new(1.0, Color32::from_rgb(0x67, 0xe8, 0xf9));
    v.hyperlink_color = ACCENT;

    let mk = |bg: Color32, border: Color32, fg: Color32| egui::style::WidgetVisuals {
        bg_fill: bg,
        weak_bg_fill: bg,
        bg_stroke: Stroke::new(1.0, border),
        fg_stroke: Stroke::new(1.0, fg),
        corner_radius: egui::CornerRadius::same(6),
        expansion: 0.0,
    };
    v.widgets.noninteractive = mk(BG_CARD, BORDER, TEXT_MUTED);
    v.widgets.inactive = mk(BG_ELEVATED, BORDER, TEXT);
    v.widgets.hovered = mk(
        Color32::from_rgb(0x24, 0x24, 0x32),
        Color32::from_rgb(0x3a, 0x3a, 0x52),
        Color32::WHITE,
    );
    v.widgets.active = mk(Color32::from_rgb(0x05, 0x90, 0xa8), ACCENT, Color32::WHITE);
    v.widgets.open = mk(BG_ELEVATED, BORDER, TEXT);

    v.window_stroke = Stroke::new(1.0, BORDER);
    v.window_corner_radius = egui::CornerRadius::same(8);

    ctx.set_visuals(v);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(6.0, 4.0);
    style.spacing.button_padding = egui::vec2(10.0, 5.0);
    style.spacing.window_margin = egui::Margin::same(0);
    style.spacing.menu_margin = egui::Margin::same(6);
    ctx.set_style(style);
}

fn icon_text(icon: Icon, size: f32) -> RichText {
    RichText::new(icon.unicode().to_string())
        .font(FontId::new(size, FontFamily::Name("lucide".into())))
}

fn icon_str(icon: Icon) -> String {
    icon.unicode().to_string()
}

/// Fit an image into a container rect maintaining aspect ratio (letterbox).
fn fit_in_rect(img_size: egui::Vec2, container: egui::Rect) -> egui::Rect {
    let scale = (container.width() / img_size.x).min(container.height() / img_size.y);
    let sz = img_size * scale;
    egui::Rect::from_center_size(container.center(), sz)
}

// ── CLI / Stdin Parsing ──────────────────────────────────────────────────────

/// Write a git blob to a temp file and return its path.
fn extract_git_blob(hash: &str, hint_ext: &str) -> Option<PathBuf> {
    let out = Command::new("git")
        .arg("cat-file")
        .arg("blob")
        .arg(hash)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let ext = if hint_ext.is_empty() { "bin" } else { hint_ext };
    let tmp = std::env::temp_dir().join(format!("idiffer_{}_{}.{}", hash, std::process::id(), ext));
    std::fs::write(&tmp, &out.stdout).ok()?;
    Some(tmp)
}

/// Parse a `git diff` (or `git difftool`) unified diff from stdin and return
/// temp-file paths for the old and new versions of the first changed image.
fn parse_git_diff_stdin() -> StartupConfig {
    let mut cfg = StartupConfig::default();

    // Only read stdin when it is a pipe, not an interactive terminal.
    if std::io::stdin().is_terminal() {
        return cfg;
    }

    let mut input = String::new();
    if std::io::stdin().lock().read_to_string(&mut input).is_err() {
        return cfg;
    }

    let mut filename = String::new();
    let mut old_hash = String::new();
    let mut new_hash = String::new();

    for line in input.lines() {
        // diff --git a/path/file.png b/path/file.png
        if line.starts_with("diff --git ") {
            // Reset for each file, use first image we find
            if !old_hash.is_empty() {
                break;
            }
            let parts: Vec<&str> = line.splitn(4, ' ').collect();
            if let Some(b) = parts.get(3) {
                filename = b.trim_start_matches("b/").to_string();
            }
        }
        // index abc1234..def5678 100644
        if line.starts_with("index ") && old_hash.is_empty() {
            let rest = line[6..].split_whitespace().next().unwrap_or("");
            if let Some(pos) = rest.find("..") {
                old_hash = rest[..pos].to_string();
                new_hash = rest[pos + 2..].to_string();
            }
        }
    }

    if old_hash.is_empty() || new_hash.is_empty() {
        return cfg;
    }

    let ext = Path::new(&filename)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("png")
        .to_string();

    let old_tmp = extract_git_blob(&old_hash, &ext);
    let new_tmp = extract_git_blob(&new_hash, &ext);

    cfg.left_path = old_tmp;
    cfg.right_path = new_tmp;
    cfg.initial_mode = DiffMode::Slider; // most useful for git diffs
    cfg.git_context = Some(GitContext {
        filename: Path::new(&filename)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or(filename),
        old_rev: old_hash.chars().take(7).collect(),
        new_rev: new_hash.chars().take(7).collect(),
    });
    cfg
}

fn parse_args() -> StartupConfig {
    let args: Vec<String> = std::env::args().skip(1).collect();

    // Handle --help / --install flags immediately.
    for arg in &args {
        if arg == "--help" || arg == "-h" {
            eprintln!("IDiffer — image comparison tool");
            eprintln!();
            eprintln!("Usage:");
            eprintln!("  idiffer                          # interactive mode");
            eprintln!("  idiffer old.png new.png         # compare two files");
            eprintln!("  git diff <file> | idiffer       # git diff pipe mode");
            eprintln!("  git difftool --tool=idiffer     # git difftool mode");
            eprintln!();
            eprintln!("To install as git difftool:");
            eprintln!("  git config --global diff.tool idiffer");
            eprintln!(
                "  git config --global difftool.idiffer.cmd 'idiffer \"$LOCAL\" \"$REMOTE\"'"
            );
            eprintln!("  git config --global difftool.prompt false");
            std::process::exit(0);
        }
        if arg == "--install" {
            let cmds = [
                ("git", &["config", "--global", "diff.tool", "idiffer"][..]),
                (
                    "git",
                    &[
                        "config",
                        "--global",
                        "difftool.idiffer.cmd",
                        "idiffer \"$LOCAL\" \"$REMOTE\"",
                    ][..],
                ),
                (
                    "git",
                    &["config", "--global", "difftool.prompt", "false"][..],
                ),
            ];
            for (cmd, a) in &cmds {
                let ok = Command::new(cmd)
                    .args(*a)
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false);
                eprintln!(
                    "{}: git config {}",
                    if ok { "✓" } else { "✗" },
                    a.last().unwrap_or(&"")
                );
            }
            eprintln!("\nDone. Run: git difftool --tool=idiffer");
            std::process::exit(0);
        }
    }

    // Two path args → difftool mode ($LOCAL $REMOTE)
    let paths: Vec<PathBuf> = args
        .iter()
        .filter(|a| !a.starts_with('-'))
        .map(|a| PathBuf::from(a))
        .collect();

    if paths.len() >= 2 {
        // Try to detect git context from environment variables git sets for difftool
        let old_rev = std::env::var("LOCAL").unwrap_or_default();
        let new_rev = std::env::var("REMOTE").unwrap_or_default();
        let filename = paths[1]
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        return StartupConfig {
            left_path: Some(paths[0].clone()),
            right_path: Some(paths[1].clone()),
            initial_mode: DiffMode::Slider,
            git_context: Some(GitContext {
                filename,
                old_rev: shorten_path_hint(&paths[0]),
                new_rev: shorten_path_hint(&paths[1]),
            }),
        };
    }

    if paths.len() == 1 {
        return StartupConfig {
            left_path: Some(paths[0].clone()),
            ..Default::default()
        };
    }

    // No args — try stdin (git diff pipe mode)
    parse_git_diff_stdin()
}

/// Extract the last component of a git temp path, or its 7-char prefix if it
/// looks like a blob hash path (e.g. /tmp/4b825dc_abc.png → abc…)
fn shorten_path_hint(p: &Path) -> String {
    p.file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.chars().take(7).collect())
        .unwrap_or_default()
}

impl Default for DiffMode {
    fn default() -> Self {
        DiffMode::Slider
    }
}

// ── Main ────────────────────────────────────────────────────────────────────

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

// ── App Impl ─────────────────────────────────────────────────────────────────

impl ImageDifferApp {
    fn new(_cc: &eframe::CreationContext<'_>, cfg: StartupConfig) -> Self {
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
            diff_mode: cfg.initial_mode,
            diff_pixel_pct: None,
            error_msg: None,
            theme: Theme::default(),
            commits_left: Vec::new(),
            commits_right: Vec::new(),
            git_context: cfg.git_context,
            pending_left: cfg.left_path,
            pending_right: cfg.right_path,
        }
    }

    fn get_git_history(&self, path: &Path) -> Vec<GitCommit> {
        let output = Command::new("git")
            .arg("log")
            .arg("--pretty=format:%h|%s|%cd")
            .arg("--date=short")
            .arg("--")
            .arg(path)
            .output();
        match output {
            Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
                .lines()
                .filter_map(|line| {
                    let p: Vec<&str> = line.splitn(3, '|').collect();
                    if p.len() >= 3 {
                        Some(GitCommit {
                            hash: p[0].into(),
                            message: p[1].into(),
                            date: p[2].into(),
                        })
                    } else {
                        None
                    }
                })
                .collect(),
            _ => vec![],
        }
    }

    fn load_from_git(&mut self, ctx: &egui::Context, path: &Path, rev: &str, is_left: bool) {
        let root = Command::new("git")
            .arg("rev-parse")
            .arg("--show-toplevel")
            .output();
        let rel = if let Ok(o) = root {
            let root_str = String::from_utf8_lossy(&o.stdout).trim().to_string();
            path.strip_prefix(&root_str).unwrap_or(path).to_path_buf()
        } else {
            path.to_path_buf()
        };

        match Command::new("git")
            .arg("show")
            .arg(format!("{}:{}", rev, rel.display()))
            .output()
        {
            Ok(o) if o.status.success() => match image::load_from_memory(&o.stdout) {
                Ok(img) => {
                    let size = [img.width() as usize, img.height() as usize];
                    let buf = img.to_rgba8();
                    let pixels = buf.as_flat_samples();
                    let ci = ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
                    let name = format!("{} ({})", path.file_name().unwrap().to_string_lossy(), rev);
                    let handle = ctx.load_texture(name, ci, Default::default());
                    if is_left {
                        self.texture_left = Some(handle);
                        self.image_left = Some(buf);
                        self.path_left = Some(path.into());
                        self.commits_left.clear();
                    } else {
                        self.texture_right = Some(handle);
                        self.image_right = Some(buf);
                        self.path_right = Some(path.into());
                        self.commits_right.clear();
                    }
                    self.texture_diff = None;
                    self.diff_pixel_pct = None;
                    self.error_msg = None;
                }
                Err(e) => self.error_msg = Some(format!("Image parse error ({rev}): {e}")),
            },
            Ok(o) => {
                self.error_msg = Some(format!("git error: {}", String::from_utf8_lossy(&o.stderr)))
            }
            Err(e) => self.error_msg = Some(format!("git failed: {e}")),
        }
    }

    fn is_in_git_repo(&self, path: &Path) -> bool {
        Command::new("git")
            .arg("rev-parse")
            .arg("--is-inside-work-tree")
            .current_dir(path.parent().unwrap_or(Path::new(".")))
            .output()
            .map(|o| o.status.success() && String::from_utf8_lossy(&o.stdout).trim() == "true")
            .unwrap_or(false)
    }

    fn load_image_to_texture(&mut self, ctx: &egui::Context, path: PathBuf, is_left: bool) {
        match image::open(&path) {
            Ok(img) => {
                let size = [img.width() as usize, img.height() as usize];
                let buf = img.to_rgba8();
                let pixels = buf.as_flat_samples();
                let ci = ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
                let name = path.file_name().unwrap().to_string_lossy();
                let handle = ctx.load_texture(name, ci, Default::default());
                if is_left {
                    self.texture_left = Some(handle);
                    self.image_left = Some(buf);
                    self.path_left = Some(path);
                    self.commits_left.clear();
                } else {
                    self.texture_right = Some(handle);
                    self.image_right = Some(buf);
                    self.path_right = Some(path);
                    self.commits_right.clear();
                }
                self.texture_diff = None;
                self.diff_pixel_pct = None;
                self.error_msg = None;
            }
            Err(e) => self.error_msg = Some(format!("Load failed: {e}")),
        }
    }

    fn generate_diff_texture(&mut self, ctx: &egui::Context) {
        if self.texture_diff.is_some() {
            return;
        }
        if let (Some(l), Some(r)) = (&self.image_left, &self.image_right) {
            let w = l.width().min(r.width());
            let h = l.height().min(r.height());
            let mut diff = image::RgbaImage::new(w, h);
            let mut diff_count: u64 = 0;
            for y in 0..h {
                for x in 0..w {
                    let p1 = l.get_pixel(x, y);
                    let p2 = r.get_pixel(x, y);
                    if p1 != p2 {
                        diff_count += 1;
                        diff.put_pixel(x, y, image::Rgba([0xfc, 0x72, 0x5d, 0xff]));
                    } else {
                        let g = (p1[0] as u32 + p1[1] as u32 + p1[2] as u32) / 4;
                        diff.put_pixel(x, y, image::Rgba([g as u8, g as u8, g as u8, 0xff]));
                    }
                }
            }
            self.diff_pixel_pct = Some(diff_count as f64 / (w * h) as f64 * 100.0);
            let size = [w as usize, h as usize];
            let px = diff.as_flat_samples();
            let ci = ColorImage::from_rgba_unmultiplied(size, px.as_slice());
            self.texture_diff = Some(ctx.load_texture("diff", ci, Default::default()));
        }
    }

    fn clear_all(&mut self) {
        self.texture_left = None;
        self.texture_right = None;
        self.texture_diff = None;
        self.image_left = None;
        self.image_right = None;
        self.path_left = None;
        self.path_right = None;
        self.diff_pixel_pct = None;
        self.error_msg = None;
        self.commits_left.clear();
        self.commits_right.clear();
    }

    // Draw a small image‑slot pill in the header (thumbnail + name + git + delete)
    fn render_slot_header(
        &mut self,
        ui: &mut egui::Ui,
        is_left: bool,
        label: &str,
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
                            let hist_btn = egui::Button::new(
                                RichText::new(icon_str(Icon::GitBranch))
                                    .size(12.0)
                                    .color(TEXT_MUTED),
                            )
                            .fill(Color32::TRANSPARENT)
                            .stroke(Stroke::NONE);
                            let _hist_resp = ui.add(hist_btn).on_hover_text("Git history");
                            ui.menu_button(RichText::new("").size(0.0), |ui| {
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
                                        .max_height(300.0)
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
                                                    let msg = if commit.message.len() > 36 {
                                                        format!("{}…", &commit.message[..34])
                                                    } else {
                                                        commit.message.clone()
                                                    };
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
                                                    ui.with_layout(
                                                        egui::Layout::right_to_left(
                                                            egui::Align::Center,
                                                        ),
                                                        |ui| {
                                                            ui.label(
                                                                RichText::new(&commit.date)
                                                                    .size(10.0)
                                                                    .color(TEXT_MUTED),
                                                            );
                                                        },
                                                    );
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
        } else {
            // Empty slot button
            let slot_label = RichText::new(format!("{} {} image", icon_str(Icon::Plus), label))
                .size(12.0)
                .color(TEXT_MUTED);
            let slot_btn = egui::Button::new(slot_label)
                .fill(BG_CARD)
                .corner_radius(egui::CornerRadius::same(8))
                .stroke(Stroke::new(1.0, BORDER));
            if ui.add(slot_btn).clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Images", &["png", "jpg", "jpeg", "webp", "bmp", "gif"])
                    .pick_file()
                {
                    self.load_image_to_texture(ctx, path, is_left);
                }
            }
        }
    }
}

// ── App::update ──────────────────────────────────────────────────────────────

impl eframe::App for ImageDifferApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ── Load pending images (deferred from startup args) ──────────────────
        if self.pending_left.is_some() || self.pending_right.is_some() {
            if let Some(p) = self.pending_left.take() {
                self.load_image_to_texture(ctx, p, true);
            }
            if let Some(p) = self.pending_right.take() {
                self.load_image_to_texture(ctx, p, false);
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
                        self.load_image_to_texture(ctx, p, true);
                    }
                }
                if let Some(f) = iter.next() {
                    if let Some(p) = f.path {
                        self.load_image_to_texture(ctx, p, false);
                    }
                }
            }
        });

        // ── Footer ───────────────────────────────────────────────────────────
        egui::TopBottomPanel::bottom("footer")
            .frame(
                egui::Frame::NONE
                    .fill(BG_DEEP)
                    .stroke(Stroke::new(1.0, BORDER)),
            )
            .show(ctx, |ui| {
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
        egui::TopBottomPanel::top("header")
            .exact_height(52.0)
            .frame(
                egui::Frame::NONE
                    .fill(BG_SURFACE)
                    .stroke(Stroke::new(1.0, BORDER)),
            )
            .show(ctx, |ui| {
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.add_space(16.0);

                    // Logo
                    ui.label(
                        RichText::new(icon_str(Icon::Aperture))
                            .size(20.0)
                            .color(ACCENT)
                            .font(FontId::new(20.0, FontFamily::Name("lucide".into()))),
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
                    self.render_slot_header(ui, true, "Original", ctx);
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(icon_str(Icon::ArrowLeftRight))
                            .size(14.0)
                            .color(TEXT_DIM)
                            .font(FontId::new(14.0, FontFamily::Name("lucide".into()))),
                    );
                    ui.add_space(8.0);
                    self.render_slot_header(ui, false, "New", ctx);

                    // Right side actions
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(16.0);

                        // Open Pair
                        let pair_btn = egui::Button::new(
                            RichText::new(format!("{} Open Pair", icon_str(Icon::FolderOpen)))
                                .size(12.0)
                                .color(BG_DEEP),
                        )
                        .fill(ACCENT)
                        .corner_radius(egui::CornerRadius::same(6u8))
                        .stroke(Stroke::NONE);
                        if ui.add(pair_btn).clicked() {
                            if let Some(paths) = rfd::FileDialog::new()
                                .add_filter("Images", &["png", "jpg", "jpeg", "webp", "bmp", "gif"])
                                .pick_files()
                            {
                                let mut iter = paths.into_iter();
                                if let Some(p) = iter.next() {
                                    self.load_image_to_texture(ctx, p, true);
                                }
                                if let Some(p) = iter.next() {
                                    self.load_image_to_texture(ctx, p, false);
                                }
                            }
                        }

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
            egui::TopBottomPanel::top("mode_toolbar")
                .exact_height(40.0)
                .frame(
                    egui::Frame::NONE
                        .fill(BG_DEEP)
                        .stroke(Stroke::new(1.0, BORDER)),
                )
                .show(ctx, |ui| {
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
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(BG_DEEP))
            .show(ctx, |ui| {
                if has_both {
                    // Clone handles up front to avoid borrow conflicts
                    let tex_left = self.texture_left.as_ref().unwrap().clone();
                    let tex_right = self.texture_right.as_ref().unwrap().clone();

                    match self.diff_mode {
                        DiffMode::Slider => {
                            let avail = ui.available_size();
                            let (rect, resp) = ui.allocate_exact_size(avail, egui::Sense::drag());
                            let p = ui.painter();

                            // Left image full
                            p.image(
                                tex_left.id(),
                                rect,
                                egui::Rect::from_min_max(
                                    egui::pos2(0.0, 0.0),
                                    egui::pos2(1.0, 1.0),
                                ),
                                Color32::WHITE,
                            );

                            // Right image clipped
                            let sx = rect.left() + rect.width() * self.slider_pos;
                            let clip = egui::Rect::from_min_max(
                                egui::pos2(sx, rect.top()),
                                egui::pos2(rect.right(), rect.bottom()),
                            );
                            ui.painter_at(clip).image(
                                tex_right.id(),
                                rect,
                                egui::Rect::from_min_max(
                                    egui::pos2(0.0, 0.0),
                                    egui::pos2(1.0, 1.0),
                                ),
                                Color32::WHITE,
                            );

                            let p = ui.painter();

                            // Corner labels
                            let lbg = Color32::from_black_alpha(160);
                            let pad = egui::vec2(8.0, 4.0);
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

                            let disp_l = fit_in_rect(tex_left.size_vec2(), left_rect);
                            let disp_r = fit_in_rect(tex_right.size_vec2(), right_rect);

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
                            self.generate_diff_texture(ctx);
                            let avail = ui.available_size();
                            let pos = ui.cursor().min;
                            let (_, _) = ui.allocate_exact_size(avail, egui::Sense::hover());
                            let p = ui.painter();

                            if let Some(ref tex_diff) = self.texture_diff {
                                let container = egui::Rect::from_min_size(pos, avail);
                                let disp = fit_in_rect(tex_diff.size_vec2(), container);
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
                            let disp = fit_in_rect(tex_left.size_vec2(), container);
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
                                        .add_filter(
                                            "Images",
                                            &["png", "jpg", "jpeg", "webp", "bmp", "gif"],
                                        )
                                        .pick_file()
                                    {
                                        let is_left_empty = self.texture_left.is_none();
                                        self.load_image_to_texture(ctx, p, is_left_empty);
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
                                let r = self.texture_right.is_some();
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 16.0;
                                    // Slot 1
                                    let (s1_fill, s1_text, s1_label) = if l {
                                        (
                                            SUCCESS.gamma_multiply(0.12),
                                            SUCCESS,
                                            format!("{} Original  ✓", icon_str(Icon::Image)),
                                        )
                                    } else {
                                        (
                                            Color32::TRANSPARENT,
                                            TEXT_MUTED,
                                            format!("{} Load Original", icon_str(Icon::Image)),
                                        )
                                    };
                                    let b1 = egui::Button::new(
                                        RichText::new(s1_label).size(13.0).color(s1_text),
                                    )
                                    .fill(s1_fill)
                                    .stroke(Stroke::new(
                                        1.0,
                                        if l {
                                            SUCCESS.gamma_multiply(0.4)
                                        } else {
                                            BORDER
                                        },
                                    ))
                                    .corner_radius(egui::CornerRadius::same(8u8))
                                    .min_size(egui::vec2(160.0, 44.0));
                                    if ui.add(b1).clicked() && !l {
                                        if let Some(p) = rfd::FileDialog::new()
                                            .add_filter(
                                                "Images",
                                                &["png", "jpg", "jpeg", "webp", "bmp"],
                                            )
                                            .pick_file()
                                        {
                                            self.load_image_to_texture(ctx, p, true);
                                        }
                                    }
                                    // Slot 2
                                    let (s2_fill, s2_text, s2_label) = if r {
                                        (
                                            SUCCESS.gamma_multiply(0.12),
                                            SUCCESS,
                                            format!("{} New  ✓", icon_str(Icon::Image)),
                                        )
                                    } else {
                                        (
                                            ACCENT.gamma_multiply(0.1),
                                            ACCENT,
                                            format!("{} Load New", icon_str(Icon::ImagePlus)),
                                        )
                                    };
                                    let b2 = egui::Button::new(
                                        RichText::new(s2_label).size(13.0).color(s2_text),
                                    )
                                    .fill(s2_fill)
                                    .stroke(Stroke::new(
                                        1.0,
                                        if r {
                                            SUCCESS.gamma_multiply(0.4)
                                        } else {
                                            ACCENT.gamma_multiply(0.4)
                                        },
                                    ))
                                    .corner_radius(egui::CornerRadius::same(8u8))
                                    .min_size(egui::vec2(160.0, 44.0));
                                    if ui.add(b2).clicked() && !r {
                                        if let Some(p) = rfd::FileDialog::new()
                                            .add_filter(
                                                "Images",
                                                &["png", "jpg", "jpeg", "webp", "bmp"],
                                            )
                                            .pick_file()
                                        {
                                            self.load_image_to_texture(ctx, p, false);
                                        }
                                    }
                                });
                            });
                        });
                    }
                } else {
                    // ── Empty State ─────────────────────────────────────────
                    let avail = ui.available_size();
                    ui.centered_and_justified(|ui| {
                        ui.vertical_centered(|ui| {
                            ui.add_space(32.0);
                            ui.label(icon_text(Icon::Layers, 48.0).color(ACCENT));
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

                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 20.0;
                                let card_size = egui::vec2(280.0, 200.0);

                                for (is_left, slot_label, icon) in [
                                    (true, "Original", Icon::Image),
                                    (false, "New", Icon::ImagePlus),
                                ] {
                                    egui::Frame::NONE
                                        .fill(BG_CARD)
                                        .corner_radius(egui::CornerRadius::same(12))
                                        .stroke(Stroke::new(1.0, BORDER))
                                        .inner_margin(egui::Margin::same(24))
                                        .show(ui, |ui| {
                                            ui.set_min_size(card_size);
                                            ui.vertical_centered(|ui| {
                                                ui.add_space(24.0);
                                                ui.label(icon_text(icon, 36.0).color(TEXT_DIM));
                                                ui.add_space(16.0);
                                                ui.label(
                                                    RichText::new(slot_label)
                                                        .size(14.0)
                                                        .color(TEXT)
                                                        .strong(),
                                                );
                                                ui.add_space(4.0);
                                                ui.label(
                                                    RichText::new("image")
                                                        .size(12.0)
                                                        .color(TEXT_MUTED),
                                                );
                                                ui.add_space(20.0);
                                                let browse_btn = egui::Button::new(
                                                    RichText::new("Browse File")
                                                        .size(12.0)
                                                        .color(TEXT),
                                                )
                                                .fill(BG_ELEVATED)
                                                .stroke(Stroke::new(1.0, BORDER))
                                                .corner_radius(egui::CornerRadius::same(6u8))
                                                .min_size(egui::vec2(120.0, 32.0));
                                                if ui.add(browse_btn).clicked() {
                                                    if let Some(p) = rfd::FileDialog::new()
                                                        .add_filter(
                                                            "Images",
                                                            &[
                                                                "png", "jpg", "jpeg", "webp",
                                                                "bmp", "gif",
                                                            ],
                                                        )
                                                        .pick_file()
                                                    {
                                                        self.load_image_to_texture(ctx, p, is_left);
                                                    }
                                                }
                                            });
                                        });
                                }
                            });
                        });
                    });
                }
            });
    }
}
