//! Block search ("Search blocks"): filter chips, collapsible categories with counts,
//! recent and favourite blocks, a detail pane and keyboard navigation.

use crate::app::App;
use crate::blocks::{self, BlockDef, CATEGORIES, Cat, Kind, PinType};
use crate::i18n::{t, tl};
use crate::model::{NodeId, PinRef};
use crate::theme;
use crate::ui::canvas::connect as canvas_connect;
use crate::ui::icons::{self, Icon};
use egui::{Color32, FontId, Id, Rect, RichText, Sense, Stroke, StrokeKind, Ui, pos2, vec2};

#[derive(Clone)]
enum Entry {
    Block(&'static BlockDef),
    Command(NodeId, String),
    NewCommand,
}

impl Entry {
    fn key(&self) -> String {
        match self {
            Entry::Block(d) => d.id.to_string(),
            Entry::Command(id, _) => format!("cmd:{id}"),
            Entry::NewCommand => "cmd:new".into(),
        }
    }
    fn name(&self) -> String {
        match self {
            Entry::Block(d) => tl(d.name).to_string(),
            Entry::Command(_, n) => format!("{} · /{n}", t("Befehl", "Command")),
            Entry::NewCommand => t("Neuer Befehl …", "New command …").to_string(),
        }
    }
    fn cat(&self) -> Cat {
        match self {
            Entry::Block(d) => d.cat,
            _ => Cat::Events,
        }
    }
    fn kind(&self) -> Kind {
        match self {
            Entry::Block(d) => d.kind,
            _ => Kind::Event,
        }
    }
    fn def(&self) -> &'static BlockDef {
        match self {
            Entry::Block(d) => d,
            _ => blocks::get("ev_command").unwrap(),
        }
    }
}

fn matches_query(e: &Entry, q: &str) -> bool {
    if q.is_empty() {
        return true;
    }
    let q = q.to_lowercase();
    let d = e.def();
    let name = e.name().to_lowercase();
    q.split_whitespace().all(|w| {
        name.contains(w)
            || d.name.0.to_lowercase().contains(w)
            || d.name.1.to_lowercase().contains(w)
            || d.id.contains(w)
            || d.desc.0.to_lowercase().contains(w)
            || d.desc.1.to_lowercase().contains(w)
            || tl(d.cat.label()).to_lowercase().contains(w)
    })
}

/// 0 = name starts with query, 1 = name contains, 2 = other match.
fn score(e: &Entry, q: &str) -> u8 {
    let q = q.trim().to_lowercase();
    if q.is_empty() {
        return 0;
    }
    let d = e.def();
    let names = [
        e.name().to_lowercase(),
        d.name.0.to_lowercase(),
        d.name.1.to_lowercase(),
    ];
    if names
        .iter()
        .any(|n| n.starts_with(&q) || n.split_whitespace().any(|w| w.starts_with(&q)))
    {
        0
    } else if names.iter().any(|n| n.contains(&q)) {
        1
    } else {
        2
    }
}

/// Pin of `d` that fits the dragged pin, if any.
pub fn fitting_pin(
    d: &'static BlockDef,
    ty: PinType,
    dragged_is_output: bool,
) -> Option<&'static str> {
    if dragged_is_output {
        d.inputs
            .iter()
            .find(|p| {
                if ty == PinType::Exec {
                    p.ty == PinType::Exec
                } else {
                    p.ty != PinType::Exec && blocks::compatible(ty, p.ty)
                }
            })
            .map(|p| p.id.as_ref())
    } else {
        d.outputs
            .iter()
            .find(|p| {
                if ty == PinType::Exec {
                    p.ty == PinType::Exec
                } else {
                    p.ty != PinType::Exec && blocks::compatible(p.ty, ty)
                }
            })
            .map(|p| p.id.as_ref())
    }
}

fn all_entries(app: &App) -> Vec<Entry> {
    let plat = app.project.settings.platform;
    let connect = app.palette.as_ref().and_then(|p| p.connect.clone());
    let mut out = vec![];
    for d in blocks::all() {
        if !d.supports(plat) {
            continue;
        }
        if d.id == "ev_command" {
            for c in &app.project.commands {
                out.push(Entry::Command(c.id, c.name.clone()));
            }
            out.push(Entry::NewCommand);
            continue;
        }
        out.push(Entry::Block(d));
    }
    if let Some((_, _, is_out, ty)) = &connect {
        out.retain(|e| fitting_pin(e.def(), *ty, *is_out).is_some());
    }
    let f = app.palette_filter;
    out.retain(|e| match f {
        1 => e.kind() == Kind::Event,
        2 => matches!(e.kind(), Kind::Action | Kind::Flow),
        3 => e.kind() == Kind::Value,
        _ => true,
    });
    out
}

/// Section of the list: title, entries, collapsible key.
struct Section {
    key: String,
    title: String,
    color: Color32,
    icon: Icon,
    entries: Vec<Entry>,
}

fn sections(app: &App, query: &str) -> Vec<Section> {
    let all = all_entries(app);
    if !query.trim().is_empty() {
        let mut hits: Vec<Entry> = all
            .into_iter()
            .filter(|e| matches_query(e, query))
            .collect();
        hits.sort_by_key(|e| score(e, query));
        return vec![Section {
            key: "hits".into(),
            title: t("Treffer", "Results").into(),
            color: theme::ACCENT,
            icon: Icon::Search,
            entries: hits,
        }];
    }
    let mut out = vec![];
    let find = |k: &String| all.iter().find(|e| e.key() == *k).cloned();
    let recent: Vec<Entry> = app
        .settings
        .recent_blocks
        .iter()
        .filter_map(find)
        .take(6)
        .collect();
    if !recent.is_empty() {
        out.push(Section {
            key: "recent".into(),
            title: t("Zuletzt", "Recent").into(),
            color: theme::TEXT_WEAK,
            icon: Icon::Clock,
            entries: recent,
        });
    }
    let favs: Vec<Entry> = app
        .settings
        .favorite_blocks
        .iter()
        .filter_map(find)
        .collect();
    if !favs.is_empty() {
        out.push(Section {
            key: "favs".into(),
            title: t("Favoriten", "Favourites").into(),
            color: theme::YELLOW,
            icon: Icon::StarFilled,
            entries: favs,
        });
    }
    for cat in CATEGORIES {
        let entries: Vec<Entry> = all.iter().filter(|e| e.cat() == cat).cloned().collect();
        if !entries.is_empty() {
            out.push(Section {
                key: format!("{cat:?}"),
                title: tl(cat.label()).into(),
                color: cat.color(),
                icon: cat.icon_kind(),
                entries,
            });
        }
    }
    out
}

fn chip(ui: &mut Ui, label: &str, on: bool) -> bool {
    let galley =
        ui.painter()
            .layout_no_wrap(label.to_string(), FontId::proportional(12.5), theme::TEXT);
    let (r, resp) = ui.allocate_exact_size(vec2(galley.size().x + 22.0, 26.0), Sense::click());
    let fill = if on {
        Color32::from_rgb(0x3A, 0x3D, 0x44)
    } else if resp.hovered() {
        Color32::from_rgb(0x2E, 0x31, 0x37)
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect(
        r,
        13.0,
        fill,
        Stroke::new(
            1.0,
            if on {
                Color32::from_rgb(0x5A, 0x5E, 0x66)
            } else {
                Color32::from_rgb(0x3A, 0x3D, 0x43)
            },
        ),
        StrokeKind::Inside,
    );
    ui.painter().galley(
        r.center() - galley.size() / 2.0,
        galley,
        if on { Color32::WHITE } else { theme::TEXT },
    );
    resp.clicked()
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    let Some(pal) = app.palette.as_ref() else {
        return;
    };
    let query = pal.query.clone();
    let secs = sections(app, &query);
    let searching = !query.trim().is_empty();
    // Flat list of visible entries for keyboard navigation.
    let visible: Vec<Entry> = secs
        .iter()
        .filter(|s| searching || !app.palette_collapsed.contains(&s.key))
        .flat_map(|s| s.entries.iter().cloned())
        .collect();

    let size = vec2(800.0, 520.0);
    let screen = ctx.content_rect();
    let mut pos = pal.screen_pos;
    pos.x = pos
        .x
        .min(screen.right() - size.x - 8.0)
        .max(screen.left() + 8.0);
    pos.y = pos
        .y
        .min(screen.bottom() - size.y - 8.0)
        .max(screen.top() + 40.0);

    let mut chosen: Option<Entry> = None;
    let mut close = false;
    let mut toggle_fav: Option<String> = None;
    let area = egui::Area::new(Id::unique("block_palette"))
        .fixed_pos(pos)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(Color32::from_rgb(0x22, 0x24, 0x28))
                .stroke(Stroke::new(1.0, Color32::from_rgb(0x3A, 0x3D, 0x43)))
                .corner_radius(10)
                .shadow(egui::epaint::Shadow {
                    offset: [0, 10],
                    blur: 34,
                    spread: 0,
                    color: Color32::from_black_alpha(150),
                })
                .show(ui, |ui| {
                    ui.set_min_size(size);
                    ui.set_max_size(size);
                    ui.spacing_mut().item_spacing = vec2(6.0, 4.0);
                    let outer = ui.max_rect();

                    // ---- search row
                    let pal = app.palette.as_mut().unwrap();
                    let top = Rect::from_min_size(outer.min, vec2(size.x, 44.0));
                    icons::paint(
                        ui.painter(),
                        Rect::from_center_size(
                            top.left_center() + vec2(22.0, 0.0),
                            vec2(16.0, 16.0),
                        ),
                        Icon::Search,
                        theme::TEXT_WEAK,
                    );
                    let te_rect = Rect::from_min_max(
                        top.min + vec2(40.0, 9.0),
                        pos2(top.right() - 150.0, top.bottom() - 9.0),
                    );
                    let te = ui.put(
                        te_rect,
                        egui::TextEdit::singleline(&mut pal.query)
                            .hint_text(t("Blöcke suchen", "Search blocks"))
                            .frame(egui::Frame::NONE)
                            .font(FontId::proportional(15.0)),
                    );
                    if pal.just_opened {
                        te.request_focus();
                        pal.just_opened = false;
                    }
                    if te.changed() {
                        pal.selected = 0;
                    }
                    let connect_ty = pal.connect.as_ref().map(|c| c.3);
                    let label_r = Rect::from_min_max(pos2(top.right() - 140.0, top.top()), top.max);
                    {
                        let mut c = ui.new_child(
                            egui::UiBuilder::new()
                                .max_rect(label_r)
                                .layout(egui::Layout::right_to_left(egui::Align::Center)),
                        );
                        c.add_space(10.0);
                        let lbl = match connect_ty {
                            Some(ty) => {
                                format!("{} {}", t("passend zu", "matching"), tl(ty.label()))
                            }
                            None => t("Alle Blöcke", "All blocks").to_string(),
                        };
                        let g = c.painter().layout_no_wrap(
                            lbl,
                            FontId::proportional(11.0),
                            theme::TEXT_WEAK,
                        );
                        let (r, _) =
                            c.allocate_exact_size(g.size() + vec2(14.0, 8.0), Sense::hover());
                        c.painter()
                            .rect_filled(r, 9.0, Color32::from_rgb(0x30, 0x33, 0x39));
                        c.painter().galley(
                            r.center() - g.size() / 2.0,
                            g,
                            connect_ty.map(|t| t.color()).unwrap_or(theme::TEXT_WEAK),
                        );
                        if icons::icon_button(
                            &mut c,
                            Icon::ChevronDown,
                            22.0,
                            t("Alle aufklappen", "Expand all"),
                        )
                        .clicked()
                        {
                            app.palette_collapsed.clear();
                        }
                        if icons::icon_button(
                            &mut c,
                            Icon::ChevronRight,
                            22.0,
                            t("Alle zuklappen", "Collapse all"),
                        )
                        .clicked()
                        {
                            for s in &secs {
                                app.palette_collapsed.insert(s.key.clone());
                            }
                        }
                    }
                    ui.painter().line_segment(
                        [top.left_bottom(), top.right_bottom()],
                        Stroke::new(1.0, Color32::from_rgb(0x34, 0x37, 0x3D)),
                    );

                    // ---- chips
                    let chips_r = Rect::from_min_size(
                        top.left_bottom() + vec2(14.0, 8.0),
                        vec2(size.x - 28.0, 28.0),
                    );
                    {
                        let mut c = ui.new_child(
                            egui::UiBuilder::new()
                                .max_rect(chips_r)
                                .layout(egui::Layout::left_to_right(egui::Align::Center)),
                        );
                        for (i, l) in [
                            t("Alle", "All"),
                            t("Events", "Events"),
                            t("Aktionen", "Actions"),
                            t("Werte", "Values"),
                        ]
                        .into_iter()
                        .enumerate()
                        {
                            if chip(&mut c, l, app.palette_filter == i as u8) {
                                app.palette_filter = i as u8;
                                if let Some(p) = app.palette.as_mut() {
                                    p.selected = 0;
                                }
                            }
                        }
                    }

                    // ---- keyboard
                    let (up, down, enter, esc) = ui.input_mut(|i| {
                        (
                            i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp),
                            i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown),
                            i.key_pressed(egui::Key::Enter),
                            i.consume_key(egui::Modifiers::NONE, egui::Key::Escape),
                        )
                    });
                    let pal = app.palette.as_mut().unwrap();
                    if down && !visible.is_empty() {
                        pal.selected = (pal.selected + 1).min(visible.len() - 1);
                    }
                    if up {
                        pal.selected = pal.selected.saturating_sub(1);
                    }
                    if enter && !visible.is_empty() {
                        chosen = Some(visible[pal.selected.min(visible.len() - 1)].clone());
                    }
                    if esc {
                        close = true;
                    }
                    let selected = pal.selected.min(visible.len().saturating_sub(1));

                    // ---- list (left) and details (right)
                    let body_top = chips_r.bottom() + 8.0;
                    let footer_h = 30.0;
                    let list_r = Rect::from_min_max(
                        pos2(outer.left(), body_top),
                        pos2(outer.left() + 340.0, outer.bottom() - footer_h),
                    );
                    let detail_r = Rect::from_min_max(
                        pos2(list_r.right(), body_top),
                        pos2(outer.right(), outer.bottom() - footer_h),
                    );
                    ui.painter().line_segment(
                        [detail_r.left_top(), detail_r.left_bottom()],
                        Stroke::new(1.0, Color32::from_rgb(0x34, 0x37, 0x3D)),
                    );
                    ui.painter().line_segment(
                        [
                            pos2(outer.left(), body_top - 1.0),
                            pos2(outer.right(), body_top - 1.0),
                        ],
                        Stroke::new(1.0, Color32::from_rgb(0x34, 0x37, 0x3D)),
                    );

                    let mut hovered: Option<Entry> = None;
                    {
                        let mut lui = ui.new_child(
                            egui::UiBuilder::new()
                                .max_rect(list_r.shrink2(vec2(8.0, 4.0)))
                                .layout(egui::Layout::top_down(egui::Align::Min)),
                        );
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .show(&mut lui, |ui| {
                                let mut flat_index = 0usize;
                                if secs.iter().all(|s| s.entries.is_empty()) {
                                    ui.add_space(10.0);
                                    ui.label(
                                        RichText::new(t("Nichts gefunden.", "Nothing found."))
                                            .color(theme::TEXT_WEAK),
                                    );
                                }
                                for s in &secs {
                                    let collapsed =
                                        !searching && app.palette_collapsed.contains(&s.key);
                                    let (hr, hresp) = ui.allocate_exact_size(
                                        vec2(ui.available_width(), 26.0),
                                        Sense::click(),
                                    );
                                    if hresp.hovered() {
                                        ui.painter().rect_filled(
                                            hr,
                                            4.0,
                                            Color32::from_white_alpha(6),
                                        );
                                    }
                                    icons::paint(
                                        ui.painter(),
                                        Rect::from_center_size(
                                            hr.left_center() + vec2(9.0, 0.0),
                                            vec2(10.0, 10.0),
                                        ),
                                        if collapsed {
                                            Icon::ChevronRight
                                        } else {
                                            Icon::ChevronDown
                                        },
                                        theme::TEXT_WEAK,
                                    );
                                    ui.painter().text(
                                        hr.left_center() + vec2(22.0, 0.0),
                                        egui::Align2::LEFT_CENTER,
                                        s.title.to_uppercase(),
                                        FontId::proportional(11.5),
                                        theme::TEXT_WEAK,
                                    );
                                    let tw = ui
                                        .painter()
                                        .layout_no_wrap(
                                            s.title.to_uppercase(),
                                            FontId::proportional(11.5),
                                            theme::TEXT_WEAK,
                                        )
                                        .size()
                                        .x;
                                    ui.painter().text(
                                        hr.left_center() + vec2(30.0 + tw, 0.0),
                                        egui::Align2::LEFT_CENTER,
                                        s.entries.len().to_string(),
                                        FontId::proportional(11.0),
                                        theme::TEXT_WEAK.gamma_multiply(0.7),
                                    );
                                    if hresp.clicked() && !searching {
                                        if collapsed {
                                            app.palette_collapsed.remove(&s.key);
                                        } else {
                                            app.palette_collapsed.insert(s.key.clone());
                                        }
                                    }
                                    if collapsed {
                                        continue;
                                    }
                                    for e in &s.entries {
                                        let is_sel = flat_index == selected;
                                        flat_index += 1;
                                        let (r, resp) = ui.allocate_exact_size(
                                            vec2(ui.available_width(), 28.0),
                                            Sense::click(),
                                        );
                                        if is_sel || resp.hovered() {
                                            ui.painter().rect_filled(
                                                r,
                                                5.0,
                                                if is_sel {
                                                    theme::ACCENT_BG
                                                } else {
                                                    Color32::from_white_alpha(8)
                                                },
                                            );
                                        }
                                        if is_sel {
                                            ui.painter().rect_filled(
                                                Rect::from_min_size(r.min, vec2(2.0, r.height())),
                                                1.0,
                                                theme::ACCENT,
                                            );
                                        }
                                        let icon = if matches!(
                                            e,
                                            Entry::Command(..) | Entry::NewCommand
                                        ) {
                                            Icon::Slash
                                        } else {
                                            e.cat().icon_kind()
                                        };
                                        icons::paint(
                                            ui.painter(),
                                            Rect::from_center_size(
                                                r.left_center() + vec2(22.0, 0.0),
                                                vec2(14.0, 14.0),
                                            ),
                                            icon,
                                            e.cat().color(),
                                        );
                                        ui.painter().text(
                                            r.left_center() + vec2(38.0, 0.0),
                                            egui::Align2::LEFT_CENTER,
                                            e.name(),
                                            FontId::proportional(13.5),
                                            if is_sel { Color32::WHITE } else { theme::TEXT },
                                        );
                                        if app.settings.favorite_blocks.contains(&e.key()) {
                                            icons::paint(
                                                ui.painter(),
                                                Rect::from_center_size(
                                                    r.right_center() - vec2(14.0, 0.0),
                                                    vec2(11.0, 11.0),
                                                ),
                                                Icon::StarFilled,
                                                theme::YELLOW,
                                            );
                                        }
                                        if is_sel && (up || down) {
                                            resp.scroll_to_me(None);
                                        }
                                        if resp.hovered() {
                                            hovered = Some(e.clone());
                                        }
                                        if resp.clicked() {
                                            chosen = Some(e.clone());
                                        }
                                    }
                                    ui.add_space(4.0);
                                }
                            });
                    }

                    // details of the hovered or selected entry
                    let detail = hovered.or_else(|| visible.get(selected).cloned());
                    if let Some(e) = detail {
                        let mut dui = ui.new_child(
                            egui::UiBuilder::new()
                                .max_rect(detail_r.shrink2(vec2(18.0, 14.0)))
                                .layout(egui::Layout::top_down(egui::Align::Min)),
                        );
                        egui::ScrollArea::vertical()
                            .id_salt("pal_detail")
                            .auto_shrink([false, false])
                            .show(&mut dui, |ui| {
                                details(ui, app, &e, &mut toggle_fav);
                            });
                    }

                    // footer
                    let fr = Rect::from_min_max(
                        pos2(outer.left(), outer.bottom() - footer_h),
                        outer.max,
                    );
                    ui.painter().line_segment(
                        [fr.left_top(), fr.right_top()],
                        Stroke::new(1.0, Color32::from_rgb(0x34, 0x37, 0x3D)),
                    );
                    ui.painter().text(
                        fr.left_center() + vec2(14.0, 0.0),
                        egui::Align2::LEFT_CENTER,
                        t(
                            "Enter: platzieren     ↑/↓: bewegen     Esc: schließen",
                            "Enter: place     Up/Down: move     Esc: close",
                        ),
                        FontId::proportional(11.5),
                        theme::TEXT_WEAK,
                    );
                });
        });

    if let Some(k) = toggle_fav {
        let f = &mut app.settings.favorite_blocks;
        if let Some(i) = f.iter().position(|x| *x == k) {
            f.remove(i);
        } else {
            f.push(k);
        }
        app.settings.save();
    }

    // Click outside closes.
    if ctx.input(|i| i.pointer.any_pressed()) {
        if let Some(p) = ctx.pointer_interact_pos() {
            if !area.response.rect.contains(p)
                && !app.palette.as_ref().map(|p| p.just_opened).unwrap_or(false)
            {
                close = true;
            }
        }
    }

    if let Some(e) = chosen {
        let pal = app.palette.take().unwrap();
        let key = e.key();
        app.settings.recent_blocks.retain(|k| *k != key);
        app.settings.recent_blocks.insert(0, key);
        app.settings.recent_blocks.truncate(12);
        let id = match &e {
            Entry::Block(d) => app.add_node(d.id, pal.scene_pos),
            Entry::Command(cid, _) => {
                let id = app.add_node("ev_command", pal.scene_pos);
                if let Some(n) = app.ws_mut().node_mut(id) {
                    n.values.insert("cmd".into(), cid.to_string());
                }
                id
            }
            Entry::NewCommand => {
                let cid = app.new_command_with_block(pal.scene_pos);
                app.open_commands(Some(cid));
                *app.selection.iter().next().unwrap_or(&0)
            }
        };
        if let Some((node, pin, is_out, ty)) = pal.connect {
            if let Some(p) = fitting_pin(e.def(), ty, is_out) {
                let ws = app.ws_mut();
                let (from, to) = if is_out {
                    (
                        PinRef { node, pin },
                        PinRef {
                            node: id,
                            pin: p.to_string(),
                        },
                    )
                } else {
                    (
                        PinRef {
                            node: id,
                            pin: p.to_string(),
                        },
                        PinRef { node, pin },
                    )
                };
                canvas_connect(ws, from, to, ty == PinType::Exec);
            }
        }
    } else if close {
        app.palette = None;
    }
}

fn pin_list(ui: &mut Ui, title: &str, pins: &[blocks::PinDef]) {
    let pins: Vec<&blocks::PinDef> = pins.iter().filter(|p| p.ty != PinType::Exec).collect();
    if pins.is_empty() {
        return;
    }
    ui.add_space(6.0);
    ui.label(RichText::new(title).size(11.5).color(theme::TEXT_WEAK));
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(6.0, 4.0);
        for p in pins {
            let txt = format!("{} ({})", p.label.text(), tl(p.ty.label()));
            let g = ui
                .painter()
                .layout_no_wrap(txt, FontId::proportional(11.5), theme::TEXT);
            let (r, _) = ui.allocate_exact_size(g.size() + vec2(20.0, 8.0), Sense::hover());
            ui.painter()
                .rect_filled(r, 9.0, Color32::from_rgb(0x2C, 0x2F, 0x35));
            ui.painter()
                .circle_filled(r.left_center() + vec2(8.0, 0.0), 3.0, p.ty.color());
            ui.painter().galley(
                r.left_center() + vec2(14.0, -g.size().y / 2.0),
                g,
                theme::TEXT,
            );
        }
    });
}

fn details(ui: &mut Ui, app: &App, e: &Entry, toggle_fav: &mut Option<String>) {
    let d = e.def();
    ui.label(
        RichText::new(e.name())
            .size(19.0)
            .strong()
            .color(theme::TEXT),
    );
    let (kind_txt, kind_col) = match d.kind {
        Kind::Event => (
            t("Event: startet einen Ablauf", "Event: starts a flow"),
            theme::GREEN,
        ),
        Kind::Action => (
            t("Aktion: macht etwas", "Action: does something"),
            theme::BLUE,
        ),
        Kind::Flow => (
            t(
                "Ablauf: steuert, was passiert",
                "Flow: controls what happens",
            ),
            Cat::Flow.color(),
        ),
        Kind::Value => (
            t(
                "Wert: liefert eine Information",
                "Value: provides information",
            ),
            theme::YELLOW,
        ),
    };
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(kind_txt).size(12.0).color(kind_col));
        crate::ui::widgets::platform_tags(ui, d);
    });
    ui.add_space(6.0);
    match e {
        Entry::Command(id, _) => {
            if let Some(c) = app.project.command(*id) {
                ui.label(RichText::new(c.usage()).monospace().color(theme::TEXT));
                if !c.description.is_empty() {
                    ui.label(RichText::new(&c.description).color(theme::TEXT));
                }
            }
        }
        Entry::NewCommand => {
            ui.label(
                RichText::new(t(
                    "Legt einen neuen Befehl an und öffnet das Befehle-Fenster.",
                    "Creates a new command and opens the commands window.",
                ))
                .color(theme::TEXT),
            );
        }
        _ => {}
    }
    ui.label(RichText::new(tl(d.desc)).size(13.0).color(theme::TEXT));
    match e {
        Entry::Block(_) => {
            pin_list(ui, t("Eingänge", "In"), &d.inputs);
            pin_list(ui, t("Ausgänge", "Out"), &d.outputs);
            let fields: Vec<String> = d.fields.iter().map(|f| tl(f.label).to_string()).collect();
            if !fields.is_empty() {
                ui.add_space(6.0);
                ui.label(
                    RichText::new(format!(
                        "{} {}",
                        t("Einstellungen:", "Settings:"),
                        fields.join(", ")
                    ))
                    .size(11.5)
                    .color(theme::TEXT_WEAK),
                );
            }
        }
        Entry::Command(id, _) => {
            if let Some(c) = app.project.command(*id) {
                let outs: Vec<String> = c
                    .options
                    .iter()
                    .map(|o| format!("{} ({})", o.name, tl(o.ty.label())))
                    .collect();
                ui.add_space(6.0);
                ui.label(
                    RichText::new(format!(
                        "{} {}",
                        t("Ausgänge: Spieler,", "Out: Player,"),
                        outs.join(", ")
                    ))
                    .size(11.5)
                    .color(theme::TEXT_WEAK),
                );
            }
        }
        Entry::NewCommand => {}
    }
    ui.add_space(14.0);
    let key = e.key();
    let fav = app.settings.favorite_blocks.contains(&key);
    if icons::button(
        ui,
        Some(if fav { Icon::StarFilled } else { Icon::Star }),
        if fav {
            t("Aus Favoriten entfernen", "Remove from favourites")
        } else {
            t("Zu Favoriten", "Add to favourites")
        },
    )
    .clicked()
    {
        *toggle_fav = Some(key);
    }
}
