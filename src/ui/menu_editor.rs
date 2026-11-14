//! The menu editor ("Edit menu" on the "Open menu" block): a chest that looks like
//! the one in Minecraft, where slots are filled through an item search.

use crate::app::App;
use crate::i18n::t;
use crate::items;
use crate::model::NodeId;
use crate::theme;
use egui::{
    Color32, FontId, Id, Painter, Rect, RichText, Sense, Stroke, StrokeKind, Ui, pos2, vec2,
};

/// One filled slot of a menu.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct MenuItem {
    pub slot: usize,
    pub item: String,
    pub name: String,
    /// Lore lines separated by `|`.
    pub lore: String,
    pub amount: u32,
    pub model: String,
    /// Players may take the item out of the menu.
    pub take: bool,
    /// Own name of the slot, e.g. `buy_diamond`; "Menu click" can react to it.
    pub id: String,
}

/// Reads the block's "items" value (new tab format and the old "slot: item | name | lore").
pub fn parse(text: &str) -> Vec<MenuItem> {
    let mut out: Vec<MenuItem> = vec![];
    for line in text.lines() {
        let mut it = MenuItem {
            amount: 1,
            ..Default::default()
        };
        if line.contains('\t') {
            let p: Vec<&str> = line.split('\t').collect();
            let Some(slot) = p.first().and_then(|s| s.trim().parse().ok()) else {
                continue;
            };
            it.slot = slot;
            it.item = p.get(1).unwrap_or(&"").trim().to_string();
            it.name = p.get(2).unwrap_or(&"").trim().to_string();
            it.lore = p.get(3).unwrap_or(&"").trim().to_string();
            it.amount = p.get(4).and_then(|s| s.trim().parse().ok()).unwrap_or(1);
            it.model = p.get(5).unwrap_or(&"").trim().to_string();
            it.take = p.get(6).map(|f| f.contains("take")).unwrap_or(false);
            it.id = p.get(7).map(|x| clean_id(x)).unwrap_or_default();
        } else {
            let Some((slot, rest)) = line.split_once(':') else {
                continue;
            };
            let Ok(slot) = slot.trim().parse() else {
                continue;
            };
            let mut p = rest.splitn(3, '|');
            it.slot = slot;
            it.item = p.next().unwrap_or("").trim().to_string();
            it.name = p.next().unwrap_or("").trim().to_string();
            it.lore = p.next().unwrap_or("").trim().to_string();
        }
        if it.item.is_empty() {
            continue;
        }
        it.amount = it.amount.clamp(1, 99);
        out.retain(|o| o.slot != it.slot);
        out.push(it);
    }
    out.sort_by_key(|i| i.slot);
    out
}

pub fn serialize(items: &[MenuItem]) -> String {
    let clean = |s: &str| s.replace(['\t', '\n', '\r'], " ");
    let mut v: Vec<&MenuItem> = items.iter().collect();
    v.sort_by_key(|i| i.slot);
    v.iter()
        .map(|i| {
            format!(
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                i.slot,
                clean(&i.item),
                clean(&i.name),
                clean(&i.lore),
                i.amount.clamp(1, 99),
                clean(&i.model),
                if i.take { "take" } else { "" },
                clean_id(&i.id)
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Menu ids: lowercase letters, digits, `_` and `-`.
pub fn clean_id(s: &str) -> String {
    s.trim()
        .to_lowercase()
        .chars()
        .map(|c| if c == ' ' { '_' } else { c })
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .collect()
}

/// IDs of the menu slots of the project, for the picker on "Menu click": (menu name, id, item).
static MENU_IDS: std::sync::RwLock<Vec<(String, String, String)>> = std::sync::RwLock::new(Vec::new());

/// Collects the slot IDs of all "Open menu" blocks (called every frame, cheap).
pub fn publish_ids(project: &crate::model::Project) {
    let mut out = vec![];
    for ws in &project.workspaces {
        for n in &ws.nodes {
            let Some(def) = crate::blocks::get(&n.kind) else { continue };
            let Some(f) = def.fields.iter().find(|f| matches!(f.kind, crate::blocks::FieldKind::Menu)) else {
                continue;
            };
            let menu = n.values.get("menu").cloned().unwrap_or_else(|| "shop".into());
            let text = n.values.get(f.id).map(|s| s.as_str()).unwrap_or(f.default);
            for it in parse(text) {
                if !it.id.is_empty() && !out.iter().any(|(m, i, _): &(String, String, String)| *m == menu && *i == it.id) {
                    out.push((menu.clone(), it.id.clone(), it.item.clone()));
                }
            }
        }
    }
    if let Ok(mut w) = MENU_IDS.write() {
        *w = out;
    }
}

/// Slot IDs of one menu: (id, item).
pub fn ids_of(menu: &str) -> Vec<(String, String)> {
    MENU_IDS
        .read()
        .map(|v| {
            v.iter()
                .filter(|(m, _, _)| m == menu)
                .map(|(_, i, it)| (i.clone(), it.clone()))
                .collect()
        })
        .unwrap_or_default()
}

pub fn rows_of(values: &std::collections::BTreeMap<String, String>) -> usize {
    values
        .get("rows")
        .and_then(|r| r.replace(',', ".").parse::<f64>().ok())
        .map(|r| r as usize)
        .unwrap_or(3)
        .clamp(1, 6)
}

/// Open menu editor.
pub struct MenuEdit {
    pub ws: usize,
    pub node: NodeId,
    pub slot: Option<usize>,
    /// Slot an item is dragged from.
    pub drag: Option<usize>,
}

impl MenuEdit {
    pub fn new(ws: usize, node: NodeId) -> Self {
        Self {
            ws,
            node,
            slot: None,
            drag: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum PickTarget {
    /// A slot of the menu being edited.
    MenuSlot(usize),
    /// A text value of a block (item id / block id input).
    NodeValue {
        ws: usize,
        node: NodeId,
        key: String,
    },
    /// Base item of a resource pack item rule (model id).
    PackBase(String),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PickTab {
    All,
    Items,
    Blocks,
    Pack,
}

/// Open item search.
pub struct ItemPicker {
    pub target: PickTarget,
    pub query: String,
    pub tab: PickTab,
    pub just_opened: bool,
}

impl ItemPicker {
    pub fn new(target: PickTarget, tab: PickTab) -> Self {
        Self {
            target,
            query: String::new(),
            tab,
            just_opened: true,
        }
    }
}

// ------------------------------------------------------------------ painting

const MC_PANEL: Color32 = Color32::from_rgb(0xC6, 0xC6, 0xC6);
const MC_SLOT: Color32 = Color32::from_rgb(0x8B, 0x8B, 0x8B);
const MC_DARK: Color32 = Color32::from_rgb(0x37, 0x37, 0x37);
const MC_SHADOW: Color32 = Color32::from_rgb(0x55, 0x55, 0x55);
const MC_LABEL: Color32 = Color32::from_rgb(0x40, 0x40, 0x40);

fn mc_color(code: char) -> Option<Color32> {
    Some(match code.to_ascii_lowercase() {
        '0' => Color32::from_rgb(0x00, 0x00, 0x00),
        '1' => Color32::from_rgb(0x00, 0x00, 0xAA),
        '2' => Color32::from_rgb(0x00, 0xAA, 0x00),
        '3' => Color32::from_rgb(0x00, 0xAA, 0xAA),
        '4' => Color32::from_rgb(0xAA, 0x00, 0x00),
        '5' => Color32::from_rgb(0xAA, 0x00, 0xAA),
        '6' => Color32::from_rgb(0xFF, 0xAA, 0x00),
        '7' => Color32::from_rgb(0xAA, 0xAA, 0xAA),
        '8' => Color32::from_rgb(0x55, 0x55, 0x55),
        '9' => Color32::from_rgb(0x55, 0x55, 0xFF),
        'a' => Color32::from_rgb(0x55, 0xFF, 0x55),
        'b' => Color32::from_rgb(0x55, 0xFF, 0xFF),
        'c' => Color32::from_rgb(0xFF, 0x55, 0x55),
        'd' => Color32::from_rgb(0xFF, 0x55, 0xFF),
        'e' => Color32::from_rgb(0xFF, 0xFF, 0x55),
        'f' => Color32::from_rgb(0xFF, 0xFF, 0xFF),
        _ => return None,
    })
}

/// Text with `&` color codes as colored layout job.
pub fn mc_text(text: &str, default: Color32, size: f32) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    let mut color = default;
    let mut italics = false;
    let mut cur = String::new();
    let mut chars = text.chars().peekable();
    let flush = |job: &mut egui::text::LayoutJob, cur: &mut String, color: Color32, italics: bool| {
        if !cur.is_empty() {
            job.append(
                cur,
                0.0,
                egui::TextFormat {
                    font_id: FontId::proportional(size),
                    color,
                    italics,
                    ..Default::default()
                },
            );
            cur.clear();
        }
    };
    while let Some(c) = chars.next() {
        if (c == '&' || c == '§') && chars.peek().is_some() {
            let code = *chars.peek().unwrap();
            if let Some(col) = mc_color(code) {
                flush(&mut job, &mut cur, color, italics);
                color = col;
                italics = false;
                chars.next();
                continue;
            }
            if "klmnorKLMNOR".contains(code) {
                flush(&mut job, &mut cur, color, italics);
                match code.to_ascii_lowercase() {
                    'o' => italics = true,
                    'r' => {
                        color = default;
                        italics = false;
                    }
                    _ => {}
                }
                chars.next();
                continue;
            }
        }
        cur.push(c);
    }
    flush(&mut job, &mut cur, color, italics);
    job
}

/// Text without color codes.
#[cfg(test)]
pub fn strip_codes(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if (c == '&' || c == '§')
            && chars
                .peek()
                .map(|n| "0123456789abcdefklmnorABCDEFKLMNOR".contains(*n))
                .unwrap_or(false)
        {
            chars.next();
            continue;
        }
        out.push(c);
    }
    out
}

/// Raised Minecraft GUI panel.
fn mc_panel(p: &Painter, r: Rect, px: f32) {
    p.rect_filled(r, px * 2.0, Color32::BLACK);
    let inner = r.shrink(px);
    p.rect_filled(inner, px, MC_PANEL);
    p.rect_filled(
        Rect::from_min_max(inner.min, pos2(inner.right() - px, inner.top() + px * 2.0)),
        0.0,
        Color32::WHITE,
    );
    p.rect_filled(
        Rect::from_min_max(inner.min, pos2(inner.left() + px * 2.0, inner.bottom() - px)),
        0.0,
        Color32::WHITE,
    );
    p.rect_filled(
        Rect::from_min_max(pos2(inner.left() + px, inner.bottom() - px * 2.0), inner.max),
        0.0,
        MC_SHADOW,
    );
    p.rect_filled(
        Rect::from_min_max(pos2(inner.right() - px * 2.0, inner.top() + px), inner.max),
        0.0,
        MC_SHADOW,
    );
}

/// Sunken inventory slot.
fn mc_slot(p: &Painter, r: Rect, px: f32) {
    p.rect_filled(r, 0.0, MC_SLOT);
    p.rect_filled(
        Rect::from_min_max(r.min, pos2(r.right() - px, r.top() + px)),
        0.0,
        MC_DARK,
    );
    p.rect_filled(
        Rect::from_min_max(r.min, pos2(r.left() + px, r.bottom() - px)),
        0.0,
        MC_DARK,
    );
    p.rect_filled(
        Rect::from_min_max(pos2(r.left() + px, r.bottom() - px), r.max),
        0.0,
        Color32::WHITE,
    );
    p.rect_filled(
        Rect::from_min_max(pos2(r.right() - px, r.top() + px), r.max),
        0.0,
        Color32::WHITE,
    );
}

/// Draws an item: its texture if there is one (resource pack or local Minecraft), else a stand-in.
pub fn draw_item(app: &mut App, ui: &Ui, rect: Rect, item: &str, model: &str) {
    match app.item_texture(ui.ctx(), item, model) {
        Some(tex) => {
            ui.painter().image(
                tex,
                rect,
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        }
        None => {
            let id = if model.trim().is_empty() { item } else { model };
            items::paint_placeholder(ui.painter(), rect, id);
        }
    }
}

fn amount_label(p: &Painter, slot: Rect, amount: u32, px: f32) {
    if amount > 1 {
        let pos = slot.right_bottom() - vec2(px, 0.0);
        let font = FontId::monospace(slot.height() * 0.42);
        p.text(
            pos + vec2(px, px),
            egui::Align2::RIGHT_BOTTOM,
            amount.to_string(),
            font.clone(),
            Color32::from_rgb(0x3F, 0x3F, 0x3F),
        );
        p.text(
            pos,
            egui::Align2::RIGHT_BOTTOM,
            amount.to_string(),
            font,
            Color32::WHITE,
        );
    }
}

/// Minecraft-style item tooltip.
fn mc_tooltip(ui: &mut Ui, it: &MenuItem) {
    egui::Frame::new()
        .fill(Color32::from_rgb(0x10, 0x00, 0x10))
        .stroke(Stroke::new(2.0, Color32::from_rgb(0x2D, 0x0A, 0x63)))
        .corner_radius(3)
        .inner_margin(8)
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            let name = if it.name.trim().is_empty() {
                items::display_name(&it.item)
            } else {
                it.name.clone()
            };
            ui.label(mc_text(&name, Color32::WHITE, 14.0));
            for line in it.lore.split('|').filter(|l| !l.trim().is_empty()) {
                ui.label(mc_text(line, Color32::from_rgb(0xAA, 0x00, 0xAA), 13.0));
            }
            ui.label(
                RichText::new(if it.model.is_empty() {
                    it.item.clone()
                } else {
                    format!("{} · {}", it.item, it.model)
                })
                .size(11.0)
                .color(Color32::from_rgb(0x55, 0x55, 0x55)),
            );
            if !it.id.is_empty() {
                ui.label(
                    RichText::new(format!("ID: {}", it.id))
                        .size(11.0)
                        .color(Color32::from_rgb(0x55, 0xFF, 0xFF)),
                );
            }
            if it.take {
                ui.label(
                    RichText::new(t("Kann herausgenommen werden", "Can be taken out"))
                        .size(11.0)
                        .color(Color32::from_rgb(0x55, 0xFF, 0x55)),
                );
            }
        });
}

// --------------------------------------------------------------- the editor

fn primary(text: &str) -> egui::Button<'_> {
    egui::Button::new(RichText::new(text).color(Color32::BLACK).strong())
        .fill(theme::ACCENT)
        .corner_radius(6)
        .min_size(vec2(110.0, 30.0))
}

fn field_label(ui: &mut Ui, text: &str) {
    ui.add_space(6.0);
    ui.label(RichText::new(text).size(11.5).color(theme::TEXT_WEAK));
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    let Some(edit) = app.menu_edit.as_ref() else {
        return;
    };
    let (ws, nid) = (edit.ws, edit.node);
    let Some(node) = app
        .project
        .workspaces
        .get(ws)
        .and_then(|w| w.node(nid))
        .cloned()
    else {
        app.menu_edit = None;
        return;
    };
    let def = crate::blocks::get(&node.kind);
    let default_items = def
        .and_then(|d| d.field("items"))
        .map(|f| f.default)
        .unwrap_or("");
    let val = |k: &str, d: &str| node.values.get(k).cloned().unwrap_or_else(|| d.to_string());
    let mut menu_id = val("menu", "shop");
    let mut title = val("title", "&8Shop");
    let mut rows = rows_of(&node.values);
    let mut list = parse(&val("items", default_items));
    let before = (menu_id.clone(), title.clone(), rows, list.clone());
    let title_linked = app.project.workspaces[ws]
        .link_into(nid, "title")
        .is_some();
    let rows_linked = app.project.workspaces[ws].link_into(nid, "rows").is_some();
    if rows_linked {
        rows = 6;
    }

    let mut selected = app.menu_edit.as_ref().and_then(|e| e.slot);
    let mut drag = app.menu_edit.as_ref().and_then(|e| e.drag);
    let mut close = false;
    let mut open_picker: Option<usize> = None;
    let picker_open = app.item_picker.is_some();

    let modal = egui::Modal::new(Id::unique("menu_editor"))
        .frame(
            egui::Frame::new()
                .fill(Color32::from_rgb(0x20, 0x22, 0x26))
                .stroke(Stroke::new(1.0, theme::BORDER))
                .corner_radius(12)
                .inner_margin(20),
        )
        .show(ctx, |ui| {
            ui.set_width(940.0);
            ui.horizontal(|ui| {
                crate::ui::icons::icon(ui, crate::ui::icons::Icon::Chest, 20.0, theme::ACCENT);
                ui.label(
                    RichText::new(t("Menü bearbeiten", "Edit menu"))
                        .size(19.0)
                        .strong(),
                );
                ui.label(
                    RichText::new(t(
                        "Klick auf einen Platz: Item suchen · Ziehen: verschieben · Rechtsklick: mehr",
                        "Click a slot: search item · drag: move · right-click: more",
                    ))
                    .size(11.5)
                    .color(theme::TEXT_WEAK),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if crate::ui::icons::icon_button(ui, crate::ui::icons::Icon::Close, 24.0, "")
                        .clicked()
                    {
                        close = true;
                    }
                });
            });
            ui.add_space(12.0);
            ui.horizontal_top(|ui| {
                // ------------------------------------------------ chest
                let cell = 54.0_f32;
                let px = 3.0_f32;
                let pad = 8.0 * px;
                let chest_w = 9.0 * cell + 2.0 * pad;
                let chest_h = pad + 30.0 + rows as f32 * cell + 34.0 + 4.0 * cell + 8.0 + pad;
                let (r, _) = ui.allocate_exact_size(vec2(chest_w, chest_h.max(520.0)), Sense::hover());
                let r = Rect::from_min_size(r.min, vec2(chest_w, chest_h));
                let p = ui.painter().clone();
                mc_panel(&p, r, px);
                let galley = ui.fonts_mut(|f| f.layout_job(mc_text(&title, MC_LABEL, 17.0)));
                p.galley(r.min + vec2(pad, pad - 4.0), galley, MC_LABEL);
                let grid_top = r.top() + pad + 26.0;
                let slot_rect = |i: usize| {
                    Rect::from_min_size(
                        pos2(
                            r.left() + pad + (i % 9) as f32 * cell,
                            grid_top + (i / 9) as f32 * cell,
                        ),
                        vec2(cell, cell),
                    )
                };
                let pointer = ui.ctx().pointer_latest_pos();
                let mut hovered_slot = None;
                let mut drop_on: Option<usize> = None;
                for i in 0..rows * 9 {
                    let sr = slot_rect(i);
                    mc_slot(&p, sr, px);
                    let resp = ui.interact(sr, Id::unique(("menu_slot", nid, i)), Sense::click_and_drag());
                    let item = list.iter().find(|x| x.slot == i).cloned();
                    if resp.hovered() || pointer.map(|pp| sr.contains(pp)).unwrap_or(false) {
                        hovered_slot = Some(i);
                    }
                    if resp.hovered() && !picker_open {
                        p.rect_filled(sr.shrink(px), 0.0, Color32::from_white_alpha(70));
                    }
                    if selected == Some(i) {
                        p.rect_stroke(
                            sr.shrink(1.0),
                            0.0,
                            Stroke::new(2.5, theme::ACCENT),
                            StrokeKind::Inside,
                        );
                    }
                    if let Some(it) = &item {
                        if drag != Some(i) {
                            draw_item(app, ui, sr.shrink(px * 2.0 + 1.0), &it.item, &it.model);
                            amount_label(&p, sr.shrink(px), it.amount, px);
                            if it.take {
                                p.circle_filled(
                                    sr.left_top() + vec2(px * 3.0, px * 3.0),
                                    3.5,
                                    Color32::from_rgb(0x30, 0xD0, 0x50),
                                );
                            }
                        }
                    }
                    if resp.drag_started() && item.is_some() {
                        drag = Some(i);
                        selected = Some(i);
                    }
                    if resp.clicked() {
                        selected = Some(i);
                        if item.is_none() {
                            open_picker = Some(i);
                        }
                    }
                    if resp.double_clicked() {
                        open_picker = Some(i);
                    }
                    let has_item = item.is_some();
                    resp.context_menu(|ui| {
                        if ui
                            .button(if has_item {
                                t("Item ändern…", "Change item…")
                            } else {
                                t("Item wählen…", "Pick item…")
                            })
                            .clicked()
                        {
                            open_picker = Some(i);
                            selected = Some(i);
                            ui.close();
                        }
                        if has_item {
                            if ui.button(t("In alle leeren Plätze kopieren", "Copy to all empty slots")).clicked() {
                                if let Some(src) = list.iter().find(|x| x.slot == i).cloned() {
                                    for s in 0..rows * 9 {
                                        if !list.iter().any(|x| x.slot == s) {
                                            list.push(MenuItem { slot: s, ..src.clone() });
                                        }
                                    }
                                }
                                ui.close();
                            }
                            if ui
                                .button(RichText::new(t("Platz leeren", "Clear slot")).color(theme::RED))
                                .clicked()
                            {
                                list.retain(|x| x.slot != i);
                                ui.close();
                            }
                        }
                    });
                    if has_item && drag.is_none() && !picker_open {
                        if let Some(it) = item {
                            resp.on_hover_ui_at_pointer(|ui| mc_tooltip(ui, &it));
                        }
                    }
                }
                // Drag & drop between slots.
                if let Some(from) = drag {
                    if ui.input(|i| i.pointer.any_released()) {
                        drop_on = hovered_slot;
                        drag = None;
                        if let Some(to) = drop_on {
                            if to != from {
                                for it in list.iter_mut() {
                                    if it.slot == from {
                                        it.slot = to;
                                    } else if it.slot == to {
                                        it.slot = from;
                                    }
                                }
                                selected = Some(to);
                            }
                        }
                    } else if let (Some(pp), Some(it)) =
                        (pointer, list.iter().find(|x| x.slot == from).cloned())
                    {
                        let dr = Rect::from_center_size(pp, vec2(cell, cell)).shrink(px * 2.0 + 1.0);
                        draw_item(app, ui, dr, &it.item, &it.model);
                        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                    }
                }
                let _ = drop_on;
                // Player inventory below, like in the game (just for the look).
                let inv_top = grid_top + rows as f32 * cell + 30.0;
                p.text(
                    pos2(r.left() + pad, inv_top - 22.0),
                    egui::Align2::LEFT_TOP,
                    t("Inventar", "Inventory"),
                    FontId::proportional(17.0),
                    MC_LABEL,
                );
                for i in 0..36 {
                    let extra = if i >= 27 { 8.0 } else { 0.0 };
                    let sr = Rect::from_min_size(
                        pos2(
                            r.left() + pad + (i % 9) as f32 * cell,
                            inv_top + (i / 9) as f32 * cell + extra,
                        ),
                        vec2(cell, cell),
                    );
                    mc_slot(&p, sr, px);
                }

                ui.add_space(18.0);
                // ----------------------------------------------- settings
                ui.vertical(|ui| {
                    ui.set_width(330.0);
                    crate::ui::widgets::section_header(ui, t("Menü", "Menu"));
                    field_label(ui, t("Menü-Name (für „Klick im Menü“)", "Menu name (for “Menu click”)"));
                    ui.add(egui::TextEdit::singleline(&mut menu_id).desired_width(f32::INFINITY));
                    field_label(ui, t("Titel (mit &-Farbcodes)", "Title (with & color codes)"));
                    if title_linked {
                        ui.label(
                            RichText::new(t(
                                "Kommt aus dem verbundenen Block.",
                                "Comes from the connected block.",
                            ))
                            .italics()
                            .color(theme::TEXT_WEAK),
                        );
                    } else {
                        ui.add(egui::TextEdit::singleline(&mut title).desired_width(f32::INFINITY));
                    }
                    field_label(ui, t("Reihen", "Rows"));
                    if rows_linked {
                        ui.label(
                            RichText::new(t(
                                "Kommt aus dem verbundenen Block (alle 6 Reihen werden gezeigt).",
                                "Comes from the connected block (all 6 rows are shown).",
                            ))
                            .italics()
                            .color(theme::TEXT_WEAK),
                        );
                    } else {
                        ui.horizontal(|ui| {
                            for n in 1..=6usize {
                                let on = rows == n;
                                let b = egui::Button::new(
                                    RichText::new(n.to_string())
                                        .strong()
                                        .color(if on { Color32::BLACK } else { theme::TEXT }),
                                )
                                .fill(if on {
                                    theme::ACCENT
                                } else {
                                    Color32::from_rgb(0x2A, 0x2C, 0x31)
                                })
                                .corner_radius(6)
                                .min_size(vec2(40.0, 30.0));
                                if ui.add(b).clicked() {
                                    rows = n;
                                }
                            }
                        });
                        ui.label(
                            RichText::new(format!(
                                "{} {}",
                                rows * 9,
                                t("Plätze (0 bis", "slots (0 to")
                            ) + &format!(" {})", rows * 9 - 1))
                            .size(11.0)
                            .color(theme::TEXT_WEAK),
                        );
                    }
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui
                            .button(t("Rand füllen", "Fill border"))
                            .on_hover_text(t(
                                "Füllt alle leeren Randplätze mit grauen Glasscheiben ohne Namen.",
                                "Fills all empty border slots with nameless gray glass panes.",
                            ))
                            .clicked()
                        {
                            for s in 0..rows * 9 {
                                let (x, y) = (s % 9, s / 9);
                                let border = x == 0 || x == 8 || y == 0 || y == rows - 1;
                                if border && !list.iter().any(|i| i.slot == s) {
                                    list.push(MenuItem {
                                        slot: s,
                                        item: "minecraft:gray_stained_glass_pane".into(),
                                        name: "&r ".into(),
                                        amount: 1,
                                        ..Default::default()
                                    });
                                }
                            }
                        }
                        if ui
                            .button(RichText::new(t("Alles leeren", "Clear all")).color(theme::RED))
                            .clicked()
                        {
                            list.clear();
                            selected = None;
                        }
                    });

                    ui.add_space(14.0);
                    ui.separator();
                    ui.add_space(8.0);
                    let sel_item = selected.and_then(|s| list.iter().position(|i| i.slot == s));
                    match (selected, sel_item) {
                        (Some(slot), Some(idx)) => {
                            crate::ui::widgets::section_header(
                                ui,
                                &format!("{} {slot}", t("Platz", "Slot")),
                            );
                            ui.add_space(4.0);
                            let (item_id, model_id) = (list[idx].item.clone(), list[idx].model.clone());
                            ui.horizontal(|ui| {
                                let (ir, _) = ui.allocate_exact_size(vec2(44.0, 44.0), Sense::hover());
                                mc_slot(ui.painter(), ir, 2.0);
                                draw_item(app, ui, ir.shrink(6.0), &item_id, &model_id);
                                ui.vertical(|ui| {
                                    ui.label(RichText::new(items::display_name(&item_id)).strong());
                                    ui.label(
                                        RichText::new(&item_id)
                                            .size(11.0)
                                            .monospace()
                                            .color(theme::TEXT_WEAK),
                                    );
                                });
                            });
                            if ui.button(t("Anderes Item wählen…", "Pick another item…")).clicked() {
                                open_picker = Some(slot);
                            }
                            let it = &mut list[idx];
                            field_label(ui, t("ID (für „Klick im Menü“)", "ID (for “Menu click”)"));
                            ui.horizontal(|ui| {
                                let mut id = it.id.clone();
                                let suggestion = clean_id(&items::strip(&it.item).replace(':', "_"));
                                if ui
                                    .add(
                                        egui::TextEdit::singleline(&mut id)
                                            .hint_text(suggestion.as_str())
                                            .desired_width(ui.available_width() - 90.0),
                                    )
                                    .changed()
                                {
                                    it.id = clean_id(&id);
                                }
                                if it.id.is_empty()
                                    && ui
                                        .button(t("Vorschlag", "Suggest"))
                                        .on_hover_text(suggestion.as_str())
                                        .clicked()
                                {
                                    it.id = suggestion.clone();
                                }
                            });
                            ui.label(
                                RichText::new(t(
                                    "Im Block „Klick im Menü“ kannst du diese ID auswählen – dann läuft er nur, wenn genau dieses Item angeklickt wird.",
                                    "In the “Menu click” block you can pick this ID – then it only runs when exactly this item is clicked.",
                                ))
                                .size(11.0)
                                .color(theme::TEXT_WEAK),
                            );
                            field_label(ui, t("Name (leer = normaler Item-Name)", "Name (empty = normal item name)"));
                            ui.add(
                                egui::TextEdit::singleline(&mut it.name)
                                    .hint_text("&bDiamant")
                                    .desired_width(f32::INFINITY),
                            );
                            field_label(ui, t("Beschreibung (eine Zeile pro Zeile)", "Lore (one line per line)"));
                            let mut lore = it.lore.replace('|', "\n");
                            if ui
                                .add(
                                    egui::TextEdit::multiline(&mut lore)
                                        .hint_text(t("&7Kostet 5 Gold", "&7Costs 5 gold"))
                                        .desired_rows(2)
                                        .desired_width(f32::INFINITY),
                                )
                                .changed()
                            {
                                it.lore = lore.replace('\n', "|");
                            }
                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(t("Anzahl", "Amount")).size(12.0));
                                ui.add(egui::DragValue::new(&mut it.amount).range(1..=99));
                            });
                            field_label(ui, t("Eigenes Modell (Resourcepack)", "Custom model (resource pack)"));
                            let models = crate::resourcepack::known_models();
                            egui::ComboBox::from_id_salt(("menu_model", nid))
                                .width(ui.available_width())
                                .selected_text(if it.model.is_empty() {
                                    t("(keins – normales Aussehen)", "(none – normal look)").to_string()
                                } else {
                                    it.model.clone()
                                })
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut it.model,
                                        String::new(),
                                        t("(keins – normales Aussehen)", "(none – normal look)"),
                                    );
                                    for m in &models {
                                        ui.selectable_value(&mut it.model, m.clone(), m);
                                    }
                                    if models.is_empty() {
                                        ui.label(
                                            RichText::new(t(
                                                "Kein Resourcepack im Projekt.",
                                                "No resource pack in the project.",
                                            ))
                                            .size(11.0)
                                            .color(theme::TEXT_WEAK),
                                        );
                                    }
                                });
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                crate::ui::widgets::toggle(ui, &mut it.take);
                                ui.label(t("Kann herausgenommen werden", "Can be taken out"));
                            });
                            ui.label(
                                RichText::new(t(
                                    "Aus: Das Item bleibt im Menü und ist nur ein Knopf.",
                                    "Off: the item stays in the menu and is just a button.",
                                ))
                                .size(11.0)
                                .color(theme::TEXT_WEAK),
                            );
                            ui.add_space(8.0);
                            if ui
                                .button(RichText::new(t("Platz leeren", "Clear slot")).color(theme::RED))
                                .clicked()
                            {
                                list.remove(idx);
                            }
                        }
                        (Some(slot), None) => {
                            crate::ui::widgets::section_header(
                                ui,
                                &format!("{} {slot}", t("Platz", "Slot")),
                            );
                            ui.add_space(6.0);
                            ui.label(
                                RichText::new(t("Dieser Platz ist leer.", "This slot is empty."))
                                    .color(theme::TEXT_WEAK),
                            );
                            ui.add_space(6.0);
                            if ui.add(primary(t("Item suchen…", "Search item…"))).clicked() {
                                open_picker = Some(slot);
                            }
                        }
                        _ => {
                            ui.label(
                                RichText::new(t(
                                    "Klicke links auf einen Platz, um ein Item hineinzulegen oder es einzustellen.",
                                    "Click a slot on the left to put an item in or to adjust it.",
                                ))
                                .color(theme::TEXT_WEAK),
                            );
                        }
                    }
                });
            });
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui.add(primary(t("Fertig", "Done"))).clicked() {
                    close = true;
                }
                ui.label(
                    RichText::new(format!(
                        "{} {}",
                        list.iter().filter(|i| i.slot < rows * 9).count(),
                        t("Items im Menü · Änderungen werden sofort übernommen", "items in the menu · changes apply right away")
                    ))
                    .size(11.5)
                    .color(theme::TEXT_WEAK),
                );
            });
        });
    if modal.should_close() && !picker_open {
        close = true;
    }
    if !picker_open && ctx.input(|i| i.key_pressed(egui::Key::Delete)) && !ctx.egui_wants_keyboard_input() {
        if let Some(s) = selected {
            list.retain(|i| i.slot != s);
        }
    }

    // Write changes back into the block.
    if !rows_linked {
        list.retain(|i| i.slot < rows * 9);
    }
    let after = (menu_id.clone(), title.clone(), rows, list.clone());
    if after != before {
        if let Some(n) = app.project.workspaces[ws].node_mut(nid) {
            n.values.insert("menu".into(), menu_id);
            if !title_linked {
                n.values.insert("title".into(), title);
            }
            if !rows_linked {
                n.values.insert("rows".into(), rows.to_string());
            }
            n.values.insert("items".into(), serialize(&list));
        }
    }
    if let Some(e) = app.menu_edit.as_mut() {
        e.slot = selected;
        e.drag = drag;
    }
    if let Some(slot) = open_picker {
        app.item_picker = Some(ItemPicker::new(PickTarget::MenuSlot(slot), PickTab::All));
    }
    if close {
        app.menu_edit = None;
        app.item_picker = None;
    }
}

// --------------------------------------------------------------- item search

/// What was picked: (item id, model id).
fn apply_pick(app: &mut App, target: &PickTarget, item: String, model: String) {
    match target {
        PickTarget::MenuSlot(slot) => {
            let Some(e) = app.menu_edit.as_ref() else {
                return;
            };
            let (ws, nid) = (e.ws, e.node);
            let kind = app
                .project
                .workspaces
                .get(ws)
                .and_then(|w| w.node(nid))
                .map(|n| n.kind.clone())
                .unwrap_or_default();
            let default_items = crate::blocks::get(&kind)
                .and_then(|d| d.field("items"))
                .map(|f| f.default)
                .unwrap_or("");
            if let Some(n) = app
                .project
                .workspaces
                .get_mut(ws)
                .and_then(|w| w.node_mut(nid))
            {
                let mut list = parse(n.values.get("items").map(|s| s.as_str()).unwrap_or(default_items));
                match list.iter_mut().find(|i| i.slot == *slot) {
                    Some(it) => {
                        it.item = item;
                        it.model = model;
                    }
                    None => list.push(MenuItem {
                        slot: *slot,
                        item,
                        model,
                        amount: 1,
                        ..Default::default()
                    }),
                }
                n.values.insert("items".into(), serialize(&list));
            }
            if let Some(e) = app.menu_edit.as_mut() {
                e.slot = Some(*slot);
            }
        }
        PickTarget::PackBase(key) => {
            let mut rule = app.project.settings.pack.rule(key);
            rule.base = item;
            app.project.settings.pack.set_rule(key, rule);
        }
        PickTarget::NodeValue { ws, node, key } => {
            if let Some(n) = app
                .project
                .workspaces
                .get_mut(*ws)
                .and_then(|w| w.node_mut(*node))
            {
                n.values.insert(key.clone(), item);
            }
        }
    }
}

pub fn item_picker(app: &mut App, ctx: &egui::Context) {
    let Some(pk) = app.item_picker.as_mut() else {
        return;
    };
    let target = pk.target.clone();
    let mut query = pk.query.clone();
    let mut tab = pk.tab;
    let just_opened = std::mem::take(&mut pk.just_opened);
    let for_menu = matches!(target, PickTarget::MenuSlot(_));
    let pack_items: Vec<(String, String)> = match &app.pack.info {
        Some(Ok(info)) => info
            .item_models
            .iter()
            .map(|m| {
                let rule = app.project.settings.pack.rule(m);
                let base = if rule.base.trim().is_empty() {
                    "minecraft:paper".to_string()
                } else {
                    rule.base.trim().to_string()
                };
                (base, m.clone())
            })
            .collect(),
        _ => vec![],
    };
    let mut picked: Option<(String, String)> = None;
    let mut close = false;
    let mut pick_textures = false;
    app.ensure_renderer();
    let extra = app.vanilla.extra_items.clone();

    let modal = egui::Modal::new(Id::unique("item_picker"))
        .frame(
            egui::Frame::new()
                .fill(Color32::from_rgb(0x22, 0x24, 0x28))
                .stroke(Stroke::new(1.0, Color32::from_rgb(0x3A, 0x3D, 0x43)))
                .corner_radius(12)
                .inner_margin(16),
        )
        .show(ctx, |ui| {
            ui.set_width(640.0);
            ui.horizontal(|ui| {
                crate::ui::icons::icon(ui, crate::ui::icons::Icon::Search, 18.0, theme::ACCENT);
                let te = ui.add(
                    egui::TextEdit::singleline(&mut query)
                        .hint_text(t(
                            "Item oder Block suchen, z. B. diamond, oak, sword …",
                            "Search item or block, e.g. diamond, oak, sword …",
                        ))
                        .desired_width(ui.available_width() - 34.0)
                        .font(FontId::proportional(16.0)),
                );
                if just_opened {
                    te.request_focus();
                }
                if crate::ui::icons::icon_button(ui, crate::ui::icons::Icon::Close, 24.0, "").clicked() {
                    close = true;
                }
            });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let mut chip = |ui: &mut Ui, label: String, value: PickTab| {
                    let on = tab == value;
                    let b = egui::Button::new(
                        RichText::new(label)
                            .size(12.5)
                            .color(if on { Color32::BLACK } else { theme::TEXT }),
                    )
                    .fill(if on {
                        theme::ACCENT
                    } else {
                        Color32::from_rgb(0x2C, 0x2E, 0x33)
                    })
                    .corner_radius(12);
                    if ui.add(b).clicked() {
                        tab = value;
                    }
                };
                chip(ui, t("Alle", "All").to_string(), PickTab::All);
                chip(ui, t("Items", "Items").to_string(), PickTab::Items);
                chip(ui, t("Blöcke", "Blocks").to_string(), PickTab::Blocks);
                if for_menu {
                    chip(
                        ui,
                        format!("{} ({})", t("Resourcepack", "Resource pack"), pack_items.len()),
                        PickTab::Pack,
                    );
                }
            });
            ui.add_space(8.0);

            // Entries: (item id, model id, label)
            let entries: Vec<(String, String, String)> = if tab == PickTab::Pack {
                let q = query.trim().to_lowercase();
                pack_items
                    .iter()
                    .filter(|(_, m)| q.is_empty() || m.contains(&q))
                    .map(|(b, m)| (b.clone(), m.clone(), m.clone()))
                    .collect()
            } else {
                let filter = match tab {
                    PickTab::Items => items::Filter::Items,
                    PickTab::Blocks => items::Filter::Blocks,
                    _ => items::Filter::All,
                };
                let mut v: Vec<(String, String, String)> = vec![];
                // Custom models first when searching in a menu.
                if for_menu && tab == PickTab::All && !query.trim().is_empty() {
                    let q = query.trim().to_lowercase();
                    v.extend(
                        pack_items
                            .iter()
                            .filter(|(_, m)| m.contains(&q))
                            .map(|(b, m)| (b.clone(), m.clone(), m.clone())),
                    );
                }
                v.extend(items::search(&query, filter).into_iter().map(|i| {
                    (
                        format!("minecraft:{}", i.id),
                        String::new(),
                        i.name.to_string(),
                    )
                }));
                // Newer items that only the texture pack knows.
                let words: Vec<String> = query
                    .trim()
                    .to_lowercase()
                    .replace("minecraft:", "")
                    .split([' ', '_'])
                    .filter(|w| !w.is_empty())
                    .map(str::to_string)
                    .collect();
                v.extend(
                    extra
                        .iter()
                        .filter(|(_, block)| match filter {
                            items::Filter::Items => !block,
                            items::Filter::Blocks => *block,
                            items::Filter::All => true,
                        })
                        .filter(|(id, _)| words.iter().all(|w| id.contains(w.as_str())))
                        .map(|(id, _)| (format!("minecraft:{id}"), String::new(), items::display_name(id))),
                );
                v
            };
            let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
            if enter {
                if let Some(first) = entries.first() {
                    picked = Some((first.0.clone(), first.1.clone()));
                }
            }

            let cell_w = 102.0_f32;
            let cell = 84.0_f32;
            let cols = 6usize;
            let n_rows = entries.len().div_ceil(cols);
            ui.label(
                RichText::new(format!("{} {}", entries.len(), t("Treffer", "results")))
                    .size(11.0)
                    .color(theme::TEXT_WEAK),
            );
            egui::Frame::new()
                .fill(Color32::from_rgb(0x1A, 0x1B, 0x1E))
                .corner_radius(8)
                .inner_margin(6)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.set_height(410.0);
                    if entries.is_empty() {
                        ui.add_space(40.0);
                        ui.vertical_centered(|ui| {
                            ui.label(
                                RichText::new(if tab == PickTab::Pack {
                                    t(
                                        "Keine eigenen Modelle. Füge ein Resourcepack unter „Texturen & Modelle“ hinzu.",
                                        "No custom models. Add a resource pack under “Textures & models”.",
                                    )
                                } else {
                                    t("Nichts gefunden.", "Nothing found.")
                                })
                                .color(theme::TEXT_WEAK),
                            );
                        });
                        return;
                    }
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show_rows(ui, cell, n_rows, |ui, range| {
                            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                            for row in range {
                                ui.horizontal(|ui| {
                                    for (item, model, label) in
                                        entries.iter().skip(row * cols).take(cols)
                                    {
                                        let (r, resp) =
                                            ui.allocate_exact_size(vec2(cell_w, cell), Sense::click());
                                        let inner = r.shrink(3.0);
                                        ui.painter().rect_filled(
                                            inner,
                                            6.0,
                                            if resp.hovered() {
                                                Color32::from_rgb(0x3A, 0x3D, 0x45)
                                            } else {
                                                Color32::from_rgb(0x26, 0x28, 0x2D)
                                            },
                                        );
                                        if !model.is_empty() {
                                            ui.painter().rect_stroke(
                                                inner,
                                                6.0,
                                                Stroke::new(1.0, crate::blocks::Cat::Resources.color().gamma_multiply(0.6)),
                                                StrokeKind::Inside,
                                            );
                                        }
                                        let icon = Rect::from_center_size(
                                            pos2(inner.center().x, inner.top() + 28.0),
                                            vec2(48.0, 48.0),
                                        );
                                        draw_item(app, ui, icon, item, model);
                                        // Name below the picture.
                                        let short = if model.is_empty() {
                                            label.clone()
                                        } else {
                                            label.rsplit(':').next().unwrap_or(label).to_string()
                                        };
                                        let mut job = egui::text::LayoutJob::simple_singleline(
                                            short,
                                            FontId::proportional(10.0),
                                            if resp.hovered() { theme::TEXT } else { theme::TEXT_WEAK },
                                        );
                                        job.wrap = egui::text::TextWrapping::truncate_at_width(cell_w - 14.0);
                                        let g = ui.fonts_mut(|f| f.layout_job(job));
                                        ui.painter().galley(
                                            pos2(inner.center().x - g.size().x / 2.0, inner.bottom() - 19.0),
                                            g,
                                            theme::TEXT_WEAK,
                                        );
                                        if resp.hovered() {
                                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                        }
                                        let tip = if model.is_empty() {
                                            format!("{label}\n{item}")
                                        } else {
                                            format!("{label}\n{} {item}", t("Basis:", "Base:"))
                                        };
                                        if resp.on_hover_text(tip).clicked() {
                                            picked = Some((item.clone(), model.clone()));
                                        }
                                    }
                                });
                            }
                        });
                });
            ui.add_space(6.0);
            let has_textures = app.vanilla.vanilla.is_some();
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(if has_textures || !app.settings.vanilla_textures {
                        t(
                            "Enter nimmt den ersten Treffer. Texturen aus deinem Resourcepack haben Vorrang.",
                            "Enter takes the first result. Textures from your resource pack come first.",
                        )
                    } else {
                        t(
                            "Ohne Minecraft-Texturen werden Platzhalter gezeigt.",
                            "Without Minecraft textures, stand-ins are shown.",
                        )
                    })
                    .size(11.0)
                    .color(theme::TEXT_WEAK),
                );
                if !has_textures && app.settings.vanilla_textures {
                    if ui
                        .small_button(t("Texturen-Pack wählen…", "Pick texture pack…"))
                        .clicked()
                    {
                        pick_textures = true;
                    }
                }
            });
        });
    if modal.should_close() {
        close = true;
    }
    if pick_textures {
        app.pick_texture_source();
    }
    if let Some((item, model)) = picked {
        apply_pick(app, &target, item, model);
        close = true;
    }
    if close {
        app.item_picker = None;
    } else if let Some(pk) = app.item_picker.as_mut() {
        pk.query = query;
        pk.tab = tab;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_items_round_trip_and_read_the_old_format() {
        let old = "11: minecraft:diamond | &bDiamant | &7Zeile 1|&7Zeile 2\n15: minecraft:emerald | &aSmaragd\nkaputt";
        let list = parse(old);
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].slot, 11);
        assert_eq!(list[0].lore, "&7Zeile 1|&7Zeile 2");
        assert_eq!(list[1].name, "&aSmaragd");
        let mut list = list;
        list[1].take = true;
        list[1].amount = 16;
        list[1].model = "magie:stab".into();
        list[1].id = "Kauf Smaragd".into();
        let text = serialize(&list);
        list[1].id = "kauf_smaragd".into();
        assert_eq!(parse(&text), list);
        assert!(text.contains("\t16\tmagie:stab\ttake"));
    }

    #[test]
    fn color_codes_are_handled() {
        assert_eq!(strip_codes("&6&lShop &7(neu)"), "Shop (neu)");
        let job = mc_text("&cRot &aGrün", Color32::WHITE, 12.0);
        assert_eq!(job.sections.len(), 2);
    }
}
