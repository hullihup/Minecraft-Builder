//! Editor for the disconnect screen of the "Reason (screen)" block: shows the
//! text like Minecraft does when a player is banned or kicked.

use crate::app::App;
use crate::i18n::{t, tl};
use crate::theme;
use crate::ui::menu_editor::mc_text;
use egui::{Color32, FontId, Id, Rect, RichText, Sense, Stroke, StrokeKind, pos2, vec2};

const COLORS: &[(char, u32)] = &[
    ('0', 0x000000),
    ('1', 0x0000AA),
    ('2', 0x00AA00),
    ('3', 0x00AAAA),
    ('4', 0xAA0000),
    ('5', 0xAA00AA),
    ('6', 0xFFAA00),
    ('7', 0xAAAAAA),
    ('8', 0x555555),
    ('9', 0x5555FF),
    ('a', 0x55FF55),
    ('b', 0x55FFFF),
    ('c', 0xFF5555),
    ('d', 0xFF55FF),
    ('e', 0xFFFF55),
    ('f', 0xFFFFFF),
];

/// Example values for the placeholders in the preview.
pub fn preview_text(text: &str, category: &str) -> String {
    text.replace("{time}", "29d 17h 34m 51s")
        .replace("{player}", "Steve")
        .replace("{category}", category)
}

/// The dark dirt background of Minecraft's menu screens.
pub fn dirt(p: &egui::Painter, r: Rect) {
    p.rect_filled(r, 0.0, Color32::from_rgb(0x2A, 0x1E, 0x14));
    let tile = 8.0;
    let (nx, ny) = (
        (r.width() / tile) as i32 + 1,
        (r.height() / tile) as i32 + 1,
    );
    for y in 0..ny {
        for x in 0..nx {
            // Stable pseudo random shade per tile.
            let h = (x as u32).wrapping_mul(374761393) ^ (y as u32).wrapping_mul(668265263);
            let h = (h ^ (h >> 13)).wrapping_mul(1274126177);
            let v = (h >> 24) % 5;
            if v == 0 {
                continue;
            }
            let c = match v {
                1 => Color32::from_rgb(0x33, 0x25, 0x19),
                2 => Color32::from_rgb(0x24, 0x1A, 0x11),
                3 => Color32::from_rgb(0x2F, 0x23, 0x1A),
                _ => Color32::from_rgb(0x20, 0x17, 0x0F),
            };
            let tr = Rect::from_min_size(
                r.min + vec2(x as f32 * tile, y as f32 * tile),
                vec2(tile, tile),
            );
            p.rect_filled(tr.intersect(r), 0.0, c);
        }
    }
}

/// Draws the screen: heading, the text lines centered, the button.
pub fn draw_screen(ui: &egui::Ui, r: Rect, text: &str, scale: f32) {
    let p = ui.painter().with_clip_rect(r);
    dirt(&p, r);
    let lines: Vec<&str> = text.split('\n').collect();
    let lh = 15.0 * scale;
    let total = (lines.len() as f32 + 4.2) * lh;
    let mut y = (r.center().y - total / 2.0).max(r.top() + 6.0 * scale);
    p.text(
        pos2(r.center().x, y),
        egui::Align2::CENTER_TOP,
        t(
            "Verbindung zum Server fehlgeschlagen",
            "Failed to connect to the server",
        ),
        FontId::monospace(12.0 * scale),
        Color32::from_rgb(0xAA, 0xAA, 0xAA),
    );
    y += lh * 1.9;
    for line in lines {
        let g = ui.fonts_mut(|f| f.layout_job(mc_text(line, Color32::WHITE, 12.0 * scale)));
        p.galley(pos2(r.center().x - g.size().x / 2.0, y), g, Color32::WHITE);
        y += lh;
    }
    y += lh * 0.8;
    let br = Rect::from_center_size(
        pos2(r.center().x, y + lh * 0.7),
        vec2(200.0 * scale, lh * 1.4),
    );
    p.rect(
        br,
        0.0,
        Color32::from_rgb(0x6F, 0x6F, 0x6F),
        Stroke::new(scale.max(1.0), Color32::BLACK),
        StrokeKind::Inside,
    );
    p.text(
        br.center(),
        egui::Align2::CENTER_CENTER,
        t("Zurück zur Serverliste", "Back to server list"),
        FontId::monospace(11.0 * scale),
        Color32::WHITE,
    );
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    let Some((ws, nid)) = app.screen_edit else {
        return;
    };
    let Some(node) = app
        .project
        .workspaces
        .get(ws)
        .and_then(|w| w.node(nid))
        .cloned()
    else {
        app.screen_edit = None;
        return;
    };
    let def = crate::blocks::get(&node.kind);
    let field = |k: &str| {
        node.values.get(k).cloned().unwrap_or_else(|| {
            def.and_then(|d| d.field(k))
                .map(|f| f.default.to_string())
                .unwrap_or_default()
        })
    };
    let mut text = field("screen");
    let mut category = field("category");
    let before = (text.clone(), category.clone());
    let cats: Vec<(&str, crate::i18n::L)> =
        match def.and_then(|d| d.field("category")).map(|f| &f.kind) {
            Some(crate::blocks::FieldKind::Choice(o)) => o.to_vec(),
            _ => vec![],
        };
    let mut close = false;
    let modal = egui::Modal::new(Id::unique("screen_editor"))
        .frame(
            egui::Frame::new()
                .fill(Color32::from_rgb(0x20, 0x22, 0x26))
                .stroke(Stroke::new(1.0, theme::BORDER))
                .corner_radius(12)
                .inner_margin(20),
        )
        .show(ctx, |ui| {
            ui.set_width(1000.0);
            ui.horizontal(|ui| {
                crate::ui::icons::icon(ui, crate::ui::icons::Icon::Warning, 20.0, crate::blocks::Cat::Moderation.color());
                ui.label(RichText::new(t("Bildschirm bearbeiten", "Edit screen")).size(19.0).strong());
                ui.label(
                    RichText::new(t(
                        "Das sieht der Spieler, wenn er gebannt oder gekickt wird",
                        "This is what the player sees when banned or kicked",
                    ))
                    .size(11.5)
                    .color(theme::TEXT_WEAK),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if crate::ui::icons::icon_button(ui, crate::ui::icons::Icon::Close, 24.0, "").clicked() {
                        close = true;
                    }
                });
            });
            ui.add_space(10.0);
            ui.horizontal_top(|ui| {
                let (r, _) = ui.allocate_exact_size(vec2(600.0, 360.0), Sense::hover());
                draw_screen(ui, r, &preview_text(&text, &category), 1.25);
                ui.painter().rect_stroke(r, 0.0, Stroke::new(1.0, Color32::BLACK), StrokeKind::Outside);
                ui.add_space(16.0);
                ui.vertical(|ui| {
                    ui.set_width(360.0);
                    ui.label(RichText::new(t("Kategorie", "Category")).size(11.5).color(theme::TEXT_WEAK));
                    egui::ComboBox::from_id_salt(("screen_cat", nid))
                        .width(ui.available_width())
                        .selected_text(
                            cats.iter()
                                .find(|c| c.0 == category)
                                .map(|c| tl(c.1))
                                .unwrap_or(category.as_str())
                                .to_string(),
                        )
                        .show_ui(ui, |ui| {
                            for (k, l) in &cats {
                                ui.selectable_value(&mut category, k.to_string(), tl(*l));
                            }
                        });
                    ui.add_space(8.0);
                    ui.label(RichText::new(t("Text (eine Zeile pro Zeile)", "Text (one line per line)")).size(11.5).color(theme::TEXT_WEAK));
                    ui.add(
                        egui::TextEdit::multiline(&mut text)
                            .font(egui::TextStyle::Monospace)
                            .desired_rows(8)
                            .desired_width(f32::INFINITY),
                    );
                    ui.add_space(6.0);
                    ui.label(RichText::new(t("Farbe anhängen", "Append color")).size(11.5).color(theme::TEXT_WEAK));
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = vec2(3.0, 3.0);
                        for (code, rgb) in COLORS {
                            let col = Color32::from_rgb((rgb >> 16) as u8, (rgb >> 8) as u8, *rgb as u8);
                            let (cr, resp) = ui.allocate_exact_size(vec2(19.0, 19.0), Sense::click());
                            ui.painter().rect(cr, 3.0, col, Stroke::new(1.0, Color32::from_gray(90)), StrokeKind::Inside);
                            if resp.on_hover_text(format!("&{code}")).clicked() {
                                text.push_str(&format!("&{code}"));
                            }
                        }
                    });
                    ui.horizontal_wrapped(|ui| {
                        for (code, l) in [
                            ("&l", t("Fett", "Bold")),
                            ("&o", t("Kursiv", "Italic")),
                            ("&n", t("Unterstrichen", "Underlined")),
                            ("&r", t("Zurücksetzen", "Reset")),
                            ("\n", t("Neue Zeile", "New line")),
                        ] {
                            if ui.small_button(l).clicked() {
                                text.push_str(code);
                            }
                        }
                    });
                    ui.add_space(6.0);
                    ui.label(RichText::new(t("Platzhalter anhängen", "Append placeholder")).size(11.5).color(theme::TEXT_WEAK));
                    ui.horizontal_wrapped(|ui| {
                        for (ph, tip) in [
                            ("{time}", t("Restzeit des Banns, z. B. 29d 17h 34m 51s", "Time left of the ban, e.g. 29d 17h 34m 51s")),
                            ("{player}", t("Name des Spielers", "Name of the player")),
                            ("{category}", t("Die Kategorie oben", "The category above")),
                        ] {
                            if ui.small_button(ph).on_hover_text(tip).clicked() {
                                text.push_str(ph);
                            }
                        }
                    });
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(t(
                            "Minecraft zeigt den Text immer zentriert und nur mit Farben und Zeilenumbrüchen – Bilder oder Knöpfe gehen dort nicht. Die Überschrift und der Knopf kommen vom Spiel. {time} wird nur beim Bannen ersetzt.",
                            "Minecraft always shows the text centered and only with colors and line breaks – pictures or buttons are not possible there. The heading and the button come from the game. {time} is only replaced when banning.",
                        ))
                        .size(11.0)
                        .color(theme::TEXT_WEAK),
                    );
                });
            });
            ui.add_space(12.0);
            if ui
                .add(
                    egui::Button::new(RichText::new(t("Fertig", "Done")).color(Color32::BLACK).strong())
                        .fill(theme::ACCENT)
                        .corner_radius(6)
                        .min_size(vec2(110.0, 30.0)),
                )
                .clicked()
            {
                close = true;
            }
        });
    if modal.should_close() {
        close = true;
    }
    if (text.clone(), category.clone()) != before {
        if let Some(n) = app.project.workspaces[ws].node_mut(nid) {
            n.values.insert("screen".into(), text);
            n.values.insert("category".into(), category);
        }
    }
    if close {
        app.screen_edit = None;
    }
}
