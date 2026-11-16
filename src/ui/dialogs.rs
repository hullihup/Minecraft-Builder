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
                    ui.set_width(780.0);
                    ui.label(
                        RichText::new(t("Neues Projekt", "New project"))
                            .size(20.0)
                            .strong(),
                    );
                    ui.add_space(10.0);
                    ui.label(
                        RichText::new(t(
                            "1 · Was möchtest du bauen?",
                            "1 · What do you want to build?",
                        ))
                        .color(theme::TEXT_WEAK),
                    );
                    ui.add_space(2.0);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 12.0;
                        for (p, title, sub, tag) in [
                            (
                                Platform::Paper,
                                t("Plugin (Paper)", "Plugin (Paper)"),
                                t(
                                    "Für Server – Spieler brauchen nichts zu installieren",
                                    "For servers – players install nothing",
                                ),
                                t(
                                    "Menüs · Scoreboard · Resourcepack per Download",
                                    "Menus · scoreboard · resource pack by download",
                                ),
                            ),
                            (
                                Platform::Fabric,
                                t("Mod (Fabric)", "Mod (Fabric)"),
                                t(
                                    "Für Fabric-Server oder Einzelspieler",
                                    "For Fabric servers or singleplayer",
                                ),
                                t(
                                    "Texturen direkt im Mod · läuft auch im Einzelspieler",
                                    "Textures inside the mod · also runs in singleplayer",
                                ),
                            ),
                        ] {
                            let on = *platform == p;
                            let (rect, resp) =
                                ui.allocate_exact_size(vec2(384.0, 78.0), egui::Sense::click());
                            ui.painter().rect(
                                rect,
                                8.0,
                                if on {
                                    theme::ACCENT_BG
                                } else if resp.hovered() {
                                    Color32::from_rgb(0x30, 0x32, 0x38)
                                } else {
                                    Color32::from_rgb(0x2A, 0x2C, 0x31)
                                },
                                egui::Stroke::new(
                                    if on { 2.0 } else { 1.0 },
                                    if on { theme::ACCENT } else { theme::BORDER },
                                ),
                                egui::StrokeKind::Inside,
                            );
                            crate::ui::icons::paint(
                                ui.painter(),
                                egui::Rect::from_center_size(
                                    rect.left_center() + vec2(30.0, 0.0),
                                    vec2(26.0, 26.0),
                                ),
                                if p == Platform::Paper {
                                    crate::ui::icons::Icon::Server
                                } else {
                                    crate::ui::icons::Icon::Puzzle
                                },
                                if on { theme::ACCENT } else { theme::TEXT_WEAK },
                            );
                            ui.painter().text(
                                rect.min + vec2(60.0, 10.0),
                                egui::Align2::LEFT_TOP,
                                title,
                                egui::FontId::proportional(15.0),
                                theme::TEXT,
                            );
                            ui.painter().text(
                                rect.min + vec2(60.0, 32.0),
                                egui::Align2::LEFT_TOP,
                                sub,
                                egui::FontId::proportional(11.5),
                                theme::TEXT_WEAK,
                            );
                            ui.painter().text(
                                rect.min + vec2(60.0, 52.0),
                                egui::Align2::LEFT_TOP,
                                tag,
                                egui::FontId::proportional(10.5),
                                if on { theme::ACCENT } else { theme::TEXT_WEAK },
                            );
                            if resp.clicked() {
                                *platform = p;
                            }
                        }
                    });
                    if !starter.supports(*platform) {
                        *starter = Starter::Welcome;
                    }
                    ui.add_space(12.0);
                    let list = Starter::for_platform(*platform);
                    ui.label(
                        RichText::new(format!(
                            "2 · {} ({} {})",
                            t("Vorlage", "Template"),
                            list.len(),
                            match *platform {
                                Platform::Paper => t("für Plugins", "for plugins"),
                                Platform::Fabric => t("für Mods", "for mods"),
                            }
                        ))
                        .color(theme::TEXT_WEAK),
                    );
                    ui.add_space(2.0);
                    let card = vec2(252.0, 92.0);
                    for row in list.chunks(3) {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 12.0;
                            for s in row {
                                let on = *starter == *s;
                                let (rect, resp) =
                                    ui.allocate_exact_size(card, egui::Sense::click());
                                let col = s.color();
                                ui.painter().rect(
                                    rect,
                                    8.0,
                                    if on {
                                        theme::ACCENT_BG
                                    } else if resp.hovered() {
                                        Color32::from_rgb(0x30, 0x32, 0x38)
                                    } else {
                                        Color32::from_rgb(0x2A, 0x2C, 0x31)
                                    },
                                    egui::Stroke::new(
                                        if on { 2.0 } else { 1.0 },
                                        if on { theme::ACCENT } else { theme::BORDER },
                                    ),
                                    egui::StrokeKind::Inside,
                                );
                                let ir = egui::Rect::from_min_size(
                                    rect.min + vec2(10.0, 10.0),
                                    vec2(28.0, 28.0),
                                );
                                ui.painter().rect_filled(ir, 6.0, col.gamma_multiply(0.18));
                                crate::ui::icons::paint(
                                    ui.painter(),
                                    ir.shrink(6.0),
                                    s.icon(),
                                    col,
                                );
                                ui.painter().text(
                                    rect.min + vec2(46.0, 15.0),
                                    egui::Align2::LEFT_TOP,
                                    tl(s.label()),
                                    egui::FontId::proportional(13.5),
                                    theme::TEXT,
                                );
                                let g = ui.painter().layout(
                                    tl(s.desc()).to_string(),
                                    egui::FontId::proportional(11.0),
                                    theme::TEXT_WEAK,
                                    card.x - 22.0,
                                );
                                ui.painter().galley(
                                    rect.min + vec2(11.0, 44.0),
                                    g,
                                    theme::TEXT_WEAK,
                                );
                                // Tags in the top right corner.
                                let mut x = rect.right() - 8.0;
                                for tag in s.tags().iter().rev().take(1) {
                                    let tg = ui.painter().layout_no_wrap(
                                        tl(*tag).to_string(),
                                        egui::FontId::proportional(9.5),
                                        col,
                                    );
                                    let tr = egui::Rect::from_min_size(
                                        egui::pos2(x - tg.size().x - 10.0, rect.bottom() - 20.0),
                                        tg.size() + vec2(10.0, 4.0),
                                    );
                                    if tr.left() > rect.left() + 150.0 {
                                        ui.painter().rect_filled(tr, 7.0, col.gamma_multiply(0.14));
                                        ui.painter().galley(tr.min + vec2(5.0, 2.0), tg, col);
                                    }
                                    x = tr.left() - 4.0;
                                }
                                let _ = x;
                                if resp.clicked() {
                                    *starter = *s;
                                }
                                if resp.double_clicked() {
                                    create = true;
                                }
                            }
                        });
                        ui.add_space(4.0);
                    }
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(t("3 · Name und Speicherort", "3 · Name and location"))
                            .color(theme::TEXT_WEAK),
                    );
                    ui.add_space(2.0);
                    ui.horizontal(|ui| {
                        let te = ui.add(
                            egui::TextEdit::singleline(name)
                                .hint_text(t("Name", "Name"))
                                .desired_width(250.0),
                        );
                        if te.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            create = true;
                        }
                        ui.add(
                            egui::TextEdit::singleline(folder)
                                .desired_width(ui.available_width() - 40.0),
                        );
                        if crate::ui::icons::icon_button(
                            ui,
                            crate::ui::icons::Icon::Folder,
                            26.0,
                            "",
                        )
                        .clicked()
                        {
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
        Dialog::ConfirmQuit => {
            let mut choice = None;
            let r = egui::Modal::new(egui::Id::unique("dlg_quit"))
                .frame(dialog_frame())
                .show(ctx, |ui| {
                    ui.set_width(380.0);
                    ui.label(
                        RichText::new(t("Minecraft Builder beenden?", "Quit Minecraft Builder?"))
                            .size(18.0)
                            .strong(),
                    );
                    ui.add_space(6.0);
                    ui.label(t(
                        "Dein Projekt ist gespeichert. Möchtest du es vor dem Beenden noch einmal speichern?",
                        "Your project is saved. Do you want to save it once more before quitting?",
                    ));
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        if ui.add(primary(t("Speichern & beenden", "Save & quit"))).clicked() {
                            choice = Some(true);
                        }
                        if ui.button(t("Beenden", "Quit")).clicked() {
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
                app.resolve_dialog_after_save(crate::app::AfterSave::Quit, save, ctx);
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
        Dialog::Replace { find, with } => {
            let n = app.count_text(find);
            let mut go = false;
            let r = egui::Modal::new(egui::Id::unique("dlg_replace"))
                .frame(dialog_frame())
                .show(ctx, |ui| {
                    ui.set_width(460.0);
                    ui.label(RichText::new(t("Suchen & Ersetzen", "Search & replace")).size(18.0).strong());
                    ui.label(
                        RichText::new(t(
                            "Sucht in allen Texten und Werten aller Blöcke und Notizen des Projekts.",
                            "Searches all texts and values of all blocks and notes of the project.",
                        ))
                        .size(11.5)
                        .color(theme::TEXT_WEAK),
                    );
                    ui.add_space(8.0);
                    ui.label(RichText::new(t("Suchen nach", "Search for")).color(theme::TEXT_WEAK));
                    ui.add(egui::TextEdit::singleline(find).desired_width(f32::INFINITY));
                    ui.add_space(4.0);
                    ui.label(RichText::new(t("Ersetzen durch", "Replace with")).color(theme::TEXT_WEAK));
                    ui.add(egui::TextEdit::singleline(with).desired_width(f32::INFINITY));
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(format!("{n} {}", t("Treffer", "matches"))).color(if n > 0 {
                            theme::GREEN
                        } else {
                            theme::TEXT_WEAK
                        }),
                    );
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(n > 0, primary(t("Alle ersetzen", "Replace all")))
                            .clicked()
                        {
                            go = true;
                        }
                        if ui.button(t("Schließen", "Close")).clicked() {
                            close = true;
                        }
                    });
                });
            if r.should_close() {
                close = true;
            }
            if go {
                let (f, w) = (find.clone(), with.clone());
                app.replace_text(&f, &w);
            }
        }
        Dialog::Settings => {
            crate::ui::settings::show(app, ctx, &mut close);
        }
        Dialog::Updates => {
            let r = egui::Modal::new(egui::Id::unique("dlg_updates")).frame(dialog_frame()).show(ctx, |ui| {
                ui.set_width(520.0);
                ui.label(RichText::new(t("Updates", "Updates")).size(20.0).strong());
                ui.add_space(6.0);
                ui.label(format!("{} {}", t("Deine Version:", "Your version:"), crate::update::current_version()));
                ui.add_space(8.0);
                let u = &app.update;
                let mut action = 0;
                if u.checking {
                    ui.horizontal(|ui| {
                        ui.add(egui::Spinner::new());
                        ui.label(t("Suche nach Updates …", "Looking for updates …"));
                    });
                } else if let Some(rel) = &u.release {
                    ui.label(RichText::new(format!("{} {}", t("Neue Version verfügbar:", "New version available:"), rel.version)).size(16.0).color(theme::GREEN));
                    if !rel.notes.trim().is_empty() {
                        ui.add_space(6.0);
                        egui::Frame::new().fill(theme::INPUT_BG).corner_radius(6).inner_margin(10).show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            egui::ScrollArea::vertical().max_height(180.0).show(ui, |ui| {
                                ui.label(RichText::new(&rel.notes).size(12.5));
                            });
                        });
                    }
                    ui.add_space(8.0);
                    if u.downloading {
                        ui.label(t("Lade herunter …", "Downloading …"));
                        ui.add(egui::ProgressBar::new(u.progress).show_percentage());
                    } else if u.installer.is_some() {
                        ui.label(RichText::new(t("Bereit zur Installation. Ohne Klick wird es beim Schließen der App automatisch installiert.", "Ready to install. Without a click it is installed automatically when you close the app.")).color(theme::TEXT_WEAK));
                        ui.add_space(6.0);
                        if ui.add(primary(t("Jetzt installieren & neu starten", "Install now & restart"))).clicked() {
                            action = 2;
                        }
                    } else if crate::update::is_installed() && rel.installer.is_some() {
                        if ui.add(primary(t("Herunterladen & installieren", "Download & install"))).clicked() {
                            action = 1;
                        }
                    } else {
                        ui.label(RichText::new(t(
                            "Automatische Updates gehen nur mit der installierten Version. Lade den Installer von GitHub:",
                            "Automatic updates only work with the installed version. Download the installer from GitHub:",
                        )).color(theme::TEXT_WEAK));
                        ui.hyperlink_to(t("Zur Download-Seite", "Open download page"), &rel.page);
                    }
                } else if u.checked && u.error.is_none() {
                    ui.label(RichText::new(t("Du hast die neueste Version.", "You have the latest version.")).color(theme::GREEN));
                }
                if let Some(e) = &u.error {
                    ui.add_space(4.0);
                    ui.label(RichText::new(e).size(12.0).color(theme::YELLOW));
                }
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if crate::ui::widgets::toggle(ui, &mut app.settings.auto_update).changed() {
                        app.settings.save();
                    }
                    ui.label(t("Updates automatisch herunterladen und beim Beenden installieren", "Download updates automatically and install them on exit"));
                });
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if ui.button(t("Erneut suchen", "Check again")).clicked() {
                        action = 3;
                    }
                    if ui.button(t("Schließen", "Close")).clicked() {
                        close = true;
                    }
                });
                action
            });
            if r.should_close() {
                close = true;
            }
            match r.inner {
                1 => app.download_update(ctx),
                2 => {
                    app.dialog = Dialog::None;
                    app.install_update_now(ctx);
                    return;
                }
                3 => app.check_updates(ctx),
                _ => {}
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
