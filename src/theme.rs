use eframe::egui;
use egui::{Color32, FontData, FontDefinitions, FontFamily, FontId, RichText, Stroke};
use lucide_icons::{Icon, LUCIDE_FONT_BYTES};

// ── Palette ────────────────────────────────────────────────────────────────
pub const ACCENT: Color32 = Color32::from_rgb(0x06, 0xb6, 0xd4);
pub const BG_DEEP: Color32 = Color32::from_rgb(0x09, 0x09, 0x0e);
pub const BG_SURFACE: Color32 = Color32::from_rgb(0x10, 0x10, 0x18);
pub const BG_CARD: Color32 = Color32::from_rgb(0x18, 0x18, 0x22);
pub const BG_ELEVATED: Color32 = Color32::from_rgb(0x22, 0x22, 0x2e);
pub const BORDER: Color32 = Color32::from_rgb(0x28, 0x28, 0x38);
pub const TEXT: Color32 = Color32::from_rgb(0xe0, 0xe6, 0xf0);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(0x58, 0x68, 0x82);
pub const TEXT_DIM: Color32 = Color32::from_rgb(0x30, 0x38, 0x48);
pub const SUCCESS: Color32 = Color32::from_rgb(0x10, 0xb9, 0x81);
pub const DANGER: Color32 = Color32::from_rgb(0xf8, 0x71, 0x71);

// ── Font / Visual Setup ─────────────────────────────────────────────────────

pub fn setup_fonts(ctx: &egui::Context) {
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

pub fn setup_visuals(ctx: &egui::Context) {
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

    let mut style: egui::Style = (*ctx.global_style()).clone();
    style.spacing.item_spacing = egui::vec2(6.0, 4.0);
    style.spacing.button_padding = egui::vec2(10.0, 5.0);
    style.spacing.window_margin = egui::Margin::same(0);
    style.spacing.menu_margin = egui::Margin::same(6);
    ctx.set_global_style(style);
}

pub fn icon_text(icon: Icon, size: f32) -> RichText {
    RichText::new(icon.unicode().to_string())
        .font(FontId::new(size, FontFamily::Name("lucide".into())))
}

pub fn icon_str(icon: Icon) -> String {
    icon.unicode().to_string()
}

/// Fit an image into a container rect maintaining aspect ratio (letterbox).
pub fn fit_in_rect(img_size: egui::Vec2, container: egui::Rect) -> egui::Rect {
    let scale = (container.width() / img_size.x).min(container.height() / img_size.y);
    let sz = img_size * scale;
    egui::Rect::from_center_size(container.center(), sz)
}

pub fn calc_image_rect(
    img_size: egui::Vec2,
    container: egui::Rect,
    zoom: f32,
    pan: egui::Vec2,
) -> egui::Rect {
    if zoom <= 0.0 {
        fit_in_rect(img_size, container)
    } else {
        egui::Rect::from_center_size(container.center() + pan, img_size * zoom)
    }
}
