//! "Plugins" panel: the MCP server that lets an AI assistant work on the project.

use crate::app::App;
use crate::i18n::t;
use crate::mcp;
use crate::theme;
use crate::ui::widgets::{section_header, toggle};
use egui::{Color32, RichText, Stroke, Ui};

fn hint(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).size(11.0).color(theme::TEXT_WEAK));
}

fn copy_row(ui: &mut Ui, label: &str, text: &str) {
    ui.label(RichText::new(label).size(11.5).color(theme::TEXT_WEAK));
    egui::Frame::new()
        .fill(theme::INPUT_BG)
        .corner_radius(4)
        .inner_margin(6)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.add(egui::Label::new(RichText::new(text).monospace().size(11.0)).wrap());
        });
    if ui.small_button(t("Kopieren", "Copy")).clicked() {
        ui.ctx().copy_text(text.to_string());
    }
    ui.add_space(4.0);
}

pub fn sidebar(app: &mut App, ui: &mut Ui) {
    section_header(ui, t("Plugins", "Plugins"));
    ui.add_space(6.0);
    egui::Frame::new()
        .fill(Color32::from_rgb(0x26, 0x28, 0x2D))
        .stroke(Stroke::new(1.0, theme::BORDER))
        .corner_radius(8)
        .inner_margin(10)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let mut on = app.settings.mcp_enabled;
                if toggle(ui, &mut on).changed() {
                    app.settings.mcp_enabled = on;
                    if !on {
                        app.mcp = None;
                    }
                    app.mcp_error = None;
                    app.settings.save();
                }
                ui.label(RichText::new("MCP").strong().size(15.0));
                ui.label(
                    RichText::new(if app.mcp.is_some() {
                        t("läuft", "running")
                    } else {
                        t("aus", "off")
                    })
                    .size(11.5)
                    .color(if app.mcp.is_some() { theme::GREEN } else { theme::TEXT_WEAK }),
                );
            });
            hint(
                ui,
                t(
                    "Verbinde eine KI (z. B. Claude) über MCP mit dem Builder. Sie kann dann dein offenes Projekt lesen, Blöcke hinzufügen und verbinden, Fehler prüfen und bauen – du siehst alles sofort auf der Fläche und kannst es mit Strg+Z zurücknehmen.",
                    "Connect an AI (e.g. Claude) to the builder via MCP. It can then read your open project, add and connect blocks, check problems and build – you see everything on the canvas at once and can undo it with Ctrl+Z.",
                ),
            );
            if let Some(e) = &app.mcp_error {
                ui.label(RichText::new(e).size(11.5).color(theme::RED));
            }
        });
    // ---- one click per AI program
    ui.add_space(12.0);
    section_header(ui, t("Zu einer KI hinzufügen", "Add to an AI program"));
    ui.add_space(2.0);
    hint(
        ui,
        t(
            "„Hinzufügen“ trägt den Builder automatisch in die Einstellungen des Programms ein. Danach das Programm einmal neu starten – fertig.",
            "“Add” enters the builder into the program's settings automatically. Then restart the program once – done.",
        ),
    );
    ui.add_space(6.0);
    let now = ui.ctx().input(|i| i.time);
    if app
        .mcp_clients
        .as_ref()
        .map(|(at, _)| now - *at > 3.0)
        .unwrap_or(true)
    {
        app.mcp_clients = Some((
            now,
            mcp::Client::ALL.iter().map(|c| (*c, c.state())).collect(),
        ));
    }
    let states = app.mcp_clients.clone().map(|c| c.1).unwrap_or_default();
    let exe =
        std::env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("minecraft-builder"));
    for (client, state) in states {
        ui.horizontal(|ui| {
            ui.label(RichText::new(client.label()).strong());
            let (txt, col) = match state {
                mcp::ClientState::Added => (t("Hinzugefügt", "Added"), theme::GREEN),
                mcp::ClientState::NotAdded => {
                    (t("Nicht hinzugefügt", "Not added"), theme::TEXT_WEAK)
                }
                mcp::ClientState::NotInstalled => (
                    t(
                        "Nicht auf diesem Computer gefunden",
                        "Not found on this computer",
                    ),
                    theme::TEXT_WEAK,
                ),
            };
            ui.label(RichText::new(txt).size(11.5).color(col));
        });
        let add = state != mcp::ClientState::Added;
        let label = if add {
            t("Hinzufügen", "Add")
        } else {
            t("Entfernen", "Remove")
        };
        let btn = if add && state == mcp::ClientState::NotAdded {
            egui::Button::new(RichText::new(label).color(Color32::BLACK).strong())
                .fill(theme::ACCENT)
        } else {
            egui::Button::new(label)
        }
        .corner_radius(6)
        .min_size(egui::vec2(96.0, 28.0));
        if ui.add(btn).clicked() {
            let res = client.set(add, &exe);
            app.mcp_note = Some(match res {
                Ok(()) if add => {
                    // The AI needs the server: switch MCP on.
                    if !app.settings.mcp_enabled {
                        app.settings.mcp_enabled = true;
                        app.mcp_error = None;
                        app.settings.save();
                    }
                    (
                        true,
                        format!(
                            "{} {} – {}",
                            t("Zu", "Added to"),
                            client.label(),
                            t(
                                "hinzugefügt. Starte das Programm jetzt neu (ganz beenden, auch im Infobereich).",
                                "Restart the program now (quit it completely, in the tray too)."
                            )
                        ),
                    )
                }
                Ok(()) => (
                    true,
                    format!("{} {}", t("Entfernt aus", "Removed from"), client.label()),
                ),
                Err(e) => (false, e),
            });
            app.mcp_clients = None;
        }
        ui.add_space(6.0);
    }
    if let Some((ok, text)) = &app.mcp_note {
        egui::Frame::new()
            .fill(if *ok {
                theme::ACCENT_BG
            } else {
                theme::RED.gamma_multiply(0.15)
            })
            .corner_radius(6)
            .inner_margin(8)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new(text).size(12.0).color(if *ok {
                    theme::TEXT
                } else {
                    theme::RED
                }));
            });
        ui.add_space(6.0);
    }
    egui::CollapsingHeader::new(RichText::new(t("Andere MCP-Programme", "Other MCP programs")).size(12.5)).show(ui, |ui| {
        copy_row(
            ui,
            t("In die MCP-Einstellungen des Programms einfügen", "Paste into the program's MCP settings"),
            &mcp::generic_config(&exe),
        );
        if app.settings.mcp_enabled {
            let url = mcp::url(app.settings.mcp_port, &app.settings.mcp_token);
            copy_row(ui, t("Oder direkt per HTTP-Adresse", "Or directly by HTTP address"), &url);
            ui.horizontal(|ui| {
                ui.label(RichText::new(t("Port", "Port")).size(12.0));
                let mut port = app.settings.mcp_port;
                if ui.add(egui::DragValue::new(&mut port).range(1024..=65535)).changed() {
                    app.settings.mcp_port = port;
                    app.mcp = None;
                    app.mcp_error = None;
                }
                if ui
                    .small_button(t("Neue Adresse", "New address"))
                    .on_hover_text(t(
                        "Erzeugt einen neuen geheimen Teil der Adresse. Hinzugefügte Programme funktionieren weiter.",
                        "Creates a new secret part of the address. Added programs keep working.",
                    ))
                    .clicked()
                {
                    app.settings.mcp_token = mcp::new_token();
                    app.mcp = None;
                    app.settings.save();
                }
            });
        }
    });
    hint(
        ui,
        t(
            "Die Werkzeuge der KI funktionieren, solange der Builder mit einem Projekt offen und MCP eingeschaltet ist. Der Server ist nur von diesem Computer aus erreichbar.",
            "The AI's tools work while the builder is open with a project and MCP is switched on. The server can only be reached from this computer.",
        ),
    );
    ui.add_space(12.0);
    section_header(ui, t("Was die KI kann", "What the AI can do"));
    ui.add_space(4.0);
    for (name, de, en) in mcp::TOOL_INFO {
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new(*name)
                    .monospace()
                    .size(11.0)
                    .color(theme::ACCENT),
            );
            ui.label(RichText::new(t(de, en)).size(11.5));
        });
    }
    if !app.mcp_log.is_empty() {
        ui.add_space(12.0);
        section_header(ui, t("Letzte Aufrufe", "Recent calls"));
        ui.add_space(4.0);
        for l in app.mcp_log.iter().rev().take(15) {
            ui.label(
                RichText::new(l)
                    .monospace()
                    .size(10.5)
                    .color(theme::TEXT_WEAK),
            );
        }
    }
}

/// Starts the server if it is switched on, and runs waiting tool calls.
pub fn poll(app: &mut App, ctx: &egui::Context) {
    if app.settings.mcp_enabled && app.mcp.is_none() && app.mcp_error.is_none() {
        if app.settings.mcp_token.is_empty() {
            app.settings.mcp_token = mcp::new_token();
            app.settings.save();
        }
        if app.settings.mcp_port == 0 {
            app.settings.mcp_port = 8765;
        }
        match mcp::start(
            app.settings.mcp_port,
            app.settings.mcp_token.clone(),
            ctx.clone(),
        ) {
            Ok(s) => app.mcp = Some(s),
            Err(e) => {
                app.mcp_error = Some(format!(
                    "{} {}: {e}",
                    t("Port nicht frei:", "Port not free:"),
                    app.settings.mcp_port
                ))
            }
        }
    }
    let mut calls = vec![];
    if let Some(s) = &app.mcp {
        while let Ok(c) = s.rx.try_recv() {
            calls.push(c);
        }
    }
    for c in calls {
        let r = mcp::handle(app, ctx, &c.tool, &c.args);
        let time = chrono::Local::now().format("%H:%M:%S");
        app.mcp_log.push(format!(
            "{time} {} {}",
            c.tool,
            if r.is_ok() { "✔" } else { "✖" }
        ));
        if app.mcp_log.len() > 200 {
            app.mcp_log.drain(0..100);
        }
        let _ = c.reply.send(r);
    }
}
