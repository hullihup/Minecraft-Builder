//! Recipe editor ("Edit recipe" on the Recipe block): looks like the crafting
//! table / furnace / stonecutter in Minecraft. Slots are filled through the item search.

use crate::app::App;
use crate::i18n::{t, tl};
use crate::items;
use crate::model::NodeId;
use crate::recipe::{self, CENTER};
use crate::theme;
use crate::ui::menu_editor::{ItemPicker, PickTab, PickTarget, draw_item, mc_panel, mc_slot};
use egui::{Color32, FontId, Id, Rect, RichText, Sense, Stroke, StrokeKind, pos2, vec2};

/// Slot index of the result in pick targets.
pub const RESULT: usize = 9;

fn primary(text: &str) -> egui::Button<'_> {
    egui::Button::new(RichText::new(text).color(Color32::BLACK).strong())
        .fill(theme::ACCENT)
        .corner_radius(6)
        .min_size(vec2(110.0, 30.0))
}

fn label(ui: &mut egui::Ui, text: &str) {
    ui.add_space(6.0);
    ui.label(RichText::new(text).size(11.5).color(theme::TEXT_WEAK));
}

/// Arrow like in the crafting table.
fn arrow(p: &egui::Painter, c: egui::Pos2, s: f32) {
    let col = Color32::from_rgb(0x8B, 0x8B, 0x8B);
    p.rect_filled(
        Rect::from_center_size(c - vec2(s * 0.15, 0.0), vec2(s * 0.7, s * 0.28)),
        0.0,
        col,
    );
    p.add(egui::Shape::convex_polygon(
        vec![
            c + vec2(s * 0.18, -s * 0.38),
            c + vec2(s * 0.62, 0.0),
            c + vec2(s * 0.18, s * 0.38),
        ],
        col,
        Stroke::NONE,
    ));
}

/// Small flame drawn under the furnace input.
fn flame(p: &egui::Painter, c: egui::Pos2, s: f32) {
    let pts = vec![
        c + vec2(0.0, -s * 0.5),
        c + vec2(s * 0.32, -s * 0.05),
        c + vec2(s * 0.22, s * 0.4),
        c + vec2(-s * 0.22, s * 0.4),
        c + vec2(-s * 0.32, -s * 0.05),
    ];
    p.add(egui::Shape::convex_polygon(pts, Color32::from_rgb(0xF0, 0x8A, 0x20), Stroke::NONE));
    p.circle_filled(c + vec2(0.0, s * 0.15), s * 0.15, Color32::from_rgb(0xFF, 0xE0, 0x60));
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    let Some((ws, nid)) = app.recipe_edit else {
        return;
    };
    let Some(node) = app
        .project
        .workspaces
        .get(ws)
        .and_then(|w| w.node(nid))
        .cloned()
    else {
        app.recipe_edit = None;
        return;
    };
    let get = |k: &str, d: &str| node.values.get(k).cloned().unwrap_or_else(|| d.to_string());
    let mut rtype = get("rtype", "shaped");
    let mut rid = get("rid", "mein_rezept");
    let mut xp = get("xp", "0.1");
    let mut secs = get("seconds", "10");
    let mut r = recipe::parse(&get("recipe", recipe::DEFAULT));
    let before = (rtype.clone(), rid.clone(), xp.clone(), secs.clone(), r.clone());
    let picker_open = app.item_picker.is_some();
    let mut pick: Option<usize> = None;
    let mut close = false;

    let modal = egui::Modal::new(Id::unique("recipe_editor"))
        .frame(
            egui::Frame::new()
                .fill(Color32::from_rgb(0x20, 0x22, 0x26))
                .stroke(Stroke::new(1.0, theme::BORDER))
                .corner_radius(12)
                .inner_margin(20),
        )
        .show(ctx, |ui| {
            ui.set_width(900.0);
            ui.horizontal(|ui| {
                crate::ui::icons::icon(ui, crate::ui::icons::Icon::Grid, 20.0, crate::blocks::Cat::Recipes.color());
                ui.label(RichText::new(t("Rezept bearbeiten", "Edit recipe")).size(19.0).strong());
                ui.label(
                    RichText::new(t(
                        "Klick auf einen Platz: Item suchen · Rechtsklick: leeren",
                        "Click a slot: search item · right-click: clear",
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
            // Kind of recipe.
            ui.horizontal_wrapped(|ui| {
                for (k, l) in recipe::TYPES {
                    let on = rtype == *k;
                    let b = egui::Button::new(
                        RichText::new(tl(*l))
                            .size(12.5)
                            .color(if on { Color32::BLACK } else { theme::TEXT }),
                    )
                    .fill(if on {
                        crate::blocks::Cat::Recipes.color()
                    } else {
                        Color32::from_rgb(0x2C, 0x2E, 0x33)
                    })
                    .corner_radius(12);
                    if ui.add(b).clicked() {
                        rtype = k.to_string();
                    }
                }
            });
            ui.add_space(12.0);
            let single = recipe::single_input(&rtype);
            ui.horizontal_top(|ui| {
                // ------------------------------------------------ workbench
                let cell = 64.0_f32;
                let px = 3.0_f32;
                let (r_area, _) = ui.allocate_exact_size(vec2(470.0, 300.0), Sense::hover());
                let p = ui.painter().clone();
                mc_panel(&p, r_area, px);
                let title = match rtype.as_str() {
                    "shaped" | "shapeless" => t("Werkbank", "Crafting"),
                    "smelting" => t("Ofen", "Furnace"),
                    "smoking" => t("Räucherofen", "Smoker"),
                    "blasting" => t("Schmelzofen", "Blast furnace"),
                    "campfire" => t("Lagerfeuer", "Campfire"),
                    _ => t("Steinsäge", "Stonecutter"),
                };
                p.text(
                    r_area.min + vec2(24.0, 18.0),
                    egui::Align2::LEFT_TOP,
                    title,
                    FontId::proportional(17.0),
                    Color32::from_rgb(0x40, 0x40, 0x40),
                );
                let grid_min = pos2(r_area.left() + 40.0, r_area.top() + 56.0);
                let slots: Vec<(usize, Rect)> = if single {
                    vec![(CENTER, Rect::from_min_size(grid_min + vec2(cell, cell * 0.5), vec2(cell, cell)))]
                } else {
                    (0..9)
                        .map(|i| {
                            (
                                i,
                                Rect::from_min_size(
                                    grid_min + vec2((i % 3) as f32 * cell, (i / 3) as f32 * cell),
                                    vec2(cell, cell),
                                ),
                            )
                        })
                        .collect()
                };
                if single && recipe::cooking(&rtype) {
                    flame(&p, grid_min + vec2(cell * 1.5, cell * 2.1), cell * 0.55);
                }
                let result_rect = Rect::from_center_size(
                    pos2(r_area.right() - 90.0, grid_min.y + cell * 1.5),
                    vec2(cell * 1.3, cell * 1.3),
                );
                arrow(&p, pos2(grid_min.x + cell * 3.0 + 40.0, result_rect.center().y), 46.0);
                for (i, sr) in slots.iter().cloned().chain(std::iter::once((RESULT, result_rect))) {
                    mc_slot(&p, sr, px);
                    let resp = ui.interact(sr, Id::unique(("recipe_slot", nid, i)), Sense::click());
                    if resp.hovered() && !picker_open {
                        p.rect_filled(sr.shrink(px), 0.0, Color32::from_white_alpha(70));
                    }
                    let (item, model) = if i == RESULT {
                        (r.result.clone(), r.model.clone())
                    } else {
                        (r.grid[i].clone(), String::new())
                    };
                    if !item.is_empty() {
                        draw_item(app, ui, sr.shrink(px * 2.0 + 2.0), &item, &model);
                        if i == RESULT && r.amount > 1 {
                            p.text(
                                sr.right_bottom() - vec2(5.0, 3.0),
                                egui::Align2::RIGHT_BOTTOM,
                                r.amount.to_string(),
                                FontId::monospace(sr.height() * 0.3),
                                Color32::WHITE,
                            );
                        }
                    }
                    if resp.clicked() {
                        pick = Some(i);
                    }
                    if resp.secondary_clicked() {
                        if i == RESULT {
                            r.result.clear();
                            r.model.clear();
                        } else {
                            r.grid[i].clear();
                        }
                    }
                    if !item.is_empty() && !picker_open {
                        resp.on_hover_text(format!("{}\n{item}", items::display_name(&item)));
                    }
                }
                if single {
                    // Ingredients left over from a crafting recipe are not used.
                    let unused = (0..9).filter(|i| *i != CENTER && !r.grid[*i].is_empty()).count();
                    if unused > 0 && r.grid[CENTER].is_empty() {
                        // Move the first one into the input slot.
                        if let Some(first) = (0..9).find(|i| !r.grid[*i].is_empty()) {
                            r.grid[CENTER] = std::mem::take(&mut r.grid[first]);
                        }
                    }
                }
                if rtype == "shapeless" {
                    p.text(
                        pos2(grid_min.x, grid_min.y + cell * 3.0 + 10.0),
                        egui::Align2::LEFT_TOP,
                        t("Ohne Form: die Anordnung ist egal", "Shapeless: the arrangement does not matter"),
                        FontId::proportional(11.5),
                        Color32::from_rgb(0x40, 0x40, 0x40),
                    );
                }

                ui.add_space(18.0);
                // ------------------------------------------------ settings
                ui.vertical(|ui| {
                    ui.set_width(370.0);
                    crate::ui::widgets::section_header(ui, t("Rezept", "Recipe"));
                    label(ui, t("Rezept-Name (einmalig, z. B. super_spitzhacke)", "Recipe name (unique, e.g. super_pickaxe)"));
                    if ui
                        .add(egui::TextEdit::singleline(&mut rid).desired_width(f32::INFINITY))
                        .lost_focus()
                    {
                        rid = recipe::clean_id(&rid);
                    }
                    if recipe::cooking(&rtype) {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                label(ui, t("Erfahrung", "Experience"));
                                crate::ui::widgets::number_edit(ui, &mut xp, 80.0, ("rxp", nid));
                            });
                            ui.vertical(|ui| {
                                label(ui, t("Brennzeit in Sekunden", "Cooking time in seconds"));
                                crate::ui::widgets::number_edit(ui, &mut secs, 80.0, ("rsec", nid));
                            });
                        });
                    }
                    ui.add_space(12.0);
                    crate::ui::widgets::section_header(ui, t("Ergebnis", "Result"));
                    if r.result.is_empty() {
                        ui.add_space(4.0);
                        ui.label(RichText::new(t("Klicke rechts im Bild auf das große Feld, um das Ergebnis zu wählen.", "Click the big slot on the right of the picture to pick the result.")).color(theme::YELLOW).size(12.0));
                    } else {
                        ui.add_space(4.0);
                        ui.label(RichText::new(format!("{} · {}", items::display_name(&r.result), r.result)).strong());
                    }
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(t("Anzahl", "Amount")).size(12.0));
                        ui.add(egui::DragValue::new(&mut r.amount).range(1..=64));
                    });
                    label(ui, t("Name (leer = normaler Name)", "Name (empty = normal name)"));
                    ui.add(egui::TextEdit::singleline(&mut r.name).hint_text("&bSuper-Spitzhacke").desired_width(f32::INFINITY));
                    label(ui, t("Beschreibung (eine Zeile pro Zeile)", "Lore (one line per line)"));
                    let mut lore = r.lore.replace('|', "\n");
                    if ui
                        .add(egui::TextEdit::multiline(&mut lore).desired_rows(2).desired_width(f32::INFINITY))
                        .changed()
                    {
                        r.lore = lore.replace('\n', "|");
                    }
                    label(ui, t("Eigenes Modell (Resourcepack)", "Custom model (resource pack)"));
                    let models = crate::resourcepack::known_models();
                    egui::ComboBox::from_id_salt(("recipe_model", nid))
                        .width(ui.available_width())
                        .selected_text(if r.model.is_empty() {
                            t("(keins – normales Aussehen)", "(none – normal look)").to_string()
                        } else {
                            r.model.clone()
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut r.model, String::new(), t("(keins – normales Aussehen)", "(none – normal look)"));
                            for m in &models {
                                ui.selectable_value(&mut r.model, m.clone(), m);
                            }
                        });
                });
            });
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                if ui.add(primary(t("Fertig", "Done"))).clicked() {
                    close = true;
                }
                let d = recipe::Def {
                    rtype: rtype.clone(),
                    id: rid.clone(),
                    recipe: r.clone(),
                    xp: 0.0,
                    seconds: 1.0,
                    discover: true,
                };
                match recipe::problem(&d) {
                    Some(p) => ui.label(RichText::new(p).color(theme::YELLOW).size(12.0)),
                    None => ui.label(
                        RichText::new(t("Änderungen werden sofort übernommen", "Changes apply right away"))
                            .color(theme::TEXT_WEAK)
                            .size(11.5),
                    ),
                };
            });
        });
    if modal.should_close() && !picker_open {
        close = true;
    }
    let after = (rtype.clone(), rid.clone(), xp.clone(), secs.clone(), r.clone());
    if after != before {
        if let Some(n) = app.project.workspaces[ws].node_mut(nid) {
            n.values.insert("rtype".into(), rtype);
            n.values.insert("rid".into(), rid);
            n.values.insert("xp".into(), xp);
            n.values.insert("seconds".into(), secs);
            n.values.insert("recipe".into(), recipe::serialize(&r));
        }
    }
    if let Some(i) = pick {
        app.item_picker = Some(ItemPicker::new(
            PickTarget::RecipeSlot(i),
            if i == RESULT { PickTab::All } else { PickTab::All },
        ));
    }
    if close {
        app.recipe_edit = None;
        app.item_picker = None;
    }
    let _ = StrokeKind::Inside;
}

/// The picked item goes into the recipe slot (`RESULT` = result, with model).
pub fn apply_pick(app: &mut App, slot: usize, item: String, model: String) {
    let Some((ws, nid)) = app.recipe_edit else {
        return;
    };
    let Some(n) = app
        .project
        .workspaces
        .get_mut(ws)
        .and_then(|w| w.node_mut(nid))
    else {
        return;
    };
    let mut r = recipe::parse(n.values.get("recipe").map(|s| s.as_str()).unwrap_or(recipe::DEFAULT));
    if slot == RESULT {
        r.result = item;
        r.model = model;
    } else if slot < 9 {
        r.grid[slot] = item;
    }
    n.values.insert("recipe".into(), recipe::serialize(&r));
}

/// Opens the editor for a recipe block.
pub fn open(app: &mut App, ws: usize, node: NodeId) {
    app.recipe_edit = Some((ws, node));
}
