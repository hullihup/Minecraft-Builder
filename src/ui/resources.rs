//! Sidebar panel "Textures & models": add a resource pack .zip, see its models and textures.

use crate::app::App;
use crate::i18n::t;
use crate::model::Platform;
use crate::theme;
use crate::ui::widgets::section_header;
use egui::{Color32, RichText, Sense, Stroke, StrokeKind, Ui, vec2};

pub fn sidebar(app: &mut App, ui: &mut Ui) {
    section_header(ui, t("Texturen & Modelle", "Textures & models"));
    ui.add_space(6.0);
    ui.label(
        RichText::new(match app.project.settings.platform {
            Platform::Paper => t(
                "Füge ein Resourcepack (.zip) hinzu. Es wird ins Plugin eingebaut und Spielern beim Betreten automatisch geschickt.",
                "Add a resource pack (.zip). It is built into the plugin and sent to players automatically when they join.",
            ),
            Platform::Fabric => t(
                "Füge ein Resourcepack (.zip) hinzu. Die Texturen und Modelle werden direkt in den Mod eingebaut.",
                "Add a resource pack (.zip). Its textures and models are built right into the mod.",
            ),
        })
        .size(11.5)
        .color(theme::TEXT_WEAK),
    );
    ui.add_space(10.0);

    let enabled = app.project.settings.pack.enabled();
    let w = ui.available_width();
    let pick = |label: &str| {
        egui::Button::new(RichText::new(label).strong().color(Color32::BLACK))
            .fill(theme::ACCENT)
            .corner_radius(6)
            .min_size(vec2(w, 32.0))
    };
    if !enabled {
        if ui
            .add(pick(t(
                "📦 Resourcepack hinzufügen…",
                "📦 Add resource pack…",
            )))
            .clicked()
        {
            if let Some(p) = rfd::FileDialog::new()
                .add_filter("Resourcepack", &["zip"])
                .pick_file()
            {
                app.add_pack(p);
            }
        }
        // Drop a zip onto the window.
        let dropped: Vec<_> = ui.ctx().input(|i| {
            i.raw
                .dropped_files
                .iter()
                .map(|f| f.path().to_path_buf())
                .filter(|p| !p.as_os_str().is_empty())
                .collect()
        });
        if let Some(p) = dropped.into_iter().find(|p| {
            p.extension()
                .map(|e| e.eq_ignore_ascii_case("zip"))
                .unwrap_or(false)
        }) {
            app.add_pack(p);
        }
        ui.add_space(8.0);
        ui.label(
            RichText::new(t(
                "Tipp: Du kannst die .zip auch ins Fenster ziehen.",
                "Tip: you can also drop the .zip onto the window.",
            ))
            .size(11.0)
            .color(theme::TEXT_WEAK),
        );
        ui.add_space(14.0);
        help(ui);
        return;
    }

    let info = app.pack.info.clone();
    match &info {
        Some(Err(e)) => {
            ui.label(RichText::new(format!("⚠ {e}")).color(theme::RED));
            ui.label(
                RichText::new(app.pack.path.display().to_string())
                    .size(11.0)
                    .color(theme::TEXT_WEAK),
            );
        }
        Some(Ok(i)) => {
            egui::Frame::new()
                .fill(Color32::from_rgb(0x26, 0x28, 0x2D))
                .corner_radius(8)
                .inner_margin(10)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(
                        RichText::new(
                            app.pack
                                .path
                                .file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_default(),
                        )
                        .strong(),
                    );
                    if !i.description.is_empty() {
                        ui.label(
                            RichText::new(&i.description)
                                .size(11.5)
                                .color(theme::TEXT_WEAK),
                        );
                    }
                    ui.label(
                        RichText::new(format!(
                            "{} {} · {} {} · {} KB",
                            i.item_models.len(),
                            t("Modelle", "models"),
                            i.textures.len(),
                            t("Texturen", "textures"),
                            i.bytes / 1024
                        ))
                        .size(11.5)
                        .color(theme::ACCENT),
                    );
                    if !i.has_mcmeta {
                        ui.label(
                            RichText::new(t(
                                "pack.mcmeta fehlt – wird beim Export automatisch erzeugt.",
                                "pack.mcmeta is missing – it is created on export.",
                            ))
                            .size(11.0)
                            .color(theme::YELLOW),
                        );
                    }
                });
        }
        None => {}
    }
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        if ui.button(t("Ersetzen…", "Replace…")).clicked() {
            if let Some(p) = rfd::FileDialog::new()
                .add_filter("Resourcepack", &["zip"])
                .pick_file()
            {
                app.add_pack(p);
            }
        }
        if ui.button(t("Neu laden", "Reload")).clicked() {
            app.refresh_pack(true);
        }
        if ui
            .button(RichText::new(t("Entfernen", "Remove")).color(theme::RED))
            .clicked()
        {
            app.project.settings.pack.path.clear();
            app.refresh_pack(true);
        }
    });

    if app.project.settings.platform == Platform::Paper {
        ui.add_space(14.0);
        section_header(ui, t("Server-Einstellungen", "Server settings"));
        ui.add_space(4.0);
        let p = &mut app.project.settings.pack;
        ui.horizontal(|ui| {
            ui.label(RichText::new(t("Port", "Port")).size(12.0));
            ui.add(egui::DragValue::new(&mut p.port).range(1024..=65535));
        });
        ui.label(RichText::new(t(
            "Über diesen Port lädt der Spieler das Pack. Er muss am Server erreichbar/freigegeben sein.",
            "Players download the pack through this port. It must be reachable/forwarded on the server.",
        )).size(10.5).color(theme::TEXT_WEAK));
        ui.horizontal(|ui| {
            crate::ui::widgets::toggle(ui, &mut p.send_on_join);
            ui.label(RichText::new(t("Beim Betreten senden", "Send on join")).size(12.0));
        });
        ui.horizontal(|ui| {
            crate::ui::widgets::toggle(ui, &mut p.required);
            ui.label(RichText::new(t("Pflicht (sonst Kick)", "Required (else kick)")).size(12.0));
        });
        ui.label(RichText::new(t("Text in der Abfrage", "Prompt text")).size(12.0));
        ui.add(egui::TextEdit::singleline(&mut p.prompt).desired_width(f32::INFINITY));
    } else {
        ui.add_space(10.0);
        ui.label(RichText::new(t(
            "Hinweis: Damit Spieler die Texturen sehen, muss der Mod auch bei ihnen installiert sein.",
            "Note: players need the mod installed too to see the textures.",
        )).size(11.0).color(theme::TEXT_WEAK));
    }

    if let Some(Ok(i)) = &info {
        ui.add_space(14.0);
        section_header(ui, t("Item-Modelle", "Item models"));
        ui.label(
            RichText::new(t(
                "Benutze sie im Block „Eigenes Item geben“. Klick kopiert die ID.",
                "Use them in the block “Give custom item”. Click copies the id.",
            ))
            .size(11.0)
            .color(theme::TEXT_WEAK),
        );
        ui.add_space(4.0);
        if i.item_models.is_empty() {
            ui.label(
                RichText::new(t(
                    "Keine Item-Modelle gefunden (assets/<name>/models/item oder items).",
                    "No item models found (assets/<name>/models/item or items).",
                ))
                .size(11.0)
                .color(theme::YELLOW),
            );
        }
        for m in &i.item_models {
            let auto = i.generated.contains(m);
            let r = ui.horizontal(|ui| {
                ui.label(RichText::new("◆").color(crate::blocks::Cat::Resources.color()));
                ui.label(RichText::new(m).monospace().size(12.0));
                if auto {
                    ui.label(RichText::new("auto").size(10.0).color(theme::TEXT_WEAK)).on_hover_text(t(
                        "Die Item-Definition (items/…json) fehlte und wird beim Export automatisch erzeugt.",
                        "The item definition (items/…json) was missing and is generated on export.",
                    ));
                }
            });
            let resp = ui.interact(
                r.response.rect,
                egui::Id::unique(("model", m)),
                Sense::click(),
            );
            if resp.hovered() {
                ui.painter()
                    .rect_filled(r.response.rect, 3.0, Color32::from_white_alpha(8));
            }
            if resp
                .on_hover_text(t("Klicken zum Kopieren", "Click to copy"))
                .clicked()
            {
                ui.ctx().copy_text(m.clone());
            }
        }

        ui.add_space(14.0);
        section_header(
            ui,
            &format!("{} ({})", t("Texturen", "Textures"), i.textures.len()),
        );
        ui.add_space(6.0);
        let size = 52.0;
        let cols = ((ui.available_width() + 6.0) / (size + 6.0))
            .floor()
            .max(1.0) as usize;
        let mut loaded_this_frame = 0;
        for row in i.textures.iter().take(240).collect::<Vec<_>>().chunks(cols) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                for tex in row {
                    let (r, resp) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
                    ui.painter().rect(
                        r,
                        4.0,
                        theme::INPUT_BG,
                        Stroke::new(1.0, theme::BORDER),
                        StrokeKind::Inside,
                    );
                    if !app.pack.thumbs.contains_key(*tex) && loaded_this_frame < 8 {
                        loaded_this_frame += 1;
                        let handle =
                            crate::resourcepack::texture_preview(&app.pack.path, tex).map(|img| {
                                ui.ctx().load_texture(
                                    format!("pack:{tex}"),
                                    img,
                                    egui::TextureOptions::NEAREST,
                                )
                            });
                        app.pack.thumbs.insert((*tex).clone(), handle);
                    }
                    if let Some(Some(h)) = app.pack.thumbs.get(*tex) {
                        ui.painter().image(
                            h.id(),
                            r.shrink(4.0),
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            Color32::WHITE,
                        );
                    }
                    resp.on_hover_text(tex.trim_start_matches("assets/"));
                }
            });
        }
        if loaded_this_frame > 0 {
            ui.ctx().request_repaint();
        }
    }
    ui.add_space(14.0);
    help(ui);
}

fn help(ui: &mut Ui) {
    egui::CollapsingHeader::new(RichText::new(t("So baust du ein Item-Modell", "How to make an item model")).size(12.0)).show(ui, |ui| {
        ui.label(RichText::new(t(
            "1. Lege die Textur als PNG ab: assets/meinpack/textures/item/zauberstab.png\n\
             2. Lege das Modell an: assets/meinpack/models/item/zauberstab.json\n   {\"parent\": \"minecraft:item/handheld\", \"textures\": {\"layer0\": \"meinpack:item/zauberstab\"}}\n\
             3. Packe den Ordner (mit pack.mcmeta) als .zip und füge ihn hier hinzu.\n\
             4. Im Block „Eigenes Item geben“ wählst du dann das Modell meinpack:zauberstab.\n\
             Die Datei items/zauberstab.json erzeugt der Builder automatisch, falls sie fehlt.",
            "1. Put the texture as PNG: assets/mypack/textures/item/wand.png\n\
             2. Create the model: assets/mypack/models/item/wand.json\n   {\"parent\": \"minecraft:item/handheld\", \"textures\": {\"layer0\": \"mypack:item/wand\"}}\n\
             3. Zip the folder (with pack.mcmeta) and add it here.\n\
             4. In the block “Give custom item” pick the model mypack:wand.\n\
             The builder creates items/wand.json automatically if it is missing.",
        )).size(11.0).color(theme::TEXT_WEAK));
    });
}
