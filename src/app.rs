use eframe::egui;
use eframe::epaint::ColorImage;
use eframe::epaint::TextureHandle;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::types::{DiffMode, GitCommit, GitContext, StartupConfig};

pub struct ImageDifferApp {
    pub(crate) texture_left: Option<TextureHandle>,
    pub(crate) texture_right: Option<TextureHandle>,
    pub(crate) texture_diff: Option<TextureHandle>,
    pub(crate) image_left: Option<image::RgbaImage>,
    pub(crate) image_right: Option<image::RgbaImage>,
    pub(crate) path_left: Option<PathBuf>,
    pub(crate) path_right: Option<PathBuf>,
    pub(crate) slider_pos: f32,
    pub(crate) fade_opacity: f32,
    pub(crate) diff_mode: DiffMode,
    pub(crate) diff_pixel_pct: Option<f64>,
    pub(crate) error_msg: Option<String>,
    pub(crate) commits_left: Vec<GitCommit>,
    pub(crate) commits_right: Vec<GitCommit>,
    pub(crate) git_context: Option<GitContext>,
    pub(crate) pending_left: Option<PathBuf>,
    pub(crate) pending_right: Option<PathBuf>,
}

impl ImageDifferApp {
    pub fn new(_cc: &eframe::CreationContext<'_>, cfg: StartupConfig) -> Self {
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
            commits_left: Vec::new(),
            commits_right: Vec::new(),
            git_context: cfg.git_context,
            pending_left: cfg.left_path,
            pending_right: cfg.right_path,
        }
    }

    pub fn get_git_history(&self, path: &Path) -> Vec<GitCommit> {
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

    pub fn load_from_git(&mut self, ctx: &egui::Context, path: &Path, rev: &str, is_left: bool) {
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

    pub fn is_in_git_repo(&self, path: &Path) -> bool {
        Command::new("git")
            .arg("rev-parse")
            .arg("--is-inside-work-tree")
            .current_dir(path.parent().unwrap_or(Path::new(".")))
            .output()
            .map(|o| o.status.success() && String::from_utf8_lossy(&o.stdout).trim() == "true")
            .unwrap_or(false)
    }

    pub fn load_image_to_texture(&mut self, ctx: &egui::Context, path: PathBuf, is_left: bool) {
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

    pub fn generate_diff_texture(&mut self, ctx: &egui::Context) {
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

    pub fn clear_all(&mut self) {
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
}
