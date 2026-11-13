//! Menu bar, activity bar, sidebar, inspector, console, tabs and status bar.

use crate::app::{Activity, App, BottomTab, ConsoleFilter, Dialog, Palette, RightTab};
use crate::blocks::{self, CATEGORIES, Kind, PinType, Special};
use crate::build::Level;
use crate::codegen;
use crate::i18n::{is_german, t, tl};
use crate::model::Platform;
use crate::theme;
use crate::ui::widgets::{self, section_header};
use egui::{Color32, CornerRadius, Frame, Margin, RichText, Sense, Stroke, Ui, vec2};

fn panel_frame(fill: Color32) -> Frame {
    Frame::new().fill(fill).inner_margin(Margin::ZERO)
}

// ---------------------------------------------------------------- top bar

pub fn top_bar(app: &mut App, ui: &mut Ui) {
    egui::Panel::top("top_bar")
        .exact_size(40.0)
        .frame(
            Frame::new()
                .fill(theme::BG_PANEL)
                .inner_margin(Margin::symmetric(8, 0))
                .stroke(Stroke::new(1.0, theme::BORDER)),
        )
        .show(ui, |ui| {
            let full = ui.max_rect();
            ui.horizontal_centered(|ui| {
                // Logo
                let (r, _) = ui.allocate_exact_size(vec2(22.0, 22.0), Sense::hover());
                ui.painter().rect_filled(r.shrink(2.0), 4.0, theme::GREEN);
                ui.painter()
                    .rect_filled(r.shrink(7.0), 2.0, theme::BG_PANEL);
                ui.add_space(4.0);
                egui::MenuBar::new().ui(ui, |ui| menus(app, ui));

                // Right side: Build button and panel toggles
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    for (on, icon, tip) in [
                        (&mut app.show_right, "◨", t("Rechte Leiste", "Right panel")),
                        (
                            &mut app.show_bottom,
                            "⬓",
                            t("Untere Leiste", "Bottom panel"),
                        ),
                        (&mut app.show_sidebar, "◧", t("Seitenleiste", "Sidebar")),
                    ] {
                        let col = if *on { theme::TEXT } else { theme::TEXT_WEAK };
                        if ui
                            .add(
                                egui::Button::new(RichText::new(icon).size(15.0).color(col))
                                    .frame(false),
                            )
                            .on_hover_text(tip)
                            .clicked()
                        {
                            *on = !*on;
                        }
                    }
                    ui.add_space(10.0);
                    if app.job.is_some() {
                        let b = egui::Button::new(
                            RichText::new(format!("⏹ {}", t("Stopp", "Stop")))
                                .color(Color32::BLACK)
                                .strong(),
                        )
                        .fill(theme::RED)
                        .corner_radius(6)
                        .min_size(vec2(76.0, 28.0));
                        if ui.add(b).clicked() {
                            app.cancel_build();
                        }
                        ui.add(egui::Spinner::new().color(theme::GREEN));
                    } else {
                        let b = egui::Button::new(
                            RichText::new(format!("▶ {}", t("Bauen", "Build")))
                                .color(Color32::BLACK)
                                .strong(),
                        )
                        .fill(theme::GREEN)
                        .corner_radius(6)
                        .min_size(vec2(84.0, 28.0));
                        if ui
                            .add(b)
                            .on_hover_text(t(
                                "Erzeugt die fertige .jar-Datei (F5)",
                                "Creates the finished .jar file (F5)",
                            ))
                            .clicked()
                        {
                            let ctx = ui.ctx().clone();
                            app.start_build(&ctx, false);
                        }
                    }
                });
            });

            // Centered search box that opens the block palette.
            let w = 380.0_f32.min(full.width() * 0.32);
            let sr = egui::Rect::from_center_size(full.center(), vec2(w, 26.0));
            let resp = ui.interact(sr, ui.scope_id().with("top_search"), Sense::click());
            let hovered = resp.hovered();
            ui.painter().rect(
                sr,
                CornerRadius::same(6),
                if hovered {
                    Color32::from_rgb(0x2C, 0x2E, 0x33)
                } else {
                    Color32::from_rgb(0x26, 0x28, 0x2C)
                },
                Stroke::new(1.0, theme::BORDER),
                egui::StrokeKind::Inside,
            );
            ui.painter().text(
                sr.left_center() + vec2(10.0, 0.0),
                egui::Align2::LEFT_CENTER,
                format!(
                    "🔍  {} {}",
                    t("Blöcke suchen in", "Search blocks in"),
                    app.project.settings.name
                ),
                egui::FontId::proportional(13.0),
                theme::TEXT_WEAK,
            );
            ui.painter().text(
                sr.right_center() - vec2(10.0, 0.0),
                egui::Align2::RIGHT_CENTER,
                if cfg!(target_os = "macos") {
                    "⌘P"
                } else {
                    "Strg+P"
                },
                egui::FontId::monospace(11.0),
                theme::TEXT_WEAK,
            );
            if resp.clicked() || std::mem::take(&mut app.canvas.focus_top_search) {
                let c = app.canvas.rect.center();
                let view = *app.view();
                let scene = ((c - app.canvas.rect.min) - view.pan) / view.zoom;
                app.palette = Some(Palette {
                    scene_pos: scene.to_pos2(),
                    screen_pos: sr.left_bottom() + vec2(30.0, 6.0),
                    query: String::new(),
                    selected: 0,
                    connect: None,
                    just_opened: true,
                });
            }
        });
}

fn menus(app: &mut App, ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    ui.menu_button(t("Datei", "File"), |ui| {
        if ui
            .button(t("Neues Projekt…   Strg+N", "New project…   Ctrl+N"))
            .clicked()
        {
            app.new_project_dialog();
        }
        if ui.button(t("Öffnen…   Strg+O", "Open…   Ctrl+O")).clicked() {
            app.open_dialog();
        }
        let recent = app.settings.recent.clone();
        ui.add_enabled_ui(!recent.is_empty(), |ui| {
            ui.menu_button(t("Zuletzt geöffnet", "Open recent"), |ui| {
                for p in recent {
                    if ui.button(p.display().to_string()).clicked() {
                        if app.dirty() {
                            app.dialog = Dialog::UnsavedChanges {
                                then: crate::app::AfterSave::Open,
                            };
                        } else {
                            app.open_path(p);
                        }
                    }
                }
            });
        });
        ui.separator();
        if ui
            .button(t("Speichern   Strg+S", "Save   Ctrl+S"))
            .clicked()
        {
            app.save(false);
        }
        if ui.button(t("Speichern unter…", "Save as…")).clicked() {
            app.save(true);
        }
        ui.separator();
        if ui.button(t("Beenden", "Quit")).clicked() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    });
    ui.menu_button(t("Bearbeiten", "Edit"), |ui| {
        if ui
            .add_enabled(
                app.can_undo(),
                egui::Button::new(t("Rückgängig   Strg+Z", "Undo   Ctrl+Z")),
            )
            .clicked()
        {
            app.undo();
        }
        if ui
            .add_enabled(
                app.can_redo(),
                egui::Button::new(t("Wiederholen   Strg+Y", "Redo   Ctrl+Y")),
            )
            .clicked()
        {
            app.redo();
        }
        ui.separator();
        let has_sel = !app.selection.is_empty();
        if ui
            .add_enabled(
                has_sel,
                egui::Button::new(t("Kopieren   Strg+C", "Copy   Ctrl+C")),
            )
            .clicked()
        {
            app.copy_selection(&ctx);
        }
        if ui
            .add_enabled(
                app.clipboard.is_some(),
                egui::Button::new(t("Einfügen   Strg+V", "Paste   Ctrl+V")),
            )
            .clicked()
        {
            if let Some(c) = app.clipboard.clone() {
                app.paste(c, None);
            }
        }
        if ui
            .add_enabled(
                has_sel,
                egui::Button::new(t("Duplizieren   Strg+D", "Duplicate   Ctrl+D")),
            )
            .clicked()
        {
            app.duplicate(&ctx);
        }
        if ui
            .add_enabled(
                has_sel,
                egui::Button::new(t("Löschen   Entf", "Delete   Del")),
            )
            .clicked()
        {
            app.delete_selection();
        }
        ui.separator();
        if ui
            .button(t("Alles auswählen   Strg+A", "Select all   Ctrl+A"))
            .clicked()
        {
            app.selection = app.ws().nodes.iter().map(|n| n.id).collect();
        }
    });
    ui.menu_button(t("Ansicht", "View"), |ui| {
        ui.checkbox(&mut app.show_sidebar, t("Seitenleiste", "Sidebar"));
        ui.checkbox(
            &mut app.show_right,
            t("Inspector / Code", "Inspector / Code"),
        );
        ui.checkbox(
            &mut app.show_bottom,
            t("Konsole / Probleme", "Console / Problems"),
        );
        ui.separator();
        if ui.button(t("Alles anzeigen   F", "Fit all   F")).clicked() {
            app.canvas.fit_request = true;
        }
        ui.checkbox(
            &mut app.settings.snap,
            t("Am Raster ausrichten", "Snap to grid"),
        );
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(t("UI-Größe", "UI scale"));
            for (lbl, z) in [("90%", 0.9), ("100%", 1.0), ("115%", 1.15), ("130%", 1.3)] {
                if ui
                    .selectable_label((app.settings.ui_scale - z).abs() < 0.01, lbl)
                    .clicked()
                {
                    app.settings.ui_scale = z;
                    ctx.set_zoom_factor(z);
                    app.settings.save();
                }
            }
        });
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(t("Sprache", "Language"));
            if ui.selectable_label(is_german(), "Deutsch").clicked() {
                crate::i18n::set_german(true);
                app.settings.german = true;
                app.settings.save();
            }
            if ui.selectable_label(!is_german(), "English").clicked() {
                crate::i18n::set_german(false);
                app.settings.german = false;
                app.settings.save();
            }
        });
    });
    ui.menu_button(t("Projekt", "Project"), |ui| {
        if ui
            .button(t("▶ Bauen (.jar)   F5", "▶ Build (.jar)   F5"))
            .clicked()
        {
            app.start_build(&ctx, false);
        }
        if ui
            .button(t(
                "Als Gradle-Projekt exportieren",
                "Export as Gradle project",
            ))
            .clicked()
        {
            app.start_build(&ctx, true);
        }
        ui.separator();
        if ui
            .button(t("Einstellungen & Versionen…", "Settings & versions…"))
            .clicked()
        {
            app.activity = Activity::Project;
            app.show_sidebar = true;
        }
        if ui
            .button(t("Neuer Arbeitsbereich", "New workspace"))
            .clicked()
        {
            app.add_workspace();
        }
    });
    ui.menu_button(t("Hilfe", "Help"), |ui| {
        if ui.button(t("Kurzanleitung", "Quick guide")).clicked() {
            app.dialog = Dialog::Message {
                title: t("Kurzanleitung", "Quick guide").into(),
                text: guide_text(),
            };
        }
        if ui
            .button(t("Über Minecraft Builder", "About Minecraft Builder"))
            .clicked()
        {
            app.dialog = Dialog::About;
        }
    });
}

fn guide_text() -> String {
    t(
        "1. Wähle unter „Projekt“ Paper (Plugin) oder Fabric (Mod).\n\
         2. Rechtsklick oder Leertaste auf die Fläche öffnet die Block-Suche.\n\
         3. Jeder Ablauf beginnt mit einem grünen Event-Block (z. B. „Spieler betritt Server“ oder „Befehl“).\n\
         4. Verbinde die grünen Pfeile (Ablauf) von links nach rechts. Farbige Punkte sind Werte (Spieler, Text, Zahl …).\n\
         5. Ziehe von einem Ausgang ins Leere, um direkt einen passenden Block zu bekommen.\n\
         6. Nicht verbundene Spieler-Eingänge benutzen automatisch den Spieler aus dem Event.\n\
         7. Farbcodes in Texten: &a grün, &c rot, &e gelb, &l fett …\n\
         8. Mit „▶ Bauen“ (F5) entsteht die .jar-Datei. Dafür muss Java installiert sein (adoptium.net). \
         Der erste Build lädt Gradle und Minecraft herunter und dauert etwas.\n\
         9. Die .jar kommt bei Paper in den Ordner plugins/, bei Fabric in mods/ (zusammen mit der Fabric API).\n\n\
         Tastenkürzel: Entf löschen · Strg+C/V/D · Strg+Z/Y · F alles anzeigen · Mausrad zoomen · \
         Ziehen auf leerer Fläche verschiebt · Shift+Ziehen wählt aus · Rechtsklick auf Pin trennt Verbindungen.",
        "1. Pick Paper (plugin) or Fabric (mod) under “Project”.\n\
         2. Right-click or press space on the canvas to search blocks.\n\
         3. Every flow starts with a green event block (e.g. “Player joins” or “Command”).\n\
         4. Connect the green arrows (flow) from left to right. Colored dots are values (player, text, number …).\n\
         5. Drag from an output into empty space to get a matching block.\n\
         6. Unconnected player inputs automatically use the player from the event.\n\
         7. Color codes in texts: &a green, &c red, &e yellow, &l bold …\n\
         8. “▶ Build” (F5) creates the .jar file. Java must be installed (adoptium.net). \
         The first build downloads Gradle and Minecraft and takes a while.\n\
         9. Put the jar into plugins/ (Paper) or mods/ (Fabric, together with Fabric API).\n\n\
         Shortcuts: Del delete · Ctrl+C/V/D · Ctrl+Z/Y · F fit all · wheel zoom · \
         drag empty space to pan · Shift+drag to select · right-click a pin to disconnect.",
    )
    .to_string()
}

// ------------------------------------------------------------- status bar

pub fn status_bar(app: &mut App, ui: &mut Ui) {
    egui::Panel::bottom("status_bar")
        .exact_size(24.0)
        .frame(
            Frame::new()
                .fill(theme::BG_PANEL)
                .inner_margin(Margin::symmetric(10, 0))
                .stroke(Stroke::new(1.0, theme::BORDER)),
        )
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                let (dot, txt) = if app.job.is_some() {
                    (theme::YELLOW, t("Build läuft…", "Building…"))
                } else {
                    (theme::TEXT_WEAK, t("Bereit", "Ready"))
                };
                ui.label(RichText::new("●").size(10.0).color(dot));
                ui.label(RichText::new(txt).size(12.0).color(theme::TEXT_WEAK));
                ui.add_space(12.0);
                let errors = app.problems.iter().filter(|p| p.error).count();
                let warns = app.problems.len() - errors;
                let ptxt = if app.problems.is_empty() {
                    t("Keine Probleme", "No problems").to_string()
                } else {
                    format!(
                        "{errors} {} · {warns} {}",
                        t("Fehler", "errors"),
                        t("Warnungen", "warnings")
                    )
                };
                let col = if errors > 0 {
                    theme::RED
                } else if warns > 0 {
                    theme::YELLOW
                } else {
                    theme::TEXT_WEAK
                };
                if ui
                    .add(
                        egui::Label::new(RichText::new(ptxt).size(12.0).color(col))
                            .sense(Sense::click()),
                    )
                    .clicked()
                {
                    app.show_bottom = true;
                    app.bottom_tab = BottomTab::Problems;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let s = &app.project.settings;
                    ui.label(
                        RichText::new(format!(
                            "{} · Minecraft {} · Java {}",
                            s.platform.label(),
                            s.versions.minecraft,
                            s.versions.java
                        ))
                        .size(12.0)
                        .color(theme::TEXT_WEAK),
                    );
                    ui.add_space(12.0);
                    ui.label(
                        RichText::new(if app.dirty() {
                            t("Nicht gespeichert", "Unsaved")
                        } else {
                            t("Gespeichert", "Saved")
                        })
                        .size(12.0)
                        .color(if app.dirty() {
                            theme::YELLOW
                        } else {
                            theme::TEXT_WEAK
                        }),
                    );
                });
            });
        });
}

// ----------------------------------------------------------- activity bar

pub fn activity_bar(app: &mut App, ui: &mut Ui) {
    egui::Panel::left("activity_bar")
        .exact_size(46.0)
        .resizable(false)
        .frame(
            Frame::new()
                .fill(theme::BG_PANEL)
                .stroke(Stroke::new(1.0, theme::BORDER)),
        )
        .show(ui, |ui| {
            ui.add_space(6.0);
            let items = [
                (
                    Activity::Workspaces,
                    "🗂",
                    t("Arbeitsbereiche", "Workspaces"),
                ),
                (
                    Activity::Blocks,
                    "🧩",
                    t("Block-Bibliothek", "Block library"),
                ),
                (
                    Activity::Project,
                    "⚙",
                    t("Projekt & Versionen", "Project & versions"),
                ),
                (Activity::Build, "📦", t("Bauen & Export", "Build & export")),
            ];
            for (act, icon, tip) in items {
                let active = app.activity == act && app.show_sidebar;
                let (r, resp) = ui.allocate_exact_size(vec2(46.0, 42.0), Sense::click());
                if active {
                    ui.painter().rect_filled(
                        egui::Rect::from_min_size(r.min, vec2(2.0, r.height())),
                        0.0,
                        theme::ACCENT,
                    );
                }
                let col = if active || resp.hovered() {
                    theme::TEXT
                } else {
                    theme::TEXT_WEAK
                };
                ui.painter().text(
                    r.center(),
                    egui::Align2::CENTER_CENTER,
                    icon,
                    egui::FontId::proportional(19.0),
                    col,
                );
                if resp.clone().on_hover_text(tip).clicked() {
                    if app.activity == act && app.show_sidebar {
                        app.show_sidebar = false;
                    } else {
                        app.activity = act;
                        app.show_sidebar = true;
                    }
                }
            }
        });
}

// --------------------------------------------------------------- sidebar

pub fn sidebar(app: &mut App, ui: &mut Ui) {
    egui::Panel::left("sidebar")
        .default_size(270.0)
        .size_range(200.0..=460.0)
        .frame(panel_frame(theme::BG_SIDEBAR).inner_margin(Margin::symmetric(10, 10)))
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| match app.activity {
                    Activity::Workspaces => sidebar_workspaces(app, ui),
                    Activity::Blocks => sidebar_blocks(app, ui),
                    Activity::Project => sidebar_project(app, ui),
                    Activity::Build => sidebar_build(app, ui),
                });
        });
}

fn list_item(
    ui: &mut Ui,
    selected: bool,
    icon: &str,
    text: &str,
    icon_col: Color32,
) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 26.0), Sense::click());
    if selected {
        ui.painter().rect_filled(r, 0.0, theme::ACCENT_BG);
        ui.painter().rect_filled(
            egui::Rect::from_min_size(r.min, vec2(2.0, r.height())),
            0.0,
            theme::ACCENT,
        );
    } else if resp.hovered() {
        ui.painter()
            .rect_filled(r, 0.0, Color32::from_rgb(0x2A, 0x2C, 0x31));
    }
    ui.painter().text(
        r.left_center() + vec2(14.0, 0.0),
        egui::Align2::LEFT_CENTER,
        icon,
        egui::FontId::proportional(13.0),
        icon_col,
    );
    ui.painter().text(
        r.left_center() + vec2(34.0, 0.0),
        egui::Align2::LEFT_CENTER,
        text,
        egui::FontId::proportional(13.5),
        theme::TEXT,
    );
    resp
}

fn sidebar_workspaces(app: &mut App, ui: &mut Ui) {
    ui.horizontal(|ui| {
        section_header(ui, t("Arbeitsbereiche", "Workspaces"));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add(egui::Button::new("➕").frame(false))
                .on_hover_text(t("Neuer Arbeitsbereich", "New workspace"))
                .clicked()
            {
                app.add_workspace();
            }
        });
    });
    ui.add_space(4.0);
    let mut switch = None;
    let mut rename = None;
    let mut delete = None;
    let mut move_up = None;
    let n = app.project.workspaces.len();
    for (i, ws) in app.project.workspaces.iter().enumerate() {
        let resp = list_item(ui, i == app.active_ws, "▤", &ws.name, theme::TEXT_WEAK);
        if resp.clicked() {
            switch = Some(i);
        }
        if resp.double_clicked() {
            rename = Some(i);
        }
        resp.context_menu(|ui| {
            if ui.button(t("Umbenennen", "Rename")).clicked() {
                rename = Some(i);
                ui.close();
            }
            if i > 0 && ui.button(t("Nach oben", "Move up")).clicked() {
                move_up = Some(i);
                ui.close();
            }
            if n > 1
                && ui
                    .button(RichText::new(t("Löschen", "Delete")).color(theme::RED))
                    .clicked()
            {
                delete = Some(i);
                ui.close();
            }
        });
    }
    if let Some(i) = switch {
        app.switch_ws(i);
    }
    if let Some(i) = rename {
        app.dialog = Dialog::RenameWorkspace {
            index: i,
            name: app.project.workspaces[i].name.clone(),
        };
    }
    if let Some(i) = move_up {
        app.project.workspaces.swap(i, i - 1);
        app.active_ws = i - 1;
    }
    if let Some(i) = delete {
        app.project.workspaces.remove(i);
        app.active_ws = app.active_ws.min(app.project.workspaces.len() - 1);
        app.selection.clear();
    }

    // Commands and events of the whole project.
    ui.add_space(14.0);
    let mut cmds = vec![];
    let mut evs = vec![];
    for (wi, ws) in app.project.workspaces.iter().enumerate() {
        for n in &ws.nodes {
            if let Some(d) = blocks::get(&n.kind) {
                if d.special == Special::Command {
                    cmds.push((
                        wi,
                        n.id,
                        format!(
                            "/{}",
                            codegen::clean_command(codegen::value_of(n, "name", "hallo"))
                        ),
                    ));
                } else if d.kind == Kind::Event {
                    evs.push((wi, n.id, tl(d.name).to_string()));
                }
            }
        }
    }
    ui.horizontal(|ui| {
        section_header(ui, t("Befehle", "Commands"));
        ui.label(
            RichText::new(cmds.len().to_string())
                .size(11.0)
                .color(theme::TEXT_WEAK),
        );
    });
    ui.add_space(4.0);
    for (wi, id, name) in &cmds {
        if list_item(
            ui,
            app.selection.contains(id) && *wi == app.active_ws,
            "⌘",
            name,
            theme::GREEN,
        )
        .clicked()
        {
            app.focus_node = Some((*wi, *id));
        }
    }
    if list_item(
        ui,
        false,
        "➕",
        t("Befehl hinzufügen…", "Add command…"),
        theme::TEXT_WEAK,
    )
    .clicked()
    {
        let c = app.canvas.rect.center();
        let view = *app.view();
        let scene = ((c - app.canvas.rect.min) - view.pan) / view.zoom;
        let id = app.add_node("ev_command", scene.to_pos2());
        app.focus_node = Some((app.active_ws, id));
    }
    ui.add_space(14.0);
    ui.horizontal(|ui| {
        section_header(ui, t("Events", "Events"));
        ui.label(
            RichText::new(evs.len().to_string())
                .size(11.0)
                .color(theme::TEXT_WEAK),
        );
    });
    ui.add_space(4.0);
    for (wi, id, name) in &evs {
        if list_item(
            ui,
            app.selection.contains(id) && *wi == app.active_ws,
            "⚡",
            name,
            theme::GREEN,
        )
        .clicked()
        {
            app.focus_node = Some((*wi, *id));
        }
    }
}

fn sidebar_blocks(app: &mut App, ui: &mut Ui) {
    section_header(ui, t("Block-Bibliothek", "Block library"));
    ui.add_space(4.0);
    ui.add(
        egui::TextEdit::singleline(&mut app.block_search)
            .hint_text(t("Filtern…", "Filter…"))
            .desired_width(f32::INFINITY),
    );
    ui.label(
        RichText::new(t(
            "Ziehen auf die Fläche oder klicken zum Hinzufügen.",
            "Drag onto the canvas or click to add.",
        ))
        .size(11.0)
        .color(theme::TEXT_WEAK),
    );
    ui.add_space(6.0);
    let plat = app.project.settings.platform;
    let q = app.block_search.to_lowercase();
    for cat in CATEGORIES {
        let defs: Vec<_> = blocks::all()
            .iter()
            .filter(|d| d.cat == cat)
            .filter(|d| q.is_empty() || tl(d.name).to_lowercase().contains(&q) || d.id.contains(&q))
            .collect();
        if defs.is_empty() {
            continue;
        }
        egui::CollapsingHeader::new(
            RichText::new(format!("{}  {}", cat.icon(), tl(cat.label()))).color(cat.color()),
        )
        .id_salt(("libcat", cat as u8))
        .default_open(true)
        .show(ui, |ui| {
            for d in defs {
                let supported = d.supports(plat);
                let id = egui::Id::unique(("lib", d.id));
                let resp = ui
                    .dnd_drag_source(id, d.id.to_string(), |ui| {
                        let txt = RichText::new(tl(d.name)).color(if supported {
                            theme::TEXT
                        } else {
                            theme::TEXT_WEAK
                        });
                        ui.add(egui::Label::new(txt).sense(Sense::click()));
                    })
                    .response;
                let resp = resp.on_hover_ui(|ui| {
                    ui.set_max_width(280.0);
                    ui.label(RichText::new(tl(d.name)).strong());
                    ui.label(tl(d.desc));
                    widgets::platform_tags(ui, d);
                });
                if resp.clicked() {
                    let c = app.canvas.rect.center();
                    let view = *app.view();
                    let scene = ((c - app.canvas.rect.min) - view.pan) / view.zoom;
                    app.add_node(d.id, scene.to_pos2());
                }
            }
        });
    }
}

fn labeled(ui: &mut Ui, label: &str, add: impl FnOnce(&mut Ui)) {
    ui.label(RichText::new(label).size(11.5).color(theme::TEXT_WEAK));
    add(ui);
    ui.add_space(6.0);
}

fn sidebar_project(app: &mut App, ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    section_header(ui, t("Projekt", "Project"));
    ui.add_space(8.0);
    ui.label(
        RichText::new(t("Plattform", "Platform"))
            .size(11.5)
            .color(theme::TEXT_WEAK),
    );
    ui.horizontal(|ui| {
        for p in [Platform::Paper, Platform::Fabric] {
            let on = app.project.settings.platform == p;
            let b = egui::Button::new(RichText::new(p.label()).color(if on {
                Color32::BLACK
            } else {
                theme::TEXT
            }))
            .fill(if on {
                theme::ACCENT
            } else {
                Color32::from_rgb(0x2A, 0x2C, 0x31)
            })
            .corner_radius(6)
            .min_size(vec2(110.0, 28.0));
            if ui.add(b).clicked() {
                app.set_platform(p);
            }
        }
    });
    ui.label(
        RichText::new(match app.project.settings.platform {
            Platform::Paper => t(
                "Plugin für Paper/Purpur-Server. Spieler brauchen keine Mods.",
                "Plugin for Paper/Purpur servers. Players need no mods.",
            ),
            Platform::Fabric => t(
                "Mod für Fabric (Server oder Einzelspieler). Braucht die Fabric API.",
                "Mod for Fabric (server or singleplayer). Needs Fabric API.",
            ),
        })
        .size(11.0)
        .color(theme::TEXT_WEAK),
    );
    ui.add_space(10.0);
    let s = &mut app.project.settings;
    labeled(ui, t("Name", "Name"), |ui| {
        if ui
            .add(egui::TextEdit::singleline(&mut s.name).desired_width(f32::INFINITY))
            .changed()
        {
            let id = crate::model::slug(&s.name);
            s.package = format!("de.mcbuilder.{id}");
            s.id = id;
        }
    });
    labeled(
        ui,
        t("ID (klein, ohne Leerzeichen)", "ID (lowercase, no spaces)"),
        |ui| {
            if ui
                .add(egui::TextEdit::singleline(&mut s.id).desired_width(f32::INFINITY))
                .changed()
            {
                s.id = crate::model::slug(&s.id);
            }
        },
    );
    labeled(ui, t("Java-Paket", "Java package"), |ui| {
        ui.add(egui::TextEdit::singleline(&mut s.package).desired_width(f32::INFINITY));
    });
    labeled(ui, t("Version", "Version"), |ui| {
        ui.add(egui::TextEdit::singleline(&mut s.version).desired_width(f32::INFINITY));
    });
    labeled(ui, t("Autor", "Author"), |ui| {
        ui.add(egui::TextEdit::singleline(&mut s.author).desired_width(f32::INFINITY));
    });
    labeled(ui, t("Beschreibung", "Description"), |ui| {
        ui.add(
            egui::TextEdit::multiline(&mut s.description)
                .desired_rows(2)
                .desired_width(f32::INFINITY),
        );
    });

    ui.add_space(10.0);
    section_header(ui, t("Versionen", "Versions"));
    ui.add_space(6.0);
    if app.mc_versions.is_empty() && app.mc_job.is_none() {
        app.fetch_mc_versions(&ctx);
    }
    let plat = app.project.settings.platform;
    let mut options: Vec<String> = app.mc_versions.clone();
    if options.is_empty() {
        options = vec![
            "26.3".into(),
            "26.2".into(),
            "26.1.2".into(),
            "1.21.11".into(),
        ];
    }
    if plat == Platform::Fabric {
        options.retain(|v| !v.starts_with("1."));
    }
    let mut new_mc = None;
    let s = &mut app.project.settings;
    labeled(ui, "Minecraft", |ui| {
        egui::ComboBox::from_id_salt("mc_version")
            .selected_text(&s.versions.minecraft)
            .width(ui.available_width())
            .show_ui(ui, |ui| {
                for v in &options {
                    if ui.selectable_label(*v == s.versions.minecraft, v).clicked() {
                        new_mc = Some(v.clone());
                    }
                }
            });
    });
    let api_label = match plat {
        Platform::Paper => "Paper API",
        Platform::Fabric => "Fabric API",
    };
    labeled(ui, api_label, |ui| {
        ui.add(egui::TextEdit::singleline(&mut s.versions.api).desired_width(f32::INFINITY));
    });
    if plat == Platform::Fabric {
        labeled(ui, "Fabric Loader", |ui| {
            ui.add(egui::TextEdit::singleline(&mut s.versions.loader).desired_width(f32::INFINITY));
        });
        labeled(ui, "Loom", |ui| {
            ui.add(egui::TextEdit::singleline(&mut s.versions.loom).desired_width(f32::INFINITY));
        });
    }
    labeled(ui, "Java", |ui| {
        ui.add(egui::DragValue::new(&mut s.versions.java).range(17..=30));
    });
    if let Some(v) = new_mc {
        s.versions.minecraft = v;
        s.versions.java = crate::build::java_for(&s.versions.minecraft);
        app.fetch_versions(&ctx);
    }
    let busy = app.version_job.is_some();
    if ui
        .add_enabled(
            !busy,
            egui::Button::new(if busy {
                t("Suche…", "Searching…")
            } else {
                t(
                    "⟳ Neueste passende Versionen laden",
                    "⟳ Load latest matching versions",
                )
            }),
        )
        .clicked()
    {
        app.fetch_versions(&ctx);
    }
    ui.label(
        RichText::new(t(
            "Die Blöcke sind für Minecraft 26.x getestet. Paper unterstützt zusätzlich 1.21.11.",
            "Blocks are tested with Minecraft 26.x. Paper also supports 1.21.11.",
        ))
        .size(11.0)
        .color(theme::TEXT_WEAK),
    );
}

fn sidebar_build(app: &mut App, ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    section_header(ui, t("Bauen & Export", "Build & export"));
    ui.add_space(8.0);
    let java = crate::build::find_java();
    match &java {
        Some(p) => {
            ui.label(
                RichText::new(format!("✔ Java: {}", p.display()))
                    .size(11.5)
                    .color(theme::GREEN),
            );
        }
        None => {
            ui.label(
                RichText::new(t("✖ Java nicht gefunden", "✖ Java not found")).color(theme::RED),
            );
            ui.label(
                RichText::new(t(
                    "Zum Bauen wird Java 21+ benötigt:",
                    "Building needs Java 21+:",
                ))
                .size(11.5),
            );
            ui.hyperlink_to("adoptium.net", "https://adoptium.net/");
        }
    }
    ui.add_space(10.0);
    let busy = app.job.is_some();
    let bw = ui.available_width();
    let big = |txt: &str, fill: Color32, fg: Color32| {
        egui::Button::new(RichText::new(txt).color(fg).strong())
            .fill(fill)
            .corner_radius(6)
            .min_size(vec2(bw, 32.0))
    };
    if ui
        .add_enabled(
            !busy,
            big(
                t("▶ .jar bauen", "▶ Build .jar"),
                theme::GREEN,
                Color32::BLACK,
            ),
        )
        .clicked()
    {
        app.start_build(&ctx, false);
    }
    ui.add_space(4.0);
    if ui
        .add_enabled(
            !busy,
            big(
                t("Als Gradle-Projekt exportieren", "Export as Gradle project"),
                Color32::from_rgb(0x2A, 0x2C, 0x31),
                theme::TEXT,
            ),
        )
        .on_hover_text(t(
            "Zum Weiterbearbeiten in IntelliJ IDEA oder VS Code",
            "To keep working in IntelliJ IDEA or VS Code",
        ))
        .clicked()
    {
        app.start_build(&ctx, true);
    }
    if busy && ui.button(t("Abbrechen", "Cancel")).clicked() {
        app.cancel_build();
    }
    ui.add_space(10.0);
    if let Some(jar) = app.last_jar.clone() {
        ui.label(
            RichText::new(t("Letzter Build:", "Last build:"))
                .size(11.5)
                .color(theme::TEXT_WEAK),
        );
        ui.label(RichText::new(jar.display().to_string()).size(11.5));
        ui.horizontal(|ui| {
            if ui.button(t("Ordner öffnen", "Open folder")).clicked() {
                if let Some(p) = jar.parent() {
                    let _ = open::that_detached(p);
                }
            }
        });
        ui.add_space(10.0);
    }
    section_header(ui, t("Test-Server", "Test server"));
    ui.add_space(4.0);
    ui.label(
        RichText::new(match app.project.settings.platform {
            Platform::Paper => t("Ordner plugins/ deines Servers – die .jar wird nach jedem Build dorthin kopiert.", "Your server's plugins/ folder – the jar is copied there after every build."),
            Platform::Fabric => t("Ordner mods/ deines Servers oder Spiels – die .jar wird nach jedem Build dorthin kopiert.", "Your server's or game's mods/ folder – the jar is copied there after every build."),
        })
        .size(11.0)
        .color(theme::TEXT_WEAK),
    );
    ui.horizontal(|ui| {
        let s = &mut app.project.settings;
        ui.add(
            egui::TextEdit::singleline(&mut s.deploy_dir)
                .desired_width(ui.available_width() - 34.0)
                .hint_text(t("(optional)", "(optional)")),
        );
        if ui.button("📁").clicked() {
            if let Some(d) = rfd::FileDialog::new().pick_folder() {
                s.deploy_dir = d.display().to_string();
            }
        }
    });
    if let Some(path) = &app.path {
        ui.add_space(10.0);
        let dir = crate::build::export_dir(path, &app.project);
        ui.label(
            RichText::new(t("Export-Ordner:", "Export folder:"))
                .size(11.5)
                .color(theme::TEXT_WEAK),
        );
        ui.label(RichText::new(dir.display().to_string()).size(11.0));
    }
}

// ------------------------------------------------------------ right panel

pub fn right_panel(app: &mut App, ui: &mut Ui) {
    egui::Panel::right("right_panel")
        .default_size(320.0)
        .size_range(240.0..=640.0)
        .frame(panel_frame(theme::BG_SIDEBAR))
        .show(ui, |ui| {
            tab_strip(
                ui,
                &mut app.right_tab,
                &[
                    (RightTab::Inspector, t("⚙ Inspector", "⚙ Inspector")),
                    (RightTab::Code, t("☕ Java-Code", "☕ Java code")),
                ],
            );
            egui::Frame::new()
                .inner_margin(Margin::same(12))
                .show(ui, |ui| match app.right_tab {
                    RightTab::Inspector => inspector(app, ui),
                    RightTab::Code => code_view(app, ui),
                });
        });
}

fn tab_strip<T: PartialEq + Copy>(ui: &mut Ui, current: &mut T, tabs: &[(T, &str)]) {
    let (strip, _) = ui.allocate_exact_size(vec2(ui.available_width(), 34.0), Sense::hover());
    ui.painter().rect_filled(strip, 0.0, theme::BG_PANEL);
    ui.painter().line_segment(
        [strip.left_bottom(), strip.right_bottom()],
        Stroke::new(1.0, theme::BORDER),
    );
    let mut x = strip.left();
    for (tab, label) in tabs {
        let g = ui.painter().layout_no_wrap(
            label.to_string(),
            egui::FontId::proportional(13.0),
            theme::TEXT,
        );
        let w = g.size().x + 28.0;
        let r = egui::Rect::from_min_size(egui::pos2(x, strip.top()), vec2(w, strip.height()));
        let resp = ui.interact(r, ui.scope_id().with(label), Sense::click());
        let active = *current == *tab;
        if active {
            ui.painter().rect_filled(r, 0.0, theme::BG_SIDEBAR);
            ui.painter().line_segment(
                [
                    r.left_bottom() + vec2(0.0, -1.0),
                    r.right_bottom() + vec2(0.0, -1.0),
                ],
                Stroke::new(2.0, theme::ACCENT),
            );
        }
        let col = if active || resp.hovered() {
            theme::TEXT
        } else {
            theme::TEXT_WEAK
        };
        ui.painter().galley(r.center() - g.size() / 2.0, g, col);
        if resp.clicked() {
            *current = *tab;
        }
        x += w;
    }
}

fn inspector(app: &mut App, ui: &mut Ui) {
    // Note selected?
    if let Some(nid) = app.selected_note {
        let ws_idx = app.active_ws;
        let mut delete = false;
        if let Some(note) = app.project.workspaces[ws_idx]
            .notes
            .iter_mut()
            .find(|n| n.id == nid)
        {
            ui.label(RichText::new(t("Notiz", "Note")).size(16.0).strong());
            ui.add_space(8.0);
            labeled(ui, t("Titel", "Title"), |ui| {
                ui.add(egui::TextEdit::singleline(&mut note.title).desired_width(f32::INFINITY));
            });
            labeled(ui, t("Text", "Text"), |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut note.text)
                        .desired_rows(4)
                        .desired_width(f32::INFINITY),
                );
            });
            labeled(ui, t("Farbe", "Color"), |ui| {
                ui.horizontal(|ui| {
                    for (i, c) in theme::NOTE_COLORS.iter().enumerate() {
                        let (r, resp) = ui.allocate_exact_size(vec2(22.0, 22.0), Sense::click());
                        ui.painter().circle_filled(r.center(), 9.0, *c);
                        if note.color == i {
                            ui.painter().circle_stroke(
                                r.center(),
                                11.0,
                                Stroke::new(2.0, Color32::WHITE),
                            );
                        }
                        if resp.clicked() {
                            note.color = i;
                        }
                    }
                });
            });
            ui.label(
                RichText::new(t(
                    "Blöcke im Rahmen bewegen sich mit, wenn du die Notiz am Kopf ziehst.",
                    "Blocks inside move along when you drag the note by its header.",
                ))
                .size(11.0)
                .color(theme::TEXT_WEAK),
            );
            ui.add_space(8.0);
            if ui
                .button(RichText::new(t("Notiz löschen", "Delete note")).color(theme::RED))
                .clicked()
            {
                delete = true;
            }
        }
        if delete {
            app.project.workspaces[ws_idx].notes.retain(|n| n.id != nid);
            app.selected_note = None;
        }
        return;
    }

    if app.selection.len() != 1 {
        if app.selection.is_empty() {
            ui.label(
                RichText::new(t(
                    "Wähle einen Block aus, um seine Einstellungen zu sehen.",
                    "Select a block to see its settings.",
                ))
                .color(theme::TEXT_WEAK),
            );
        } else {
            ui.label(format!(
                "{} {}",
                app.selection.len(),
                t("Blöcke ausgewählt", "blocks selected")
            ));
            ui.add_space(8.0);
            if ui
                .button(RichText::new(t("Auswahl löschen", "Delete selection")).color(theme::RED))
                .clicked()
            {
                app.delete_selection();
            }
        }
        return;
    }
    let id = *app.selection.iter().next().unwrap();
    let ws_idx = app.active_ws;
    let problems: Vec<(bool, String)> = app
        .problems
        .iter()
        .filter(|p| p.ws == ws_idx && p.node == Some(id))
        .map(|p| (p.error, p.msg.clone()))
        .collect();
    let connected: Vec<String> = app.project.workspaces[ws_idx]
        .links
        .iter()
        .filter(|l| l.to.node == id)
        .map(|l| l.to.pin.clone())
        .collect();
    let mut delete = false;
    {
        let Some(node) = app.project.workspaces[ws_idx].node_mut(id) else {
            return;
        };
        let Some(def) = blocks::get(&node.kind) else {
            ui.label(format!("Unbekannter Block: {}", node.kind));
            return;
        };
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(def.cat.icon())
                    .size(18.0)
                    .color(def.cat.color()),
            );
            ui.label(RichText::new(tl(def.name)).size(17.0).strong());
            ui.label(RichText::new(format!("#{}", node.id)).color(theme::TEXT_WEAK));
        });
        ui.horizontal(|ui| {
            widgets::pill(ui, tl(def.cat.label()), def.cat.color());
            widgets::platform_tags(ui, def);
        });
        ui.add_space(6.0);
        ui.label(RichText::new(tl(def.desc)).color(theme::TEXT_WEAK));
        ui.add_space(10.0);

        for (err, msg) in &problems {
            Frame::new()
                .fill(if *err {
                    theme::RED.gamma_multiply(0.15)
                } else {
                    theme::YELLOW.gamma_multiply(0.12)
                })
                .corner_radius(6)
                .inner_margin(8)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(RichText::new(msg).color(if *err {
                        theme::RED
                    } else {
                        theme::YELLOW
                    }));
                });
            ui.add_space(6.0);
        }

        let w = ui.available_width();
        if !def.fields.is_empty() {
            section_header(ui, t("Einstellungen", "Settings"));
            ui.add_space(4.0);
            for f in &def.fields {
                labeled(ui, tl(f.label), |ui| {
                    widgets::field_editor(ui, f, &mut node.values, w, ("insp_f", id, f.id));
                });
            }
        }
        let inputs: Vec<_> = def
            .inputs
            .iter()
            .filter(|p| p.ty != PinType::Exec)
            .collect();
        if !inputs.is_empty() {
            section_header(ui, t("Eingänge", "Inputs"));
            ui.add_space(4.0);
            for p in inputs {
                let is_conn = connected.iter().any(|c| c == p.id);
                labeled(
                    ui,
                    &format!("{} · {}", tl(p.label), tl(p.ty.label())),
                    |ui| {
                        if is_conn {
                            ui.label(
                                RichText::new(t("↳ verbunden", "↳ connected")).color(p.ty.color()),
                            );
                        } else if p.ty.has_inline() {
                            widgets::pin_editor(
                                ui,
                                p.ty,
                                p.id,
                                p.default,
                                &mut node.values,
                                w,
                                ("insp_p", id, p.id),
                            );
                        } else {
                            ui.label(
                                RichText::new(t(
                                    "automatisch (Spieler aus dem Event)",
                                    "automatic (player from the event)",
                                ))
                                .italics()
                                .color(theme::TEXT_WEAK),
                            );
                        }
                    },
                );
            }
        }
        let outs: Vec<_> = def
            .outputs
            .iter()
            .filter(|p| p.ty != PinType::Exec)
            .collect();
        if !outs.is_empty() {
            section_header(ui, t("Ausgänge", "Outputs"));
            ui.add_space(4.0);
            for p in outs {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("●").color(p.ty.color()));
                    ui.label(format!("{} · {}", tl(p.label), tl(p.ty.label())));
                });
            }
            ui.add_space(8.0);
        }
        ui.add_space(8.0);
        if ui
            .button(RichText::new(t("Block löschen", "Delete block")).color(theme::RED))
            .clicked()
        {
            delete = true;
        }
    }
    if delete {
        app.delete_selection();
    }
}

fn code_view(app: &mut App, ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("{}.java", app.project.settings.main_class())).strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(t("Kopieren", "Copy")).clicked() {
                ui.ctx().copy_text(app.code_preview.clone());
            }
        });
    });
    ui.label(
        RichText::new(t(
            "Live-Vorschau des erzeugten Codes (nur lesen).",
            "Live preview of the generated code (read only).",
        ))
        .size(11.0)
        .color(theme::TEXT_WEAK),
    );
    ui.add_space(6.0);
    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let job = java_highlight(&app.code_preview);
            ui.add(egui::Label::new(job).selectable(true).extend());
        });
}

/// Very small Java syntax highlighter for the preview.
fn java_highlight(code: &str) -> egui::text::LayoutJob {
    use egui::text::{LayoutJob, TextFormat};
    const KW: &[&str] = &[
        "package",
        "import",
        "public",
        "private",
        "static",
        "final",
        "class",
        "void",
        "return",
        "if",
        "else",
        "for",
        "new",
        "true",
        "false",
        "null",
        "int",
        "double",
        "boolean",
        "long",
        "float",
        "extends",
        "implements",
        "instanceof",
        "this",
        "try",
        "catch",
        "switch",
        "case",
    ];
    let font = egui::FontId::monospace(12.0);
    let mut job = LayoutJob::default();
    let fmt = |c: Color32| TextFormat {
        font_id: font.clone(),
        color: c,
        ..Default::default()
    };
    for line in code.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") {
            job.append(line, 0.0, fmt(Color32::from_rgb(0x6A, 0x99, 0x55)));
            job.append("\n", 0.0, fmt(theme::TEXT));
            continue;
        }
        let mut chars = line.char_indices().peekable();
        let mut word = String::new();
        let flush = |job: &mut LayoutJob, word: &mut String| {
            if word.is_empty() {
                return;
            }
            let c = if KW.contains(&word.as_str()) {
                Color32::from_rgb(0xC5, 0x86, 0xC0)
            } else if word.starts_with('@') {
                theme::YELLOW
            } else if word
                .chars()
                .next()
                .map(|c| c.is_ascii_digit())
                .unwrap_or(false)
            {
                Color32::from_rgb(0xB5, 0xCE, 0xA8)
            } else if word
                .chars()
                .next()
                .map(|c| c.is_ascii_uppercase())
                .unwrap_or(false)
            {
                Color32::from_rgb(0x4E, 0xC9, 0xB0)
            } else {
                Color32::from_rgb(0x9C, 0xDC, 0xFE)
            };
            job.append(word, 0.0, fmt(c));
            word.clear();
        };
        while let Some((_, ch)) = chars.next() {
            if ch == '"' {
                flush(&mut job, &mut word);
                let mut s = String::from('"');
                let mut esc = false;
                for (_, c2) in chars.by_ref() {
                    s.push(c2);
                    if c2 == '"' && !esc {
                        break;
                    }
                    esc = c2 == '\\' && !esc;
                }
                job.append(&s, 0.0, fmt(Color32::from_rgb(0xCE, 0x91, 0x78)));
            } else if ch.is_alphanumeric()
                || ch == '_'
                || ch == '@'
                || (ch == '.' && word.chars().all(|c| c.is_ascii_digit()) && !word.is_empty())
            {
                word.push(ch);
            } else {
                flush(&mut job, &mut word);
                job.append(
                    &ch.to_string(),
                    0.0,
                    fmt(Color32::from_rgb(0xC8, 0xCC, 0xD4)),
                );
            }
        }
        flush(&mut job, &mut word);
        job.append("\n", 0.0, fmt(theme::TEXT));
    }
    job
}

// ----------------------------------------------------------- bottom panel

pub fn bottom_panel(app: &mut App, ui: &mut Ui) {
    egui::Panel::bottom("bottom_panel")
        .resizable(true)
        .default_size(200.0)
        .size_range(90.0..=600.0)
        .frame(panel_frame(theme::BG_PANEL))
        .show(ui, |ui| {
            let errors = app.problems.iter().filter(|p| p.error).count();
            let probs_label = format!("⚠ {} {}", t("Probleme", "Problems"), app.problems.len());
            let cons_label = t("▤ Konsole", "▤ Console").to_string();
            tab_strip(
                ui,
                &mut app.bottom_tab,
                &[
                    (BottomTab::Console, &cons_label),
                    (BottomTab::Problems, &probs_label),
                ],
            );
            let _ = errors;
            egui::Frame::new()
                .inner_margin(Margin::symmetric(12, 8))
                .show(ui, |ui| match app.bottom_tab {
                    BottomTab::Console => console(app, ui),
                    BottomTab::Problems => problems(app, ui),
                });
        });
}

fn console(app: &mut App, ui: &mut Ui) {
    ui.horizontal(|ui| {
        if ui.button(t("Leeren", "Clear")).clicked() {
            app.console.clear();
        }
        ui.add_space(6.0);
        let count = |l: Level| app.console.iter().filter(|x| x.level == l).count();
        let chips = [
            (ConsoleFilter::All, format!("{}", t("Alle", "All"))),
            (ConsoleFilter::Info, format!("Info {}", count(Level::Info))),
            (
                ConsoleFilter::Done,
                format!("{} {}", t("Erfolg", "Done"), count(Level::Done)),
            ),
            (
                ConsoleFilter::Warn,
                format!("{} {}", t("Warnungen", "Warnings"), count(Level::Warn)),
            ),
            (
                ConsoleFilter::Error,
                format!("{} {}", t("Fehler", "Errors"), count(Level::Error)),
            ),
        ];
        for (f, label) in chips {
            let on = app.console_filter == f;
            let b = egui::Button::new(RichText::new(label).size(12.0).color(if on {
                Color32::BLACK
            } else {
                theme::TEXT
            }))
            .fill(if on {
                theme::TEXT
            } else {
                Color32::from_rgb(0x26, 0x28, 0x2D)
            })
            .corner_radius(12);
            if ui.add(b).clicked() {
                app.console_filter = f;
            }
        }
        ui.add(
            egui::TextEdit::singleline(&mut app.console_search)
                .hint_text(t("Filtern…", "Filter…"))
                .desired_width(180.0),
        );
    });
    ui.add_space(6.0);
    let filter = app.console_filter;
    let q = app.console_search.to_lowercase();
    let lines: Vec<&crate::app::LogLine> = app
        .console
        .iter()
        .filter(|l| match filter {
            ConsoleFilter::All => true,
            ConsoleFilter::Info => l.level == Level::Info,
            ConsoleFilter::Done => l.level == Level::Done,
            ConsoleFilter::Warn => l.level == Level::Warn,
            ConsoleFilter::Error => l.level == Level::Error,
        })
        .filter(|l| q.is_empty() || l.text.to_lowercase().contains(&q))
        .collect();
    let row_h = 18.0;
    ui.spacing_mut().item_spacing.y = 2.0;
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .stick_to_bottom(true)
        .show_rows(ui, row_h, lines.len(), |ui, range| {
            for l in &lines[range] {
                ui.horizontal(|ui| {
                    let (tag, col) = match l.level {
                        Level::Info => ("INFO ", theme::BLUE),
                        Level::Done => ("DONE ", theme::GREEN),
                        Level::Warn => ("WARN ", theme::YELLOW),
                        Level::Error => ("ERROR", theme::RED),
                    };
                    ui.label(
                        RichText::new(&l.time)
                            .monospace()
                            .size(12.0)
                            .color(theme::TEXT_WEAK),
                    );
                    ui.label(RichText::new(tag).monospace().size(12.0).color(col));
                    ui.add(
                        egui::Label::new(
                            RichText::new(&l.text)
                                .monospace()
                                .size(12.0)
                                .color(theme::TEXT),
                        )
                        .selectable(true),
                    );
                });
            }
        });
}

fn problems(app: &mut App, ui: &mut Ui) {
    if app.problems.is_empty() {
        ui.label(
            RichText::new(t("✔ Keine Probleme gefunden.", "✔ No problems found."))
                .color(theme::GREEN),
        );
        return;
    }
    let mut focus = None;
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for p in &app.problems {
                let ws_name = app
                    .project
                    .workspaces
                    .get(p.ws)
                    .map(|w| w.name.as_str())
                    .unwrap_or("?");
                let (icon, col) = if p.error {
                    ("✖", theme::RED)
                } else {
                    ("⚠", theme::YELLOW)
                };
                let r = ui
                    .horizontal(|ui| {
                        ui.label(RichText::new(icon).color(col));
                        ui.label(RichText::new(&p.msg).color(theme::TEXT));
                        if let Some(n) = p.node {
                            ui.label(
                                RichText::new(format!("{ws_name} · #{n}"))
                                    .size(11.0)
                                    .color(theme::TEXT_WEAK),
                            );
                        }
                    })
                    .response;
                let r = ui.interact(
                    r.rect,
                    egui::Id::unique(("prob", p.ws, p.node, &p.msg)),
                    Sense::click(),
                );
                if r.hovered() {
                    ui.painter()
                        .rect_filled(r.rect, 3.0, Color32::from_white_alpha(8));
                }
                if r.clicked() {
                    if let Some(n) = p.node {
                        focus = Some((p.ws, n));
                    }
                }
            }
        });
    if let Some(f) = focus {
        app.focus_node = Some(f);
    }
}

// ------------------------------------------------------------------ tabs

pub fn tabs(app: &mut App, ui: &mut Ui) {
    let (strip, _) = ui.allocate_exact_size(vec2(ui.available_width(), 34.0), Sense::hover());
    ui.painter().rect_filled(strip, 0.0, theme::BG_PANEL);
    ui.painter().line_segment(
        [strip.left_bottom(), strip.right_bottom()],
        Stroke::new(1.0, theme::BORDER),
    );
    let mut x = strip.left();
    let mut switch = None;
    for (i, ws) in app.project.workspaces.iter().enumerate() {
        let label = format!("▤  {}", ws.name);
        let g = ui
            .painter()
            .layout_no_wrap(label, egui::FontId::proportional(13.0), theme::TEXT);
        let w = g.size().x + 30.0;
        let r = egui::Rect::from_min_size(egui::pos2(x, strip.top()), vec2(w, strip.height()));
        if r.left() > strip.right() {
            break;
        }
        let resp = ui.interact(r, ui.scope_id().with(("ws_tab", i)), Sense::click());
        let active = i == app.active_ws;
        if active {
            ui.painter().rect_filled(r, 0.0, theme::BG_CANVAS);
            ui.painter().line_segment(
                [r.left_top(), r.right_top()],
                Stroke::new(2.0, theme::ACCENT),
            );
        }
        ui.painter().line_segment(
            [r.right_top(), r.right_bottom()],
            Stroke::new(1.0, theme::BORDER),
        );
        let col = if active || resp.hovered() {
            theme::TEXT
        } else {
            theme::TEXT_WEAK
        };
        ui.painter()
            .galley(r.left_center() + vec2(15.0, -g.size().y / 2.0), g, col);
        if resp.clicked() {
            switch = Some(i);
        }
        x += w;
    }
    let plus = egui::Rect::from_min_size(egui::pos2(x, strip.top()), vec2(34.0, strip.height()));
    let pr = ui.interact(plus, ui.scope_id().with("ws_tab_plus"), Sense::click());
    ui.painter().text(
        plus.center(),
        egui::Align2::CENTER_CENTER,
        "+",
        egui::FontId::proportional(16.0),
        if pr.hovered() {
            theme::TEXT
        } else {
            theme::TEXT_WEAK
        },
    );
    if pr
        .on_hover_text(t("Neuer Arbeitsbereich", "New workspace"))
        .clicked()
    {
        app.add_workspace();
    }
    if let Some(i) = switch {
        app.switch_ws(i);
    }
}
