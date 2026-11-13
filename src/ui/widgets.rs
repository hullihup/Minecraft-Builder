//! Small reusable widgets: value editors, toggle switch, icon buttons.

use crate::blocks::{FieldDef, FieldKind, PinType};
use crate::i18n::{t, tl};
use crate::theme;
use egui::{Color32, Response, Sense, Ui, Vec2};
use std::collections::BTreeMap;

/// iOS-style switch.
pub fn toggle(ui: &mut Ui, on: &mut bool) -> Response {
    let size = Vec2::new(34.0, 18.0);
    let (rect, mut resp) = ui.allocate_exact_size(size, Sense::click());
    if resp.clicked() {
        *on = !*on;
        resp.mark_changed();
    }
    let how_on = ui.ctx().animate_bool_responsive(resp.id, *on);
    let bg = if *on {
        Color32::from_rgb(0x1C, 0x8F, 0x4A)
    } else {
        Color32::from_rgb(0x3A, 0x3D, 0x43)
    };
    let r = rect.height() / 2.0;
    ui.painter().rect_filled(rect, r, bg);
    let cx = egui::lerp((rect.left() + r)..=(rect.right() - r), how_on);
    ui.painter().circle_filled(
        egui::pos2(cx, rect.center().y),
        r - 3.0,
        Color32::from_rgb(0xE8, 0xEA, 0xEE),
    );
    resp
}

/// Gets a value from a node's map, falling back to a default.
pub fn get_val(values: &BTreeMap<String, String>, id: &str, default: &str) -> String {
    values
        .get(id)
        .cloned()
        .unwrap_or_else(|| default.to_string())
}

/// Editor for a block field. Returns true if changed.
pub fn field_editor(
    ui: &mut Ui,
    f: &FieldDef,
    values: &mut BTreeMap<String, String>,
    width: f32,
    id_salt: impl std::hash::Hash + std::fmt::Debug,
) -> bool {
    let mut v = get_val(values, f.id, f.default);
    let before = v.clone();
    match &f.kind {
        FieldKind::Text => {
            ui.add(
                egui::TextEdit::singleline(&mut v)
                    .desired_width(width)
                    .id_salt(id_salt),
            );
        }
        FieldKind::Multiline => {
            ui.add(
                egui::TextEdit::multiline(&mut v)
                    .desired_width(width)
                    .desired_rows(2)
                    .id_salt(id_salt),
            );
        }
        FieldKind::Number => {
            number_edit(ui, &mut v, width, id_salt);
        }
        FieldKind::Bool => {
            let mut b = v == "true";
            if toggle(ui, &mut b).changed() {
                v = if b { "true".into() } else { "false".into() };
            }
        }
        FieldKind::Choice(opts) => {
            let current = opts
                .iter()
                .find(|(k, _)| *k == v)
                .map(|(_, l)| tl(*l))
                .unwrap_or(v.as_str())
                .to_string();
            egui::ComboBox::from_id_salt(id_salt)
                .width(width)
                .selected_text(current)
                .show_ui(ui, |ui| {
                    for (k, l) in opts.iter() {
                        ui.selectable_value(&mut v, k.to_string(), tl(*l));
                    }
                });
        }
        FieldKind::ItemModel => {
            let models = crate::resourcepack::known_models();
            let shown = if v.is_empty() {
                t("(kein eigenes Modell)", "(no custom model)").to_string()
            } else {
                v.clone()
            };
            egui::ComboBox::from_id_salt(id_salt)
                .width(width)
                .selected_text(shown)
                .height(300.0)
                .show_ui(ui, |ui| {
                    ui.set_min_width(220.0);
                    ui.label(egui::RichText::new(t("Eigene ID eingeben:", "Type an id:")).size(11.0).color(theme::TEXT_WEAK));
                    ui.add(egui::TextEdit::singleline(&mut v).hint_text("namespace:modell").desired_width(f32::INFINITY));
                    ui.separator();
                    if models.is_empty() {
                        ui.label(egui::RichText::new(t(
                            "Kein Resourcepack hinzugefügt. Füge eins unter „Texturen & Modelle“ in der Seitenleiste hinzu.",
                            "No resource pack added. Add one under “Textures & models” in the sidebar.",
                        )).size(11.0).color(theme::TEXT_WEAK));
                    }
                    ui.selectable_value(&mut v, String::new(), t("(kein eigenes Modell)", "(no custom model)"));
                    for m in &models {
                        ui.selectable_value(&mut v, m.clone(), m);
                    }
                });
        }
    }
    if v != before {
        values.insert(f.id.to_string(), v);
        true
    } else {
        false
    }
}

/// Number entry as text field that only accepts numbers.
pub fn number_edit(
    ui: &mut Ui,
    v: &mut String,
    width: f32,
    id_salt: impl std::hash::Hash + std::fmt::Debug,
) -> Response {
    let mut r = ui.add(
        egui::TextEdit::singleline(v)
            .desired_width(width)
            .align(egui::Align2::CENTER_CENTER)
            .id_salt(id_salt),
    );
    if r.changed() {
        let cleaned: String = v
            .chars()
            .filter(|c| c.is_ascii_digit() || matches!(c, '.' | ',' | '-'))
            .collect();
        if cleaned != *v {
            *v = cleaned;
            r.mark_changed();
        }
    }
    r
}

/// Inline editor for an unconnected Text/Number/Bool pin.
pub fn pin_editor(
    ui: &mut Ui,
    ty: PinType,
    pin: &str,
    default: &str,
    values: &mut BTreeMap<String, String>,
    width: f32,
    id_salt: impl std::hash::Hash + std::fmt::Debug,
) -> bool {
    let mut v = get_val(values, pin, default);
    let before = v.clone();
    match ty {
        PinType::Text => {
            ui.add(
                egui::TextEdit::singleline(&mut v)
                    .desired_width(width)
                    .id_salt(id_salt),
            );
        }
        PinType::Number => {
            number_edit(ui, &mut v, width, id_salt);
        }
        PinType::Bool => {
            let mut b = v == "true";
            if toggle(ui, &mut b).changed() {
                v = if b { "true".into() } else { "false".into() };
            }
        }
        _ => {}
    }
    if v != before {
        values.insert(pin.to_string(), v);
        true
    } else {
        false
    }
}

/// Flat icon+text button like a toolbar button.
pub fn tool_button(ui: &mut Ui, text: &str) -> Response {
    ui.add(
        egui::Button::new(egui::RichText::new(text).size(13.5))
            .fill(Color32::from_rgb(0x26, 0x28, 0x2D))
            .stroke(egui::Stroke::new(1.0, Color32::from_rgb(0x3A, 0x3D, 0x43)))
            .corner_radius(6)
            .min_size(Vec2::new(32.0, 30.0)),
    )
}

/// Small pill used for platform tags.
pub fn pill(ui: &mut Ui, text: &str, color: Color32) {
    let galley =
        ui.painter()
            .layout_no_wrap(text.to_string(), egui::FontId::proportional(10.5), color);
    let size = galley.size() + Vec2::new(10.0, 4.0);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    ui.painter()
        .rect_filled(rect, 8.0, color.gamma_multiply(0.15));
    ui.painter()
        .galley(rect.center() - galley.size() / 2.0, galley, color);
}

/// Section header like "ARBEITSBEREICHE".
pub fn section_header(ui: &mut Ui, text: &str) {
    ui.label(
        egui::RichText::new(text.to_uppercase())
            .size(11.0)
            .color(theme::TEXT_WEAK)
            .strong(),
    );
}

pub fn platform_tags(ui: &mut Ui, def: &crate::blocks::BlockDef) {
    match (def.paper.is_some(), def.fabric.is_some()) {
        (true, true) => pill(ui, t("Paper + Fabric", "Paper + Fabric"), theme::GREEN),
        (true, false) => pill(ui, t("nur Paper", "Paper only"), theme::YELLOW),
        (false, true) => pill(ui, t("nur Fabric", "Fabric only"), theme::BLUE),
        _ => {}
    }
}
