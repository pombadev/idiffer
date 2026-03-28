# IDIFFER

Image comparison tool with git integration and various comparison modes.
Built with Rust and the egui framework

## Features

- **Precision Diffing**:
  - Difference
  - Fade modes

- **Git Integration**: Directly browse and compare file versions from your Git history

- **Dynamic Comparison Modes**:
  - Slider
  - Side-by-Side

- **Cross-Platform**: Designed for Linux (Wayland/X11), macOS, and Windows

## Installation

### From Source

Ensure you have the latest stable Rust toolchain installed:

```bash
cargo install --git https://github.com/pombadev/idiffer.git
```

### Git Integration

Test it out:

```bash
git difftool --tool=idiffer HEAD somehash -- '*.png'
```

To use IDIFFER as your global git difftool:

```bash
# Register idiffer with git
git config --global difftool.idiffer.cmd 'idiffer "$LOCAL" "$REMOTE"'
git config --global diff.tool idiffer
```

Compare versions from your workspace:

```bash
# Compare a file with its staged version
git difftool some_file.png

# Compare a file across two commits
git difftool HEAD~1 HEAD -- some_file.png
```

## Usage

Simply run the application or pass paths as arguments:

```bash
# Open two local files
idiffer image1.png image2.png

# Open from within a git repo to automatically detect history
idiffer some_file.webp
```

## License

This project is licensed under the **Apache License 2.0**. See the [LICENSE](LICENSE) file for details.
