//! Separate "Commands" window: all chat commands of the project in one place.

use crate::app::App;
use crate::codegen::{clean_command, value_of};
use crate::i18n::{t, tl};
use crate::model::{Node, NodeId};
use crate::theme;
use crate::ui::widgets;
use egui::{Color32, Margin, RichText, Sense, Stroke, StrokeKind, vec2};

fn commands(app: &App) -> Vec<(usize, NodeId, String)> {
    let mut v = vec![];
    for (wi, ws) in app.project.workspaces.iter().enumerate() {
        for n in ws.nodes.iter().filter(|n| n.kind == "ev_command") {
            v.push((wi, n.id, clean_command(value_of(n, "name", "hallo"))));
        }
    }
    v.sort_by(|a, b| a.2.cmp(&b.2));
    v
}

fn node_mut(app: &mut App, ws: usize, id: NodeId) -> Option<&mut Node> {
    app.project
        .workspaces
        .get_mut(ws)
        .and_then(|w| w.node_mut(id))
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    let vid = egui::ViewportId::from_hash_of("mcb_commands");
    let builder = egui::ViewportBuilder::default()
        .with_title(t(
            "Befehle – Minecraft Builder",
            "Commands – Minecraft Builder",
        ))
        .with_inner_size([820.0, 560.0])
        .with_min_inner_size([620.0, 400.0])
        .with_icon(theme::app_icon());
    ctx.show_viewport_immediate(vid, builder, |ctx, class| {
        if ctx.input(|i| i.viewport().close_requested()) {
            app.show_commands = false;
        }
        if class == egui::ViewportClass::EmbeddedWindow {
            // Platform without multi-window support: show as a normal egui window.
            let mut open = true;
            egui::Window::new(t("Befehle", "Commands"))
                .open(&mut open)
                .default_size([760.0, 500.0])
                .show(ctx, |ui| content(app, ui));
            if !open {
                app.show_commands = false;
            }
        } else {
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::new()
                        .fill(theme::BG_SIDEBAR)
                        .inner_margin(Margin::ZERO),
                )
                .show(ctx, |ui| content(app, ui));
        }
    });
}

fn content(app: &mut App, ui: &mut egui::Ui) {
    let list = commands(app);
    if app
        .selected_command
        .map(|(w, id)| !list.iter().any(|c| c.0 == w && c.1 == id))
        .unwrap_or(true)
    {
        app.selected_command = list.first().map(|c| (c.0, c.1));
    }

    egui::Panel::left("cmd_list")
        .exact_size(230.0)
        .resizable(false)
        .frame(
            egui::Frame::new()
                .fill(theme::BG_PANEL)
                .inner_margin(Margin::same(12))
                .stroke(Stroke::new(1.0, theme::BORDER)),
        )
        .show(ui, |ui| {
            widgets::section_header(
                ui,
                &format!("{} ({})", t("Befehle", "Commands"), list.len()),
            );
            ui.add_space(8.0);
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .max_height(ui.available_height() - 44.0)
                .show(ui, |ui| {
                    for (w, id, name) in &list {
                        let sel = app.selected_command == Some((*w, *id));
                        let (r, resp) = ui
                            .allocate_exact_size(vec2(ui.available_width(), 30.0), Sense::click());
                        if sel {
                            ui.painter().rect_filled(r, 4.0, theme::ACCENT_BG);
                        } else if resp.hovered() {
                            ui.painter()
                                .rect_filled(r, 4.0, Color32::from_rgb(0x2A, 0x2C, 0x31));
                        }
                        ui.painter().text(
                            r.left_center() + vec2(10.0, 0.0),
                            egui::Align2::LEFT_CENTER,
                            format!("/{name}"),
                            egui::FontId::proportional(14.0),
                            theme::TEXT,
                        );
                        let ws_name = app
                            .project
                            .workspaces
                            .get(*w)
                            .map(|x| x.name.clone())
                            .unwrap_or_default();
                        ui.painter().text(
                            r.right_center() - vec2(8.0, 0.0),
                            egui::Align2::RIGHT_CENTER,
                            ws_name,
                            egui::FontId::proportional(10.5),
                            theme::TEXT_WEAK,
                        );
                        if resp.clicked() {
                            app.selected_command = Some((*w, *id));
                        }
                    }
                });
            ui.add_space(6.0);
            if ui
                .add(
                    egui::Button::new(
                        RichText::new(t("＋ Neuer Befehl", "＋ New command")).strong(),
                    )
                    .min_size(vec2(ui.available_width(), 30.0)),
                )
                .clicked()
            {
                let c = app.canvas.rect.center();
                let view = *app.view();
                let scene = ((c - app.canvas.rect.min) - view.pan) / view.zoom;
                let id = app.add_node("ev_command", scene.to_pos2());
                let n = commands(app).len();
                if let Some(node) = app.ws_mut().node_mut(id) {
                    node.values.insert("name".into(), format!("befehl{n}"));
                }
                app.selected_command = Some((app.active_ws, id));
            }
        });

    egui::CentralPanel::default().frame(egui::Frame::new().fill(theme::BG_SIDEBAR).inner_margin(Margin::same(20))).show(ui, |ui| {
        let Some((w, id)) = app.selected_command else {
            ui.label(RichText::new(t("Noch keine Befehle. Lege links einen an.", "No commands yet. Create one on the left.")).color(theme::TEXT_WEAK));
            return;
        };
        let def = crate::blocks::get("ev_command").unwrap();
        let mut delete = false;
        let mut focus = false;
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            let Some(node) = node_mut(app, w, id) else { return };
            let name = clean_command(value_of(node, "name", "hallo"));
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("/{name}")).size(24.0).strong().color(theme::GREEN));
                ui.label(RichText::new(format!("#{id}")).color(theme::TEXT_WEAK));
            });
            // Preview of how players use it.
            let min = value_of(node, "min_args", "0").parse::<f64>().unwrap_or(0.0) as usize;
            let mut usage = format!("/{name}");
            for i in 0..min.max(0) {
                usage.push_str(&format!(" <{}{}>", t("argument", "argument"), i + 1));
            }
            ui.label(RichText::new(format!("{}: {usage}", t("So tippen Spieler", "Players type"))).monospace().color(theme::TEXT_WEAK));
            ui.add_space(14.0);

            let width = ui.available_width().min(520.0);
            let field = |ui: &mut egui::Ui, node: &mut Node, id_: &str, hint: (&'static str, &'static str)| {
                if let Some(f) = def.field(id_) {
                    ui.label(RichText::new(tl(f.label)).size(12.0).color(theme::TEXT_WEAK));
                    widgets::field_editor(ui, f, &mut node.values, width, ("cmdwin", id, id_));
                    if !hint.0.is_empty() {
                        ui.label(RichText::new(tl(hint)).size(11.0).color(theme::TEXT_WEAK.gamma_multiply(0.85)));
                    }
                    ui.add_space(10.0);
                }
            };
            widgets::section_header(ui, t("Allgemein", "General"));
            ui.add_space(6.0);
            field(ui, node, "name", ("Ohne /, nur Kleinbuchstaben, Zahlen, _ und -", "Without /, only lowercase letters, numbers, _ and -"));
            field(ui, node, "description", ("", ""));
            field(ui, node, "aliases", ("Weitere Namen, mit Komma getrennt, z. B.: h, heilen", "More names, comma separated, e.g.: h, cure"));
            widgets::section_header(ui, t("Wer darf ihn benutzen?", "Who may use it?"));
            ui.add_space(6.0);
            field(ui, node, "players_only", ("Aus: auch die Server-Konsole darf ihn benutzen", "Off: the server console may use it too"));
            field(ui, node, "op", ("Nur Operatoren (OPs) dürfen den Befehl benutzen", "Only operators (OPs) may use the command"));
            field(ui, node, "permission", ("Paper: eigene Berechtigung, z. B. meinplugin.heal (für LuckPerms usw.)", "Paper: own permission, e.g. myplugin.heal (for LuckPerms etc.)"));
            widgets::section_header(ui, t("Argumente", "Arguments"));
            ui.add_space(6.0);
            field(ui, node, "min_args", ("Wie viele Argumente mindestens nötig sind", "How many arguments are required at least"));
            field(ui, node, "usage", ("Wird gezeigt, wenn zu wenige Argumente angegeben werden", "Shown when too few arguments are given"));
            field(ui, node, "suggest", ("Tab-Vorschläge für das erste Argument, mit Komma getrennt", "Tab suggestions for the first argument, comma separated"));
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.add(egui::Button::new(RichText::new(t("Im Editor zeigen", "Show in editor")).color(Color32::BLACK)).fill(theme::ACCENT)).clicked() {
                    focus = true;
                }
                if ui.button(RichText::new(t("Befehl löschen", "Delete command")).color(theme::RED)).clicked() {
                    delete = true;
                }
            });
            ui.add_space(10.0);
            let (r, _) = ui.allocate_exact_size(vec2(width, 54.0), Sense::hover());
            ui.painter().rect(r, 8.0, theme::ACCENT_BG.gamma_multiply(0.5), Stroke::new(1.0, theme::BORDER), StrokeKind::Inside);
            let g = ui.painter().layout(
                t(
                    "Was der Befehl tut, baust du im Editor: verbinde Blöcke mit dem grünen Pfeil des Befehl-Blocks. Argument 1–3 sind die Wörter nach dem Befehl.",
                    "What the command does is built in the editor: connect blocks to the green arrow of the command block. Arguments 1–3 are the words after the command.",
                )
                .to_string(),
                egui::FontId::proportional(11.5),
                theme::TEXT,
                r.width() - 20.0,
            );
            ui.painter().galley(r.min + vec2(10.0, 8.0), g, theme::TEXT);
        });
        if focus {
            app.focus_node = Some((w, id));
        }
        if delete {
            if let Some(ws) = app.project.workspaces.get_mut(w) {
                ws.remove_node(id);
            }
            app.selection.remove(&id);
            app.selected_command = None;
        }
    });
}
