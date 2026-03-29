use std::io::{IsTerminal, Read};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::types::{DiffMode, GitContext, StartupConfig};

// ── CLI / Stdin Parsing ──────────────────────────────────────────────────────

/// Write a git blob to a temp file and return its path.
pub fn extract_git_blob(hash: &str, hint_ext: &str) -> Option<PathBuf> {
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
pub fn parse_git_diff_stdin() -> StartupConfig {
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

pub fn parse_args() -> StartupConfig {
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

    // Filter out empty arguments (can happen if $LOCAL/$REMOTE are empty strings in git config)
    let non_empty_args: Vec<String> = args.iter().filter(|a| !a.is_empty()).cloned().collect();

    // Handle Git External Diff format (7 arguments)
    // path old-file old-hex old-mode new-file new-hex new-mode
    if non_empty_args.len() == 7 {
        let filename = non_empty_args[0].clone();
        let old_file = non_empty_args[1].clone();
        let new_file = non_empty_args[4].clone();
        let old_rev = non_empty_args[2].chars().take(7).collect();
        let new_rev = non_empty_args[5].chars().take(7).collect();

        return StartupConfig {
            left_path: Some(PathBuf::from(old_file)),
            right_path: Some(PathBuf::from(new_file)),
            initial_mode: DiffMode::Slider,
            git_context: Some(GitContext {
                filename: Path::new(&filename)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or(filename),
                old_rev,
                new_rev,
            }),
        };
    }

    // Traditional two path args → difftool mode ($LOCAL $REMOTE)
    let paths: Vec<PathBuf> = non_empty_args
        .iter()
        .filter(|a| !a.starts_with('-'))
        .map(|a| PathBuf::from(a))
        .collect();

    if paths.len() >= 2 {
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
pub fn shorten_path_hint(p: &Path) -> String {
    p.file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.chars().take(7).collect())
        .unwrap_or_default()
}
