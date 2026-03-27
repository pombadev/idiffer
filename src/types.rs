use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct GitCommit {
    pub hash: String,
    pub message: String,
    pub date: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffMode {
    Slider,
    SideBySide,
    Difference,
    Fade,
}

impl Default for DiffMode {
    fn default() -> Self {
        DiffMode::Slider
    }
}

/// Metadata about how the app was launched from git.
#[derive(Clone, Debug, Default)]
pub struct GitContext {
    /// e.g. "icon.png" — the file being diffed
    pub filename: String,
    /// The old blob hash (7-char)
    pub old_rev: String,
    /// The new blob hash (7-char)
    pub new_rev: String,
}

/// Startup configuration parsed from args / stdin before the UI is created.
#[derive(Default)]
pub struct StartupConfig {
    pub left_path: Option<PathBuf>,
    pub right_path: Option<PathBuf>,
    pub git_context: Option<GitContext>,
    pub initial_mode: DiffMode,
}
