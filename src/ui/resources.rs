//! Sidebar panel "Textures & models": resource packs (several are merged into one),
//! per-item settings and previews.

use crate::app::App;
use crate::i18n::{t, tl};
use crate::model::{ItemMode, Platform};
use crate::theme;
use crate::ui::icons::{self, Icon};
use crate::ui::menu_editor::{ItemPicker, PickTab, PickTarget};
use crate::ui::widgets::{section_header, toggle};
use egui::{Color32, RichText, Sense, Stroke, StrokeKind, Ui, vec2};

fn hint(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).size(11.0).color(theme::TEXT_WEAK));
}

fn pick_pack(app: &mut App) {
    if let Some(files) = rfd::FileDialog::new()
        .add_filter("Resourcepack", &["zip"])
        .pick_files()
    {
        for p in files {
            app.add_pack(p);
        }
    }
}

pub fn sidebar(app: &mut App, ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    section_header(ui, t("Texturen & Modelle", "Textures & models"));
    ui.add_space(6.0);
    hint(
        ui,
        match app.project.settings.platform {
            Platform::Paper => t(
                "Füge Resourcepacks (.zip) hinzu. Sie werden zu einem Pack zusammengeführt, ins Plugin eingebaut und Spielern beim Betreten geschickt.",
                "Add resource packs (.zip). They are merged into one pack, built into the plugin and sent to players when they join.",
            ),
            Platform::Fabric => t(
                "Füge Resourcepacks (.zip) hinzu. Sie werden zusammengeführt und direkt in den Mod eingebaut.",
                "Add resource packs (.zip). They are merged and built right into the mod.",
            ),
        },
    );
    ui.add_space(10.0);

    // Drop zips onto the window.
    let dropped: Vec<_> = ui.ctx().input(|i| {
        i.raw
            .dropped_files
            .iter()
            .map(|f| f.path().to_path_buf())
            .filter(|p| {
                p.extension()
                    .map(|e| e.eq_ignore_ascii_case("zip"))
                    .unwrap_or(false)
            })
            .collect()
    });
    for p in dropped {
        app.add_pack(p);
    }

    let has_packs = !app.project.settings.pack.packs.is_empty();
    let w = ui.available_width();
    if !has_packs {
        if ui
            .add(
                egui::Button::new(
                    RichText::new(t("Resourcepack hinzufügen…", "Add resource pack…"))
                        .strong()
                        .color(Color32::BLACK),
                )
                .fill(theme::ACCENT)
                .corner_radius(6)
                .min_size(vec2(w, 32.0)),
            )
            .clicked()
        {
            pick_pack(app);
        }
        ui.add_space(8.0);
        hint(
            ui,
            t(
                "Tipp: Du kannst die .zip auch ins Fenster ziehen – auch mehrere auf einmal.",
                "Tip: you can also drop the .zip onto the window – several at once too.",
            ),
        );
        ui.add_space(14.0);
        help(ui);
        return;
    }

    // ------------------------------------------------------------- pack list
    section_header(
        ui,
        &format!(
            "{} ({})",
            t("Packs", "Packs"),
            app.project.settings.pack.packs.len()
        ),
    );
    ui.add_space(4.0);
    let stats = match &app.pack.info {
        Some(Ok(i)) => i.packs.clone(),
        _ => vec![],
    };
    let n = app.project.settings.pack.packs.len();
    let mut action: Option<(usize, i32)> = None;
    for idx in 0..n {
        let entry = app.project.settings.pack.packs[idx].clone();
        let abs = crate::model::PackSettings::resolve_one(&entry.path, app.path.as_deref());
        let stat = stats.iter().find(|s| s.path == abs);
        let name = abs
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| entry.path.clone());
        egui::Frame::new()
            .fill(Color32::from_rgb(0x26, 0x28, 0x2D))
            .stroke(Stroke::new(1.0, theme::BORDER))
            .corner_radius(8)
            .inner_margin(8)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    let mut on = entry.enabled;
                    if toggle(ui, &mut on).changed() {
                        app.project.settings.pack.packs[idx].enabled = on;
                    }
                    ui.add(
                        egui::Label::new(RichText::new(&name).strong().color(if entry.enabled {
                            theme::TEXT
                        } else {
                            theme::TEXT_WEAK
                        }))
                        .truncate(),
                    );
                });
                ui.horizontal(|ui| {
                    let text = match stat {
                        Some(s) if s.error.is_some() => {
                            RichText::new(s.error.clone().unwrap_or_default()).color(theme::RED)
                        }
                        Some(s) => RichText::new(format!(
                            "{} {} · {} KB",
                            s.files,
                            t("Dateien", "files"),
                            (s.bytes / 1024).max(1)
                        ))
                        .color(theme::TEXT_WEAK),
                        None if !abs.is_file() => {
                            RichText::new(t("Datei nicht gefunden", "File not found"))
                                .color(theme::RED)
                        }
                        None => RichText::new(t("ausgeschaltet", "switched off"))
                            .color(theme::TEXT_WEAK),
                    };
                    ui.label(text.size(11.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if icons::icon_button(ui, Icon::Trash, 20.0, t("Entfernen", "Remove"))
                            .clicked()
                        {
                            action = Some((idx, 0));
                        }
                        if idx + 1 < n
                            && icons::icon_button(
                                ui,
                                Icon::ArrowDown,
                                20.0,
                                t("Niedrigerer Vorrang", "Lower priority"),
                            )
                            .clicked()
                        {
                            action = Some((idx, 1));
                        }
                        if idx > 0
                            && icons::icon_button(
                                ui,
                                Icon::ArrowUp,
                                20.0,
                                t("Höherer Vorrang", "Higher priority"),
                            )
                            .clicked()
                        {
                            action = Some((idx, -1));
                        }
                        if n > 1 {
                            ui.label(
                                RichText::new(format!("#{}", idx + 1))
                                    .size(10.5)
                                    .color(theme::TEXT_WEAK),
                            );
                        }
                    });
                });
            });
        ui.add_space(4.0);
    }
    if let Some((idx, dir)) = action {
        let packs = &mut app.project.settings.pack.packs;
        match dir {
            0 => {
                packs.remove(idx);
            }
            d => packs.swap(idx, (idx as i32 + d) as usize),
        }
    }
    ui.horizontal(|ui| {
        if icons::button(ui, Some(Icon::Plus), t("Pack hinzufügen", "Add pack")).clicked() {
            pick_pack(app);
        }
        if icons::icon_button(ui, Icon::Refresh, 28.0, t("Neu laden", "Reload")).clicked() {
            app.refresh_pack(true);
        }
    });
    if n > 1 {
        ui.add_space(4.0);
        hint(
            ui,
            t(
                "Alle eingeschalteten Packs werden zu einem zusammengeführt. Gibt es dieselbe Datei mehrfach, gewinnt das Pack weiter oben. Sprach-, Sound- und Schrift-Dateien werden gemischt.",
                "All switched-on packs are merged into one. If the same file exists several times, the pack further up wins. Language, sound and font files are mixed.",
            ),
        );
    }
    app.refresh_pack(false);

    let info = app.pack.info.clone();
    let info = match info {
        Some(Ok(i)) => i,
        Some(Err(e)) => {
            ui.add_space(6.0);
            ui.label(RichText::new(e).color(theme::RED));
            return;
        }
        None => {
            ui.add_space(6.0);
            hint(ui, t("Kein Pack eingeschaltet.", "No pack switched on."));
            return;
        }
    };

    // -------------------------------------------------------------- summary
    ui.add_space(8.0);
    egui::Frame::new()
        .fill(theme::ACCENT_BG.gamma_multiply(0.6))
        .corner_radius(8)
        .inner_margin(10)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                RichText::new(if info.packs.len() > 1 {
                    t("Zusammengeführtes Pack", "Merged pack")
                } else {
                    t("Dein Pack", "Your pack")
                })
                .strong(),
            );
            ui.label(
                RichText::new(format!(
                    "{} {} · {} {} · {} KB",
                    info.item_models.len(),
                    t("Modelle", "models"),
                    info.textures.len(),
                    t("Texturen", "textures"),
                    (info.bytes / 1024).max(1)
                ))
                .size(11.5)
                .color(theme::ACCENT),
            );
            if !info.merged.is_empty() {
                ui.label(
                    RichText::new(format!(
                        "{} {}",
                        info.merged.len(),
                        t("Dateien gemischt", "files mixed")
                    ))
                    .size(11.0)
                    .color(theme::GREEN),
                )
                .on_hover_text(info.merged.join("\n"));
            }
            if !info.conflicts.is_empty() {
                let list: Vec<&str> = info.conflicts.iter().take(25).map(|s| s.as_str()).collect();
                ui.label(
                    RichText::new(format!(
                        "{} {}",
                        info.conflicts.len(),
                        t(
                            "Dateien doppelt – oberes Pack gewinnt",
                            "duplicate files – upper pack wins"
                        )
                    ))
                    .size(11.0)
                    .color(theme::YELLOW),
                )
                .on_hover_text(list.join("\n"));
            }
            if !info.has_mcmeta {
                ui.label(
                    RichText::new(t(
                        "pack.mcmeta fehlt – wird beim Export erzeugt.",
                        "pack.mcmeta is missing – it is created on export.",
                    ))
                    .size(11.0)
                    .color(theme::YELLOW),
                );
            }
        });

    // -------------------------------------------------------- give command
    ui.add_space(14.0);
    section_header(ui, t("Befehl für eigene Items", "Command for custom items"));
    ui.add_space(4.0);
    {
        let pack = &mut app.project.settings.pack;
        let mut on = !pack.give_command.trim().is_empty();
        ui.horizontal(|ui| {
            if toggle(ui, &mut on).changed() {
                pack.give_command = if on { "items".into() } else { String::new() };
            }
            if on {
                ui.label("/");
                ui.add(egui::TextEdit::singleline(&mut pack.give_command).desired_width(110.0));
            } else {
                ui.label(RichText::new(t("Befehl erzeugen", "Create command")).size(12.5));
            }
        });
        if let Some(c) = pack.give_cmd() {
            hint(
                ui,
                &format!(
                    "/{c} <item> [{}] [{}] – {}",
                    t("anzahl", "amount"),
                    t("spieler", "player"),
                    t(
                        "gibt OPs jedes eigene Item, mit Tab-Vorschlägen.",
                        "gives OPs any custom item, with tab suggestions."
                    )
                ),
            );
        } else {
            hint(
                ui,
                t(
                    "Erzeugt einen OP-Befehl, der alle Items des Packs ausgeben kann – ganz ohne eigene Blöcke.",
                    "Creates an OP command that can hand out all items of the pack – without any blocks.",
                ),
            );
        }
    }

    // ------------------------------------------------------- paper settings
    if app.project.settings.platform == Platform::Paper {
        ui.add_space(14.0);
        egui::CollapsingHeader::new(
            RichText::new(t("Senden an Spieler", "Sending to players")).size(12.5),
        )
        .show(ui, |ui| {
            let p = &mut app.project.settings.pack;
            ui.horizontal(|ui| {
                ui.label(RichText::new(t("Port", "Port")).size(12.0));
                ui.add(egui::DragValue::new(&mut p.port).range(1024..=65535));
            });
            hint(
                ui,
                t(
                    "Über diesen Port lädt der Spieler das Pack. Er muss am Server erreichbar/freigegeben sein.",
                    "Players download the pack through this port. It must be reachable/forwarded on the server.",
                ),
            );
            ui.horizontal(|ui| {
                toggle(ui, &mut p.send_on_join);
                ui.label(RichText::new(t("Beim Betreten senden", "Send on join")).size(12.0));
            });
            ui.horizontal(|ui| {
                toggle(ui, &mut p.required);
                ui.label(RichText::new(t("Pflicht (sonst Kick)", "Required (else kick)")).size(12.0));
            });
            ui.label(RichText::new(t("Text in der Abfrage", "Prompt text")).size(12.0));
            ui.add(egui::TextEdit::singleline(&mut p.prompt).desired_width(f32::INFINITY));
        });
    } else {
        ui.add_space(10.0);
        hint(
            ui,
            t(
                "Hinweis: Damit Spieler die Texturen sehen, muss der Mod auch bei ihnen installiert sein.",
                "Note: players need the mod installed too to see the textures.",
            ),
        );
    }

    // ----------------------------------------------------------------- items
    ui.add_space(14.0);
    let custom = info.items.iter().filter(|i| !i.vanilla).count();
    let vanilla = info.items.len() - custom;
    section_header(ui, t("Items des Packs", "Items of the pack"));
    ui.add_space(2.0);
    hint(
        ui,
        t(
            "Lege für jedes Item fest: Soll es ein Vanilla-Item ersetzen, nur per Befehl/Block erhältlich sein oder gar nicht eingebaut werden?",
            "Decide for every item: should it replace a vanilla item, be obtainable by command/block only, or be left out?",
        ),
    );
    ui.add_space(4.0);
    let tab_id = egui::Id::unique("pack_item_tab");
    let mut show_vanilla = ctx
        .data(|d| d.get_temp::<bool>(tab_id))
        .unwrap_or(custom == 0 && vanilla > 0);
    let filter_id = egui::Id::unique("pack_item_filter");
    let mut filter = ctx
        .data(|d| d.get_temp::<String>(filter_id))
        .unwrap_or_default();
    ui.horizontal(|ui| {
        if ui
            .selectable_label(
                !show_vanilla,
                format!("{} ({custom})", t("Eigene", "Custom")),
            )
            .clicked()
        {
            show_vanilla = false;
        }
        if ui
            .selectable_label(
                show_vanilla,
                format!("{} ({vanilla})", t("Ersetzt Vanilla", "Replaces vanilla")),
            )
            .clicked()
        {
            show_vanilla = true;
        }
    });
    ui.add(
        egui::TextEdit::singleline(&mut filter)
            .hint_text(t("Filtern…", "Filter…"))
            .desired_width(f32::INFINITY),
    );
    ctx.data_mut(|d| {
        d.insert_temp(tab_id, show_vanilla);
        d.insert_temp(filter_id, filter.clone());
    });
    let q = filter.trim().to_lowercase();
    let shown: Vec<&crate::resourcepack::PackItem> = info
        .items
        .iter()
        .filter(|i| i.vanilla == show_vanilla && (q.is_empty() || i.key.contains(&q)))
        .collect();
    if show_vanilla && !shown.is_empty() {
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new(t("Alle gezeigten:", "All shown:"))
                    .size(11.0)
                    .color(theme::TEXT_WEAK),
            );
            for (mode, label) in [
                (ItemMode::Auto, t("ersetzen", "replace")),
                (ItemMode::CommandOnly, t("nur per Befehl", "command only")),
                (ItemMode::Off, t("aus", "off")),
            ] {
                if ui.small_button(label).clicked() {
                    for it in &shown {
                        let mut r = app.project.settings.pack.rule(&it.key);
                        r.mode = mode;
                        app.project.settings.pack.set_rule(&it.key, r);
                    }
                }
            }
        });
    }
    if shown.is_empty() {
        hint(
            ui,
            if show_vanilla {
                t(
                    "Dieses Pack ersetzt keine Vanilla-Items.",
                    "This pack does not replace vanilla items.",
                )
            } else {
                t(
                    "Keine eigenen Item-Modelle gefunden (assets/<name>/models/item oder items).",
                    "No custom item models found (assets/<name>/models/item or items).",
                )
            },
        );
    }
    const LIMIT: usize = 150;
    for it in shown.iter().take(LIMIT) {
        let mut rule = app.project.settings.pack.rule(&it.key);
        let before = rule.clone();
        let (item_part, model_for_icon) = if it.vanilla {
            (it.key.clone(), String::new())
        } else {
            ("minecraft:paper".to_string(), it.key.clone())
        };
        egui::Frame::new()
            .fill(Color32::from_rgb(0x24, 0x26, 0x2A))
            .corner_radius(6)
            .inner_margin(6)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    let (r, resp) = ui.allocate_exact_size(vec2(34.0, 34.0), Sense::click());
                    ui.painter().rect(
                        r,
                        4.0,
                        theme::INPUT_BG,
                        Stroke::new(1.0, theme::BORDER),
                        StrokeKind::Inside,
                    );
                    match it.texture.as_deref().and_then(|e| app.pack_thumb(&ctx, e)) {
                        Some(tex) => {
                            ui.painter().image(
                                tex,
                                r.shrink(3.0),
                                egui::Rect::from_min_max(
                                    egui::pos2(0.0, 0.0),
                                    egui::pos2(1.0, 1.0),
                                ),
                                if rule.mode == ItemMode::Off {
                                    Color32::from_white_alpha(70)
                                } else {
                                    Color32::WHITE
                                },
                            );
                        }
                        None => crate::items::paint_placeholder(
                            ui.painter(),
                            r.shrink(5.0),
                            if model_for_icon.is_empty() {
                                &item_part
                            } else {
                                &model_for_icon
                            },
                        ),
                    }
                    let usable = if it.model.is_empty() {
                        &it.key
                    } else {
                        &it.model
                    };
                    if resp
                        .on_hover_text(format!(
                            "{usable}\n{}",
                            t("Klick kopiert die ID", "Click copies the id")
                        ))
                        .clicked()
                    {
                        ui.ctx().copy_text(usable.clone());
                    }
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 2.0;
                        ui.add(
                            egui::Label::new(
                                RichText::new(if it.model.is_empty() || it.model == it.key {
                                    it.key.clone()
                                } else {
                                    format!("{} → {}", it.key, it.model)
                                })
                                .monospace()
                                .size(11.5),
                            )
                            .truncate(),
                        );
                        let modes: &[ItemMode] = if it.vanilla {
                            &[ItemMode::Auto, ItemMode::CommandOnly, ItemMode::Off]
                        } else {
                            &[ItemMode::Auto, ItemMode::Replace, ItemMode::Off]
                        };
                        let name_of = |m: ItemMode| -> &'static str {
                            match (m, it.vanilla) {
                                (ItemMode::Auto, true) => {
                                    t("Ersetzt das Vanilla-Item", "Replaces the vanilla item")
                                }
                                (ItemMode::Auto, false) => {
                                    t("Nur per Befehl / Block", "Command / block only")
                                }
                                _ => tl(m.label()),
                            }
                        };
                        egui::ComboBox::from_id_salt(("item_mode", &it.key))
                            .width(ui.available_width() - 4.0)
                            .selected_text(RichText::new(name_of(rule.mode)).size(11.5).color(
                                match rule.mode {
                                    ItemMode::Off => theme::TEXT_WEAK,
                                    ItemMode::Replace => theme::YELLOW,
                                    ItemMode::CommandOnly => theme::GREEN,
                                    ItemMode::Auto if it.vanilla => theme::YELLOW,
                                    ItemMode::Auto => theme::GREEN,
                                },
                            ))
                            .show_ui(ui, |ui| {
                                for m in modes {
                                    ui.selectable_value(&mut rule.mode, *m, name_of(*m));
                                }
                            });
                    });
                });
                if rule.mode == ItemMode::Replace && !it.vanilla {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(t("Ersetzt:", "Replaces:")).size(11.5));
                        ui.add(
                            egui::TextEdit::singleline(&mut rule.base)
                                .hint_text("minecraft:stick")
                                .desired_width(ui.available_width() - 34.0),
                        );
                        if icons::icon_button(
                            ui,
                            Icon::Search,
                            22.0,
                            t("Item suchen", "Search item"),
                        )
                        .clicked()
                        {
                            app.item_picker = Some(ItemPicker::new(
                                PickTarget::PackBase(it.key.clone()),
                                PickTab::Items,
                            ));
                        }
                    });
                    if rule.base.trim().is_empty() {
                        ui.label(
                            RichText::new(t(
                                "Wähle das Vanilla-Item, das so aussehen soll.",
                                "Pick the vanilla item that should look like this.",
                            ))
                            .size(10.5)
                            .color(theme::YELLOW),
                        );
                    }
                }
                if it.generated && rule.mode != ItemMode::Off {
                    ui.label(
                        RichText::new(t(
                            "Item-Definition wird automatisch erzeugt",
                            "item definition is generated automatically",
                        ))
                        .size(10.0)
                        .color(theme::TEXT_WEAK),
                    );
                }
            });
        ui.add_space(3.0);
        if rule != before {
            app.project.settings.pack.set_rule(&it.key, rule);
        }
    }
    if shown.len() > LIMIT {
        hint(
            ui,
            &format!(
                "… {} {}",
                shown.len() - LIMIT,
                t("weitere – benutze den Filter.", "more – use the filter.")
            ),
        );
    }

    // -------------------------------------------------------------- textures
    ui.add_space(14.0);
    egui::CollapsingHeader::new(
        RichText::new(format!(
            "{} ({})",
            t("Alle Texturen", "All textures"),
            info.textures.len()
        ))
        .size(12.5),
    )
    .show(ui, |ui| {
        let size = 48.0;
        let cols = ((ui.available_width() + 6.0) / (size + 6.0))
            .floor()
            .max(1.0) as usize;
        for row in info
            .textures
            .iter()
            .take(240)
            .collect::<Vec<_>>()
            .chunks(cols)
        {
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
                    if let Some(id) = app.pack_thumb(&ctx, tex) {
                        ui.painter().image(
                            id,
                            r.shrink(4.0),
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            Color32::WHITE,
                        );
                    }
                    resp.on_hover_text(tex.trim_start_matches("assets/"));
                }
            });
        }
    });
    ui.add_space(10.0);
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
