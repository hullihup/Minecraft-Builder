//! Modal dialogs: new project, unsaved changes, rename, about, messages.

use crate::app::{App, Dialog};
use crate::examples::Starter;
use crate::i18n::{t, tl};
use crate::model::Platform;
use crate::theme;
use egui::{Color32, RichText, vec2};

fn primary(text: &str) -> egui::Button<'_> {
    egui::Button::new(RichText::new(text).color(Color32::BLACK).strong())
        .fill(theme::ACCENT)
        .corner_radius(6)
        .min_size(vec2(110.0, 30.0))
}

fn dialog_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(Color32::from_rgb(0x20, 0x22, 0x26))
        .stroke(egui::Stroke::new(1.0, theme::BORDER))
        .corner_radius(12)
        .inner_margin(24)
        .shadow(egui::epaint::Shadow {
            offset: [0, 10],
            blur: 40,
            spread: 0,
            color: Color32::from_black_alpha(160),
        })
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    let mut dialog = app.dialog.clone();
    let mut close = false;
    match &mut dialog {
        Dialog::None => return,
        Dialog::NewProject {
            name,
            platform,
            starter,
            folder,
        } => {
            let mut create = false;
            let r = egui::Modal::new(egui::Id::unique("dlg_new"))
                .frame(dialog_frame())
                .show(ctx, |ui| {
                    ui.set_width(600.0);
                    ui.label(
                        RichText::new(t("Neues Projekt", "New project"))
                            .size(20.0)
                            .strong(),
                    );
                    ui.add_space(10.0);
                    ui.label(RichText::new(t("Name", "Name")).color(theme::TEXT_WEAK));
                    let te = ui.add(egui::TextEdit::singleline(name).desired_width(f32::INFINITY));
                    if te.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        create = true;
                    }
                    ui.add_space(10.0);
                    ui.label(
                        RichText::new(t("Was möchtest du bauen?", "What do you want to build?"))
                            .color(theme::TEXT_WEAK),
                    );
                    ui.horizontal(|ui| {
                        for (p, title, sub) in [
                            (
                                Platform::Paper,
                                t("Plugin (Paper)", "Plugin (Paper)"),
                                t(
                                    "Für Server – Spieler brauchen nichts zu installieren",
                                    "For servers – players install nothing",
                                ),
                            ),
                            (
                                Platform::Fabric,
                                t("Mod (Fabric)", "Mod (Fabric)"),
                                t(
                                    "Für Fabric-Server oder Einzelspieler",
                                    "For Fabric servers or singleplayer",
                                ),
                            ),
                        ] {
                            let on = *platform == p;
                            let (rect, resp) =
                                ui.allocate_exact_size(vec2(292.0, 64.0), egui::Sense::click());
                            ui.painter().rect(
                                rect,
                                8.0,
                                if on {
                                    theme::ACCENT_BG
                                } else {
                                    Color32::from_rgb(0x2A, 0x2C, 0x31)
                                },
                                egui::Stroke::new(
                                    if on { 2.0 } else { 1.0 },
                                    if on { theme::ACCENT } else { theme::BORDER },
                                ),
                                egui::StrokeKind::Inside,
                            );
                            ui.painter().text(
                                rect.min + vec2(12.0, 12.0),
                                egui::Align2::LEFT_TOP,
                                title,
                                egui::FontId::proportional(15.0),
                                theme::TEXT,
                            );
                            ui.painter().text(
                                rect.min + vec2(12.0, 36.0),
                                egui::Align2::LEFT_TOP,
                                sub,
                                egui::FontId::proportional(11.0),
                                theme::TEXT_WEAK,
                            );
                            if resp.clicked() {
                                *platform = p;
                            }
                        }
                    });
                    ui.add_space(10.0);
                    ui.label(RichText::new(t("Speicherort", "Location")).color(theme::TEXT_WEAK));
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(folder)
                                .desired_width(ui.available_width() - 40.0),
                        );
                        if ui.button("📁").clicked() {
                            if let Some(d) = rfd::FileDialog::new()
                                .set_directory(folder.as_str())
                                .pick_folder()
                            {
                                *folder = d.display().to_string();
                            }
                        }
                    });
                    ui.label(
                        RichText::new(format!(
                            "→ {}",
                            std::path::Path::new(folder.trim())
                                .join(crate::model::slug(name))
                                .join(format!("{}.mcb", crate::model::slug(name)))
                                .display()
                        ))
                        .size(11.0)
                        .color(theme::TEXT_WEAK),
                    );
                    ui.add_space(10.0);
                    ui.label(RichText::new(t("Vorlage", "Template")).color(theme::TEXT_WEAK));
                    for s in Starter::ALL {
                        ui.horizontal(|ui| {
                            ui.radio_value(starter, s, RichText::new(tl(s.label())).strong());
                            ui.label(
                                RichText::new(tl(s.desc()))
                                    .size(11.5)
                                    .color(theme::TEXT_WEAK),
                            );
                        });
                    }
                    ui.add_space(14.0);
                    ui.horizontal(|ui| {
                        if ui.add(primary(t("Erstellen", "Create"))).clicked() {
                            create = true;
                        }
                        if ui.button(t("Abbrechen", "Cancel")).clicked() {
                            close = true;
                        }
                    });
                });
            if r.should_close() {
                close = true;
            }
            if create && !name.trim().is_empty() {
                let (n, p, s, f) = (name.trim().to_string(), *platform, *starter, folder.clone());
                app.dialog = Dialog::None;
                app.create_project(&n, p, s, &f);
                return;
            }
        }
        Dialog::UnsavedChanges { then } => {
            let then = *then;
            let mut choice = None;
            let r = egui::Modal::new(egui::Id::unique("dlg_unsaved"))
                .frame(dialog_frame())
                .show(ctx, |ui| {
                    ui.set_width(380.0);
                    ui.label(
                        RichText::new(t("Ungespeicherte Änderungen", "Unsaved changes"))
                            .size(18.0)
                            .strong(),
                    );
                    ui.add_space(6.0);
                    ui.label(t(
                        "Möchtest du deine Änderungen speichern?",
                        "Do you want to save your changes?",
                    ));
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        if ui.add(primary(t("Speichern", "Save"))).clicked() {
                            choice = Some(true);
                        }
                        if ui.button(t("Nicht speichern", "Don't save")).clicked() {
                            choice = Some(false);
                        }
                        if ui.button(t("Abbrechen", "Cancel")).clicked() {
                            close = true;
                        }
                    });
                });
            if r.should_close() {
                close = true;
            }
            if let Some(save) = choice {
                app.resolve_dialog_after_save(then, save, ctx);
                return;
            }
        }
        Dialog::RenameWorkspace { index, name } => {
            let mut ok = false;
            let r = egui::Modal::new(egui::Id::unique("dlg_rename"))
                .frame(dialog_frame())
                .show(ctx, |ui| {
                    ui.set_width(320.0);
                    ui.label(
                        RichText::new(t("Arbeitsbereich umbenennen", "Rename workspace"))
                            .size(16.0)
                            .strong(),
                    );
                    ui.add_space(6.0);
                    let te = ui.add(egui::TextEdit::singleline(name).desired_width(f32::INFINITY));
                    te.request_focus();
                    if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        ok = true;
                    }
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.add(primary("OK")).clicked() {
                            ok = true;
                        }
                        if ui.button(t("Abbrechen", "Cancel")).clicked() {
                            close = true;
                        }
                    });
                });
            if r.should_close() {
                close = true;
            }
            if ok {
                if let Some(ws) = app.project.workspaces.get_mut(*index) {
                    if !name.trim().is_empty() {
                        ws.name = name.trim().to_string();
                    }
                }
                close = true;
            }
        }
        Dialog::About => {
            let r = egui::Modal::new(egui::Id::unique("dlg_about")).frame(dialog_frame()).show(ctx, |ui| {
                ui.set_width(380.0);
                ui.label(RichText::new("Minecraft Builder").size(20.0).strong().color(theme::GREEN));
                ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
                ui.add_space(8.0);
                ui.label(t(
                    "Baue Minecraft-Plugins (Paper) und Mods (Fabric), indem du Blöcke verbindest. Geschrieben in Rust mit egui.",
                    "Build Minecraft plugins (Paper) and mods (Fabric) by connecting blocks. Written in Rust with egui.",
                ));
                ui.add_space(6.0);
                ui.label(RichText::new(format!("{} {}", crate::blocks::all().len(), t("Blöcke", "blocks"))).color(theme::TEXT_WEAK));
                ui.label(RichText::new(t("Nicht offiziell von Mojang oder Microsoft.", "Not affiliated with Mojang or Microsoft.")).size(11.0).color(theme::TEXT_WEAK));
                ui.add_space(10.0);
                if ui.add(primary("OK")).clicked() {
                    close = true;
                }
            });
            if r.should_close() {
                close = true;
            }
        }
        Dialog::Message { title, text } => {
            let r = egui::Modal::new(egui::Id::unique("dlg_msg"))
                .frame(dialog_frame())
                .show(ctx, |ui| {
                    ui.set_width(520.0);
                    ui.label(RichText::new(title.as_str()).size(18.0).strong());
                    ui.add_space(6.0);
                    ui.label(text.as_str());
                    ui.add_space(10.0);
                    if ui.add(primary("OK")).clicked() {
                        close = true;
                    }
                });
            if r.should_close() {
                close = true;
            }
        }
    }
    if close {
        app.dialog = Dialog::None;
    } else if app.dialog != Dialog::None {
        app.dialog = dialog;
    }
}
