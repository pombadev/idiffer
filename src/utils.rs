use std::path::PathBuf;

pub fn pick_image_file() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter("Images", &["png", "jpg", "jpeg", "webp", "bmp", "gif"])
        .pick_file()
}

pub fn pick_image_files() -> Option<Vec<PathBuf>> {
    rfd::FileDialog::new()
        .add_filter("Images", &["png", "jpg", "jpeg", "webp", "bmp", "gif"])
        .pick_files()
}
