//! "Commands" window: create and edit chat commands with typed options.
//! Edits happen on a draft and are applied with "Save".

use crate::app::App;
use crate::codegen::clean_command;
use crate::i18n::{t, tl};
use crate::model::{CmdAccess, CmdOption, CmdSenders, CommandDef, NodeId, OptType};
use crate::theme;
use crate::ui::icons::{self, Icon};
use crate::ui::widgets;
use egui::{Color32, Margin, RichText, Sense, Stroke, StrokeKind, Ui, vec2};

pub const MAX_OPTIONS: usize = 10;

/// Working copy shown in the window.
pub struct Draft {
    pub commands: Vec<CommandDef>,
    pub selected: Option<NodeId>,
    pub json: bool,
    pub json_text: String,
    pub json_error: Option<String>,
    pub next_id: NodeId,
}

impl App {
    /// Opens the commands window, optionally with a command selected.
    pub fn open_commands(&mut self, select: Option<NodeId>) {
        let commands = self.project.commands.clone();
        let selected = select.or_else(|| commands.first().map(|c| c.id));
        self.cmd_draft = Some(Draft {
            commands,
            selected,
            json: false,
            json_text: String::new(),
            json_error: None,
            next_id: self.project.next_id,
        });
        self.show_commands = true;
    }

    /// Creates a new command and its event block on the canvas.
    pub fn new_command_with_block(&mut self, at: egui::Pos2) -> NodeId {
        let id = self.project.alloc_id();
        let n = self.project.commands.len() + 1;
        let mut c = CommandDef::new(id, &format!("befehl{n}"));
        c.description = t("Ein eigener Befehl", "A custom command").into();
        self.project.commands.push(c);
        let node = self.add_node("ev_command", at);
        if let Some(nd) = self.ws_mut().node_mut(node) {
            nd.values.insert("cmd".into(), id.to_string());
        }
        id
    }

    /// Places the event block of a command (or shows the existing one).
    pub fn place_command_block(&mut self, cmd: NodeId) {
        for (wi, ws) in self.project.workspaces.iter().enumerate() {
            if let Some(n) = ws.nodes.iter().find(|n| {
                n.kind == "ev_command"
                    && n.values
                        .get("cmd")
                        .map(|v| v == &cmd.to_string())
                        .unwrap_or(false)
            }) {
                self.focus_node = Some((wi, n.id));
                return;
            }
        }
        let c = self.canvas.rect.center();
        let view = *self.view();
        let scene = ((c - self.canvas.rect.min) - view.pan) / view.zoom;
        let node = self.add_node("ev_command", scene.to_pos2());
        if let Some(nd) = self.ws_mut().node_mut(node) {
            nd.values.insert("cmd".into(), cmd.to_string());
        }
        self.focus_node = Some((self.active_ws, node));
    }

    fn save_commands(&mut self) {
        let Some(d) = &self.cmd_draft else { return };
        let kept: Vec<String> = d.commands.iter().map(|c| c.id.to_string()).collect();
        self.project.commands = d.commands.clone();
        self.project.next_id = self.project.next_id.max(d.next_id);
        // Blocks of deleted commands go away too.
        for ws in &mut self.project.workspaces {
            let gone: Vec<NodeId> = ws
                .nodes
                .iter()
                .filter(|n| {
                    n.kind == "ev_command"
                        && n.values
                            .get("cmd")
                            .map(|v| !kept.contains(v))
                            .unwrap_or(false)
                })
                .map(|n| n.id)
                .collect();
            for id in gone {
                ws.remove_node(id);
            }
        }
        // Links from options that no longer exist.
        let proj = self.project.clone();
        for ws in &mut self.project.workspaces {
            let nodes = ws.nodes.clone();
            ws.links.retain(|l| {
                let Some(n) = nodes.iter().find(|n| n.id == l.from.node) else {
                    return true;
                };
                if n.kind != "ev_command" {
                    return true;
                }
                crate::blocks::shape(n, &proj)
                    .map(|s| s.output(&l.from.pin).is_some())
                    .unwrap_or(false)
            });
        }
    }
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    if app.cmd_draft.is_none() {
        app.open_commands(None);
    }
    let vid = egui::ViewportId::from_hash_of("mcb_commands");
    let builder = egui::ViewportBuilder::default()
        .with_title(t(
            "Befehle – Minecraft Builder",
            "Commands – Minecraft Builder",
        ))
        .with_inner_size([1180.0, 760.0])
        .with_min_inner_size([900.0, 560.0])
        .with_icon(theme::app_icon());
    ctx.show_viewport_immediate(vid, builder, |ctx, class| {
        if ctx.input(|i| i.viewport().close_requested()) {
            app.show_commands = false;
            app.cmd_draft = None;
        }
        if class == egui::ViewportClass::EmbeddedWindow {
            let mut open = true;
            egui::Window::new(t("Befehle", "Commands"))
                .open(&mut open)
                .default_size([1100.0, 700.0])
                .show(ctx, |ui| window(app, ui));
            if !open {
                app.show_commands = false;
                app.cmd_draft = None;
            }
        } else {
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::new()
                        .fill(theme::BG_SIDEBAR)
                        .inner_margin(Margin::ZERO),
                )
                .show(ctx, |ui| window(app, ui));
        }
    });
}

fn hint(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).size(11.0).color(theme::TEXT_WEAK));
}

fn section(ui: &mut Ui, text: &str) {
    ui.add_space(10.0);
    ui.label(
        RichText::new(text.to_uppercase())
            .size(11.0)
            .color(theme::TEXT_WEAK),
    );
    ui.add_space(4.0);
}

fn window(app: &mut App, ui: &mut Ui) {
    // Footer
    let mut action: Option<u8> = None; // 0 cancel, 1 save, 2 save+close
    egui::Panel::bottom("cmd_footer")
        .exact_size(56.0)
        .frame(
            egui::Frame::new()
                .fill(theme::BG_PANEL)
                .inner_margin(Margin::symmetric(16, 0))
                .stroke(Stroke::new(1.0, theme::BORDER)),
        )
        .show(ui, |ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if icons::styled_button(
                    ui,
                    None,
                    t("Speichern & schließen", "Save and close"),
                    Some((theme::ACCENT, Color32::BLACK)),
                )
                .clicked()
                {
                    action = Some(2);
                }
                if icons::button(ui, None, t("Speichern", "Save")).clicked() {
                    action = Some(1);
                }
                if icons::button(ui, None, t("Abbrechen", "Cancel")).clicked() {
                    action = Some(0);
                }
            });
        });
    // Header
    egui::Panel::top("cmd_header")
        .exact_size(64.0)
        .frame(
            egui::Frame::new()
                .fill(theme::BG_SIDEBAR)
                .inner_margin(Margin::symmetric(20, 0)),
        )
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                let (r, _) = ui.allocate_exact_size(vec2(34.0, 34.0), Sense::hover());
                ui.painter()
                    .rect_filled(r, 8.0, Color32::from_rgb(0x12, 0x4A, 0x55));
                icons::paint(ui.painter(), r.shrink(7.0), Icon::Slash, theme::ACCENT);
                ui.add_space(4.0);
                ui.vertical(|ui| {
                    ui.add_space(12.0);
                    ui.label(
                        RichText::new(t("Befehle", "Commands"))
                            .size(16.0)
                            .color(theme::TEXT),
                    );
                    ui.label(
                        RichText::new(t(
                            "Werden beim Start des Plugins/Mods automatisch registriert",
                            "Registered automatically when the plugin/mod starts",
                        ))
                        .size(11.5)
                        .color(theme::TEXT_WEAK),
                    );
                });
            });
        });

    let draft = app.cmd_draft.as_mut().unwrap();
    // Left: list
    egui::Panel::left("cmd_list")
        .exact_size(250.0)
        .resizable(false)
        .frame(
            egui::Frame::new()
                .fill(theme::BG_SIDEBAR)
                .inner_margin(Margin {
                    left: 16,
                    right: 12,
                    top: 8,
                    bottom: 8,
                }),
        )
        .show(ui, |ui| {
            hint(ui, &t("BEFEHLE", "COMMANDS").to_string());
            ui.add_space(6.0);
            let mut delete = None;
            egui::ScrollArea::vertical()
                .auto_shrink([false, true])
                .max_height(ui.available_height() - 50.0)
                .show(ui, |ui| {
                    for c in &draft.commands {
                        ui.horizontal(|ui| {
                            let sel = draft.selected == Some(c.id);
                            let w = ui.available_width() - 40.0;
                            let (r, resp) = ui.allocate_exact_size(vec2(w, 34.0), Sense::click());
                            let fill = if sel {
                                Color32::from_rgb(0x0E, 0x7C, 0x8C)
                            } else if resp.hovered() {
                                Color32::from_rgb(0x2A, 0x2C, 0x31)
                            } else {
                                Color32::TRANSPARENT
                            };
                            ui.painter().rect_filled(r, 6.0, fill);
                            ui.painter().text(
                                r.center(),
                                egui::Align2::CENTER_CENTER,
                                format!("/{}", c.name),
                                egui::FontId::monospace(13.5),
                                if sel { Color32::WHITE } else { theme::TEXT },
                            );
                            if resp.clicked() {
                                draft.selected = Some(c.id);
                                draft.json_error = None;
                            }
                            let (tr, tresp) =
                                ui.allocate_exact_size(vec2(34.0, 34.0), Sense::click());
                            ui.painter().rect(
                                tr,
                                6.0,
                                if tresp.hovered() {
                                    Color32::from_rgb(0x3A, 0x22, 0x26)
                                } else {
                                    Color32::TRANSPARENT
                                },
                                Stroke::new(1.0, Color32::from_rgb(0x44, 0x47, 0x4E)),
                                StrokeKind::Inside,
                            );
                            icons::paint(
                                ui.painter(),
                                tr.shrink(9.0),
                                Icon::Trash,
                                if tresp.hovered() {
                                    theme::RED
                                } else {
                                    theme::TEXT
                                },
                            );
                            if tresp
                                .on_hover_text(t("Befehl löschen", "Delete command"))
                                .clicked()
                            {
                                delete = Some(c.id);
                            }
                        });
                        ui.add_space(4.0);
                    }
                });
            if let Some(id) = delete {
                draft.commands.retain(|c| c.id != id);
                if draft.selected == Some(id) {
                    draft.selected = draft.commands.first().map(|c| c.id);
                }
            }
            ui.add_space(6.0);
            if icons::button(ui, Some(Icon::Plus), t("Neuer Befehl", "New command")).clicked() {
                let id = draft.next_id;
                draft.next_id += 1;
                let n = draft.commands.len() + 1;
                let mut c = CommandDef::new(id, &format!("befehl{n}"));
                c.description = t("Beschreibe, was er tut", "Describe what it does").into();
                draft.commands.push(c);
                draft.selected = Some(id);
            }
        });

    let sel = draft.selected;
    let platform = app.project.settings.platform;
    // Right: preview
    let mut place = None;
    egui::Panel::right("cmd_preview")
        .exact_size(360.0)
        .resizable(false)
        .frame(
            egui::Frame::new()
                .fill(theme::BG_SIDEBAR)
                .inner_margin(Margin {
                    left: 8,
                    right: 20,
                    top: 8,
                    bottom: 8,
                }),
        )
        .show(ui, |ui| {
            let draft = app.cmd_draft.as_ref().unwrap();
            if let Some(c) = sel.and_then(|id| draft.commands.iter().find(|c| c.id == id)) {
                place = preview(ui, c);
            }
        });

    // Center: editor
    egui::CentralPanel::default()
        .frame(
            egui::Frame::new()
                .fill(theme::BG_SIDEBAR)
                .inner_margin(Margin {
                    left: 16,
                    right: 16,
                    top: 8,
                    bottom: 8,
                }),
        )
        .show(ui, |ui| {
            let (r, _) = ui.allocate_exact_size(vec2(1.0, ui.available_height()), Sense::hover());
            ui.painter().line_segment(
                [
                    r.left_top() + vec2(-16.0, 0.0),
                    r.left_bottom() + vec2(-16.0, 0.0),
                ],
                Stroke::new(1.0, theme::BORDER),
            );
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(ui.max_rect()));
            let draft = app.cmd_draft.as_mut().unwrap();
            let Some(idx) = sel.and_then(|id| draft.commands.iter().position(|c| c.id == id))
            else {
                child.add_space(40.0);
                child.label(
                    RichText::new(t(
                        "Noch keine Befehle. Lege links einen neuen an.",
                        "No commands yet. Create one on the left.",
                    ))
                    .color(theme::TEXT_WEAK),
                );
                return;
            };
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(&mut child, |ui| editor(ui, draft, idx, platform));
        });

    if let Some(id) = place {
        app.save_commands();
        app.place_command_block(id);
    }
    match action {
        Some(0) => {
            app.show_commands = false;
            app.cmd_draft = None;
        }
        Some(1) => app.save_commands(),
        Some(2) => {
            app.save_commands();
            app.show_commands = false;
            app.cmd_draft = None;
        }
        _ => {}
    }
}

fn tab_switch(ui: &mut Ui, json: &mut bool) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        for (on, label) in [(false, "Builder"), (true, "JSON")] {
            let active = *json == on;
            let (r, resp) = ui.allocate_exact_size(vec2(82.0, 30.0), Sense::click());
            let rad = if on {
                egui::CornerRadius {
                    nw: 0,
                    sw: 0,
                    ne: 6,
                    se: 6,
                }
            } else {
                egui::CornerRadius {
                    nw: 6,
                    sw: 6,
                    ne: 0,
                    se: 0,
                }
            };
            ui.painter().rect(
                r,
                rad,
                if active {
                    theme::ACCENT
                } else {
                    Color32::TRANSPARENT
                },
                Stroke::new(
                    1.0,
                    if active {
                        theme::ACCENT
                    } else {
                        Color32::from_rgb(0x44, 0x47, 0x4E)
                    },
                ),
                StrokeKind::Inside,
            );
            ui.painter().text(
                r.center(),
                egui::Align2::CENTER_CENTER,
                label,
                egui::FontId::proportional(13.5),
                if active { Color32::BLACK } else { theme::TEXT },
            );
            if resp.clicked() {
                *json = on;
            }
        }
    });
}

fn editor(ui: &mut Ui, draft: &mut Draft, idx: usize, platform: crate::model::Platform) {
    let mut json = draft.json;
    tab_switch(ui, &mut json);
    if json != draft.json {
        draft.json = json;
        draft.json_text = serde_json::to_string_pretty(&draft.commands[idx]).unwrap_or_default();
        draft.json_error = None;
    }
    if draft.json {
        ui.add_space(10.0);
        hint(
            ui,
            t(
                "Der Befehl als JSON. Änderungen werden übernommen, sobald das JSON gültig ist.",
                "The command as JSON. Changes apply as soon as the JSON is valid.",
            ),
        );
        ui.add_space(4.0);
        let resp = ui.add(
            egui::TextEdit::multiline(&mut draft.json_text)
                .code_editor()
                .desired_rows(24)
                .desired_width(f32::INFINITY),
        );
        if resp.changed() {
            match serde_json::from_str::<CommandDef>(&draft.json_text) {
                Ok(mut c) => {
                    c.id = draft.commands[idx].id;
                    draft.commands[idx] = c;
                    draft.json_error = None;
                }
                Err(e) => draft.json_error = Some(e.to_string()),
            }
        }
        if let Some(e) = &draft.json_error {
            ui.label(RichText::new(e).size(11.5).color(theme::RED));
        }
        return;
    }
    let c = &mut draft.commands[idx];
    let full = ui.available_width();
    section(ui, t("Befehl", "Command"));
    let half = (full - 12.0) / 2.0;
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 12.0;
        ui.vertical(|ui| {
            ui.set_width(half);
            ui.label(RichText::new(t("Name", "Name")).size(13.0));
            let mut name = c.name.clone();
            if ui
                .add(
                    egui::TextEdit::singleline(&mut name)
                        .desired_width(half)
                        .margin(vec2(8.0, 6.0)),
                )
                .changed()
            {
                c.name = clean_command(&name);
            }
            hint(
                ui,
                t(
                    "Was Spieler nach / eintippen. Nur Kleinbuchstaben, Zahlen, _ und -.",
                    "What players type after /. Only lowercase letters, numbers, _ and -.",
                ),
            );
        });
        ui.vertical(|ui| {
            ui.set_width(half);
            ui.label(RichText::new(t("Beschreibung", "Description")).size(13.0));
            ui.add(
                egui::TextEdit::singleline(&mut c.description)
                    .desired_width(half)
                    .margin(vec2(8.0, 6.0)),
            );
            hint(
                ui,
                t(
                    "Steht in der Befehlsliste (/help).",
                    "Shown in the command list (/help).",
                ),
            );
        });
    });
    ui.add_space(8.0);
    ui.label(RichText::new(t("Aliase", "Aliases")).size(13.0));
    let mut aliases = c.aliases.join(", ");
    if ui
        .add(
            egui::TextEdit::singleline(&mut aliases)
                .desired_width(full)
                .margin(vec2(8.0, 6.0))
                .hint_text("h, heilen"),
        )
        .changed()
    {
        c.aliases = aliases
            .split(',')
            .map(|a| clean_command(a))
            .filter(|a| !a.is_empty())
            .collect();
    }
    hint(
        ui,
        t(
            "Weitere Namen für denselben Befehl, mit Komma getrennt.",
            "More names for the same command, comma separated.",
        ),
    );

    section(ui, t("Wer und wo", "Who and where"));
    ui.horizontal(|ui| {
        ui.label(RichText::new(t("Wer darf ihn benutzen", "Who can use it")).size(13.0));
        let label = match c.access {
            CmdAccess::Everyone => t("Alle", "Everyone"),
            CmdAccess::Op => t("Nur Operatoren (OP)", "Operators only (OP)"),
            CmdAccess::Permission => t("Mit Berechtigung", "With permission"),
        };
        egui::ComboBox::from_id_salt(("acc", c.id))
            .selected_text(label)
            .width(220.0)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut c.access, CmdAccess::Everyone, t("Alle", "Everyone"));
                ui.selectable_value(
                    &mut c.access,
                    CmdAccess::Op,
                    t("Nur Operatoren (OP)", "Operators only (OP)"),
                );
                ui.selectable_value(
                    &mut c.access,
                    CmdAccess::Permission,
                    t("Mit Berechtigung", "With permission"),
                );
            });
        if c.access == CmdAccess::Permission {
            ui.add(
                egui::TextEdit::singleline(&mut c.permission)
                    .desired_width(200.0)
                    .hint_text("meinplugin.heal"),
            );
        }
    });
    hint(
        ui,
        match platform {
            crate::model::Platform::Paper => t(
                "Berechtigungen kannst du z. B. mit LuckPerms vergeben. Wer sie nicht hat, sieht den Befehl nicht.",
                "Permissions can be given with e.g. LuckPerms. People without it don't see the command.",
            ),
            crate::model::Platform::Fabric => t(
                "Fabric kennt keine eigenen Berechtigungen – „Mit Berechtigung“ wirkt dort wie „Nur OP“.",
                "Fabric has no own permissions – “With permission” works like “OP only” there.",
            ),
        },
    );
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new(t("Wer kann ihn ausführen", "Who can run it")).size(13.0));
        let label = match c.senders {
            CmdSenders::Players => t("Nur Spieler", "Players only"),
            CmdSenders::Everyone => t("Spieler und Konsole", "Players and console"),
        };
        egui::ComboBox::from_id_salt(("snd", c.id))
            .selected_text(label)
            .width(220.0)
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut c.senders,
                    CmdSenders::Players,
                    t("Nur Spieler", "Players only"),
                );
                ui.selectable_value(
                    &mut c.senders,
                    CmdSenders::Everyone,
                    t("Spieler und Konsole", "Players and console"),
                );
            });
    });
    hint(
        ui,
        t(
            "Mit „Spieler und Konsole“ kann auch die Server-Konsole den Befehl nutzen – dann ist der Spieler-Ausgang manchmal leer.",
            "With “Players and console” the server console can use it too – then the player output is sometimes empty.",
        ),
    );

    section(
        ui,
        &format!(
            "{} · {} {} {MAX_OPTIONS}",
            t("Optionen", "Options"),
            c.options.len(),
            t("von", "of")
        ),
    );
    let n = c.options.len();
    let mut action: Option<(usize, i8)> = None; // (index, -1 up / 1 down / 0 remove)
    for i in 0..n {
        let o = &mut c.options[i];
        egui::Frame::new()
            .fill(Color32::from_rgb(0x28, 0x2A, 0x2F))
            .stroke(Stroke::new(1.0, theme::BORDER))
            .corner_radius(8)
            .inner_margin(12)
            .show(ui, |ui| {
                ui.set_width(full - 24.0);
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut o.name).desired_width(170.0).margin(vec2(8.0, 6.0)).hint_text(t("name", "name")));
                    let cleaned: String = o.name.to_lowercase().chars().filter(|ch| ch.is_alphanumeric() || *ch == '_' || *ch == '-').collect();
                    if cleaned != o.name {
                        o.name = cleaned;
                    }
                    egui::ComboBox::from_id_salt(("ty", i, o.name.len())).selected_text(tl(o.ty.label())).width(130.0).show_ui(ui, |ui| {
                        for ty in OptType::ALL {
                            ui.selectable_value(&mut o.ty, ty, tl(ty.label()));
                        }
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if icons::styled_button(ui, None, t("Entfernen", "Remove"), Some((Color32::from_rgb(0x3A, 0x22, 0x26), theme::RED))).clicked() {
                            action = Some((i, 0));
                        }
                        ui.add_enabled_ui(i + 1 < n, |ui| {
                            if icons::button(ui, None, t("Runter", "Down")).clicked() {
                                action = Some((i, 1));
                            }
                        });
                        ui.add_enabled_ui(i > 0, |ui| {
                            if icons::button(ui, None, t("Hoch", "Up")).clicked() {
                                action = Some((i, -1));
                            }
                        });
                    });
                });
                ui.add_space(6.0);
                ui.add(egui::TextEdit::singleline(&mut o.description).desired_width(f32::INFINITY).margin(vec2(8.0, 6.0)).hint_text(t("Beschreibe diese Option", "Describe this option")));
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new(t("Pflicht", "Required")).size(13.0));
                    widgets::toggle(ui, &mut o.required);
                });
                if matches!(o.ty, OptType::Choice | OptType::Word) {
                    ui.add_space(6.0);
                    let mut list = o.choices.join(", ");
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(if o.ty == OptType::Choice { t("Erlaubte Werte", "Allowed values") } else { t("Vorschläge (Tab)", "Suggestions (tab)") }).size(13.0));
                        if ui.add(egui::TextEdit::singleline(&mut list).desired_width(f32::INFINITY).hint_text("rot, grün, blau")).changed() {
                            o.choices = list.split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect();
                        }
                    });
                }
                ui.add_space(4.0);
                hint(ui, match o.ty {
                    OptType::Word => t("Ein einzelnes Wort.", "A single word."),
                    OptType::Text => t("Alles ab hier bis zum Ende der Nachricht – nur als letzte Option.", "Everything from here to the end of the message – only as last option."),
                    OptType::Number => t("Eine Zahl wie 5 oder 2,5. Sonst bekommt der Spieler eine Fehlermeldung.", "A number like 5 or 2.5. Otherwise the player gets an error message."),
                    OptType::Integer => t("Eine ganze Zahl wie 1, 2, 3.", "A whole number like 1, 2, 3."),
                    OptType::Player => t("Ein Online-Spieler. Tab schlägt Namen vor; ist er offline, kommt eine Fehlermeldung.", "An online player. Tab suggests names; if offline, an error message is shown."),
                    OptType::Bool => t("ja/nein (auch an/aus, true/false).", "yes/no (also on/off, true/false)."),
                    OptType::Choice => t("Nur einer der erlaubten Werte – Tab schlägt sie vor.", "Only one of the allowed values – tab suggests them."),
                });
            });
        ui.add_space(8.0);
    }
    if let Some((i, a)) = action {
        match a {
            0 => {
                c.options.remove(i);
            }
            -1 if i > 0 => c.options.swap(i, i - 1),
            1 if i + 1 < c.options.len() => c.options.swap(i, i + 1),
            _ => {}
        }
    }
    ui.add_enabled_ui(c.options.len() < MAX_OPTIONS, |ui| {
        if icons::button(ui, Some(Icon::Plus), t("Option hinzufügen", "Add option")).clicked() {
            let n = c.options.len() + 1;
            c.options.push(CmdOption {
                name: format!("{}{n}", t("option", "option")),
                description: String::new(),
                ty: OptType::Word,
                required: !c.options.iter().any(|o| !o.required),
                choices: vec![],
            });
        }
    });
    hint(
        ui,
        t(
            "Pflicht-Optionen müssen vor den optionalen stehen.",
            "Required options must come before optional ones.",
        ),
    );
    ui.add_space(20.0);
}

/// Right side: how it looks in Minecraft + the event block. Returns a command to place.
fn preview(ui: &mut Ui, c: &CommandDef) -> Option<NodeId> {
    let mut place = None;
    egui::Frame::new()
        .fill(Color32::from_rgb(0x2A, 0x2C, 0x31))
        .corner_radius(10)
        .inner_margin(14)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                icons::icon(ui, Icon::Eye, 14.0, theme::TEXT_WEAK);
                ui.label(
                    RichText::new(t(
                        "So sieht es in Minecraft aus",
                        "How it looks in Minecraft",
                    ))
                    .size(12.0)
                    .color(theme::TEXT_WEAK),
                );
            });
            ui.add_space(10.0);
            // Suggestion popup like the Minecraft chat.
            egui::Frame::new()
                .fill(Color32::from_rgb(0x16, 0x17, 0x1A))
                .corner_radius(4)
                .inner_margin(10)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    if c.options.is_empty() {
                        ui.label(
                            RichText::new(t("BEFEHLE", "COMMANDS"))
                                .size(10.5)
                                .color(theme::TEXT_WEAK),
                        );
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!("/{}", c.name))
                                    .monospace()
                                    .color(Color32::WHITE),
                            );
                            ui.label(
                                RichText::new(&c.description)
                                    .size(11.5)
                                    .color(theme::TEXT_WEAK),
                            );
                        });
                    } else {
                        ui.label(
                            RichText::new(t("OPTIONEN", "OPTIONS"))
                                .size(10.5)
                                .color(theme::TEXT_WEAK),
                        );
                        for o in &c.options {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(&o.name).monospace().color(if o.required {
                                    Color32::WHITE
                                } else {
                                    Color32::from_rgb(0xAA, 0xAA, 0xAA)
                                }));
                                let d = if o.description.is_empty() {
                                    tl(o.ty.label()).to_string()
                                } else {
                                    o.description.clone()
                                };
                                ui.label(RichText::new(d).size(11.5).color(theme::TEXT_WEAK));
                            });
                        }
                    }
                });
            ui.add_space(4.0);
            egui::Frame::new()
                .fill(Color32::from_black_alpha(140))
                .corner_radius(3)
                .inner_margin(8)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(
                        RichText::new(c.usage())
                            .monospace()
                            .size(14.0)
                            .color(Color32::WHITE),
                    );
                });
            if !c.aliases.is_empty() {
                ui.add_space(4.0);
                hint(
                    ui,
                    &format!(
                        "{} {}",
                        t("Auch:", "Also:"),
                        c.aliases
                            .iter()
                            .map(|a| format!("/{a}"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                );
            }
            ui.add_space(14.0);
            // Dashed box with the event block.
            let inner = egui::Frame::new().inner_margin(12).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(
                    RichText::new(t(
                        "Dieser Befehl ist als Event-Block verfügbar:",
                        "This command is available as an event block:",
                    ))
                    .size(12.0),
                );
                ui.add_space(6.0);
                let (r, resp) =
                    ui.allocate_exact_size(vec2(ui.available_width(), 38.0), Sense::click());
                ui.painter().rect(
                    r,
                    6.0,
                    if resp.hovered() {
                        Color32::from_rgb(0x38, 0x3B, 0x42)
                    } else {
                        Color32::from_rgb(0x32, 0x35, 0x3B)
                    },
                    Stroke::new(1.0, theme::BORDER),
                    StrokeKind::Inside,
                );
                ui.painter().text(
                    r.left_center() + vec2(12.0, 0.0),
                    egui::Align2::LEFT_CENTER,
                    format!("{} · /{}", t("Befehl", "Command"), c.name),
                    egui::FontId::proportional(13.5),
                    theme::TEXT,
                );
                icons::paint(
                    ui.painter(),
                    egui::Rect::from_center_size(
                        r.right_center() - vec2(18.0, 0.0),
                        vec2(14.0, 14.0),
                    ),
                    Icon::Play,
                    theme::GREEN,
                );
                if resp
                    .on_hover_text(t(
                        "Speichert und setzt den Block auf die Fläche (oder zeigt ihn)",
                        "Saves and places the block on the canvas (or shows it)",
                    ))
                    .clicked()
                {
                    place = Some(c.id);
                }
                ui.add_space(6.0);
                if c.options.is_empty() {
                    hint(
                        ui,
                        t(
                            "Er hat noch keine Options-Ausgänge – nur „Spieler“.",
                            "It has no option outputs yet – only “Player”.",
                        ),
                    );
                } else {
                    let outs: Vec<String> = c
                        .options
                        .iter()
                        .map(|o| {
                            format!("{} ({})", o.name, tl(crate::blocks::opt_type(o.ty).label()))
                        })
                        .collect();
                    hint(
                        ui,
                        &format!(
                            "{} {}",
                            t(
                                "Seine Ausgänge sind deine Optionen:",
                                "Its outputs are your options:"
                            ),
                            outs.join(", ")
                        ),
                    );
                }
            });
            dashed_rect(
                ui,
                inner.response.rect,
                theme::TEXT_WEAK.gamma_multiply(0.6),
            );
        });
    place
}

fn dashed_rect(ui: &Ui, r: egui::Rect, color: Color32) {
    let s = Stroke::new(1.0, color);
    let pts = [
        r.left_top(),
        r.right_top(),
        r.right_bottom(),
        r.left_bottom(),
        r.left_top(),
    ];
    for w in pts.windows(2) {
        ui.painter()
            .extend(egui::Shape::dashed_line(&[w[0], w[1]], s, 5.0, 4.0));
    }
}
