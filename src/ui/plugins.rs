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
    if app.settings.mcp_enabled {
        ui.add_space(10.0);
        let url = mcp::url(app.settings.mcp_port, &app.settings.mcp_token);
        copy_row(ui, t("Adresse (MCP-Server, HTTP)", "Address (MCP server, HTTP)"), &url);
        copy_row(
            ui,
            t("Für Claude Code (im Terminal einfügen)", "For Claude Code (paste into the terminal)"),
            &format!("claude mcp add --transport http minecraft-builder {url}"),
        );
        ui.horizontal(|ui| {
            ui.label(RichText::new(t("Port", "Port")).size(12.0));
            let mut port = app.settings.mcp_port;
            if ui
                .add(egui::DragValue::new(&mut port).range(1024..=65535))
                .changed()
            {
                app.settings.mcp_port = port;
                app.mcp = None;
            }
            if ui
                .small_button(t("Neue Adresse", "New address"))
                .on_hover_text(t(
                    "Erzeugt einen neuen geheimen Teil der Adresse. Die alte Adresse geht dann nicht mehr.",
                    "Creates a new secret part of the address. The old address stops working.",
                ))
                .clicked()
            {
                app.settings.mcp_token = mcp::new_token();
                app.mcp = None;
                app.settings.save();
            }
        });
        hint(
            ui,
            t(
                "Der Server ist nur von diesem Computer aus erreichbar. Wer die Adresse kennt, kann dein offenes Projekt ändern – gib sie nicht weiter.",
                "The server can only be reached from this computer. Whoever knows the address can change your open project – do not share it.",
            ),
        );
    }
    ui.add_space(12.0);
    section_header(ui, t("Was die KI kann", "What the AI can do"));
    ui.add_space(4.0);
    for (name, de, en) in mcp::TOOL_INFO {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(*name).monospace().size(11.0).color(theme::ACCENT));
            ui.label(RichText::new(t(de, en)).size(11.5));
        });
    }
    if !app.mcp_log.is_empty() {
        ui.add_space(12.0);
        section_header(ui, t("Letzte Aufrufe", "Recent calls"));
        ui.add_space(4.0);
        for l in app.mcp_log.iter().rev().take(15) {
            ui.label(RichText::new(l).monospace().size(10.5).color(theme::TEXT_WEAK));
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
        match mcp::start(app.settings.mcp_port, app.settings.mcp_token.clone(), ctx.clone()) {
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
