//! Dark theme inspired by modern node editors: near-black panels, cyan accent,
//! green "run" color.

use egui::{Color32, CornerRadius, Stroke, Visuals};

pub const BG_PANEL: Color32 = Color32::from_rgb(0x1B, 0x1C, 0x1F);
pub const BG_SIDEBAR: Color32 = Color32::from_rgb(0x1F, 0x20, 0x24);
pub const BG_CANVAS: Color32 = Color32::from_rgb(0x24, 0x26, 0x2A);
pub const GRID_DOT: Color32 = Color32::from_rgb(0x3A, 0x3D, 0x43);
pub const NODE_BG: Color32 = Color32::from_rgb(0x2C, 0x2E, 0x33);
pub const NODE_HEADER: Color32 = Color32::from_rgb(0x34, 0x37, 0x3D);
pub const NODE_BORDER: Color32 = Color32::from_rgb(0x44, 0x47, 0x4E);
pub const INPUT_BG: Color32 = Color32::from_rgb(0x1E, 0x1F, 0x23);
pub const BORDER: Color32 = Color32::from_rgb(0x30, 0x32, 0x37);
pub const TEXT: Color32 = Color32::from_rgb(0xDA, 0xDD, 0xE3);
pub const TEXT_WEAK: Color32 = Color32::from_rgb(0x8E, 0x93, 0x9C);
pub const ACCENT: Color32 = Color32::from_rgb(0x00, 0xE5, 0xFF);
pub const ACCENT_BG: Color32 = Color32::from_rgb(0x16, 0x3A, 0x44);
pub const GREEN: Color32 = Color32::from_rgb(0x3D, 0xFF, 0x6E);
pub const RED: Color32 = Color32::from_rgb(0xFF, 0x5C, 0x5C);
pub const YELLOW: Color32 = Color32::from_rgb(0xFF, 0xC8, 0x4D);
pub const BLUE: Color32 = Color32::from_rgb(0x4D, 0xA3, 0xFF);

/// Colors for note frames.
pub const NOTE_COLORS: [Color32; 6] = [
    Color32::from_rgb(0x3D, 0xFF, 0x6E),
    Color32::from_rgb(0x00, 0xE5, 0xFF),
    Color32::from_rgb(0xB0, 0x7C, 0xFF),
    Color32::from_rgb(0xFF, 0xC8, 0x4D),
    Color32::from_rgb(0xFF, 0x7A, 0xC6),
    Color32::from_rgb(0x9A, 0xA4, 0xB2),
];

pub fn apply(ctx: &egui::Context) {
    ctx.set_theme(egui::Theme::Dark);
    ctx.all_styles_mut(style_mut);
}

fn style_mut(style: &mut egui::Style) {
    let mut v = Visuals::dark();
    v.panel_fill = BG_PANEL;
    v.window_fill = Color32::from_rgb(0x23, 0x25, 0x29);
    v.window_stroke = Stroke::new(1.0, BORDER);
    v.window_corner_radius = CornerRadius::same(8);
    v.menu_corner_radius = CornerRadius::same(6);
    v.extreme_bg_color = INPUT_BG;
    v.text_edit_bg_color = Some(INPUT_BG);
    v.faint_bg_color = Color32::from_rgb(0x22, 0x24, 0x28);
    v.code_bg_color = Color32::from_rgb(0x1A, 0x1B, 0x1E);
    v.hyperlink_color = ACCENT;
    v.warn_fg_color = YELLOW;
    v.error_fg_color = RED;
    v.selection.bg_fill = Color32::from_rgb(0x0E, 0x5A, 0x66);
    v.selection.stroke = Stroke::new(1.0, ACCENT);
    v.override_text_color = None;

    let r = CornerRadius::same(5);
    v.widgets.noninteractive.bg_fill = BG_PANEL;
    v.widgets.noninteractive.weak_bg_fill = BG_PANEL;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT);
    v.widgets.noninteractive.corner_radius = r;

    v.widgets.inactive.bg_fill = Color32::from_rgb(0x2A, 0x2C, 0x31);
    v.widgets.inactive.weak_bg_fill = Color32::from_rgb(0x2A, 0x2C, 0x31);
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(0x3A, 0x3D, 0x43));
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT);
    v.widgets.inactive.corner_radius = r;

    v.widgets.hovered.bg_fill = Color32::from_rgb(0x33, 0x36, 0x3C);
    v.widgets.hovered.weak_bg_fill = Color32::from_rgb(0x33, 0x36, 0x3C);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, Color32::from_rgb(0x55, 0x59, 0x61));
    v.widgets.hovered.fg_stroke = Stroke::new(1.5, Color32::WHITE);
    v.widgets.hovered.corner_radius = r;

    v.widgets.active.bg_fill = Color32::from_rgb(0x1C, 0x4A, 0x54);
    v.widgets.active.weak_bg_fill = Color32::from_rgb(0x1C, 0x4A, 0x54);
    v.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT);
    v.widgets.active.fg_stroke = Stroke::new(1.5, Color32::WHITE);
    v.widgets.active.corner_radius = r;

    v.widgets.open = v.widgets.active;

    style.visuals = v;
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(10.0, 4.0);
    style.spacing.interact_size.y = 24.0;
    style.spacing.menu_margin = egui::Margin::same(6);
    style.spacing.window_margin = egui::Margin::same(14);
    style.spacing.combo_height = 320.0;
    style.interaction.selectable_labels = false;
}

/// Window icon (64x64 RGBA, same artwork as the .exe icon).
pub fn app_icon() -> egui::IconData {
    egui::IconData {
        rgba: include_bytes!("../assets/icon64.rgba").to_vec(),
        width: 64,
        height: 64,
    }
}

/// The logo as a texture (cached).
pub fn logo(ctx: &egui::Context) -> egui::TextureHandle {
    let id = egui::Id::unique("mcb_logo_texture");
    if let Some(t) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(id)) {
        return t;
    }
    let img =
        egui::ColorImage::from_rgba_unmultiplied([64, 64], include_bytes!("../assets/icon64.rgba"));
    let t = ctx.load_texture("mcb_logo", img, egui::TextureOptions::LINEAR);
    ctx.data_mut(|d| d.insert_temp(id, t.clone()));
    t
}
